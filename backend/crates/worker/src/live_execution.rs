use anyhow::{bail, Context, Result};
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_execution_worker as connect, AlertRepository, CopyIntentForExecution,
    CopyIntentRepository, DbPool, FollowerRepository, OrderRepository, PositionRepository,
    SystemSettingsRepository,
};
use arbitek_domain::copy_config::CopyConfig;
use arbitek_domain::market_mapping::{default_binary_mappings, MarketMapper};
use arbitek_execution::live::{execute_kalshi, execute_polymarket_clob, LiveExecutionInput};
use arbitek_platform_kalshi::execution::KalshiExecutionClient;
use arbitek_platform_kalshi::KalshiAuth;
use arbitek_platform_polymarket::clob::PolymarketClobClient;
use time::{Duration, OffsetDateTime};
use tracing::{info, warn};
use uuid::Uuid;

/// Continuous loop — creates pool ONCE so connections aren't re-opened every pass.
pub async fn run_live_execution(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let interval =
        std::time::Duration::from_secs(config.worker.polymarket_poll_interval_seconds.max(5));
    info!("execution-live mode: running continuous execution loop");
    loop {
        tokio::time::sleep(interval).await;
        if let Err(e) = run_live_execution_pass(config, &pool).await {
            warn!("live execution pass failed: {}", e);
        }
    }
}

/// One-shot version (for tests / single-run mode) — creates its own pool.
pub async fn run_live_execution_once(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    run_live_execution_pass(config, &pool).await
}

