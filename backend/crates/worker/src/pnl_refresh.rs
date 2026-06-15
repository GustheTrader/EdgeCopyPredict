use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_worker as connect, AlertRepository, MarketMappingRecord, MarketMappingRepository,
    PositionRepository, RawEventRepository, SystemSettingsRepository,
};
use arbitek_platform_kalshi::KalshiDataApiClient;
use arbitek_platform_polymarket::clob::PolymarketClobClient;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde_json::json;
use std::collections::HashMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

const PAPER_FOLLOWER_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

/// Refresh unrealized PnL for all open positions by fetching current market prices.
pub async fn run_pnl_refresh(config: &AppConfig) -> Result<()> {
    run_pnl_refresh_inner(config, false).await
}

/// Run a single pnl-refresh + settlement pass and return. Used as an on-demand
/// "settle/backfill sweep" (worker mode `pnl-refresh-once`): it backfills durable
/// condition_ids, settles any open position whose market has resolved, and alerts on
/// positions that cannot be settled.
pub async fn run_pnl_refresh_once(config: &AppConfig) -> Result<()> {
    run_pnl_refresh_inner(config, true).await
}

async fn run_pnl_refresh_inner(config: &AppConfig, run_once: bool) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let position_repo = PositionRepository::new(pool.clone());
    let mapping_repo = MarketMappingRepository::new(pool.clone());
    let raw_repo = RawEventRepository::new(pool.clone());
    let alert_repo = AlertRepository::new(pool.clone());
    let settings_repo = SystemSettingsRepository::new(pool.clone());
    let paper_follower_id = Uuid::parse_str(PAPER_FOLLOWER_ID).expect("paper follower UUID");

    let kalshi_client = KalshiDataApiClient::new(&config.kalshi.rest_api_base_url, None);
    let poly_clob_client = PolymarketClobClient::new(&config.polymarket.clob_api_base_url);

    let interval = std::time::Duration::from_millis(config.worker.pnl_refresh_interval_ms.max(250));

    info!("starting pnl refresh worker");

    loop {
        let positions = match position_repo.list_open(500).await {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "failed to list open positions");
                if run_once {
                    return Err(e);
                }
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        if positions.is_empty() {
            if run_once {
                info!("no open positions to settle");
                return Ok(());
            }
            tokio::time::sleep(interval).await;
            continue;
        }

        // Fetch all market mappings once, build a lookup map — avoids N individual DB queries.
        let mappings: HashMap<Uuid, MarketMappingRecord> = match mapping_repo.list(None).await {
            Ok(ms) => ms.into_iter().map(|m| (m.id, m)).collect(),
            Err(e) => {
                error!(error = %e, "failed to fetch market mappings");
                if run_once {
                    return Err(e);
                }
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let mapped_count = positions
            .iter()
            .filter(|p| mappings.contains_key(&p.market_id))
            .count();
        debug!(
            total = positions.len(),
            mapped = mapped_count,
            "refreshing unrealized pnl"
        );

        for pos in &positions {
            // Resolve the Polymarket condition_id, most-durable source first:
            //   1. the platform_market_id stored on the position (set at execution time
            //      or self-healed on a previous pass) — survives raw_events pruning;
            //   2. a market_mapping entry;
            //   3. recovery from raw_events (then persisted back so it never needs the
            //      raw_event again).
            let poly_condition_id: Option<String> = if pos.platform == "polymarket" {
                if let Some(cid) = pos
                    .platform_market_id
                    .clone()
                    .filter(|c| c.starts_with("0x"))
                {
                    Some(cid)
                } else if let Some(m) = mappings.get(&pos.market_id) {
                    Some(m.polymarket_condition_id.clone())
                } else {
                    match raw_repo
                        .find_condition_id_by_market_uuid(pos.market_id)
                        .await
                    {
                        Ok(Some(cid)) => {
                            // Self-heal: persist so future passes don't depend on raw_events.
                            if let Err(e) = position_repo.set_platform_market_id(pos.id, &cid).await
                            {
                                warn!(position_id = %pos.id, error = %e, "failed to persist recovered condition_id");
                            }
                            Some(cid)
                        }
                        Ok(None) => None,
                        Err(e) => {
                            warn!(position_id = %pos.id, error = %e, "failed to look up condition_id");
                            None
                        }
                    }
                }
            } else if let Some(m) = mappings.get(&pos.market_id) {
                Some(m.polymarket_condition_id.clone())
            } else {
                None
            };

            // Never silently skip an open Polymarket position we cannot settle — raise a
            // (deduplicated) alert so the operator sees it instead of it sitting open forever.
            if pos.platform == "polymarket" && poly_condition_id.is_none() {
                let message = format!(
                    "Open Polymarket position {} ({} contracts, outcome '{}', market '{}') cannot be settled: \
                     no condition_id is recoverable (missing platform_market_id, market mapping, and raw_event). \
                     It will remain open until resolved manually.",
                    pos.id,
                    pos.contracts,
                    pos.outcome,
                    pos.market_title.as_deref().unwrap_or("unknown"),
                );
                match alert_repo
                    .insert_if_absent(
                        "critical",
                        "unsettleable_position",
                        &message,
                        Some("position"),
                        Some(pos.id),
                    )
                    .await
                {
                    Ok(true) => {
                        warn!(position_id = %pos.id, "raised unsettleable_position alert — cannot recover condition_id")
                    }
                    Ok(false) => {}
                    Err(e) => {
                        warn!(position_id = %pos.id, error = %e, "failed to raise unsettleable_position alert")
                    }
                }
                continue;
            }

            let current_price = match pos.platform.as_str() {
                "kalshi" => {
                    let mapping = match mappings.get(&pos.market_id) {
                        Some(m) => m,
                        None => continue,
                    };
                    let ticker = &mapping.kalshi_ticker;
                    match kalshi_client.get_market_price(ticker).await {
                        Ok(Some(price)) => Some(price),
                        Ok(None) => {
                            warn!(ticker, "kalshi market price not available");
                            None
                        }
                        Err(e) => {
                            warn!(ticker, error = %e, "failed to fetch kalshi market price");
                            None
                        }
                    }
                }
                "polymarket" => {
                    let condition_id = match poly_condition_id.as_deref() {
                        Some(c) => c.to_owned(),
                        None => continue,
                    };
                    match poly_clob_client
                        .get_market_by_condition(&condition_id)
                        .await
                    {
                        Ok(Some(market_info)) => {
                            if is_closed_market(&market_info) {
                                if let Some(price) = extract_token_price(&market_info, &pos.outcome)
                                {
                                    let close_time = extract_close_time(&market_info);
                                    match position_repo
                                        .settle_resolved_position(pos.id, price, close_time)
                                        .await
                                    {
                                        Ok(Some(settlement)) => {
                                            if settlement.follower_account_id == paper_follower_id {
                                                let payload = json!({
                                                    "position_id": pos.id,
                                                    "market_id": pos.market_id,
                                                    "market_title": pos.market_title.as_deref(),
                                                    "outcome": pos.outcome.as_str(),
                                                    "contracts": pos.contracts,
                                                    "settlement_price": price,
                                                    "settlement_cash": settlement.settlement_cash
                                                });
                                                match settings_repo
                                                    .record_paper_wallet_movement(
                                                        settlement.follower_account_id,
                                                        "settlement",
                                                        settlement.settlement_cash,
                                                        Some("position"),
                                                        Some(pos.id),
                                                        Some("Paper settlement credit"),
                                                        &payload,
                                                        Decimal::new(1000, 0),
                                                    )
                                                    .await
                                                {
                                                    Ok(movement) => {
                                                        info!(
                                                            position_id = %pos.id,
                                                            settlement_cash = %settlement.settlement_cash,
                                                            paper_wallet_balance = %movement.balance_after,
                                                            "credited paper settlement cash"
                                                        );
                                                    }
                                                    Err(e) => {
                                                        warn!(
                                                            position_id = %pos.id,
                                                            settlement_cash = %settlement.settlement_cash,
                                                            error = %e,
                                                            "failed to credit paper settlement cash"
                                                        );
                                                    }
                                                }
                                            }
                                            info!(
                                                position_id = %pos.id,
                                                price = %price,
                                                "settled closed polymarket position"
                                            );
                                        }
                                        Ok(None) => {}
                                        Err(e) => {
                                            warn!(
                                                position_id = %pos.id,
                                                error = %e,
                                                "failed to settle closed polymarket position"
                                            );
                                        }
                                    }
                                } else {
                                    warn!(
                                        condition_id,
                                        outcome = %pos.outcome,
                                        "closed polymarket market has no settlement price"
                                    );
                                    let message = format!(
                                        "Polymarket market for position {} (outcome '{}') is closed but \
                                         exposes no settlement price for that outcome — cannot auto-settle.",
                                        pos.id, pos.outcome,
                                    );
                                    let _ = alert_repo
                                        .insert_if_absent(
                                            "critical",
                                            "settlement_price_missing",
                                            &message,
                                            Some("position"),
                                            Some(pos.id),
                                        )
                                        .await;
                                }
                                None
                            } else if let Some(price) =
                                extract_token_price(&market_info, &pos.outcome)
                            {
                                Some(price)
                            } else {
                                let token_id = extract_token_id(&market_info, &pos.outcome);
                                match token_id {
                                    Some(tid) => match poly_clob_client.get_mid_price(&tid).await {
                                        Ok(Some(price)) => Some(price),
                                        Ok(None) => {
                                            warn!(condition_id, outcome = %pos.outcome, "polymarket mid price not available");
                                            None
                                        }
                                        Err(e) => {
                                            warn!(condition_id, error = %e, "failed to fetch polymarket mid price");
                                            None
                                        }
                                    },
                                    None => {
                                        warn!(condition_id, outcome = %pos.outcome, "token id not found for outcome");
                                        None
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            warn!(condition_id, "polymarket market info not found");
                            None
                        }
                        Err(e) => {
                            warn!(condition_id, error = %e, "failed to fetch polymarket market info");
                            None
                        }
                    }
                }
                other => {
                    warn!(platform = %other, "unsupported platform for pnl refresh");
                    None
                }
            };

            if let Some(price) = current_price {
                match position_repo.refresh_unrealized_pnl(pos.id, price).await {
                    Ok(true) => {
                        debug!(position_id = %pos.id, price = %price, "refreshed unrealized pnl");
                    }
                    Ok(false) => {}
                    Err(e) => {
                        warn!(position_id = %pos.id, error = %e, "failed to refresh unrealized pnl");
                    }
                }
            }
        }

        if run_once {
            info!("settlement sweep complete");
            return Ok(());
        }
        tokio::time::sleep(interval).await;
    }
}

fn is_closed_market(market_info: &serde_json::Value) -> bool {
    market_info
        .get("closed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn extract_close_time(market_info: &serde_json::Value) -> Option<OffsetDateTime> {
    market_info
        .get("end_date_iso")
        .or_else(|| market_info.get("endDateIso"))
        .and_then(|v| v.as_str())
        .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
}

fn extract_token_id(market_info: &serde_json::Value, outcome: &str) -> Option<String> {
    market_info
        .get("tokens")
        .or_else(|| market_info.get("outcomes"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find(|token| {
                token
                    .get("outcome")
                    .and_then(|o| o.as_str())
                    .map(|s| s.eq_ignore_ascii_case(outcome))
                    .unwrap_or(false)
            })
        })
        .and_then(|token| {
            token
                .get("token_id")
                .or_else(|| token.get("asset_id"))
                .and_then(|v| v.as_str().map(String::from))
        })
}

fn extract_token_price(market_info: &serde_json::Value, outcome: &str) -> Option<Decimal> {
    market_info
        .get("tokens")
        .or_else(|| market_info.get("outcomes"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find(|token| {
                token
                    .get("outcome")
                    .and_then(|o| o.as_str())
                    .map(|s| s.eq_ignore_ascii_case(outcome))
                    .unwrap_or(false)
            })
        })
        .and_then(|token| token.get("price").or_else(|| token.get("Price")))
        .and_then(|price| {
            price
                .as_str()
                .and_then(|s| s.parse::<Decimal>().ok())
                .or_else(|| price.as_f64().and_then(Decimal::from_f64))
        })
}
