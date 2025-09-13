//! Identity (pass-through) adapter.
//!
//! Wraps the contract's *own* stored feed behind the [`PriceSource`] trait, so a
//! consumer written against `PriceSource` can read the on-chain oracle through
//! the same interface it uses for every other source. The price is returned
//! unchanged.
//!
//! ## Trust assumptions
//!
//! Identical to the base feed: the admin is trusted to publish correct,
//! honestly-timestamped prices. The adapter performs no freshness filtering —
//! wrap it in a staleness adapter where that matters.

use crate::adapters::source::PriceSource;
use crate::{DataKey, PriceData};
use soroban_sdk::{Address, Env};

/// A [`PriceSource`] backed by the oracle's own persistent price storage.
pub struct IdentitySource;

impl IdentitySource {
    /// Construct a pass-through source over the local oracle storage.
    pub fn new() -> Self {
        IdentitySource
    }
}

impl Default for IdentitySource {
    fn default() -> Self {
        Self::new()
    }
}

impl PriceSource for IdentitySource {
    /// # Panics
    /// - `"no price for asset"` if the oracle has no price for `asset`.
    fn price(&self, env: &Env, asset: &Address) -> i128 {
        let data: PriceData = env
            .storage()
            .persistent()
            .get(&DataKey::Price(asset.clone()))
            .expect("no price for asset");
        data.price
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Oracle, OracleClient};
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn returns_stored_price_unchanged() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(&env, &id);
        client.init(&admin);

        let xlm = Address::generate(&env);
        client.set_price(&xlm, &6_5000000);

        // Read through the adapter from inside the contract's storage context.
        let observed = env.as_contract(&id, || {
            let src = IdentitySource::new();
            src.price(&env, &xlm)
        });
        assert_eq!(observed, 6_5000000);
    }

    #[test]
    #[should_panic(expected = "no price for asset")]
    fn unknown_asset_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(&env, &id);
        client.init(&admin);

        let unknown = Address::generate(&env);
        env.as_contract(&id, || {
            let src = IdentitySource::new();
            src.price(&env, &unknown);
        });
    }
}
