//! Per-follower copy configuration parsed from `followers.risk_config` (JSONB).
//!
//! This is the single source of truth for how a follower copies a master's trades.
//! The preferred config is intentionally small: mirror the master's trades and size
//! them by the follower/master bankroll ratio. Legacy fields are still accepted so
//! older saved configs continue to work.
//!
//! All fields are optional so partially-configured followers still work; getters apply
//! permissive, back-compatible defaults (e.g. missing odds range = accept everything).
//! The `0014` migration seeds the recommended defaults onto existing followers.

use rust_decimal::Decimal;
use serde::Deserialize;

/// What to do with a master trade under the configured trade behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyAction {
    /// "buy" or "sell" for the follower.
    pub action: &'static str,
    /// When true, copy the opposite outcome (counter-trading); the caller must flip
    /// the outcome and skip the trade if the outcome has no known opposite.
    pub invert_outcome: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct CopyConfig {
    // --- preferred simple config ---
    pub copy_mode: Option<String>,   // mirror
    pub sizing_mode: Option<String>, // proportional_bankroll
    pub follower_bankroll: Option<Decimal>,
    pub master_bankroll: Option<Decimal>,

    // --- sizing ---
    pub buy_method: Option<String>, // percentage | fixed_amount | exact_copy | portfolio_weighted
    pub buy_percentage: Option<Decimal>, // % of trader's position
    pub fixed_amount: Option<Decimal>, // $ per trade
    pub copy_scale: Option<Decimal>, // legacy multiplier (0..1)

    // --- execution ---
    pub slippage_pct: Option<Decimal>,

    // --- entry filters ---
    pub odds_min: Option<Decimal>, // implied probability 0..1
    pub odds_max: Option<Decimal>,
    pub min_trigger: Option<Decimal>, // copy only master trades with notional >= this ($)
    pub max_trigger: Option<Decimal>, // ignore master trades with notional > this ($); 0/None = no cap

    // --- behavior ---
    pub trade_behavior: Option<String>, // buys_only | buys_sells | counter | all

    // --- follower-side limits ---
    pub min_buy: Option<Decimal>, // min follower order notional ($)
    pub max_buy: Option<Decimal>, // max follower order notional ($); 0/None = no cap
    pub max_spend_total: Option<Decimal>,
    pub max_spend_per_market: Option<Decimal>,
    pub daily_spend: Option<Decimal>,
    pub bankroll: Option<Decimal>,
    pub max_trade_bankroll_pct: Option<Decimal>,
    // legacy aliases for min/max order notional
    pub min_notional: Option<Decimal>,
    pub max_notional: Option<Decimal>,

    // --- misc ---
    pub max_days_out: Option<i64>,
    pub mute_notifications: Option<bool>,
    // Stored in P1, enforced in P2 (stop-loss/take-profit, market metadata filters):
    pub stop_loss_type: Option<String>,
    pub stop_loss_pct: Option<Decimal>,
    pub take_profit_pct: Option<Decimal>,
    pub categories: Option<Vec<String>>,
    pub min_liquidity: Option<Decimal>,
    pub min_volume: Option<Decimal>,
}

