use anyhow::Result;
use arbitek_platform_kalshi::execution::{KalshiExecutionClient, KalshiOrderRequest};
use arbitek_platform_polymarket::clob::{ClobOrder, PolymarketClobClient};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

/// Unified live execution input.
#[derive(Clone, Debug)]
pub struct LiveExecutionInput {
    pub order_id: Uuid,
    pub copy_intent_id: Uuid,
    pub follower_account_id: Uuid,
    pub platform: String,
    pub market_id: Option<Uuid>,
    pub outcome: String,
    pub side: String,
    pub target_contracts: Decimal,
    pub price_limit: Decimal,
    pub client_order_id: String,
}

/// Unified live execution output.
#[derive(Clone, Debug)]
pub struct LiveExecutionOutput {
    pub order_id: Uuid,
    pub platform_order_id: Option<String>,
    pub status: String,
    pub filled_contracts: Decimal,
    pub avg_fill_price: Option<Decimal>,
    pub submitted_at: OffsetDateTime,
    pub request_payload: Value,
    pub response_payload: Value,
}

/// Execute an order on Kalshi (live).
pub async fn execute_kalshi(
    input: &LiveExecutionInput,
    client: &KalshiExecutionClient,
    ticker: &str,
) -> Result<LiveExecutionOutput> {
    // Convert (outcome, side) → Kalshi bid/ask per PRD FR-EXE-005:
    //   buy YES  → bid at master price
    //   sell YES → ask at master price
    //   buy NO   → ask at (1 - master price)  [buying NO = short YES leg]
    //   sell NO  → bid at (1 - master price)
    let outcome = input.outcome.to_lowercase();
    let side = input.side.to_lowercase();
    let (kalshi_side, raw_price) = match (outcome.as_str(), side.as_str()) {
        ("yes", "buy") => ("bid".to_owned(), input.price_limit),
        ("yes", "sell") => ("ask".to_owned(), input.price_limit),
        ("no", "buy") => ("ask".to_owned(), Decimal::ONE - input.price_limit),
        ("no", "sell") => ("bid".to_owned(), Decimal::ONE - input.price_limit),
        _ => (side.clone(), input.price_limit),
    };

    // Apply slippage cap per PRD default (3 cents):
    //   bid orders: willing to pay up to 3¢ above master price → cap at 0.97
    //   ask orders: willing to receive 3¢ below master price   → floor at 0.03
    let max_slippage = Decimal::new(3, 2);
    let order_price = if kalshi_side == "bid" {
        (raw_price + max_slippage).min(Decimal::new(97, 2))
    } else {
        (raw_price - max_slippage).max(Decimal::new(3, 2))
    };

    let request = KalshiOrderRequest {
        ticker: ticker.to_owned(),
        side: kalshi_side,
        count: input.target_contracts,
        price: order_price,
        action: input.side.clone(),
        client_order_id: input.client_order_id.clone(),
        expiration_time: None,
    };

    let response = client.create_order(&request).await?;

    let now = OffsetDateTime::now_utc();
    Ok(LiveExecutionOutput {
        order_id: input.order_id,
        platform_order_id: response.order_id,
        status: response.status,
        filled_contracts: response.filled_count.unwrap_or_default(),
        avg_fill_price: response.avg_price,
        submitted_at: now,
        request_payload: json!({
            "ticker": request.ticker,
            "side": request.side,
            "count": request.count.to_string(),
            "price": request.price.to_string(),
            "time_in_force": "immediate_or_cancel",
            "client_order_id": request.client_order_id,
        }),
        response_payload: response.raw,
    })
}

/// Execute an order on Polymarket CLOB (live).
pub async fn execute_polymarket_clob(
    input: &LiveExecutionInput,
    client: &PolymarketClobClient,
    token_id: &str,
) -> Result<LiveExecutionOutput> {
    // Apply slippage cap per PRD default (3 cents):
    //   buy: willing to pay up to 3¢ above master price → cap at 0.97
    //   sell: willing to accept 3¢ below master price   → floor at 0.03
    let max_slippage = Decimal::new(3, 2);
    let order_price = if input.side.to_lowercase() == "buy" {
        (input.price_limit + max_slippage).min(Decimal::new(97, 2))
    } else {
        (input.price_limit - max_slippage).max(Decimal::new(3, 2))
    };

    let order = ClobOrder {
        token_id: token_id.to_owned(),
        price: order_price,
        size: input.target_contracts,
        side: input.side.clone(),
        fee_rate_bps: 0,
        nonce: now_nanos(),
        expiration: now_plus_seconds(30),
    };

    // Sign with the client's configured key so signing and submission use the same order.
    let signature = client.sign_order(&order).unwrap_or_default();
    let response = client.post_order(&order, &signature).await?;

    let now = OffsetDateTime::now_utc();
    let platform_order_id = response
        .get("orderID")
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| response.get("id").and_then(Value::as_str).map(String::from));

    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("submitted")
        .to_owned();

    let filled = response
        .get("filledSize")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let avg_fill_price = response
        .get("avgPrice")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok());

    Ok(LiveExecutionOutput {
        order_id: input.order_id,
        platform_order_id,
        status,
        filled_contracts: filled,
        avg_fill_price,
        submitted_at: now,
        request_payload: json!({
            "token_id": order.token_id,
            "price": order.price.to_string(),
            "size": order.size.to_string(),
            "side": order.side,
            "nonce": order.nonce,
            "expiration": order.expiration,
        }),
        response_payload: response,
    })
}

pub fn now_nanos() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}

pub fn now_plus_seconds(seconds: i64) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    now + seconds
}
