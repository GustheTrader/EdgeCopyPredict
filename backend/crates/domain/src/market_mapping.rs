use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Cross-platform market mapping: Polymarket condition_id -> Kalshi ticker.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketMapEntry {
    pub id: Uuid,
    pub polymarket_condition_id: String,
    pub polymarket_slug: Option<String>,
    pub kalshi_ticker: String,
    pub kalshi_event_ticker: Option<String>,
    pub confidence: String,
    pub enabled: bool,
}

/// In-memory + database-backed market mapper.
#[derive(Clone, Default)]
pub struct MarketMapper {
    by_poly_condition: HashMap<String, MarketMapEntry>,
    by_kalshi_ticker: HashMap<String, MarketMapEntry>,
}

impl MarketMapper {
    pub fn new(entries: Vec<MarketMapEntry>) -> Self {
        let mut by_poly_condition = HashMap::with_capacity(entries.len());
        let mut by_kalshi_ticker = HashMap::with_capacity(entries.len());
        for entry in entries {
            by_poly_condition.insert(entry.polymarket_condition_id.clone(), entry.clone());
            by_kalshi_ticker.insert(entry.kalshi_ticker.clone(), entry);
        }
        Self {
            by_poly_condition,
            by_kalshi_ticker,
        }
    }

    pub fn kalshi_ticker_for_poly_condition(&self, condition_id: &str) -> Option<&MarketMapEntry> {
        self.by_poly_condition.get(condition_id)
    }

    pub fn poly_condition_for_kalshi_ticker(&self, ticker: &str) -> Option<&MarketMapEntry> {
        self.by_kalshi_ticker.get(ticker)
    }
}

/// Static mapping fixture for common binary markets.
pub fn default_binary_mappings() -> Vec<MarketMapEntry> {
    vec![
        MarketMapEntry {
            id: Uuid::new_v5(&Uuid::NAMESPACE_URL, b"marketmap:btc-updown-5m"),
            polymarket_condition_id: "0xbtc_condition_placeholder".to_owned(),
            polymarket_slug: Some("btc-updown-5m".to_owned()),
            kalshi_ticker: "BTC-5MIN".to_owned(),
            kalshi_event_ticker: Some("BTC-5MIN-EVENT".to_owned()),
            confidence: "manual".to_owned(),
            enabled: false,
        },
        MarketMapEntry {
            id: Uuid::new_v5(&Uuid::NAMESPACE_URL, b"marketmap:eth-updown-5m"),
            polymarket_condition_id: "0xeth_condition_placeholder".to_owned(),
            polymarket_slug: Some("eth-updown-5m".to_owned()),
            kalshi_ticker: "ETH-5MIN".to_owned(),
            kalshi_event_ticker: Some("ETH-5MIN-EVENT".to_owned()),
            confidence: "manual".to_owned(),
            enabled: false,
        },
        MarketMapEntry {
            id: Uuid::new_v5(&Uuid::NAMESPACE_URL, b"marketmap:sp500-daily"),
            polymarket_condition_id: "0xsp500_condition_placeholder".to_owned(),
            polymarket_slug: Some("sp500-updown-daily".to_owned()),
            kalshi_ticker: "SPX-DAILY".to_owned(),
            kalshi_event_ticker: Some("SPX-DAILY-EVENT".to_owned()),
            confidence: "manual".to_owned(),
            enabled: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapper_looks_up_by_condition_id() {
        let entries = default_binary_mappings();
        let mapper = MarketMapper::new(entries);
        assert!(mapper
            .kalshi_ticker_for_poly_condition("0xbtc_condition_placeholder")
            .is_some());
    }

    #[test]
    fn mapper_returns_none_for_unknown() {
        let mapper = MarketMapper::default();
        assert!(mapper.kalshi_ticker_for_poly_condition("unknown").is_none());
    }
}