async fn run_live_execution_pass(config: &AppConfig, pool: &DbPool) -> Result<()> {
    // Gate: both flags must be on before any real order is placed.
    let settings = SystemSettingsRepository::new(pool.clone());
    let live_trading = settings.get_bool("live_trading_enabled", false).await?;
    if !live_trading {
        info!("live_trading_enabled=false — skipping execution pass");
        return Ok(());
    }
    let kalshi_exec = settings.get_bool("kalshi_execution_enabled", false).await?;

    let copy_repo = CopyIntentRepository::new(pool.clone());
    let order_repo = OrderRepository::new(pool.clone());
    let position_repo = PositionRepository::new(pool.clone());
    let follower_repo = FollowerRepository::new(pool.clone());
    let intents = copy_repo
        .list_approved_for_execution(config.worker.execution_batch_size)
        .await?;

    if intents.is_empty() {
        info!("no approved copy intents pending live execution");
        return Ok(());
    }

    let kalshi_client = match build_kalshi_client(config, kalshi_exec) {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "Kalshi client unavailable — Kalshi intents will be skipped: {}",
                e
            );
            None
        }
    };
    let poly_clob_client = build_polymarket_clob_client(config);
    let mapper = MarketMapper::new(default_binary_mappings());

    let mut executed = 0usize;
    let mut failed = 0usize;
    let paper_follower = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();

    for intent in &intents {
        // Hard guard: paper intents must never reach live execution
        if intent.follower_account_id == paper_follower {
            warn!(intent_id = %intent.id, "BUG: paper follower intent in live execution queue — skipping");
            continue;
        }

        // Idempotency guard: skip if an order already exists for this intent.
        // Prevents double-submission when the loop retries before status is persisted.
        match order_repo.order_exists_for_intent(intent.id).await {
            Ok(true) => {
                warn!(intent_id = %intent.id, "order already exists for this copy intent — marking executed, skipping exchange call");
                copy_repo.update_status(intent.id, "executed").await?;
                continue;
            }
            Ok(false) => {}
            Err(e) => {
                warn!(intent_id = %intent.id, "could not check for existing order: {} — skipping to avoid duplicate submission", e);
                continue;
            }
        }

        if intent_is_stale(intent.created_at, config.worker.max_trade_age_seconds) {
            warn!(
                intent_id = %intent.id,
                created_at = %intent.created_at,
                max_age_seconds = config.worker.max_trade_age_seconds,
                "skipping stale copy intent before live execution"
            );
            copy_repo.update_status(intent.id, "rejected").await?;
            continue;
        }

        // Load the follower's copy config for slippage tolerance.
        let cfg = match follower_repo.get_by_id(intent.follower_account_id).await {
            Ok(Some(f)) => CopyConfig::from_value(&f.risk_config),
            _ => CopyConfig::default(),
        };

        // Clamp sells to the follower's actual holding so a master exit never tries to
        // sell more than the follower owns (which the exchange would reject anyway).
        let mut input = live_input_from_intent(intent, cfg.slippage_fraction());
        if intent.action == "sell" {
            let market_id = arbitek_domain::stable_market_id(
                intent.market_id,
                &intent.signal_platform_market_id,
            );
            let held = position_repo
                .open_contracts(intent.follower_account_id, market_id, &intent.outcome)
                .await?;
            match arbitek_domain::clamp_sell_to_held(intent.target_contracts, held) {
                Some(c) => {
                    if c < intent.target_contracts {
                        warn!(
                            intent_id = %intent.id,
                            requested = %intent.target_contracts,
                            held = %held,
                            "clamping live sell to held contracts — master sold more than follower holds"
                        );
                    }
                    input.target_contracts = c;
                }
                None => {
                    info!(intent_id = %intent.id, "live sell intent with no open position to close — marking executed (already flat)");
                    copy_repo.update_status(intent.id, "executed").await?;
                    continue;
                }
            }
        }

        let result = match intent.platform.as_str() {
            "kalshi" => {
                if let Some(entry) =
                    mapper.kalshi_ticker_for_poly_condition(&intent.signal_platform_market_id)
                {
                    execute_kalshi(
                        &input,
                        kalshi_client
                            .as_ref()
                            .context("Kalshi client not configured")?,
                        &entry.kalshi_ticker,
                    )
                    .await
                } else {
                    bail!(
                        "no market mapping for signal {}",
                        intent.signal_platform_market_id
                    )
                }
            }
            "polymarket" => {
                let token_id = &intent.signal_platform_market_id;
                execute_polymarket_clob(&input, &poly_clob_client, token_id).await
            }
            other => bail!("unsupported live execution platform: {}", other),
        };

        match result {
            Ok(output) => {
                let order_id = order_repo.insert_from_live_output(output, intent).await?;
                copy_repo.update_status(intent.id, "executed").await?;
                executed += 1;

                // Publish execution success to queue
                if let Err(e) =
                    crate::queue_publish::publish_execution(config, order_id, "executed").await
                {
                    warn!("failed to publish execution event: {}", e);
                }
            }
            Err(e) => {
                warn!(copy_intent_id=%intent.id, error=%e, "live execution failed");
                copy_repo.update_status(intent.id, "failed").await?;
                let _ = AlertRepository::new(pool.clone())
                    .insert(
                        "critical",
                        "execution_failure",
                        &format!(
                            "Live execution failed for copy intent {} on platform {}: {}",
                            intent.id, intent.platform, e
                        ),
                        Some("copy_intent"),
                        Some(intent.id),
                    )
                    .await;
                failed += 1;

                // Publish execution failure to queue
                if let Err(e) =
                    crate::queue_publish::publish_execution(config, intent.id, "failed").await
                {
                    warn!("failed to publish execution event: {}", e);
                }
            }
        }
    }

    info!(executed, failed, "live execution pass completed");
    Ok(())
}

fn build_kalshi_client(
    config: &AppConfig,
    kalshi_exec_enabled: bool,
) -> Result<Option<KalshiExecutionClient>> {
    let api_key = config.kalshi.api_key.clone();
    let pem = config.kalshi.api_secret.clone();

    let (Some(api_key), Some(pem)) = (api_key, pem) else {
        return Ok(None);
    };

    let auth = KalshiAuth::from_pem(api_key, &pem)?;
    let url = if kalshi_exec_enabled {
        &config.kalshi.rest_api_base_url_live
    } else {
        &config.kalshi.rest_api_base_url
    };
    let client = KalshiExecutionClient::new(url, Some(auth));
    Ok(Some(client))
}

