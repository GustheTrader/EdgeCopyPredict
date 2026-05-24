use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::warn;

const DEFAULT_BASE_URL: &str = "https://api.oddpool.com";

#[derive(Clone)]
pub struct OddpoolClient {
    http: Client,
    api_key: String,
    base_url: String,
}

/// A pre-matched Polymarket ↔ Kalshi pair from Oddpool /arbitrage/current.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArbPair {
    pub event_id: Option<serde_json::Value>,
    pub event_title: Option<String>,
    pub kalshi_event_ticker: Option<String>,
    pub polymarket_event_slug: Option<String>,
    pub market_type: Option<String>,
    pub outcome_key: Option<String>,
    pub label: Option<String>,
    pub resolution_time: Option<String>,
    pub kalshi: Option<KalshiSide>,
    pub polymarket: Option<PolymarketSide>,
    pub buy_yes_market: Option<String>,
    pub buy_no_market: Option<String>,
    pub gross_cents: Option<f64>,
    pub net_cents: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KalshiSide {
    pub market_ticker: Option<String>,
    pub yes_ask: Option<f64>,
    pub no_ask: Option<f64>,
    pub volume: Option<f64>,
    pub volume_24h: Option<f64>,
    pub open_interest: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PolymarketSide {
    pub condition_id: Option<String>,
    pub yes_token_id: Option<String>,
    pub no_token_id: Option<String>,
    pub yes_ask: Option<f64>,
    pub no_ask: Option<f64>,
    pub volume: Option<f64>,
    pub liquidity: Option<f64>,
}

/// A market record from Oddpool /search/markets.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OddpoolMarket {
    pub market_id: Option<String>,
    pub exchange: Option<String>,
    pub series_id: Option<String>,
    pub question: Option<String>,
    pub status: Option<String>,
    pub volume: Option<f64>,
    pub liquidity: Option<f64>,
    pub last_yes_price: Option<String>,
    pub last_no_price: Option<String>,
    pub event_id: Option<String>,
    pub event_title: Option<String>,
    pub slug: Option<String>,
}

/// What we need to execute a mirrored trade.
#[derive(Clone, Debug)]
pub struct KalshiMatchResult {
    pub kalshi_ticker: String,
    /// Current yes-side ask price (0.0–1.0). Use as limit price.
    pub yes_ask: f64,
    /// Current no-side ask price (0.0–1.0).
    pub no_ask: f64,
    pub event_title: String,
}

impl OddpoolClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_owned(),
        }
    }

    /// Fetch all current Polymarket↔Kalshi pre-matched arbitrage pairs.
    pub async fn get_arbitrage_pairs(&self) -> Result<Vec<ArbPair>> {
        let url = format!("{}/arbitrage/current", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Oddpool /arbitrage/current request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Oddpool /arbitrage/current returned {}: {}", status, body);
        }

        resp.json::<Vec<ArbPair>>()
            .await
            .context("failed to decode Oddpool arbitrage pairs")
    }

    /// Find the Kalshi market matching a Polymarket condition_id.
    /// Searches the pre-matched pairs from Oddpool. Returns None if no pair is found.
    pub async fn find_kalshi_for_condition(
        &self,
        condition_id: &str,
    ) -> Result<Option<KalshiMatchResult>> {
        let pairs = self.get_arbitrage_pairs().await?;
        let condition_lc = condition_id.to_lowercase();

        let found = pairs.into_iter().find(|p| {
            p.polymarket
                .as_ref()
                .and_then(|pm| pm.condition_id.as_deref())
                .map(|cid| cid.to_lowercase() == condition_lc)
                .unwrap_or(false)
        });

        Ok(found.and_then(|pair| {
            let kalshi = pair.kalshi.as_ref()?;
            let ticker = kalshi.market_ticker.as_deref()?;
            let yes_ask = kalshi.yes_ask.unwrap_or(0.5);
            let no_ask = kalshi.no_ask.unwrap_or(0.5);
            Some(KalshiMatchResult {
                kalshi_ticker: ticker.to_owned(),
                yes_ask,
                no_ask,
                event_title: pair.event_title.unwrap_or_default(),
            })
        }))
    }

    /// Fall-back: search Kalshi markets by question text when condition_id lookup misses.
    pub async fn search_kalshi_by_title(
        &self,
        question: &str,
        limit: u8,
    ) -> Result<Vec<OddpoolMarket>> {
        let url = format!("{}/search/markets", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("q", question),
                ("exchange", "kalshi"),
                ("status", "active"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .context("Oddpool /search/markets request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Oddpool /search/markets returned {}: {}", status, body);
            return Ok(vec![]);
        }

        resp.json::<Vec<OddpoolMarket>>()
            .await
            .context("failed to decode Oddpool search markets")
    }
}
