use anyhow::{Context, Result};
use reqwest::Client;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

pub const KALSHI_DATA_API_SOURCE: &str = "kalshi_data_api";
pub const DEFAULT_DATA_API_BASE_URL: &str = "https://external-api.kalshi.com/trade-api/v2";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct KalshiDataApiTrade {
    pub raw: Value,
    pub order_id: String,
    pub user_id: String,
    pub market_ticker: String,
    pub side: Option<String>,
    pub action: Option<String>,
    pub count: Option<Decimal>,
    pub price: Option<Decimal>,
    pub status: Option<String>,
    pub timestamp: Option<OffsetDateTime>,
}

impl KalshiDataApiTrade {
    pub fn from_raw(raw: Value) -> Result<Self> {
        let order_id =
            string_field(&raw, &["order_id", "id"]).context("Kalshi trade missing order_id")?;
        let user_id =
            string_field(&raw, &["user_id", "user"]).context("Kalshi trade missing user_id")?;

        let market_ticker =
            string_field(&raw, &["market_ticker", "ticker", "event_ticker"]).unwrap_or_default();

        Ok(Self {
            side: string_field(&raw, &["side"]),
            action: string_field(&raw, &["action"]),
            count: decimal_field(&raw, &["count", "filled_count", "quantity"]),
            price: decimal_field(&raw, &["price", "avg_price", "price_cents"]),
            status: string_field(&raw, &["status"]),
            timestamp: timestamp_field(&raw, &["created_time", "timestamp", "created_at"]),
            raw,
            order_id,
            user_id,
            market_ticker,
        })
    }

    pub fn external_event_id(&self) -> String {
        let side = self.side.as_deref().unwrap_or("unknown");
        let action = self.action.as_deref().unwrap_or("unknown");
        format!("{}:{side}:{action}", self.order_id)
    }
}

#[derive(Clone)]
pub struct KalshiDataApiClient {
    http: Client,
    base_url: String,
    #[allow(dead_code)]
    auth: Option<crate::KalshiAuth>,
    rate_limiter: crate::rate_limit::RateLimiter,
}

impl KalshiDataApiClient {
    pub fn new(base_url: impl Into<String>, auth: Option<crate::KalshiAuth>) -> Self {
        Self {
            http: Client::new(),
            base_url: trim_base_url(base_url.into()),
            auth,
            rate_limiter: crate::rate_limit::RateLimiter::new(10.0, 5.0),
        }
    }

    /// Get current market price (best bid/ask mid) for a ticker.
    pub async fn get_market_price(&self, ticker: &str) -> Result<Option<Decimal>> {
        let url = format!(
            "{}/markets/{}?with_nested_markets=true",
            self.base_url, ticker
        );
        self.rate_limiter.acquire("kalshi_data_api_market").await;
        let response: Value = self
            .http
            .get(&url)
            .send()
            .await
            .context("failed to call Kalshi market endpoint")?
            .error_for_status()
            .context("Kalshi market endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Kalshi market response")?;

        // Try to extract yes_price / no_price from nested markets or top-level
        let price = response
            .get("market")
            .or_else(|| Some(&response))
            .and_then(|m| m.get("yes_price").or_else(|| m.get("no_price")))
            .and_then(|v| v.as_i64())
            .map(|cents| Decimal::from(cents) / Decimal::new(100, 0));

        Ok(price)
    }

    /// Get trades for the authenticated account (requires auth).
    pub async fn get_trades_for_user(
        &self,
        _user_id: &str,
        limit: u16,
    ) -> Result<Vec<KalshiDataApiTrade>> {
        let limit_str = limit.clamp(1, 500).to_string();
        let path = "/portfolio/orders";
        let url = format!("{}{}", self.base_url, path);

        let mut req = self
            .http
            .get(&url)
            .query(&[("limit", limit_str.as_str()), ("status", "resting,filled")]);

        if let Some(auth) = &self.auth {
            if let Ok(headers) = auth.headers(&reqwest::Method::GET, path) {
                req = req.headers(headers);
            }
        }

        self.rate_limiter.acquire("kalshi_data_api_orders").await;
        let body: Value = req
            .send()
            .await
            .context("failed to call Kalshi orders endpoint")?
            .error_for_status()
            .context("Kalshi orders endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Kalshi orders response")?;

        // Kalshi returns {"orders": [...]}
        let raw_trades = body
            .get("orders")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        raw_trades
            .into_iter()
            .map(KalshiDataApiTrade::from_raw)
            .collect()
    }
}

fn trim_base_url(base_url: String) -> String {
    base_url.trim_end_matches('/').to_owned()
}

fn string_field(raw: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| raw.get(*key))
        .and_then(value_to_string)
        .filter(|value| !value.trim().is_empty())
}

fn decimal_field(raw: &Value, keys: &[&str]) -> Option<Decimal> {
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

fn timestamp_field(raw: &Value, keys: &[&str]) -> Option<OffsetDateTime> {
    keys.iter()
        .find_map(|key| raw.get(*key))
        .and_then(value_to_i64)
        .and_then(|timestamp| {
            let seconds = if timestamp > 10_000_000_000 {
                timestamp / 1_000
            } else {
                timestamp
            };
            OffsetDateTime::from_unix_timestamp(seconds).ok()
        })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(inner) => Some(inner.clone()),
        Value::Number(inner) => Some(inner.to_string()),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(inner) => inner.as_i64(),
        Value::String(inner) => inner.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_kalshi_trade_fixture() {
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

        assert_eq!(trade.order_id, "ord-123");
        assert_eq!(trade.user_id, "user-abc");
        assert_eq!(trade.market_ticker, "BTC-YES");
        assert_eq!(trade.side.as_deref(), Some("yes"));
        assert_eq!(trade.external_event_id(), "ord-123:yes:buy");
        assert!(trade.timestamp.is_some());
    }

    #[test]
    fn rejects_trade_without_order_id() {
        let err = KalshiDataApiTrade::from_raw(json!({
            "user_id": "user-abc"
        }))
        .unwrap_err()
        .to_string();

        assert!(err.contains("order_id"));
    }
}