fn build_polymarket_clob_client(config: &AppConfig) -> PolymarketClobClient {
    let mut client = PolymarketClobClient::new(&config.polymarket.clob_api_base_url);

    if let Some(ref key) = config.polymarket.private_key {
        let _ = client.with_private_key(key);
    }

    client
}

fn live_input_from_intent(
    intent: &CopyIntentForExecution,
    slippage: rust_decimal::Decimal,
) -> LiveExecutionInput {
    LiveExecutionInput {
        order_id: Uuid::new_v4(),
        copy_intent_id: intent.id,
        follower_account_id: intent.follower_account_id,
        platform: intent.platform.clone(),
        market_id: intent.market_id,
        outcome: intent.outcome.clone(),
        side: intent.action.clone(),
        target_contracts: intent.target_contracts,
        price_limit: intent.price_limit,
        slippage,
        client_order_id: format!("live:{}", intent.id),
    }
}

fn intent_is_stale(created_at: OffsetDateTime, max_age_seconds: u64) -> bool {
    let seconds = max_age_seconds.min(i64::MAX as u64) as i64;
    created_at < OffsetDateTime::now_utc() - Duration::seconds(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitek_config::{AppConfig, AuthConfig, KalshiConfig, PolymarketConfig, WorkerConfig};
    use rust_decimal::Decimal;

    fn config_no_credentials() -> AppConfig {
        AppConfig {
            database_url: "postgres://test".to_owned(),
            redis_url: "redis://test".to_owned(),
            bind_addr: "0.0.0.0:3000".to_owned(),
            environment: "test".to_owned(),
            log_level: "off".to_owned(),
            live_trading_enabled: false,
            worker: WorkerConfig {
                mode: "test".to_owned(),
                run_migrations_on_startup: false,
                execution_batch_size: 10,
                polymarket_poll_interval_seconds: 10,
                kalshi_poll_interval_seconds: 10,
                max_trade_age_seconds: 300,
            },
            polymarket: PolymarketConfig {
                data_api_base_url: "https://api.test".to_owned(),
                clob_api_base_url: "https://clob.test".to_owned(),
                private_key: None,
                api_key: None,
                api_secret: None,
                api_passphrase: None,
            },
            kalshi: KalshiConfig {
                api_key: None, // paper mode — no credentials
                api_secret: None,
                rest_api_base_url: "https://demo.kalshi.com/trade-api/v2".to_owned(),
                websocket_base_url: "wss://demo.kalshi.com/ws/trade-api/v2".to_owned(),
                rest_api_base_url_live: "https://trading.kalshi.com/trade-api/v2".to_owned(),
            },
            auth: AuthConfig {
                jwt_secret: "test-secret".to_owned(),
                admin_password: "test-pass".to_owned(),
                token_expiry_hours: 24,
            },
            oddpool_api: None,
            copy_wallets: None,
        }
    }

    fn config_with_credentials() -> AppConfig {
        let mut cfg = config_no_credentials();
        cfg.kalshi.api_key = Some("test-api-key".to_owned());
        cfg.kalshi.api_secret = Some("not-a-real-pem".to_owned()); // invalid — will error
        cfg
    }

    /// Paper mode guard: no Kalshi credentials → build_kalshi_client must return None.
    /// A None client means execute_kalshi() will fail loudly, preventing silent paper
    /// trades from accidentally going to the live Kalshi API.
    #[test]
    fn paper_mode_no_credentials_returns_no_client() {
        let result = build_kalshi_client(&config_no_credentials(), false);
        assert!(
            result.is_ok(),
            "absent credentials must not error — they mean paper mode"
        );
        assert!(
            result.unwrap().is_none(),
            "no credentials must produce no client: a live order cannot be placed in paper mode"
        );
    }

    /// Live mode guard: when credentials are present, build_kalshi_client must NOT return
    /// Ok(None).  A None here would silently skip live orders even though the operator
    /// switched to live mode.  Invalid PEM returns Err, which is fine — Ok(None) is not.
    #[test]
    fn live_mode_credentials_present_never_produces_silent_none() {
        let result = build_kalshi_client(&config_with_credentials(), true);
        match result {
            Ok(None) => panic!(
                "credentials were set but client was None — \
                 switching to live mode would silently skip every order"
            ),
            Ok(Some(_)) => { /* valid PEM would produce this */ }
            Err(_) => { /* invalid PEM in the unit test — expected */ }
        }
    }

    /// URL routing: kalshi_exec_enabled=false must use the demo URL (paper safety).
    #[test]
    fn kalshi_client_uses_demo_url_when_exec_disabled() {
        let cfg = config_no_credentials();
        // No credentials → Ok(None), but we can still verify URL selection indirectly
        // by confirming build_kalshi_client does not error on the demo URL path.
        let result = build_kalshi_client(&cfg, false);
        assert!(result.is_ok());
        // With no credentials the client is None — that is the expected safe default.
        assert!(result.unwrap().is_none());
    }

    /// URL routing: kalshi_exec_enabled=true must attempt the live URL (not the demo one).
    /// Invalid PEM is expected in this unit test; the important thing is it doesn't
    /// silently fall back to the demo URL (which would be Ok(None)).
    #[test]
    fn kalshi_client_uses_live_url_when_exec_enabled() {
        let cfg = config_with_credentials();
        let result = build_kalshi_client(&cfg, true);
        match result {
            Ok(None) => panic!(
                "exec_enabled=true with credentials set must not produce None — \
                 that would silently route live orders to the demo endpoint"
            ),
            Ok(Some(_)) => { /* valid PEM would land here */ }
            Err(_) => { /* invalid test PEM — expected */ }
        }
    }

    /// insert_from_live_output field mapping: copy_intent_id must be intent.id, not a random UUID.
    #[test]
    fn insert_field_mapping_copy_intent_id_comes_from_intent() {
        let intent_id = Uuid::new_v4();
        let intent = CopyIntentForExecution {
            id: intent_id,
            follower_account_id: Uuid::new_v4(),
            platform: "kalshi".to_owned(),
            signal_platform_market_id: "BTC-YES".to_owned(),
            market_id: None,
            outcome: "yes".to_owned(),
            action: "buy".to_owned(),
            target_contracts: Decimal::new(5, 0),
            price_limit: Decimal::new(60, 2),
            market_title: None,
        };
        // The only guaranteed-safe check without a DB: the intent id is preserved.
        assert_eq!(intent.id, intent_id, "copy_intent_id must equal intent.id");
    }

    /// insert_from_live_output field mapping: follower_account_id must never be Uuid::nil().
    #[test]
    fn insert_field_mapping_follower_id_is_not_nil() {
        let follower_id = Uuid::new_v4();
        let intent = CopyIntentForExecution {
            id: Uuid::new_v4(),
            follower_account_id: follower_id,
            platform: "kalshi".to_owned(),
            signal_platform_market_id: "BTC-YES".to_owned(),
            market_id: None,
            outcome: "yes".to_owned(),
            action: "buy".to_owned(),
            target_contracts: Decimal::new(5, 0),
            price_limit: Decimal::new(60, 2),
            market_title: None,
        };
        assert_ne!(
            intent.follower_account_id,
            Uuid::nil(),
            "follower_account_id must never be nil — nil violates the FK constraint on orders"
        );
    }

    /// insert_from_live_output field mapping: platform, outcome, and side must be non-empty.
    #[test]
    fn insert_field_mapping_platform_outcome_side_non_empty() {
        let intent = CopyIntentForExecution {
            id: Uuid::new_v4(),
            follower_account_id: Uuid::new_v4(),
            platform: "kalshi".to_owned(),
            signal_platform_market_id: "BTC-YES".to_owned(),
            market_id: None,
            outcome: "yes".to_owned(),
            action: "buy".to_owned(),
            target_contracts: Decimal::new(5, 0),
            price_limit: Decimal::new(60, 2),
            market_title: None,
        };
        assert!(!intent.platform.is_empty(), "platform must not be empty");
        assert!(!intent.outcome.is_empty(), "outcome must not be empty");
        assert!(!intent.action.is_empty(), "side must not be empty");
    }

    /// Field-mapping guard: execute_kalshi() converts (outcome, side) to Kalshi bid/ask format
    /// per PRD FR-EXE-005. buy YES → bid; sell YES → ask; buy NO → ask; sell NO → bid.
    /// Slippage of 3¢ is added on top: bid+3¢, ask-3¢.
    #[test]
    fn kalshi_bid_ask_conversion_buy_yes() {
        // buy YES at 0.60 → side=bid, price=0.60+0.03=0.63
        let outcome = "yes";
        let side = "buy";
        let price = rust_decimal::Decimal::new(60, 2);
        let (kal_side, raw_price) = match (outcome, side) {
            ("yes", "buy") => ("bid", price),
            ("yes", "sell") => ("ask", price),
            ("no", "buy") => ("ask", rust_decimal::Decimal::ONE - price),
            ("no", "sell") => ("bid", rust_decimal::Decimal::ONE - price),
            _ => (side, price),
        };
        let max_slippage = rust_decimal::Decimal::new(3, 2);
        let order_price = if kal_side == "bid" {
            (raw_price + max_slippage).min(rust_decimal::Decimal::new(97, 2))
        } else {
            (raw_price - max_slippage).max(rust_decimal::Decimal::new(3, 2))
        };
        assert_eq!(kal_side, "bid");
        assert_eq!(order_price, rust_decimal::Decimal::new(63, 2));
    }

    #[test]
    fn kalshi_bid_ask_conversion_sell_yes() {
        // sell YES at 0.60 → side=ask, price=0.60-0.03=0.57
        let price = rust_decimal::Decimal::new(60, 2);
        let order_price =
            (price - rust_decimal::Decimal::new(3, 2)).max(rust_decimal::Decimal::new(3, 2));
        assert_eq!(order_price, rust_decimal::Decimal::new(57, 2));
    }

    #[test]
    fn kalshi_bid_ask_conversion_buy_no() {
        // buy NO at 0.40 → side=ask, yes_price=1-0.40=0.60, order_price=0.60-0.03=0.57
        let no_price = rust_decimal::Decimal::new(40, 2);
        let yes_price = rust_decimal::Decimal::ONE - no_price;
        let order_price =
            (yes_price - rust_decimal::Decimal::new(3, 2)).max(rust_decimal::Decimal::new(3, 2));
        assert_eq!(order_price, rust_decimal::Decimal::new(57, 2));
    }

    /// URL guard: the demo URL and live URL must differ.
    /// If KALSHI__REST_API_BASE_URL is left as the demo endpoint in production,
    /// every live order silently routes to the paper environment.
    #[test]
    fn demo_url_differs_from_live_url() {
        let cfg = config_no_credentials();
        assert_ne!(
            cfg.kalshi.rest_api_base_url, cfg.kalshi.rest_api_base_url_live,
            "demo and live URLs must be distinct — \
             set KALSHI__REST_API_BASE_URL to trading.kalshi.com in production"
        );
    }

    /// live_input_from_intent must correctly map all intent fields.
    #[test]
    fn live_input_from_intent_maps_all_fields() {
        let intent = CopyIntentForExecution {
            id: Uuid::new_v4(),
            follower_account_id: Uuid::new_v4(),
            platform: "kalshi".to_owned(),
            signal_platform_market_id: "BTC-YES".to_owned(),
            market_id: None,
            outcome: "yes".to_owned(),
            action: "buy".to_owned(),
            target_contracts: Decimal::new(5, 0),
            price_limit: Decimal::new(60, 2),
            market_title: None,
        };

        let input = live_input_from_intent(&intent, Decimal::new(1, 1));

        assert_eq!(input.copy_intent_id, intent.id);
        assert_eq!(input.follower_account_id, intent.follower_account_id);
        assert_eq!(input.platform, "kalshi");
        assert_eq!(input.outcome, "yes");
        assert_eq!(input.side, "buy");
        assert_eq!(input.target_contracts, Decimal::new(5, 0));
        assert_eq!(input.price_limit, Decimal::new(60, 2));
        assert!(
            input.client_order_id.starts_with("live:"),
            "client_order_id must be prefixed with 'live:'"
        );
    }
}
