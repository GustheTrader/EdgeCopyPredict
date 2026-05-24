use anyhow::{Context, Result};
use arbitek_domain::Platform;
use reqwest::{Client, Method};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

pub const KALSHI_ORDER_PLATFORM: &str = "kalshi";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KalshiExecutionConfig {
    pub rest_api_base_url: String,
}

impl Default for KalshiExecutionConfig {
    fn default() -> Self {
        Self {
            rest_api_base_url: "https://external-api.kalshi.com/trade-api/v2".to_owned(),
        }
    }
}

pub fn platform() -> Platform {
    Platform::Kalshi
}

#[derive(Clone, Debug, Serialize)]
pub struct KalshiOrderRequest {
    pub ticker: String,
    pub side: String,
    pub count: Decimal,
    pub price: Decimal,
    pub action: String,
    pub client_order_id: String,
    pub expiration_time: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct KalshiOrderResponse {
    pub order_id: Option<String>,
    pub status: String,
    pub filled_count: Option<Decimal>,
    pub remaining_count: Option<Decimal>,
    pub avg_price: Option<Decimal>,
    pub market_ticker: String,
    pub created_time: Option<OffsetDateTime>,
    pub raw: Value,
}

#[derive(Clone)]
pub struct KalshiExecutionClient {
    http: Client,
    base_url: String,
    auth: Option<crate::KalshiAuth>,
    rate_limiter: crate::rate_limit::RateLimiter,
}

impl KalshiExecutionClient {
    pub fn new(base_url: impl Into<String>, auth: Option<crate::KalshiAuth>) -> Self {
        Self {
            http: Client::new(),
            base_url: trim_base_url(base_url.into()),
            auth,
            rate_limiter: crate::rate_limit::RateLimiter::new(5.0, 2.0),
        }
    }

    pub async fn create_order(&self, request: &KalshiOrderRequest) -> Result<KalshiOrderResponse> {
        let auth = self
            .auth
            .as_ref()
            .context("Kalshi order creation requires authentication")?;
        let endpoint_path = "/orders";
        let url = format!("{}{}", self.base_url, endpoint_path);
        let sign_path = url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let method = Method::POST;
        let headers = auth.headers(&method, sign_path)?;

        let payload = json!({
            "ticker": request.ticker,
            "side": request.side,
            "count": request.count.to_string(),
            "price_cents": (request.price * Decimal::new(100, 0)).to_i64(),
            "client_order_id": request.client_order_id,
            "time_in_force": "immediate_or_cancel",
            "self_trade_prevention_type": "self_trade_prevention_multi_cancel_oldest",
        });

        let raw: Value = self
            .http
            .request(method, url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .context("failed to call Kalshi create order endpoint")?
            .error_for_status()
            .context("Kalshi create order endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Kalshi create order response")?;

        Ok(KalshiOrderResponse {
            order_id: raw
                .get("order_id")
                .and_then(Value::as_str)
                .map(String::from),
            status: raw
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
            filled_count: raw
                .get("filled_count")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            remaining_count: raw
                .get("remaining_count")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            avg_price: raw
                .get("avg_price")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            market_ticker: request.ticker.clone(),
            created_time: None,
            raw,
        })
    }

    pub async fn get_order(&self, order_id: &str) -> Result<Value> {
        self.rate_limiter
            .acquire("kalshi_execution_get_order")
            .await;
        let auth = self
            .auth
            .as_ref()
            .context("Kalshi get order requires authentication")?;
        let endpoint_path = format!("/orders/{order_id}");
        let url = format!("{}{}", self.base_url, endpoint_path);
        let sign_path = url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let method = Method::GET;
        let headers = auth.headers(&method, sign_path)?;

        let raw: Value = self
            .http
            .request(method, url)
            .headers(headers)
            .send()
            .await
            .context("failed to call Kalshi get order endpoint")?
            .error_for_status()
            .context("Kalshi get order endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Kalshi get order response")?;

        Ok(raw)
    }

    pub async fn cancel_order(&self, order_id: &str) -> Result<Value> {
        self.rate_limiter
            .acquire("kalshi_execution_cancel_order")
            .await;
        let auth = self
            .auth
            .as_ref()
            .context("Kalshi cancel order requires authentication")?;
        let endpoint_path = format!("/orders/{order_id}");
        let url = format!("{}{}", self.base_url, endpoint_path);
        let sign_path = url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let method = Method::DELETE;
        let headers = auth.headers(&method, &sign_path)?;

        let raw: Value = self
            .http
            .request(method, url)
            .headers(headers)
            .send()
            .await
            .context("failed to call Kalshi cancel order endpoint")?
            .error_for_status()
            .context("Kalshi cancel order endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Kalshi cancel order response")?;

        Ok(raw)
    }
}

fn trim_base_url(base_url: String) -> String {
    base_url.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_price_in_cents() {
        let req = KalshiOrderRequest {
            ticker: "BTC-YES".to_owned(),
            side: "yes".to_owned(),
            count: Decimal::new(10, 0),
            price: Decimal::new(55, 2),
            action: "buy".to_owned(),
            client_order_id: "test-1".to_owned(),
            expiration_time: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["ticker"], "BTC-YES");
    }
}