impl CopyConfig {
    /// Parse from a `risk_config` JSON value. Never fails — unknown/absent keys are
    /// ignored and fall back to permissive defaults.
    pub fn from_value(value: &serde_json::Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    /// Follower contracts to trade for a master trade of `master_contracts` at
    /// `entry_price`, given the configured buy method. `master_notional` is the
    /// master's traded dollar amount (`master_contracts * entry_price`).
    pub fn sized_contracts(
        &self,
        master_contracts: Decimal,
        entry_price: Decimal,
        master_notional: Decimal,
    ) -> Decimal {
        if entry_price <= Decimal::ZERO {
            return master_contracts;
        }
        if self.uses_proportional_bankroll() {
            return self.proportional_bankroll_contracts(master_contracts);
        }

        match self.buy_method.as_deref().unwrap_or("percentage") {
            "fixed_amount" => match self.fixed_amount {
                Some(amt) if amt > Decimal::ZERO => amt / entry_price,
                _ => self.percentage_contracts(master_contracts),
            },
            "exact_copy" => {
                if master_notional > Decimal::ZERO {
                    master_notional / entry_price
                } else {
                    master_contracts
                }
            }
            // portfolio_weighted is a Phase-2 feature; fall back to percentage for now.
            _ => self.percentage_contracts(master_contracts),
        }
    }

    fn uses_proportional_bankroll(&self) -> bool {
        matches!(
            self.sizing_mode.as_deref(),
            Some("proportional_bankroll") | Some("bankroll_ratio")
        ) || (self.sizing_mode.is_none()
            && self.follower_bankroll.is_some()
            && self.master_bankroll.is_some())
    }

    fn proportional_bankroll_contracts(&self, master_contracts: Decimal) -> Decimal {
        match (self.follower_bankroll, self.master_bankroll) {
            (Some(follower), Some(master))
                if follower > Decimal::ZERO && master > Decimal::ZERO =>
            {
                master_contracts * follower / master
            }
            _ => self.percentage_contracts(master_contracts),
        }
    }

    fn percentage_contracts(&self, master_contracts: Decimal) -> Decimal {
        // Prefer explicit buy_percentage; fall back to legacy copy_scale (0..1 → %);
        // default to a full 100% copy when nothing is configured.
        let pct = self
            .buy_percentage
            .or_else(|| self.copy_scale.map(|s| s * Decimal::from(100)))
            .unwrap_or_else(|| Decimal::from(100));
        if pct <= Decimal::ZERO {
            master_contracts
        } else {
            master_contracts * pct / Decimal::from(100)
        }
    }

    /// True if the master entry price (implied probability, 0..1) is within the odds band.
    pub fn passes_odds(&self, entry_price: Decimal) -> bool {
        let lo = self.odds_min.unwrap_or(Decimal::ZERO);
        let hi = self.odds_max.unwrap_or(Decimal::ONE);
        entry_price >= lo && entry_price <= hi
    }

    /// True if the master's traded dollar amount is within the trigger band.
    pub fn passes_trigger(&self, master_notional: Decimal) -> bool {
        if let Some(min) = self.min_trigger {
            if master_notional < min {
                return false;
            }
        }
        if let Some(max) = self.max_trigger {
            if max > Decimal::ZERO && master_notional > max {
                return false;
            }
        }
        true
    }

    /// Resolve the configured behavior for a master trade side ("BUY"/"SELL").
    /// Returns None when the trade should be skipped entirely.
    pub fn behavior_for(&self, master_side: &str) -> Option<CopyAction> {
        let is_buy = master_side.eq_ignore_ascii_case("buy");
        match self.trade_behavior.as_deref().unwrap_or("buys_sells") {
            "buys_only" => is_buy.then_some(CopyAction {
                action: "buy",
                invert_outcome: false,
            }),
            "counter" => Some(CopyAction {
                action: if is_buy { "buy" } else { "sell" },
                invert_outcome: true,
            }),
            // "buys_sells" and "all" both follow buys and sells directly in Phase 1
            // (the extra counter leg of "all" is deferred to Phase 2).
            _ => Some(CopyAction {
                action: if is_buy { "buy" } else { "sell" },
                invert_outcome: false,
            }),
        }
    }

    /// Configured slippage as a fraction (e.g. 10% → 0.10). Defaults to 0.10.
    pub fn slippage_fraction(&self) -> Decimal {
        let pct = self.slippage_pct.unwrap_or_else(|| Decimal::from(10));
        pct / Decimal::from(100)
    }

    /// Minimum follower order notional ($). Falls back to legacy `min_notional`.
    pub fn min_order_notional(&self) -> Option<Decimal> {
        self.min_buy.or(self.min_notional)
    }

    /// Maximum follower order notional ($); None / 0 means no cap. Falls back to `max_notional`.
    pub fn max_order_notional(&self) -> Option<Decimal> {
        self.max_buy
            .or(self.max_notional)
            .filter(|m| *m > Decimal::ZERO)
    }

    pub fn notifications_muted(&self) -> bool {
        self.mute_notifications.unwrap_or(false)
    }
}

/// The opposite outcome for known binary markets, used by counter-trading.
/// Returns None for non-binary/unknown outcomes (counter trade is then skipped).
pub fn opposite_outcome(outcome: &str) -> Option<&'static str> {
    match outcome.trim().to_lowercase().as_str() {
        "yes" => Some("no"),
        "no" => Some("yes"),
        "up" => Some("down"),
        "down" => Some("up"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    fn cfg(v: serde_json::Value) -> CopyConfig {
        CopyConfig::from_value(&v)
    }

    #[test]
    fn percentage_sizing_halves_position() {
        let c = cfg(json!({"buy_method":"percentage","buy_percentage":50}));
        assert_eq!(c.sized_contracts(dec!(100), dec!(0.5), dec!(50)), dec!(50));
    }

    #[test]
    fn fixed_amount_sizing_divides_by_price() {
        let c = cfg(json!({"buy_method":"fixed_amount","fixed_amount":10}));
        // $10 at $0.50 → 20 contracts
        assert_eq!(c.sized_contracts(dec!(100), dec!(0.5), dec!(50)), dec!(20));
    }

    #[test]
    fn exact_copy_matches_master_notional() {
        let c = cfg(json!({"buy_method":"exact_copy"}));
        assert_eq!(c.sized_contracts(dec!(100), dec!(0.5), dec!(50)), dec!(100));
    }

    #[test]
    fn legacy_copy_scale_still_applies() {
        let c = cfg(json!({"copy_scale":0.3}));
        assert_eq!(c.sized_contracts(dec!(100), dec!(0.5), dec!(50)), dec!(30));
    }

    #[test]
    fn proportional_bankroll_sizes_by_account_ratio() {
        let c = cfg(json!({
            "copy_mode": "mirror",
            "sizing_mode": "proportional_bankroll",
            "follower_bankroll": 1000,
            "master_bankroll": 50000
        }));
        assert_eq!(
            c.sized_contracts(dec!(1000), dec!(0.5), dec!(500)),
            dec!(20)
        );
    }

    #[test]
    fn default_config_full_copies() {
        let c = cfg(json!({}));
        assert_eq!(c.sized_contracts(dec!(100), dec!(0.5), dec!(50)), dec!(100));
        assert!(c.passes_odds(dec!(0.01)));
        assert!(c.passes_trigger(dec!(0.01)));
    }

    #[test]
    fn odds_filter_band() {
        let c = cfg(json!({"odds_min":0.2,"odds_max":0.8}));
        assert!(!c.passes_odds(dec!(0.1)));
        assert!(c.passes_odds(dec!(0.5)));
        assert!(!c.passes_odds(dec!(0.9)));
    }

    #[test]
    fn trigger_filter_band() {
        let c = cfg(json!({"min_trigger":5,"max_trigger":50}));
        assert!(!c.passes_trigger(dec!(4)));
        assert!(c.passes_trigger(dec!(25)));
        assert!(!c.passes_trigger(dec!(75)));
    }

    #[test]
    fn behavior_buys_only_skips_sells() {
        let c = cfg(json!({"trade_behavior":"buys_only"}));
        assert!(c.behavior_for("BUY").is_some());
        assert!(c.behavior_for("SELL").is_none());
    }

    #[test]
    fn behavior_counter_inverts() {
        let c = cfg(json!({"trade_behavior":"counter"}));
        let a = c.behavior_for("BUY").unwrap();
        assert_eq!(a.action, "buy");
        assert!(a.invert_outcome);
    }

    #[test]
    fn behavior_default_follows_both() {
        let c = cfg(json!({}));
        assert_eq!(c.behavior_for("BUY").unwrap().action, "buy");
        assert_eq!(c.behavior_for("SELL").unwrap().action, "sell");
    }

    #[test]
    fn slippage_default_and_custom() {
        assert_eq!(cfg(json!({})).slippage_fraction(), dec!(0.10));
        assert_eq!(
            cfg(json!({"slippage_pct":2})).slippage_fraction(),
            dec!(0.02)
        );
    }

    #[test]
    fn opposite_outcomes() {
        assert_eq!(opposite_outcome("Yes"), Some("no"));
        assert_eq!(opposite_outcome("DOWN"), Some("up"));
        assert_eq!(opposite_outcome("Alejandro"), None);
    }
}
