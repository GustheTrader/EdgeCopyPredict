use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_worker as connect, MarketMappingRecord, MarketMappingRepository, PositionRepository,
    RawEventRepository,
};
use arbitek_platform_kalshi::KalshiDataApiClient;
use arbitek_platform_polymarket::{clob::PolymarketClobClient, PolymarketDataApiClient};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Refresh unrealized PnL for all open positions by fetching current market prices.
pub async fn run_pnl_refresh(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let position_repo = PositionRepository::new(pool.clone());
    let mapping_repo = MarketMappingRepository::new(pool.clone());
    let raw_repo = RawEventRepository::new(pool.clone());

    let kalshi_client = KalshiDataApiClient::new(&config.kalshi.rest_api_base_url, None);
    let poly_data_client = PolymarketDataApiClient::new(&config.polymarket.data_api_base_url);
    let poly_clob_client = PolymarketClobClient::new(&config.polymarket.clob_api_base_url);

    let interval =
        std::time::Duration::from_secs(config.worker.polymarket_poll_interval_seconds.max(10));

    info!("starting pnl refresh worker");

    loop {
        let positions = match position_repo.list_open(500).await {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "failed to list open positions");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        if positions.is_empty() {
            tokio::time::sleep(interval).await;
            continue;
        }

        // Fetch all market mappings once, build a lookup map — avoids N individual DB queries.
        let mappings: HashMap<Uuid, MarketMappingRecord> = match mapping_repo.list(None).await {
            Ok(ms) => ms.into_iter().map(|m| (m.id, m)).collect(),
            Err(e) => {
                error!(error = %e, "failed to fetch market mappings");
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let mapped_count = positions
            .iter()
            .filter(|p| mappings.contains_key(&p.market_id))
            .count();
        info!(
            total = positions.len(),
            mapped = mapped_count,
            "refreshing unrealized pnl"
        );

        for pos in &positions {
            // Resolve condition_id: either from a market mapping or by recovering it
            // from raw_events for unmapped positions (no market_mappings entry).
            let poly_condition_id: Option<String> = if let Some(m) = mappings.get(&pos.market_id) {
                Some(m.polymarket_condition_id.clone())
            } else if pos.platform == "polymarket" {
                match raw_repo
                    .find_condition_id_by_market_uuid(pos.market_id)
                    .await
                {
                    Ok(Some(cid)) => Some(cid),
                    Ok(None) => {
                        warn!(market_id = %pos.market_id, outcome = %pos.outcome, "no condition_id in raw_events for this market — skipping pnl refresh");
                        None
                    }
                    Err(e) => {
                        warn!(position_id = %pos.id, error = %e, "failed to look up condition_id");
                        None
                    }
                }
            } else {
                None
            };

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
                    match poly_data_client
                        .get_market_by_condition(&condition_id)
                        .await
                    {
                        Ok(Some(market_info)) => {
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
                if let Err(e) = position_repo.refresh_unrealized_pnl(pos.id, price).await {
                    warn!(position_id = %pos.id, error = %e, "failed to refresh unrealized pnl");
                } else {
                    info!(position_id = %pos.id, price = %price, "refreshed unrealized pnl");
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
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
