use anyhow::{bail, Context, Result};
use arbitek_domain::Platform;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

pub mod clob;
pub mod rate_limit;

pub const POLYMARKET_PLATFORM: &str = "polymarket";
pub const POLYMARKET_DATA_API_SOURCE: &str = "polymarket_data_api";
pub const DEFAULT_DATA_API_BASE_URL: &str = "https://data-api.polymarket.com";
pub const DEFAULT_CLOB_API_BASE_URL: &str = "https://clob.polymarket.com";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolymarketClientConfig {
    pub data_api_base_url: String,
    pub clob_api_base_url: String,
}

impl Default for PolymarketClientConfig {
    fn default() -> Self {
        Self {
            data_api_base_url: DEFAULT_DATA_API_BASE_URL.to_owned(),
            clob_api_base_url: DEFAULT_CLOB_API_BASE_URL.to_owned(),
        }
    }
}

pub fn platform() -> Platform {
    Platform::Polymarket
}

#[derive(Clone)]
pub struct PolymarketDataApiClient {
    http: Client,
    base_url: String,
    rate_limiter: rate_limit::RateLimiter,
}

impl PolymarketDataApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: trim_base_url(base_url.into()),
            rate_limiter: rate_limit::RateLimiter::new(10.0, 5.0),
        }
    }

    pub fn with_http_client(base_url: impl Into<String>, http: Client) -> Self {
        Self {
            http,
            base_url: trim_base_url(base_url.into()),
            rate_limiter: rate_limit::RateLimiter::new(10.0, 5.0),
        }
    }

    /// Get market info by condition id (returns token_id for each outcome).
    pub async fn get_market_by_condition(
        &self,
        condition_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        let url = format!("{}/markets/{}", self.base_url, condition_id);
        self.rate_limiter
            .acquire("polymarket_data_api_market")
            .await;
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("failed to fetch Polymarket market info")?
            .error_for_status()
            .context("Polymarket market endpoint returned an error")?
            .json::<serde_json::Value>()
            .await
            .context("failed to decode Polymarket market response")?;
        Ok(Some(response))
    }

    pub async fn get_trades_for_user(
        &self,
        user_address: &str,
        limit: u16,
    ) -> Result<Vec<PolymarketDataApiTrade>> {
        if user_address.trim().is_empty() {
            bail!("user_address must not be empty");
        }

        let limit = limit.clamp(1, 500).to_string();
        let url = format!("{}/trades", self.base_url);
        self.rate_limiter
            .acquire("polymarket_data_api_trades")
            .await;
        let raw_trades: Vec<Value> = self
            .http
            .get(url)
            .query(&[("user", user_address), ("limit", limit.as_str())])
            .send()
            .await
            .context("failed to call Polymarket Data API trades endpoint")?
            .error_for_status()
            .context("Polymarket Data API trades endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Polymarket Data API trades response")?;

        raw_trades
            .into_iter()
            .map(PolymarketDataApiTrade::from_raw)
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolymarketDataApiTrade {
    pub raw: Value,
    pub transaction_hash: String,
    pub proxy_wallet: String,
    pub asset: Option<String>,
    pub condition_id: Option<String>,
    pub side: Option<String>,
    pub outcome: Option<String>,
    pub timestamp: Option<OffsetDateTime>,
}

impl PolymarketDataApiTrade {
    pub fn from_raw(raw: Value) -> Result<Self> {
        let transaction_hash = string_field(&raw, &["transactionHash", "transaction_hash", "hash"])
            .context("Polymarket trade missing transaction hash")?;
        let proxy_wallet = string_field(&raw, &["proxyWallet", "proxy_wallet", "user", "address"])
            .context("Polymarket trade missing proxy wallet")?;

        Ok(Self {
            asset: string_field(&raw, &["asset", "assetId", "token_id", "tokenId"]),
            condition_id: string_field(&raw, &["conditionId", "condition_id"]),
            side: string_field(&raw, &["side"]),
            outcome: string_field(&raw, &["outcome", "outcomeName", "outcome_name"]),
            timestamp: timestamp_field(&raw, &["timestamp", "createdAt", "created_at"]),
            raw,
            transaction_hash,
            proxy_wallet,
        })
    }

    pub fn external_event_id(&self) -> String {
        let asset = self.asset.as_deref().unwrap_or("unknown_asset");
        let side = self.side.as_deref().unwrap_or("unknown_side");
        let outcome = self.outcome.as_deref().unwrap_or("unknown_outcome");

        format!("{}:{asset}:{side}:{outcome}", self.transaction_hash)
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
    fn parses_data_api_trade_fixture() {
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

        assert_eq!(trade.proxy_wallet, "0xabc");
        assert_eq!(trade.transaction_hash, "0xtx");
        assert_eq!(trade.asset.as_deref(), Some("123"));
        assert_eq!(trade.condition_id.as_deref(), Some("0xcondition"));
        assert_eq!(trade.external_event_id(), "0xtx:123:BUY:YES");
        assert!(trade.timestamp.is_some());
    }

    #[test]
    fn rejects_trade_without_transaction_hash() {
        let err = PolymarketDataApiTrade::from_raw(json!({
            "proxyWallet": "0xabc"
        }))
        .unwrap_err()
        .to_string();

        assert!(err.contains("transaction hash"));
    }
}
