use anyhow::{Context, Result};
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_worker as connect, CopyIntentRepository, FollowerRepository, MasterRepository,
    RawEventRecord, RawEventRepository, SignalRecord, SignalRepository,
};
use arbitek_domain::market_mapping::{default_binary_mappings, MarketMapper};
use arbitek_platform_kalshi::KalshiDataApiTrade;
use arbitek_platform_polymarket::PolymarketDataApiTrade;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use tracing::{info, warn};
use uuid::Uuid;

/// Normalize raw events into signals and create copy intents.
pub async fn run_signal_normalize(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let raw_repo = RawEventRepository::new(pool.clone());
    let signal_repo = SignalRepository::new(pool.clone());
    let follower_repo = FollowerRepository::new(pool.clone());
    let master_repo = MasterRepository::new(pool.clone());
    let copy_repo = CopyIntentRepository::new(pool.clone());
    let mapper = MarketMapper::new(default_binary_mappings());

    info!("starting signal normalization worker");

    loop {
        let events = raw_repo.list_unprocessed(100).await?;
        if events.is_empty() {
            info!("no new raw events pending");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            continue;
        }

        info!(count = events.len(), "normalizing raw events");

        for event in &events {
            match normalize_event_internal(
                event,
                &mapper,
                &signal_repo,
                &follower_repo,
                &master_repo,
                &copy_repo,
                config,
            )
            .await
            {
                Ok(Some(signal_id)) => {
                    if let Err(e) = raw_repo.mark_processed(event.id).await {
                        warn!(event_id = %event.id, error = %e, "failed to mark event processed");
                    } else {
                        info!(event_id = %event.id, signal_id = %signal_id, "normalized event to signal");
                    }
                }
                Ok(None) => {
                    if let Err(e) = raw_repo.mark_processed(event.id).await {
                        warn!(event_id = %event.id, error = %e, "failed to mark event processed");
                    } else {
                        info!(event_id = %event.id, "skipped event - no matching market mapping, wallet mismatch, or unsupported platform");
                    }
                }
                Err(e) => {
                    warn!(event_id = %event.id, error = %e, "failed to normalize event");
                }
            }
        }
    }
}

pub async fn normalize_single_event(
    event: &RawEventRecord,
    mapper: &MarketMapper,
    signal_repo: &SignalRepository,
    follower_repo: &FollowerRepository,
    master_repo: &MasterRepository,
    copy_repo: &CopyIntentRepository,
    config: &AppConfig,
) -> Result<Option<Uuid>> {
    normalize_event_internal(
        event,
        mapper,
        signal_repo,
        follower_repo,
        master_repo,
        copy_repo,
        config,
    )
    .await
}

async fn normalize_event_internal(
    event: &RawEventRecord,
    mapper: &MarketMapper,
    signal_repo: &SignalRepository,
    follower_repo: &FollowerRepository,
    master_repo: &MasterRepository,
    copy_repo: &CopyIntentRepository,
    config: &AppConfig,
) -> Result<Option<Uuid>> {
    match event.platform.as_str() {
        "polymarket" => {
            normalize_polymarket_event(
                event,
                mapper,
                signal_repo,
                follower_repo,
                master_repo,
                copy_repo,
                config,
            )
            .await
        }
        "kalshi" => {
            normalize_kalshi_event(event, mapper, signal_repo, follower_repo, copy_repo, config)
                .await
        }
        other => {
            warn!("unknown platform for normalization: {}", other);
            Ok(None)
        }
    }
}

