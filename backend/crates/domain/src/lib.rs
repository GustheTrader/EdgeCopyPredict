use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub mod market_mapping;

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
}
