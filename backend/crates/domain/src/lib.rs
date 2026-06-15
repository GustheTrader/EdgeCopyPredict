use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod copy_config;
pub mod market_mapping;

/// Resolve the stable internal `market_id` UUID for a position/order.
///
/// When the copy intent carries a mapped `market_id` (a row in `market_mappings`),
/// that is authoritative and returned as-is. When it does not — i.e. an *unmapped*
/// Polymarket market, which is the common case for the high-frequency crypto
/// up/down markets that have no Kalshi counterpart — we derive a deterministic UUID
/// from the platform market identifier so the same market always maps to the same id.
///
/// For a Polymarket `condition_id` (`0x` + 64 hex chars), the first 16 bytes are used
/// directly as the UUID. This is what makes the `pnl-refresh`/settlement worker's
/// reverse lookup work: `find_condition_id_by_market_uuid` rebuilds the `0x{32hex}%`
/// prefix from the UUID and `LIKE`-matches it against the stored `condition_id`.
///
/// Without this, unmapped Polymarket positions were stored with a NULL/nil `market_id`,
/// which (a) collapsed every unmapped position sharing an outcome into one row via the
/// `(follower, market_id, outcome)` key, and (b) made resolution settlement impossible
/// because the condition_id could never be recovered — so positions were held open
/// indefinitely even after the master's market resolved.
pub fn stable_market_id(market_id: Option<Uuid>, platform_market_id: &str) -> Uuid {
    market_id.unwrap_or_else(|| derive_market_id(platform_market_id))
}

/// Clamp a copy sell to what the follower actually holds.
///
/// Copy intents are sized at the master's (scaled) trade quantity, which can exceed
/// the follower's own position — the follower may have skipped earlier buys (risk
/// rejections, insufficient balance, a late start) or only partially filled. Selling
/// the full master quantity would drive the follower's position negative (a phantom
/// short it never opened). This returns the contracts to actually sell: never more
/// than `held`, never negative. `None` means there is nothing to close.
pub fn clamp_sell_to_held(target: Decimal, held: Decimal) -> Option<Decimal> {
    if held <= Decimal::ZERO {
        return None;
    }
    Some(target.min(held))
}

/// Deterministically derive a `market_id` UUID from a platform market identifier.
/// Polymarket condition_ids use the first 16 bytes of the hash; anything else falls
/// back to a name-based (v5) UUID so the mapping is still stable and collision-free.
pub fn derive_market_id(platform_market_id: &str) -> Uuid {
    if let Some(hex_str) = platform_market_id.strip_prefix("0x") {
        if hex_str.len() >= 32 {
            if let Ok(bytes) = hex::decode(&hex_str[..32]) {
                if let Ok(arr) = <[u8; 16]>::try_from(bytes.as_slice()) {
                    return Uuid::from_bytes(arr);
                }
            }
        }
    }
    Uuid::new_v5(&Uuid::NAMESPACE_URL, platform_market_id.as_bytes())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Polymarket,
    Kalshi,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositiveDecimal(Decimal);

impl PositiveDecimal {
    pub fn new(value: Decimal) -> Result<Self, String> {
        if value > Decimal::ZERO {
            Ok(Self(value))
        } else {
            Err(format!("value must be positive, got {}", value))
        }
    }

    pub fn inner(&self) -> Decimal {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Price(Decimal);

impl Price {
    pub fn new(value: Decimal) -> Result<Self, String> {
        if value >= Decimal::ZERO && value <= Decimal::ONE {
            Ok(Self(value))
        } else {
            Err(format!("price must be between 0 and 1, got {}", value))
        }
    }

    pub fn inner(&self) -> Decimal {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn positive_decimal_accepts_positive() {
        assert!(PositiveDecimal::new(dec!(1)).is_ok());
    }

    #[test]
    fn positive_decimal_rejects_zero() {
        assert!(PositiveDecimal::new(Decimal::ZERO).is_err());
    }

    #[test]
    fn price_accepts_between_zero_and_one() {
        assert!(Price::new(dec!(0.5)).is_ok());
    }

    #[test]
    fn price_rejects_above_one() {
        assert!(Price::new(dec!(1.5)).is_err());
    }

    #[test]
    fn stable_market_id_prefers_mapped_id() {
        let mapped = Uuid::new_v4();
        assert_eq!(
            stable_market_id(Some(mapped), "0xdeadbeef"),
            mapped,
            "a mapped market_id must always win"
        );
    }

    #[test]
    fn derive_market_id_uses_condition_prefix() {
        let condition = "0x777bac38b3e18bab29f0a3a26b948eb74322f07297e46616fab2e7e88a7ddc2f";
        let id = derive_market_id(condition);
        // The reverse lookup rebuilds "0x{32hex}%" from the UUID's simple form and
        // LIKE-matches the condition_id. Guarantee that round-trip holds.
        let pattern_prefix = format!("0x{}", id.as_simple());
        assert!(
            condition.starts_with(&pattern_prefix),
            "derived id {id} must prefix-match its condition_id for settlement lookup"
        );
    }

    #[test]
    fn derive_market_id_is_deterministic() {
        let condition = "0x2b15ce8e56d4938a4774161feef9dddfaaf7a82d6e9fb32a41b1379b871e9917";
        assert_eq!(derive_market_id(condition), derive_market_id(condition));
    }

    #[test]
    fn derive_market_id_distinct_conditions_dont_collide() {
        let a =
            derive_market_id("0x777bac38b3e18bab29f0a3a26b948eb74322f07297e46616fab2e7e88a7ddc2f");
        let b =
            derive_market_id("0x2b15ce8e56d4938a4774161feef9dddfaaf7a82d6e9fb32a41b1379b871e9917");
        assert_ne!(a, b, "different markets must not share a market_id");
    }

    #[test]
    fn clamp_sell_caps_at_held() {
        // master exit of 100 but follower only holds 30 → sell 30, not 100.
        assert_eq!(clamp_sell_to_held(dec!(100), dec!(30)), Some(dec!(30)));
    }

    #[test]
    fn clamp_sell_allows_full_when_within_holding() {
        assert_eq!(clamp_sell_to_held(dec!(20), dec!(50)), Some(dec!(20)));
    }

    #[test]
    fn clamp_sell_none_when_nothing_held() {
        assert_eq!(clamp_sell_to_held(dec!(10), dec!(0)), None);
        assert_eq!(clamp_sell_to_held(dec!(10), dec!(-5)), None);
    }

    #[test]
    fn derive_market_id_falls_back_for_non_hex() {
        // Non-Polymarket identifiers (e.g. a Kalshi ticker) still get a stable id.
        let id = derive_market_id("BTC-YES");
        assert_eq!(id, derive_market_id("BTC-YES"));
        assert_ne!(id, Uuid::nil());
    }
}
