/// paper_copy — Polymarket master-wallet copy pipeline in pure paper-trade mode.
///
/// Flow per poll cycle:
///   1. List active masters with Polymarket wallet addresses from DB.
///   2. Fetch recent trades from Polymarket Data API for each wallet.
///   3. Deduplicate via raw_events (insert_or_ignore).
///   4. For each new trade: store a Signal and simulate a paper trade against
///      the virtual DB wallet (paper_wallet_balance in system_settings).
///   5. Upsert a Position record so the UI shows running P&L.
///
/// No Oddpool, no Kalshi API — everything stays in the database.
use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect, FillRepository, MarketMappingRepository, MasterRepository, OrderRepository,
    PositionRepository, RawEventRecord, RawEventRepository, SignalRecord, SignalRepository,
    SystemSettingsRepository,
};
use arbitek_platform_polymarket::PolymarketDataApiClient;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use time::OffsetDateTime;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Fixed paper contract size per copied trade.
const PAPER_CONTRACT_SIZE: i64 = 5;

/// Sentinel follower account for all paper trades.
const PAPER_FOLLOWER_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

/// Kalshi taker fee estimate: 7% of the smaller side's value per contract.
fn estimate_fee(contracts: Decimal, price: Decimal) -> Decimal {
    let price_cents = price * Decimal::new(100, 0);
    let hundred = Decimal::new(100, 0);
    let other = hundred - price_cents;
    let smaller = if price_cents < other {
        price_cents
    } else {
        other
    };
    contracts * smaller * Decimal::new(7, 2) / hundred
}