async fn normalize_polymarket_event(
    event: &RawEventRecord,
    mapper: &MarketMapper,
    signal_repo: &SignalRepository,
    follower_repo: &FollowerRepository,
    master_repo: &MasterRepository,
    copy_repo: &CopyIntentRepository,
    config: &AppConfig,
) -> Result<Option<Uuid>> {
    // Parse trade from raw payload
    let trade: PolymarketDataApiTrade = serde_json::from_value(event.raw_payload.clone())
        .context("failed to parse PolymarketDataApiTrade from raw payload")?;

    // WALLET OWNERSHIP GUARD: only create signals for trades that actually
    // came from the master's own wallet. The Polymarket /trades?user= endpoint
    // can return counterparty fills; we must reject those.
    if let Some(master_id) = event.source_entity_id {
        if let Ok(Some(master)) = master_repo.get_by_id(master_id).await {
            if let Some(ref master_wallet) = master.wallet_address {
                let proxy_lc = trade.proxy_wallet.to_lowercase();
                let master_lc = master_wallet.to_lowercase();
                if proxy_lc != master_lc {
                    warn!(
                        event_id = %event.id,
                        proxy_wallet = %proxy_lc,
                        master_wallet = %master_lc,
                        "signal_normalize: dropping event — proxyWallet does not match master wallet"
                    );
                    return Ok(None);
                }
            }
        }
    }

    let master_id = event
        .source_entity_id
        .context("raw event missing source_entity_id (master_id)")?;

    let condition_id = trade
        .condition_id
        .as_deref()
        .context("trade missing condition_id")?;

    let side = trade.side.as_deref().unwrap_or("UNKNOWN").to_uppercase();

    let outcome = trade.outcome.as_deref().unwrap_or("unknown").to_lowercase();

    // Look up market mapping (optional — signals are created even without a mapping)
    let mapping = mapper
        .kalshi_ticker_for_poly_condition(condition_id)
        .filter(|m| m.enabled);

    // Parse size and price from raw JSON (Data API specific fields)
    let size_contracts = parse_decimal_from_value(&trade.raw, &["size", "amount", "quantity"])
        .unwrap_or_else(|| Decimal::ONE);

    let entry_price = parse_decimal_from_value(&trade.raw, &["price", "avgPrice", "avg_price"])
        .unwrap_or_else(|| Decimal::from_f64(0.5).unwrap_or_default());

    let notional = size_contracts * entry_price;

    // Extract human-readable market title. The Polymarket /trades endpoint does
    // not include "title"; the ingest worker injects it via a /markets lookup.
    // Fall back to condition_id so signals are never completely unlabelled.
    let market_title = trade
        .raw
        .get("title")
        .or_else(|| trade.raw.get("question"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| Some(condition_id.to_owned()));

    let signal = SignalRecord {
        id: Uuid::new_v4(),
        platform: "polymarket".to_owned(),
        master_id,
        market_id: mapping.map(|m| m.id),
        outcome: outcome.clone(),
        side: side.clone(),
        position_effect: if side == "BUY" { "open" } else { "close" }.to_owned(),
        size_contracts,
        notional,
        currency: "USD".to_owned(),
        entry_price,
        event_timestamp: trade.timestamp.unwrap_or(event.detected_at),
        detected_at: event.detected_at,
        confidence_score: None,
        status: "new".to_owned(),
        market_title,
    };

    let signal_id = signal_repo
        .insert(&signal)
        .await
        .context("failed to insert signal")?;

    if let Some(ref m) = mapping {
        info!(signal_id = %signal_id, market_id = %m.id, "created signal with mapping");
    } else {
        info!(signal_id = %signal_id, "created signal (no mapping)");
    }

    // Publish signal normalized event
    if let Err(e) = crate::queue_publish::publish_signal(config, signal_id).await {
        warn!("failed to publish signal normalized event: {}", e);
    }

    // Create copy intents for all active followers.
    // Polymarket followers always get intents; Kalshi followers only when a mapping exists.
    let followers = follower_repo.list().await?;
    let action = match side.as_str() {
        "BUY" => "buy",
        "SELL" => "sell",
        _ => "buy",
    };

    // Polymarket followers: always copy using condition_id (mapping optional)
    let poly_followers: Vec<_> = followers
        .iter()
        .filter(|f| f.status == "active" && f.platform == "polymarket")
        .collect();

    if !poly_followers.is_empty() {
        info!(
            count = poly_followers.len(),
            "creating polymarket copy intents for followers"
        );
        for follower in poly_followers {
            match copy_repo
                .insert(
                    follower.id,
                    "polymarket",
                    condition_id,
                    mapping.map(|m| m.id),
                    &outcome,
                    action,
                    size_contracts,
                    entry_price,
                    signal_id,
                )
                .await
            {
                Ok(intent_id) => {
                    info!(intent_id = %intent_id, follower_id = %follower.id, "created polymarket copy intent");
                    if let Err(e) =
                        crate::queue_publish::publish_copy_intent(config, intent_id).await
                    {
                        warn!("failed to publish copy intent event: {}", e);
                    }
                }
                Err(e) => {
                    warn!(follower_id = %follower.id, "failed to create polymarket copy intent: {}", e);
                }
            }
        }
    }

    // Kalshi cross-platform copy: only when a mapping exists
    if let Some(ref m) = mapping {
        let kalshi_outcome = outcome.to_lowercase();
        let kalshi_followers: Vec<_> = followers
            .iter()
            .filter(|f| f.status == "active" && f.platform == "kalshi")
            .collect();

        if !kalshi_followers.is_empty() {
            info!(count = kalshi_followers.len(), kalshi_ticker = %m.kalshi_ticker, "creating cross-platform kalshi copy intents");
            for follower in kalshi_followers {
                match copy_repo
                    .insert(
                        follower.id,
                        "kalshi",
                        &m.kalshi_ticker,
                        Some(m.id),
                        &kalshi_outcome,
                        action,
                        size_contracts,
                        entry_price,
                        signal_id,
                    )
                    .await
                {
                    Ok(intent_id) => {
                        info!(intent_id = %intent_id, follower_id = %follower.id, "created cross-platform kalshi copy intent");
                        if let Err(e) =
                            crate::queue_publish::publish_copy_intent(config, intent_id).await
                        {
                            warn!("failed to publish cross-platform copy intent event: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!(follower_id = %follower.id, "failed to create kalshi copy intent: {}", e);
                    }
                }
            }
        }
    }

    Ok(Some(signal_id))
}

async fn normalize_kalshi_event(
    event: &RawEventRecord,
    mapper: &MarketMapper,
    signal_repo: &SignalRepository,
    follower_repo: &FollowerRepository,
    copy_repo: &CopyIntentRepository,
    config: &AppConfig,
) -> Result<Option<Uuid>> {
    // Parse trade from raw payload
    let trade: KalshiDataApiTrade = serde_json::from_value(event.raw_payload.clone())
        .context("failed to parse KalshiDataApiTrade from raw payload")?;

    let master_id = event
        .source_entity_id
        .context("raw event missing source_entity_id (master_id)")?;

    let market_ticker = trade.market_ticker.as_str();
    if market_ticker.is_empty() {
        anyhow::bail!("kalshi trade missing market_ticker");
    }

    // outcome = which contract leg (yes/no); side = trade direction (buy/sell)
    let outcome = trade.side.as_deref().unwrap_or("yes").to_lowercase();

    let side = trade.action.as_deref().unwrap_or("buy").to_lowercase();

    // Look up market mapping (optional — signals are created even without a mapping)
    let mapping = mapper
        .poly_condition_for_kalshi_ticker(market_ticker)
        .filter(|m| m.enabled);

    let size_contracts = trade.count.unwrap_or_else(|| Decimal::ONE);
    let entry_price = trade
        .price
        .unwrap_or_else(|| Decimal::from_f64(0.5).unwrap_or_default());
    let notional = size_contracts * entry_price;

    // Use market_ticker as the human-readable title for Kalshi
    let market_title = Some(market_ticker.to_string());

    let signal = SignalRecord {
        id: Uuid::new_v4(),
        platform: "kalshi".to_owned(),
        master_id,
        market_id: mapping.map(|m| m.id),
        outcome: outcome.clone(),
        side: side.clone(),
        position_effect: if side == "buy" { "open" } else { "close" }.to_owned(),
        size_contracts,
        notional,
        currency: "USD".to_owned(),
        entry_price,
        event_timestamp: trade.timestamp.unwrap_or(event.detected_at),
        detected_at: event.detected_at,
        confidence_score: None,
        status: "new".to_owned(),
        market_title,
    };

    let signal_id = signal_repo
        .insert(&signal)
        .await
        .context("failed to insert signal")?;

    if let Some(ref m) = mapping {
        info!(signal_id = %signal_id, market_id = %m.id, "created kalshi signal with mapping");
    } else {
        info!(signal_id = %signal_id, "created kalshi signal (no mapping)");
    }

    if let Err(e) = crate::queue_publish::publish_signal(config, signal_id).await {
        warn!("failed to publish signal normalized event: {}", e);
    }

    // Create copy intents for all active kalshi followers (only when mapping exists)
    if let Some(ref m) = mapping {
        let followers = follower_repo.list().await?;
        let active_followers: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == "active" && f.platform == "kalshi")
            .collect();

        info!(
            count = active_followers.len(),
            "creating copy intents for kalshi followers"
        );

        for follower in active_followers {
            let action = side.as_str(); // already "buy" or "sell"

            match copy_repo
                .insert(
                    follower.id,
                    "kalshi",
                    market_ticker,
                    Some(m.id),
                    &outcome,
                    action,
                    size_contracts,
                    entry_price,
                    signal_id,
                )
                .await
            {
                Ok(intent_id) => {
                    info!(intent_id = %intent_id, follower_id = %follower.id, "created copy intent");
                    if let Err(e) =
                        crate::queue_publish::publish_copy_intent(config, intent_id).await
                    {
                        warn!("failed to publish copy intent event: {}", e);
                    }
                }
                Err(e) => {
                    warn!(follower_id = %follower.id, "failed to create copy intent: {}", e);
                }
            }
        }
    }

    Ok(Some(signal_id))
}

/// Try to parse a decimal value from a JSON object using multiple candidate keys.
fn parse_decimal_from_value(raw: &serde_json::Value, keys: &[&str]) -> Option<Decimal> {
    for key in keys {
        if let Some(v) = raw.get(*key) {
            if let Some(s) = v.as_str() {
                if let Ok(d) = s.parse::<Decimal>() {
                    return Some(d);
                }
            }
            if let Some(n) = v.as_f64() {
                if let Some(d) = Decimal::from_f64(n) {
                    return Some(d);
                }
            }
            if let Some(n) = v.as_i64() {
                return Some(Decimal::from(n));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_decimal_from_string_value() {
        let raw = json!({"price": "0.55"});
        let result = parse_decimal_from_value(&raw, &["price"]);
        assert_eq!(result, Some(Decimal::from_f64(0.55).unwrap()));
    }

    #[test]
    fn parse_decimal_from_number_value() {
        let raw = json!({"price": 0.55});
        let result = parse_decimal_from_value(&raw, &["price"]);
        assert_eq!(result, Some(Decimal::from_f64(0.55).unwrap()));
    }

    #[test]
    fn parse_decimal_from_int_value() {
        let raw = json!({"count": 10});
        let result = parse_decimal_from_value(&raw, &["count"]);
        assert_eq!(result, Some(Decimal::from(10)));
    }

    #[test]
    fn parse_decimal_falls_back_to_second_key() {
        let raw = json!({"avg_price": "0.60"});
        let result = parse_decimal_from_value(&raw, &["price", "avg_price"]);
        assert_eq!(result, Some(Decimal::from_f64(0.60).unwrap()));
    }

    #[test]
    fn parse_decimal_returns_none_for_missing_key() {
        let raw = json!({"other": "value"});
        let result = parse_decimal_from_value(&raw, &["price"]);
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod kalshi_tests {
    use super::*;
    use arbitek_platform_kalshi::KalshiDataApiTrade;
    use serde_json::json;

    #[test]
    fn kalshi_trade_to_raw_event_maps_fields() {
        let trade = KalshiDataApiTrade::from_raw(json!({
            "order_id": "ord-456",
            "user_id": "user-def",
            "market_ticker": "BTC-YES",
            "side": "yes",
            "action": "buy",
            "count": "5",
            "price": "0.60",
            "status": "filled",
            "created_time": 1770000000
        }))
        .unwrap();

        assert_eq!(trade.order_id, "ord-456");
        assert_eq!(trade.market_ticker, "BTC-YES");
        assert_eq!(trade.side.as_deref(), Some("yes"));
        assert_eq!(trade.price, Some(Decimal::from_f64(0.60).unwrap()));
        assert!(trade.timestamp.is_some());
    }

    #[test]
    fn kalshi_trade_external_event_id_format() {
        let trade = KalshiDataApiTrade::from_raw(json!({
            "order_id": "ord-789",
            "user_id": "user-ghi",
            "side": "no",
            "action": "sell",
        }))
        .unwrap();

        assert_eq!(trade.external_event_id(), "ord-789:no:sell");
    }
}
