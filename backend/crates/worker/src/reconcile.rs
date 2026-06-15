use anyhow::{Context, Result};
use arbitek_config::AppConfig;
use arbitek_db::{connect_worker as connect, FillRepository, OrderRepository, PositionRepository};
use arbitek_platform_kalshi::execution::KalshiExecutionClient;
use arbitek_platform_polymarket::clob::PolymarketClobClient;
use rust_decimal::Decimal;
use serde_json::Value;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Reconcile submitted orders with platform fills and update positions.
pub async fn run_reconcile(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let order_repo = OrderRepository::new(pool.clone());
    let fill_repo = FillRepository::new(pool.clone());
    let position_repo = PositionRepository::new(pool.clone());

    let poly_client = build_polymarket_clob_client(config);
    let kalshi_client = build_kalshi_client(config);

    info!("starting order reconciliation worker");

    let interval =
        std::time::Duration::from_secs(config.worker.polymarket_poll_interval_seconds.max(5));

    loop {
        info!("reconciling orders");

        // Fetch orders that are still pending or submitted
        let pending = match order_repo.list_by_status("pending", 50).await {
            Ok(o) => o,
            Err(e) => {
                error!("failed to list pending orders: {}", e);
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let submitted = match order_repo.list_by_status("submitted", 50).await {
            Ok(o) => o,
            Err(e) => {
                error!("failed to list submitted orders: {}", e);
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let orders: Vec<_> = pending.into_iter().chain(submitted.into_iter()).collect();

        info!(count = orders.len(), "orders to reconcile");

        for order in &orders {
            match order.platform.as_str() {
                "polymarket" => {
                    if let Some(ref platform_order_id) = order.platform_order_id {
                        match poly_client.get_order(platform_order_id).await {
                            Ok(response) => {
                                if let Err(e) = handle_polymarket_order_response(
                                    order,
                                    &response,
                                    &order_repo,
                                    &fill_repo,
                                    &position_repo,
                                )
                                .await
                                {
                                    warn!(order_id = %order.id, "failed to handle polymarket response: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!(order_id = %order.id, "failed to fetch polymarket order: {}", e);
                            }
                        }
                    } else {
                        warn!(order_id = %order.id, "polymarket order missing platform_order_id");
                    }
                }
                "kalshi" => {
                    if let Some(ref platform_order_id) = order.platform_order_id {
                        if let Some(ref client) = kalshi_client {
                            match client.get_order(platform_order_id).await {
                                Ok(response) => {
                                    if let Err(e) = handle_kalshi_order_response(
                                        order,
                                        &response,
                                        &order_repo,
                                        &fill_repo,
                                        &position_repo,
                                    )
                                    .await
                                    {
                                        warn!(order_id = %order.id, "failed to handle kalshi response: {}", e);
                                    }
                                }
                                Err(e) => {
                                    warn!(order_id = %order.id, "failed to fetch kalshi order: {}", e);
                                }
                            }
                        } else {
                            warn!("kalshi client not configured, skipping kalshi order reconciliation");
                        }
                    } else {
                        warn!(order_id = %order.id, "kalshi order missing platform_order_id");
                    }
                }
                other => {
                    warn!(order_id = %order.id, platform = %other, "unsupported platform for reconciliation");
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
}

async fn handle_polymarket_order_response(
    order: &arbitek_db::OrderRecord,
    response: &Value,
    order_repo: &OrderRepository,
    fill_repo: &FillRepository,
    position_repo: &PositionRepository,
) -> Result<()> {
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let filled_size = response
        .get("filledSize")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<Decimal>().ok())
        .unwrap_or_default();

    let avg_price = response
        .get("avgPrice")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<Decimal>().ok());

    // Update order status
    order_repo
        .update_status(
            order.id,
            status,
            if filled_size > Decimal::ZERO {
                Some(filled_size)
            } else {
                None
            },
            avg_price,
        )
        .await?;

    // If there are fills in the response, insert them
    if let Some(fills) = response.get("fills").and_then(Value::as_array) {
        for fill in fills {
            let fill_id = fill
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned();

            let price = fill
                .get("price")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<Decimal>().ok())
                .unwrap_or_default();

            let size = fill
                .get("size")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<Decimal>().ok())
                .unwrap_or_default();

            let notional = price * size;
            let fee = fill
                .get("fee")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<Decimal>().ok())
                .unwrap_or_default();

            let is_taker = fill.get("taker").and_then(Value::as_bool);

            let filled_at_str = fill.get("timestamp").and_then(Value::as_str).unwrap_or("");

            let filled_at = time::OffsetDateTime::parse(
                filled_at_str,
                &time::format_description::well_known::Rfc3339,
            )
            .unwrap_or_else(|_| time::OffsetDateTime::now_utc());

            fill_repo
                .insert(
                    order.id,
                    "polymarket",
                    &fill_id,
                    order.follower_account_id,
                    order.market_id,
                    None,
                    &order.outcome,
                    &order.side,
                    price,
                    size,
                    notional,
                    fee,
                    is_taker,
                    filled_at,
                    fill,
                )
                .await
                .context("failed to insert fill")?;

            info!(order_id = %order.id, fill_id = %fill_id, "inserted polymarket fill");

            // Update position
            let contracts_delta = match order.side.as_str() {
                "buy" => size,
                "sell" => -size,
                _ => size,
            };

            position_repo
                .upsert(
                    order.follower_account_id,
                    "polymarket",
                    order.market_id.unwrap_or_else(Uuid::nil),
                    None, // condition_id self-healed by the settlement worker
                    None,
                    None,
                    &order.outcome,
                    contracts_delta,
                    price,
                    fee,
                )
                .await
                .context("failed to upsert position")?;
        }
    }

    Ok(())
}

async fn handle_kalshi_order_response(
    order: &arbitek_db::OrderRecord,
    response: &Value,
    order_repo: &OrderRepository,
    fill_repo: &FillRepository,
    position_repo: &PositionRepository,
) -> Result<()> {
    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let filled_count = response
        .get("filled_count")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<Decimal>().ok())
        .unwrap_or_default();

    let avg_price = response
        .get("avg_price")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<Decimal>().ok());

    order_repo
        .update_status(
            order.id,
            status,
            if filled_count > Decimal::ZERO {
                Some(filled_count)
            } else {
                None
            },
            avg_price,
        )
        .await?;

    // Kalshi orders may not have individual fills in the get_order response,
    // but we record the order-level fill info if fully/partially filled.
    if filled_count > Decimal::ZERO {
        let order_id_str = order.platform_order_id.as_deref().unwrap_or("unknown");
        let price = avg_price.unwrap_or_default();
        let notional = price * filled_count;

        // Kalshi returns total fees in the "fees" or "fee" field (in dollars, not cents).
        let fee = response
            .get("fees")
            .or_else(|| response.get("fee"))
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<Decimal>().ok())
                    .or_else(|| v.as_f64().and_then(|f| Decimal::try_from(f).ok()))
            })
            .unwrap_or(Decimal::ZERO);

        fill_repo
            .insert(
                order.id,
                "kalshi",
                order_id_str,
                order.follower_account_id,
                order.market_id,
                None,
                &order.outcome,
                &order.side,
                price,
                filled_count,
                notional,
                fee,
                None,
                time::OffsetDateTime::now_utc(),
                response,
            )
            .await
            .context("failed to insert kalshi fill")?;

        info!(order_id = %order.id, fee = %fee, "inserted kalshi fill");

        let contracts_delta = match order.side.as_str() {
            "buy" => filled_count,
            "sell" => -filled_count,
            _ => filled_count,
        };

        position_repo
            .upsert(
                order.follower_account_id,
                "kalshi",
                order.market_id.unwrap_or_else(Uuid::nil),
                None, // kalshi market identifier resolved via market_mappings
                None,
                None,
                &order.outcome,
                contracts_delta,
                price,
                fee,
            )
            .await
            .context("failed to upsert position")?;
    }

    Ok(())
}

fn build_polymarket_clob_client(config: &AppConfig) -> PolymarketClobClient {
    let mut client = PolymarketClobClient::new(&config.polymarket.clob_api_base_url);

    if let Some(ref key) = config.polymarket.private_key {
        let _ = client.with_private_key(key);
    }

    client
}

fn build_kalshi_client(config: &AppConfig) -> Option<KalshiExecutionClient> {
    match (&config.kalshi.api_key, &config.kalshi.api_secret) {
        (Some(key), Some(secret)) => {
            match arbitek_platform_kalshi::KalshiAuth::from_pem(key.clone(), secret) {
                Ok(auth) => Some(KalshiExecutionClient::new(
                    &config.kalshi.rest_api_base_url,
                    Some(auth),
                )),
                Err(e) => {
                    warn!("failed to build Kalshi auth: {}", e);
                    None
                }
            }
        }
        _ => None,
    }
}