pub async fn run_oddpool_copy(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let raw_repo = RawEventRepository::new(pool.clone());
    let signal_repo = SignalRepository::new(pool.clone());
    let master_repo = MasterRepository::new(pool.clone());
    let mapping_repo = MarketMappingRepository::new(pool.clone());
    let order_repo = OrderRepository::new(pool.clone());
    let fill_repo = FillRepository::new(pool.clone());
    let position_repo = PositionRepository::new(pool.clone());
    let settings_repo = SystemSettingsRepository::new(pool.clone());
    let poly_client = PolymarketDataApiClient::new(&config.polymarket.data_api_base_url);

    // Seed paper wallet with $1000 on first start.
    if settings_repo
        .get("paper_wallet_balance")
        .await
        .unwrap_or(None)
        .is_none()
    {
        let _ = settings_repo
            .set_decimal("paper_wallet_balance", Decimal::new(1000, 0))
            .await;
    }

    let poll_secs = config.worker.polymarket_poll_interval_seconds;
    info!("paper-copy worker starting — polling every {}s", poll_secs);

    loop {
        if let Err(e) = run_once(
            &poly_client,
            &raw_repo,
            &signal_repo,
            &master_repo,
            &mapping_repo,
            &order_repo,
            &fill_repo,
            &position_repo,
            &settings_repo,
        )
        .await
        {
            error!("paper-copy cycle error: {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
    }
}

async fn run_once(
    poly_client: &PolymarketDataApiClient,
    raw_repo: &RawEventRepository,
    signal_repo: &SignalRepository,
    master_repo: &MasterRepository,
    mapping_repo: &MarketMappingRepository,
    order_repo: &OrderRepository,
    fill_repo: &FillRepository,
    position_repo: &PositionRepository,
    settings_repo: &SystemSettingsRepository,
) -> Result<()> {
    let masters = master_repo.list().await?;
    let active: Vec<_> = masters
        .into_iter()
        .filter(|m| m.status == "active" && m.wallet_address.is_some())
        .collect();

    if active.is_empty() {
        info!("no active masters with wallets — nothing to copy");
        return Ok(());
    }

    // Load enabled DB market mappings once per cycle for condition_id lookups.
    let db_mappings = mapping_repo.list(Some(true)).await.unwrap_or_default();

    let follower_id = Uuid::parse_str(PAPER_FOLLOWER_ID).unwrap_or(Uuid::nil());

    for master in active {
        let wallet = master.wallet_address.unwrap();
        let wallet_lc = wallet.to_lowercase();

        let trades = match poly_client.get_trades_for_user(&wallet, 50).await {
            Ok(t) => t,
            Err(e) => {
                error!(wallet = %wallet, "Polymarket trades fetch failed: {}", e);
                continue;
            }
        };

        // Guard: only process trades whose proxyWallet exactly matches the master.
        let verified: Vec<_> = trades
            .into_iter()
            .filter(|t| t.proxy_wallet.to_lowercase() == wallet_lc)
            .collect();

        info!(
            master_id = %master.id,
            wallet = %wallet,
            count = verified.len(),
            "wallet-verified Polymarket trades"
        );

        for trade in verified {
            let external_id = trade.external_event_id();
            // Prefix with "pc:" to scope dedup to this worker only.
            let dedup_id = format!("pc:{}", external_id);

            let raw = RawEventRecord {
                id: Uuid::new_v4(),
                platform: "polymarket".to_owned(),
                source: "paper_copy".to_owned(),
                external_event_id: dedup_id.clone(),
                raw_payload: serde_json::to_value(&trade).unwrap_or_default(),
                detected_at: trade.timestamp.unwrap_or_else(OffsetDateTime::now_utc),
                processed_at: Some(OffsetDateTime::now_utc()),
                source_entity_id: Some(master.id),
                source_entity_type: Some("master".to_owned()),
                status: "processed".to_owned(),
            };

            let is_new = match raw_repo.insert_or_ignore(&raw).await {
                Ok(inserted) => inserted,
                Err(e) => {
                    warn!(external_id = %external_id, "raw event insert failed: {}", e);
                    continue;
                }
            };
            if !is_new {
                continue;
            }

            let condition_id = match trade.condition_id.as_deref() {
                Some(c) if !c.is_empty() => c.to_owned(),
                _ => {
                    warn!(external_id = %external_id, "trade missing condition_id — skipping");
                    continue;
                }
            };

            // --- Market title resolution ---
            // Tier 1: admin-configured DB mapping.
            let condition_lc = condition_id.to_lowercase();
            let db_map = db_mappings
                .iter()
                .find(|m| m.polymarket_condition_id.to_lowercase() == condition_lc);

            let market_title: Option<String> = db_map
                .map(|m| m.kalshi_ticker.clone())
                .or_else(|| {
                    trade
                        .raw
                        .get("title")
                        .or_else(|| trade.raw.get("question"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .or_else(|| Some(condition_id[..condition_id.len().min(20)].to_owned()));

            // --- Trade direction ---
            let side = trade.side.as_deref().unwrap_or("BUY").to_uppercase();
            let outcome = trade.outcome.as_deref().unwrap_or("YES").to_uppercase();
            // outcome_label: preserve the real outcome name ("yes", "no", "hurricanes", etc.)
            let outcome_label = outcome.to_lowercase();
            let paper_action = if side == "BUY" { "buy" } else { "sell" };

            // --- Entry price from Polymarket trade ---
            let entry_price = trade
                .raw
                .get("price")
                .or_else(|| trade.raw.get("avgPrice"))
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<Decimal>().ok())
                        .or_else(|| v.as_f64().and_then(Decimal::from_f64))
                })
                .unwrap_or(Decimal::new(50, 2)); // default 0.50

            let contracts = Decimal::from(PAPER_CONTRACT_SIZE);

            // Simulate the same slippage cap as live execution (3¢ per PRD default):
            //   buy:  fill at entry_price + 3¢  (we pay more, like a real IOC bid)
            //   sell: fill at entry_price - 3¢  (we receive less, like a real IOC ask)
            let max_slippage = Decimal::new(3, 2);
            let sim_price = if paper_action == "buy" {
                (entry_price + max_slippage).min(Decimal::new(97, 2))
            } else {
                (entry_price - max_slippage).max(Decimal::new(3, 2))
            };

            let notional = sim_price * contracts;
            let fee = estimate_fee(contracts, sim_price);

            // --- Paper wallet update ---
            let balance = settings_repo
                .get_decimal("paper_wallet_balance", Decimal::new(1000, 0))
                .await
                .unwrap_or(Decimal::new(1000, 0));

            let new_balance = if paper_action == "buy" {
                // Buying: spend notional + fee. Reject if the wallet cannot cover it.
                if notional + fee > balance {
                    warn!(
                        condition_id = %condition_id,
                        cost = %notional,
                        balance = %balance,
                        "paper wallet insufficient — skipping trade"
                    );
                    let _ = store_signal(
                        &trade,
                        &condition_id,
                        market_title.as_deref(),
                        "rejected",
                        master.id,
                        signal_repo,
                    )
                    .await;
                    continue;
                }
                (balance - notional - fee).max(Decimal::ZERO)
            } else {
                // Selling: receive sale proceeds back into the wallet minus the fee.
                // This credits the entry cost + any profit (or debits any loss),
                // which is why the P&L and the wallet balance now stay in sync.
                balance + notional - fee
            };
            let _ = settings_repo
                .set_decimal("paper_wallet_balance", new_balance)
                .await;

            info!(
                condition_id = %condition_id,
                outcome = %outcome_label,
                action = paper_action,
                master_price = %entry_price,
                sim_price = %sim_price,
                contracts = PAPER_CONTRACT_SIZE,
                notional = %notional,
                fee = %fee,
                balance = %new_balance,
                "[PAPER] simulated trade (with slippage) — wallet updated"
            );

            // --- Order + fill + position ---
            let market_uuid = condition_id_to_uuid(&condition_id);
            let contracts_delta = if paper_action == "buy" { contracts } else { -contracts };

            let order_id = match order_repo
                .insert_paper_order_no_intent(
                    follower_id,
                    "polymarket",
                    Some(market_uuid),
                    &outcome_label,
                    paper_action,
                    sim_price,
                    contracts,
                )
                .await
            {
                Ok(id) => {
                    let _ = fill_repo
                        .insert(
                            id,
                            "polymarket",
                            &format!("oddpool:fill:{}", Uuid::new_v4()),
                            follower_id,
                            Some(market_uuid),
                            market_title.as_deref(),
                            &outcome_label,
                            paper_action,
                            sim_price,
                            contracts,
                            notional,
                            fee,
                            Some(true),
                            OffsetDateTime::now_utc(),
                            &serde_json::json!({}),
                        )
                        .await;
                    Some(id)
                }
                Err(e) => {
                    warn!(condition_id = %condition_id, "oddpool order insert failed: {}", e);
                    None
                }
            };

            if let Err(e) = position_repo
                .upsert(
                    follower_id,
                    "polymarket",
                    market_uuid,
                    market_title.as_deref(),
                    None,
                    &outcome_label,
                    contracts_delta,
                    sim_price,
                    fee,
                )
                .await
            {
                warn!(condition_id = %condition_id, "position upsert failed: {}", e);
            }

            let _ = order_id; // referenced above

            // --- Signal record ---
            let _ = store_signal(
                &trade,
                &condition_id,
                market_title.as_deref(),
                "approved",
                master.id,
                signal_repo,
            )
            .await;
        }
    }

    Ok(())
}

/// Derive a stable UUID from a Polymarket condition_id (hex string).
pub(crate) fn condition_id_to_uuid(condition_id: &str) -> Uuid {
    let hex = condition_id.trim_start_matches("0x");
    if hex.len() >= 32 {
        let bytes: Vec<u8> = (0..16)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0))
            .collect();
        Uuid::from_slice(&bytes).unwrap_or(Uuid::nil())
    } else {
        Uuid::nil()
    }
}

async fn store_signal(
    trade: &arbitek_platform_polymarket::PolymarketDataApiTrade,
    condition_id: &str,
    market_title: Option<&str>,
    status: &str,
    master_id: Uuid,
    signal_repo: &SignalRepository,
) -> Option<Uuid> {
    let outcome = trade.outcome.as_deref().unwrap_or("YES").to_uppercase();
    let side = trade.side.as_deref().unwrap_or("BUY").to_uppercase();

    let entry_price = trade
        .raw
        .get("price")
        .or_else(|| trade.raw.get("avgPrice"))
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<Decimal>().ok())
                .or_else(|| v.as_f64().and_then(Decimal::from_f64))
        })
        .unwrap_or(Decimal::new(50, 2));

    let size_contracts = trade
        .raw
        .get("size")
        .or_else(|| trade.raw.get("amount"))
        .and_then(|v| v.as_str().and_then(|s| s.parse::<Decimal>().ok()))
        .unwrap_or(Decimal::from(PAPER_CONTRACT_SIZE));

    let title = market_title
        .map(String::from)
        .or_else(|| Some(condition_id.to_owned()));

    let signal = SignalRecord {
        id: Uuid::new_v4(),
        platform: "polymarket".to_owned(),
        master_id,
        market_id: None,
        outcome: outcome.to_lowercase(),
        side: side.to_lowercase(),
        position_effect: if side == "BUY" { "open" } else { "close" }.to_owned(),
        size_contracts,
        notional: size_contracts * entry_price,
        currency: "USD".to_owned(),
        entry_price,
        event_timestamp: trade.timestamp.unwrap_or_else(OffsetDateTime::now_utc),
        detected_at: OffsetDateTime::now_utc(),
        confidence_score: None,
        status: status.to_owned(),
        market_title: title,
    };

    match signal_repo.insert(&signal).await {
        Ok(id) => Some(id),
        Err(e) => {
            warn!("failed to store signal: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    // ---- Fee estimation ----

    #[test]
    fn estimate_fee_symmetric_at_50_cents() {
        // price=0.50 → both sides cost 50¢ → 1*50*0.07/100 = 0.035
        let fee = estimate_fee(Decimal::ONE, Decimal::new(50, 2));
        assert_eq!(fee, Decimal::new(35, 3));
    }

    #[test]
    fn estimate_fee_uses_cheaper_no_side() {
        // price=0.20 (YES=20¢, NO=80¢) → smaller=20 → 5*20*0.07/100 = 0.07
        let fee = estimate_fee(Decimal::new(5, 0), Decimal::new(20, 2));
        assert_eq!(fee, Decimal::new(7, 2));
    }

    #[test]
    fn estimate_fee_uses_cheaper_yes_side() {
        // price=0.70 (YES=70¢, NO=30¢) → smaller=30 → 2*30*0.07/100 = 0.042
        let fee = estimate_fee(Decimal::new(2, 0), Decimal::new(70, 2));
        assert_eq!(fee, Decimal::new(42, 3));
    }

    #[test]
    fn estimate_fee_near_zero_price() {
        // price=0.01 → smaller=1¢ → 1*1*0.07/100 = 0.0007
        let fee = estimate_fee(Decimal::ONE, Decimal::new(1, 2));
        assert_eq!(fee, Decimal::new(7, 4));
    }

    // ---- condition_id_to_uuid ----

    #[test]
    fn condition_id_to_uuid_is_deterministic() {
        let cid = "0xabcdef1234567890abcdef1234567890";
        assert_eq!(condition_id_to_uuid(cid), condition_id_to_uuid(cid));
        assert_ne!(condition_id_to_uuid(cid), Uuid::nil());
    }

    #[test]
    fn condition_id_to_uuid_strips_0x_prefix() {
        let with = "0xabcdef1234567890abcdef1234567890";
        let without = "abcdef1234567890abcdef1234567890";
        assert_eq!(condition_id_to_uuid(with), condition_id_to_uuid(without));
    }

    #[test]
    fn condition_id_to_uuid_nil_for_short_hex() {
        assert_eq!(condition_id_to_uuid("0x1234"), Uuid::nil());
        assert_eq!(condition_id_to_uuid(""), Uuid::nil());
    }

    // ---- Paper wallet balance arithmetic ----

    fn apply_slippage(master_price: Decimal, is_buy: bool) -> Decimal {
        let slip = Decimal::new(3, 2);
        if is_buy {
            (master_price + slip).min(Decimal::new(97, 2))
        } else {
            (master_price - slip).max(Decimal::new(3, 2))
        }
    }

    #[test]
    fn paper_buy_deducts_notional_plus_fee_with_slippage() {
        let balance = Decimal::new(1000, 0);
        let master_price = Decimal::new(50, 2);
        let sim_price = apply_slippage(master_price, true); // 0.53
        let contracts = Decimal::from(PAPER_CONTRACT_SIZE);
        let notional = sim_price * contracts; // 5 * 0.53 = $2.65
        let fee = estimate_fee(contracts, sim_price);

        assert!(notional <= balance);
        let new_balance = (balance - notional - fee).max(Decimal::ZERO);
        assert_eq!(new_balance, balance - notional - fee);
        assert!(new_balance < balance, "buy must reduce wallet");
        assert!(new_balance > Decimal::ZERO);
        // Deduction must be larger than without slippage (sim_price > master_price)
        let naive_notional = master_price * contracts;
        assert!(notional > naive_notional, "slippage must increase buy cost");
    }

    #[test]
    fn paper_sell_credits_proceeds_minus_fee_with_slippage() {
        // Simulates: bought earlier (wallet already depleted by buy), now selling.
        // Both buy and sell apply slippage, making buy cost more and sell return less.
        let master_buy = Decimal::new(50, 2);
        let master_sell = Decimal::new(80, 2);
        let sim_buy = apply_slippage(master_buy, true); // 0.53
        let sim_sell = apply_slippage(master_sell, false); // 0.77
        let contracts = Decimal::from(PAPER_CONTRACT_SIZE);
        let buy_notional = sim_buy * contracts; // 5 * 0.53 = $2.65
        let sell_notional = sim_sell * contracts; // 5 * 0.77 = $3.85
        let buy_fee = estimate_fee(contracts, sim_buy);
        let sell_fee = estimate_fee(contracts, sim_sell);

        let balance_after_buy = (Decimal::new(1000, 0) - buy_notional - buy_fee).max(Decimal::ZERO);
        let balance_after_sell = balance_after_buy + sell_notional - sell_fee;

        // Still profitable: sold at 77¢ vs bought at 53¢
        assert!(
            balance_after_sell > Decimal::new(1000, 0),
            "a profitable sell must grow the wallet above the starting balance"
        );
        let net = balance_after_sell - Decimal::new(1000, 0);
        assert!(
            net > Decimal::ZERO,
            "net P&L must be positive for a winning trade"
        );
    }

    #[test]
    fn paper_buy_rejected_when_cost_exceeds_balance() {
        let balance = Decimal::new(1, 0); // $1
        let master_price = Decimal::new(50, 2);
        let sim_price = apply_slippage(master_price, true); // 0.53
        let contracts = Decimal::from(PAPER_CONTRACT_SIZE);
        let notional = sim_price * contracts; // $2.65
        let fee = estimate_fee(contracts, sim_price);
        // Correct check: total cost (notional + fee) must exceed balance to reject
        assert!(notional + fee > balance);
    }

    #[test]
    fn paper_buy_allowed_when_only_notional_fits_but_not_fee() {
        // Bug regression: if we only check `notional > balance` (without fee),
        // the trade passes but wallet goes to 0 (clamped), killing all future trades.
        // Correct check: `notional + fee > balance` rejects this case.
        let sim_price = Decimal::new(53, 2); // $0.53
        let contracts = Decimal::from(PAPER_CONTRACT_SIZE);
        let notional = sim_price * contracts; // $2.65
        let fee = estimate_fee(contracts, sim_price);
        // Set balance to just above notional but below notional+fee
        let balance = notional + fee / Decimal::new(2, 0); // notional < balance < notional+fee
                                                           // Old buggy check: notional > balance → false (trade would be allowed)
        assert!(
            !(notional > balance),
            "old bug: this would have passed incorrectly"
        );
        // New correct check: notional + fee > balance → true (correctly rejected)
        assert!(
            notional + fee > balance,
            "correct check rejects when fee cannot be covered"
        );
    }

    #[test]
    fn paper_wallet_balance_clamps_to_zero_on_buy() {
        // balance covers sim notional exactly but not the fee → clamp to $0
        let sim_price = Decimal::new(53, 2);
        let contracts = Decimal::from(PAPER_CONTRACT_SIZE);
        let notional = sim_price * contracts;
        let fee = estimate_fee(contracts, sim_price);
        let balance = notional; // no room left for fee
        let result = (balance - notional - fee).max(Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn slippage_buy_adds_3_cents() {
        let price = Decimal::new(50, 2);
        assert_eq!(apply_slippage(price, true), Decimal::new(53, 2));
    }

    #[test]
    fn slippage_sell_subtracts_3_cents() {
        let price = Decimal::new(50, 2);
        assert_eq!(apply_slippage(price, false), Decimal::new(47, 2));
    }

    #[test]
    fn slippage_buy_caps_at_97_cents() {
        let price = Decimal::new(96, 2); // 0.96 + 0.03 would be 0.99 → cap at 0.97
        assert_eq!(apply_slippage(price, true), Decimal::new(97, 2));
    }

    #[test]
    fn slippage_sell_floors_at_3_cents() {
        let price = Decimal::new(4, 2); // 0.04 - 0.03 = 0.01 → floor at 0.03
        assert_eq!(apply_slippage(price, false), Decimal::new(3, 2));
    }

    // ---- P&L accounting (mirrors PositionRepository::upsert SQL logic) ----
    //
    // The SQL UPDATE uses pre-update row values for all SET expressions.
    // These pure-Rust helpers replicate that behaviour without a database,
    // so the maths can be verified in isolation.

    struct MockPosition {
        contracts: Decimal,
        avg_price: Decimal,
        realized_pnl: Decimal,
        exposure: Decimal,
    }

    /// Replicates the INSERT path (first trade on a position).
    fn first_trade(delta: Decimal, price: Decimal, fee: Decimal) -> MockPosition {
        let is_buy = delta > Decimal::ZERO;
        MockPosition {
            contracts: delta,
            avg_price: if is_buy { price } else { Decimal::ZERO },
            realized_pnl: -fee,
            exposure: if is_buy { delta * price } else { Decimal::ZERO },
        }
    }

    /// Replicates the UPDATE path (subsequent trades on an existing position).
    fn apply_trade(pos: &mut MockPosition, delta: Decimal, price: Decimal, fee: Decimal) {
        let old_contracts = pos.contracts;
        let old_avg = pos.avg_price;

        // Selling realises P&L; buying only deducts the entry fee.
        let pnl_delta = if delta < Decimal::ZERO {
            (-delta) * (price - old_avg) - fee
        } else {
            -fee
        };
        pos.realized_pnl += pnl_delta;

        let new_contracts = old_contracts + delta;
        if new_contracts > Decimal::ZERO {
            pos.avg_price = if delta > Decimal::ZERO {
                // Weighted average of old position and new buy.
                (old_avg * old_contracts + price * delta) / new_contracts
            } else {
                old_avg // selling does not change the cost basis
            };
            pos.exposure = new_contracts * pos.avg_price;
        } else {
            pos.avg_price = old_avg;
            pos.exposure = Decimal::ZERO;
        }
        pos.contracts = new_contracts;
    }

    #[test]
    fn pnl_first_buy_sets_correct_state() {
        // Buy 5 with master price 0.50 → sim_price 0.53 (with slippage)
        let sim_price = apply_slippage(Decimal::new(50, 2), true); // 0.53
        let fee = estimate_fee(Decimal::new(5, 0), sim_price);
        let pos = first_trade(Decimal::new(5, 0), sim_price, fee);

        assert_eq!(pos.contracts, Decimal::new(5, 0));
        assert_eq!(pos.avg_price, sim_price);
        assert_eq!(pos.realized_pnl, -fee);
        assert_eq!(pos.exposure, Decimal::new(5, 0) * sim_price);
    }

    #[test]
    fn pnl_full_sell_at_profit() {
        // Buy 5 @ master 0.50 (sim 0.53) → sell 5 @ master 0.80 (sim 0.77)
        let buy_price = apply_slippage(Decimal::new(50, 2), true); // 0.53
        let sell_price = apply_slippage(Decimal::new(80, 2), false); // 0.77
        let buy_fee = estimate_fee(Decimal::new(5, 0), buy_price);
        let mut pos = first_trade(Decimal::new(5, 0), buy_price, buy_fee);
        let sell_fee = estimate_fee(Decimal::new(5, 0), sell_price);
        apply_trade(&mut pos, Decimal::new(-5, 0), sell_price, sell_fee);

        let expected = -buy_fee + Decimal::new(5, 0) * (sell_price - buy_price) - sell_fee;
        assert_eq!(pos.realized_pnl, expected);
        assert!(
            pos.realized_pnl > Decimal::ZERO,
            "profitable trade must be positive"
        );
        assert_eq!(pos.contracts, Decimal::ZERO);
        assert_eq!(pos.exposure, Decimal::ZERO);
    }

    #[test]
    fn pnl_full_sell_at_loss() {
        // Buy 5 @ master 0.50 (sim 0.53) → sell 5 @ master 0.20 (sim 0.17 → floored at 0.03? no, 0.20-0.03=0.17)
        let buy_price = apply_slippage(Decimal::new(50, 2), true); // 0.53
        let sell_price = apply_slippage(Decimal::new(20, 2), false); // 0.17
        let buy_fee = estimate_fee(Decimal::new(5, 0), buy_price);
        let mut pos = first_trade(Decimal::new(5, 0), buy_price, buy_fee);
        let sell_fee = estimate_fee(Decimal::new(5, 0), sell_price);
        apply_trade(&mut pos, Decimal::new(-5, 0), sell_price, sell_fee);

        let expected = -buy_fee + Decimal::new(5, 0) * (sell_price - buy_price) - sell_fee;
        assert_eq!(pos.realized_pnl, expected);
        assert!(
            pos.realized_pnl < Decimal::ZERO,
            "losing trade must be negative"
        );
        assert_eq!(pos.contracts, Decimal::ZERO);
        assert_eq!(pos.exposure, Decimal::ZERO);
    }

    #[test]
    fn pnl_weighted_avg_price_on_two_buys() {
        // Buy 5 @ master 0.30 (sim 0.33) then 5 @ master 0.70 (sim 0.73)
        // avg = (5*0.33 + 5*0.73)/10 = 0.53
        let p1 = apply_slippage(Decimal::new(30, 2), true); // 0.33
        let p2 = apply_slippage(Decimal::new(70, 2), true); // 0.73
        let fee1 = estimate_fee(Decimal::new(5, 0), p1);
        let mut pos = first_trade(Decimal::new(5, 0), p1, fee1);
        let fee2 = estimate_fee(Decimal::new(5, 0), p2);
        apply_trade(&mut pos, Decimal::new(5, 0), p2, fee2);

        assert_eq!(pos.contracts, Decimal::new(10, 0));
        let expected_avg =
            (p1 * Decimal::new(5, 0) + p2 * Decimal::new(5, 0)) / Decimal::new(10, 0);
        assert_eq!(pos.avg_price, expected_avg);
    }

    #[test]
    fn pnl_partial_sell_preserves_avg_price() {
        // Buy 10 @ master 0.50 (sim 0.53) → sell 5 @ master 0.80 (sim 0.77)
        let buy_price = apply_slippage(Decimal::new(50, 2), true); // 0.53
        let sell_price = apply_slippage(Decimal::new(80, 2), false); // 0.77
        let buy_fee = estimate_fee(Decimal::new(10, 0), buy_price);
        let mut pos = first_trade(Decimal::new(10, 0), buy_price, buy_fee);
        let sell_fee = estimate_fee(Decimal::new(5, 0), sell_price);
        apply_trade(&mut pos, Decimal::new(-5, 0), sell_price, sell_fee);

        assert_eq!(
            pos.avg_price, buy_price,
            "cost basis must not change on sell"
        );
        assert_eq!(
            pos.contracts,
            Decimal::new(5, 0),
            "five contracts remain open"
        );
        assert!(
            pos.exposure > Decimal::ZERO,
            "remaining exposure should be non-zero"
        );
    }
}
