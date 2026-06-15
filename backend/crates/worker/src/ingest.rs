use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{connect_worker as connect, MasterRepository, RawEventRecord, RawEventRepository};
use arbitek_platform_kalshi::{KalshiAuth, KalshiDataApiClient, KalshiDataApiTrade};
#[allow(unused_imports)]
use arbitek_platform_polymarket::{
    clob::PolymarketClobClient, PolymarketDataApiClient, PolymarketDataApiTrade,
};
use serde_json;
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::{error, info, warn};
use uuid::Uuid;

const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 1_000;
const POLYMARKET_TRADE_FETCH_LIMIT: u16 = 25;

/// Poll Polymarket Data API for new trades and store them as raw events.
pub async fn run_polymarket_ingest(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let raw_repo = RawEventRepository::new(pool.clone());
    let master_repo = MasterRepository::new(pool.clone());
    let client = PolymarketDataApiClient::new(&config.polymarket.data_api_base_url);
    let market_client = PolymarketClobClient::new(&config.polymarket.clob_api_base_url);

    info!("starting polymarket data api ingestion");

    // Cache condition_id -> (market title, end_date_iso) across poll cycles to avoid
    // repeated /markets lookups. Both are injected into the raw payload so the signal
    // normalizer can label signals and enforce the follower's max_days_out filter.
    let mut market_meta_cache: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

    loop {
        info!("polling polymarket trades");

        match master_repo.list().await {
            Ok(masters) => {
                let active_masters: Vec<_> = masters
                    .into_iter()
                    .filter(|m| m.status == "active")
                    .filter(|m| m.wallet_address.is_some())
                    .collect();

                info!(count = active_masters.len(), "active masters with wallet");

                for master in active_masters {
                    let wallet = master.wallet_address.unwrap();
                    let trades = match retry_with_backoff(
                        || async {
                            client
                                .get_trades_for_user(&wallet, POLYMARKET_TRADE_FETCH_LIMIT)
                                .await
                        },
                        &format!("polymarket trades for {}", wallet),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(e) => {
                            error!(
                                master_id = %master.id,
                                wallet = %wallet,
                                "failed to fetch trades from Polymarket Data API after retries: {}",
                                e
                            );
                            continue;
                        }
                    };

                    let wallet_lc = wallet.to_lowercase();
                    let total = trades.len();
                    let verified: Vec<_> = trades
                        .into_iter()
                        .filter(|t| t.proxy_wallet.to_lowercase() == wallet_lc)
                        .collect();

                    if verified.len() < total {
                        warn!(
                            master_id = %master.id,
                            wallet = %wallet,
                            returned = total,
                            matched = verified.len(),
                            "Polymarket API returned trades from other wallets — filtered out"
                        );
                    }

                    info!(
                        master_id = %master.id,
                        wallet = %wallet,
                        trade_count = verified.len(),
                        "fetched trades (wallet-verified)"
                    );

                    for trade in verified {
                        // Skip historical trades — only act on recent ones so the
                        // back-fill isn't copied as if the master just traded.
                        if trade_is_stale(trade.timestamp, config.worker.max_trade_age_seconds) {
                            continue;
                        }

                        // Fetch market title + end date from Polymarket /markets API if not cached.
                        let (injected_title, injected_end_date): (Option<String>, Option<String>) =
                            if let Some(cond_id) = trade.condition_id.as_deref() {
                                if let Some(cached) = market_meta_cache.get(cond_id) {
                                    cached.clone()
                                } else {
                                    match market_client.get_market_by_condition(cond_id).await {
                                        Ok(Some(market_info)) => {
                                            let title = market_info
                                                .get("question")
                                                .or_else(|| market_info.get("title"))
                                                .and_then(|v| v.as_str())
                                                .map(String::from);
                                            let end_date = market_info
                                                .get("end_date_iso")
                                                .or_else(|| market_info.get("endDateIso"))
                                                .and_then(|v| v.as_str())
                                                .map(String::from);
                                            market_meta_cache.insert(
                                                cond_id.to_owned(),
                                                (title.clone(), end_date.clone()),
                                            );
                                            (title, end_date)
                                        }
                                        Ok(None) => (None, None),
                                        Err(e) => {
                                            warn!(condition_id = %cond_id, "failed to fetch market info: {}", e);
                                            (None, None)
                                        }
                                    }
                                }
                            } else {
                                (None, None)
                            };

                        let mut event = trade_to_raw_event(&trade, master.id);

                        // Inject title + end date into the nested raw JSON so the signal
                        // normalizer can pick them up via trade.raw.get(...).
                        if let Some(raw_obj) = event
                            .raw_payload
                            .get_mut("raw")
                            .and_then(|v| v.as_object_mut())
                        {
                            if let Some(title) = injected_title {
                                raw_obj
                                    .insert("title".to_owned(), serde_json::Value::String(title));
                            }
                            if let Some(end_date) = injected_end_date {
                                raw_obj.insert(
                                    "end_date_iso".to_owned(),
                                    serde_json::Value::String(end_date),
                                );
                            }
                        }

                        let external_id = event.external_event_id.clone();
                        match raw_repo.insert_or_ignore(&event).await {
                            Ok(inserted) => {
                                if inserted {
                                    info!(
                                        external_event_id = %external_id,
                                        "inserted new raw event"
                                    );
                                    if let Err(e) = crate::queue_publish::publish_raw_event(
                                        config,
                                        "polymarket",
                                        &external_id,
                                    )
                                    .await
                                    {
                                        warn!("failed to publish raw event to queue: {}", e);
                                    }
                                } else {
                                    info!(
                                        external_event_id = %external_id,
                                        "raw event already exists (duplicate)"
                                    );
                                }
                            }
                            Err(e) => {
                                error!(external_event_id = %external_id, "failed to insert raw event: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("failed to list masters: {}", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(
            config.worker.polymarket_poll_interval_seconds,
        ))
        .await;
    }
}

/// Poll Kalshi REST API for our own account's orders and store them as raw events.
/// This fetches the authenticated account's portfolio — not per-master wallet addresses
/// (Kalshi doesn't have Ethereum wallets; masters are Polymarket-only).
pub async fn run_kalshi_ingest(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let raw_repo = RawEventRepository::new(pool.clone());

    let auth = match (&config.kalshi.api_key, &config.kalshi.api_secret) {
        (Some(key), Some(secret)) => match KalshiAuth::from_pem(key.clone(), secret) {
            Ok(a) => Some(a),
            Err(e) => {
                warn!("failed to build Kalshi auth: {}", e);
                None
            }
        },
        _ => {
            warn!("no Kalshi credentials configured — kalshi ingest disabled");
            return Ok(());
        }
    };

    let client = KalshiDataApiClient::new(&config.kalshi.rest_api_base_url, auth);

    // Use a fixed sentinel ID for our own Kalshi account (the follower/slave).
    let own_account_id =
        uuid::Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap_or_default();

    info!("starting kalshi rest api ingestion (own account)");

    loop {
        info!("polling kalshi own-account orders");

        match retry_with_backoff(
            || async { client.get_trades_for_user("self", 100).await },
            "kalshi own-account orders",
        )
        .await
        {
            Ok(trades) => {
                info!(
                    trade_count = trades.len(),
                    "fetched kalshi own-account orders"
                );
                for trade in trades {
                    // Skip historical trades — only act on recent ones.
                    if trade_is_stale(trade.timestamp, config.worker.max_trade_age_seconds) {
                        continue;
                    }
                    let event = kalshi_trade_to_raw_event(&trade, own_account_id);
                    let external_id = event.external_event_id.clone();
                    match raw_repo.insert_or_ignore(&event).await {
                        Ok(inserted) => {
                            if inserted {
                                info!(external_event_id = %external_id, "inserted new kalshi raw event");
                                if let Err(e) = crate::queue_publish::publish_raw_event(
                                    config,
                                    "kalshi",
                                    &external_id,
                                )
                                .await
                                {
                                    warn!("failed to publish raw event to queue: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!(external_event_id = %external_id, "failed to insert kalshi raw event: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "failed to fetch kalshi own-account orders after retries: {}",
                    e
                );
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(
            config.worker.kalshi_poll_interval_seconds,
        ))
        .await;
    }
}

/// Convert a Polymarket trade to a raw event record.
fn trade_to_raw_event(trade: &PolymarketDataApiTrade, master_id: Uuid) -> RawEventRecord {
    RawEventRecord {
        id: Uuid::new_v4(),
        platform: "polymarket".to_owned(),
        source: "data_api".to_owned(),
        external_event_id: trade.external_event_id(),
        raw_payload: serde_json::to_value(trade).unwrap_or_default(),
        // When *we* observed the trade. The master's on-platform trade time is
        // preserved in the raw payload (trade.timestamp) and surfaced as the
        // signal's event_timestamp; keep these two clocks distinct.
        detected_at: OffsetDateTime::now_utc(),
        processed_at: None,
        source_entity_id: Some(master_id),
        source_entity_type: Some("master".to_owned()),
        status: "new".to_owned(),
    }
}

/// Convert a Kalshi trade to a raw event record.
fn kalshi_trade_to_raw_event(trade: &KalshiDataApiTrade, master_id: Uuid) -> RawEventRecord {
    RawEventRecord {
        id: Uuid::new_v4(),
        platform: "kalshi".to_owned(),
        source: "data_api".to_owned(),
        external_event_id: trade.external_event_id(),
        raw_payload: serde_json::to_value(trade).unwrap_or_default(),
        // When *we* observed the trade — see trade_to_raw_event for rationale.
        detected_at: OffsetDateTime::now_utc(),
        processed_at: None,
        source_entity_id: Some(master_id),
        source_entity_type: Some("master".to_owned()),
        status: "new".to_owned(),
    }
}

/// Returns true if a trade's on-platform timestamp is older than `max_age_seconds`
/// and should be skipped during ingest. This prevents the historical back-fill
/// (the API returns the last N trades regardless of age) from being replayed as
/// live signals. Trades without a timestamp are treated as fresh (not skipped).
fn trade_is_stale(timestamp: Option<OffsetDateTime>, max_age_seconds: u64) -> bool {
    match timestamp {
        Some(ts) => {
            (OffsetDateTime::now_utc() - ts) > time::Duration::seconds(max_age_seconds as i64)
        }
        None => false,
    }
}

/// Retry an async operation with exponential backoff.
async fn retry_with_backoff<T, F, Fut>(mut operation: F, description: &str) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_err = Some(e);
                let delay = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
                warn!(
                    attempt = attempt + 1,
                    max_retries = MAX_RETRIES,
                    delay_ms = delay,
                    "{} failed, retrying...",
                    description
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
    Err(last_err.unwrap().context(format!(
        "{} failed after {} retries",
        description, MAX_RETRIES
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitek_platform_kalshi::KalshiDataApiTrade;
    use arbitek_platform_polymarket::PolymarketDataApiTrade;
    use serde_json::json;

    #[test]
    fn polymarket_trade_to_raw_event_maps_fields() {
        let trade = PolymarketDataApiTrade::from_raw(json!({
            "proxyWallet": "0xabc",
            "transactionHash": "0xtx",
            "asset": "123",
            "conditionId": "0xcondition",
            "side": "BUY",
            "outcome": "YES",
            "timestamp": 1770000000
        }))
        .unwrap();

        let master_id = Uuid::new_v4();
        let event = trade_to_raw_event(&trade, master_id);

        assert_eq!(event.platform, "polymarket");
        assert_eq!(event.source, "data_api");
        assert_eq!(event.external_event_id, "0xtx:123:BUY:YES");
        assert_eq!(event.source_entity_id, Some(master_id));
        assert_eq!(event.source_entity_type, Some("master".to_owned()));
        assert_eq!(event.status, "new");
    }

    #[test]
    fn kalshi_trade_to_raw_event_maps_fields() {
        let trade = KalshiDataApiTrade::from_raw(json!({
            "order_id": "ord-123",
            "user_id": "user-abc",
            "market_ticker": "BTC-YES",
            "side": "yes",
            "action": "buy",
            "count": "10",
            "price": "0.55",
            "status": "filled",
            "created_time": 1770000000
        }))
        .unwrap();

        let master_id = Uuid::new_v4();
        let event = kalshi_trade_to_raw_event(&trade, master_id);

        assert_eq!(event.platform, "kalshi");
        assert_eq!(event.source, "data_api");
        assert_eq!(event.external_event_id, "ord-123:yes:buy");
        assert_eq!(event.source_entity_id, Some(master_id));
        assert_eq!(event.source_entity_type, Some("master".to_owned()));
        assert_eq!(event.status, "new");
    }

    #[test]
    fn trade_is_stale_filters_old_trades() {
        let now = OffsetDateTime::now_utc();
        // 10 minutes old, 5-minute window → stale.
        assert!(trade_is_stale(Some(now - time::Duration::minutes(10)), 300));
        // 1 minute old, 5-minute window → fresh.
        assert!(!trade_is_stale(Some(now - time::Duration::minutes(1)), 300));
        // Missing timestamp → treated as fresh.
        assert!(!trade_is_stale(None, 300));
    }

    #[tokio::test]
    async fn retry_with_backoff_succeeds_on_first_try() {
        let result = retry_with_backoff(|| async { Ok::<i32, anyhow::Error>(42) }, "test op").await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn retry_with_backoff_succeeds_after_failure() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts2 = Arc::clone(&attempts);
        let result = retry_with_backoff(
            move || {
                let attempts = Arc::clone(&attempts2);
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst);
                    if count < 1 {
                        Err::<i32, anyhow::Error>(anyhow::anyhow!("transient failure"))
                    } else {
                        Ok(42)
                    }
                }
            },
            "test op",
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retry_with_backoff_fails_after_max_retries() {
        let result = retry_with_backoff(
            || async { Err::<i32, anyhow::Error>(anyhow::anyhow!("persistent failure")) },
            "test op",
        )
        .await;

        assert!(result.is_err());
    }
}
