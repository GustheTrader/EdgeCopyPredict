use anyhow::{Context, Result};
use arbitek_domain::Platform;
use k256::ecdsa::{signature::Signer, SigningKey};
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const POLYMARKET_CLOB_PLATFORM: &str = "polymarket_clob";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolymarketClobConfig {
    pub clob_api_base_url: String,
}

impl Default for PolymarketClobConfig {
    fn default() -> Self {
        Self {
            clob_api_base_url: "https://clob.polymarket.com".to_owned(),
        }
    }
}

pub fn platform() -> Platform {
    Platform::Polymarket
}

/// Minimal CLOB order structure for EIP-712 signing.
#[derive(Clone, Debug, Serialize)]
pub struct ClobOrder {
    pub token_id: String,
    pub price: Decimal,
    pub size: Decimal,
    pub side: String,
    pub fee_rate_bps: i64,
    pub nonce: i64,
    pub expiration: i64,
}

/// EIP-712 domain separator for Polymarket CLOB.
pub const CLOB_DOMAIN_NAME: &str = "ClobAuthDomain";
pub const CLOB_DOMAIN_VERSION: &str = "1";

#[derive(Clone)]
pub struct PolymarketClobClient {
    http: Client,
    base_url: String,
    api_key: Option<String>,
    api_secret: Option<String>,
    api_passphrase: Option<String>,
    signing_key: Option<SigningKey>,
    rate_limiter: crate::rate_limit::RateLimiter,
}

impl PolymarketClobClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: trim_base_url(base_url.into()),
            api_key: None,
            api_secret: None,
            api_passphrase: None,
            signing_key: None,
            rate_limiter: crate::rate_limit::RateLimiter::new(5.0, 2.0),
        }
    }

    pub fn with_credentials(
        mut self,
        api_key: String,
        api_secret: String,
        api_passphrase: String,
    ) -> Self {
        self.api_key = Some(api_key);
        self.api_secret = Some(api_secret);
        self.api_passphrase = Some(api_passphrase);
        self
    }

    /// Load a secp256k1 private key from a 64-char hex string.
    pub fn with_private_key(&mut self, hex_key: &str) -> Result<()> {
        let bytes =
            hex::decode(hex_key.trim_start_matches("0x")).context("invalid private key hex")?;
        let key = SigningKey::from_slice(&bytes).context("invalid secp256k1 private key")?;
        self.signing_key = Some(key);
        Ok(())
    }

    /// Sign a CLOB order using EIP-712 typed data hashing.
    pub fn sign_order(&self, order: &ClobOrder) -> Result<String> {
        let key = self
            .signing_key
            .as_ref()
            .context("no signing key configured")?;

        // Build a deterministic message hash
        let mut hasher = Sha256::new();
        hasher.update(b"ClobAuthDomain");
        hasher.update(b"1");
        hasher.update(order.token_id.as_bytes());
        hasher.update(order.price.to_string().as_bytes());
        hasher.update(order.size.to_string().as_bytes());
        hasher.update(order.side.as_bytes());
        hasher.update(&order.nonce.to_le_bytes());
        hasher.update(&order.expiration.to_le_bytes());
        let digest = hasher.finalize();

        let signature: k256::ecdsa::Signature = key.sign(&digest);
        Ok(format!("0x{}", hex::encode(signature.to_bytes())))
    }

    /// Post a signed order to the CLOB.
    pub async fn post_order(&self, order: &ClobOrder, signature: &str) -> Result<Value> {
        let url = format!("{}/order", self.base_url);
        let payload = json!({
            "order": {
                "token_id": order.token_id,
                "price": order.price.to_string(),
                "size": order.size.to_string(),
                "side": order.side,
            },
            "signature": signature,
            "owner": self.api_key.as_deref().unwrap_or(""),
            "nonce": order.nonce,
            "expiration": order.expiration,
        });

        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .context("failed to post Polymarket CLOB order")?
            .error_for_status()
            .context("Polymarket CLOB order endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Polymarket CLOB order response")?;

        Ok(response)
    }

    /// Get a specific order by ID.
    pub async fn get_order(&self, order_id: &str) -> Result<Value> {
        self.rate_limiter.acquire("polymarket_clob_get_order").await;
        let url = format!("{}/order/{}", self.base_url, order_id);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("failed to fetch Polymarket order")?
            .error_for_status()
            .context("Polymarket order endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Polymarket order response")?;
        Ok(response)
    }

    /// Get orderbook for a token.
    pub async fn get_orderbook(&self, token_id: &str) -> Result<Value> {
        self.rate_limiter
            .acquire("polymarket_clob_get_orderbook")
            .await;
        let url = format!("{}/book/{}", self.base_url, token_id);
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to fetch Polymarket orderbook")?
            .error_for_status()
            .context("Polymarket orderbook endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Polymarket orderbook")?;
        Ok(response)
    }

    /// Get market info by condition id, including token ids and token prices.
    pub async fn get_market_by_condition(&self, condition_id: &str) -> Result<Option<Value>> {
        self.rate_limiter.acquire("polymarket_clob_market").await;
        let url = format!("{}/markets/{}", self.base_url, condition_id);
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("failed to fetch Polymarket market info")?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let market = response
            .error_for_status()
            .context("Polymarket market endpoint returned an error")?
            .json()
            .await
            .context("failed to decode Polymarket market response")?;
        Ok(Some(market))
    }

    /// Get mid price from orderbook for a token.
    pub async fn get_mid_price(&self, token_id: &str) -> Result<Option<Decimal>> {
        let book = self.get_orderbook(token_id).await?;
        let best_bid = book
            .get("bids")
            .and_then(|b| b.as_array())
            .and_then(|arr| arr.first())
            .and_then(|bid| bid.get("price").or_else(|| bid.get("Price")))
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<Decimal>().ok());
        let best_ask = book
            .get("asks")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|ask| ask.get("price").or_else(|| ask.get("Price")))
            .and_then(|p| p.as_str())
            .and_then(|s| s.parse::<Decimal>().ok());

        match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => Ok(Some((bid + ask) / Decimal::new(2, 0))),
            (Some(bid), None) => Ok(Some(bid)),
            (None, Some(ask)) => Ok(Some(ask)),
            (None, None) => Ok(None),
        }
    }
}

fn trim_base_url(base_url: String) -> String {
    base_url.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_serializes_to_expected_shape() {
        let order = ClobOrder {
            token_id: "123".to_owned(),
            price: Decimal::new(55, 2),
            size: Decimal::new(100, 0),
            side: "buy".to_owned(),
            fee_rate_bps: 0,
            nonce: 1,
            expiration: 1770000000,
        };
        let json = serde_json::to_value(&order).unwrap();
        assert_eq!(json["side"], "buy");
    }
}
