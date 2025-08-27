//! Fixed-price stub adapter.
//!
//! A deterministic constant-price source for tests and local development. It
//! ignores the asset entirely and always returns a configured value at
//! `DECIMALS` precision. It is the simplest [`PriceSource`] and a template for
//! the rest of the adapter family.
//!
//! ## Trust assumptions
//!
//! **Test-only.** A fixed price tracks nothing real and must never gate
//! production value flows — using it to price collateral or liquidations is
//! equivalent to trusting a hard-coded number. It exists to make consumer tests
//! deterministic.

use crate::adapters::source::PriceSource;
use soroban_sdk::{Address, Env};

/// A [`PriceSource`] that always reports the same constant price.
pub struct FixedSource {
    /// The constant `DECIMALS`-scaled price to report.
    pub price: i128,
}

impl FixedSource {
    /// Construct a fixed source reporting `price` for every asset.
    pub fn new(price: i128) -> Self {
        FixedSource { price }
    }
}

impl PriceSource for FixedSource {
    fn price(&self, _env: &Env, _asset: &Address) -> i128 {
        self.price
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn returns_configured_constant() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let src = FixedSource::new(4_2000000);
        assert_eq!(src.price(&env, &asset), 4_2000000);
    }

    #[test]
    fn ignores_asset_identity() {
        let env = Env::default();
        let a = Address::generate(&env);
        let b = Address::generate(&env);
        let src = FixedSource::new(1_0000000);
        assert_eq!(src.price(&env, &a), src.price(&env, &b));
    }
}
