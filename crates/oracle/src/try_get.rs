//! Non-panicking price read.
//!
//! The shipped [`Oracle::get_price`] panics on an unset asset, which forces a
//! consumer that wants fallback logic to pre-check with `has_price`. This module
//! offers an `Option`-style read so callers can branch on absence directly.
//!
//! ## Trust assumptions
//!
//! Identical to the base feed: the admin is trusted to write correct prices.
//! This read changes only the *absence* signalling (None vs panic); it does not
//! validate freshness — combine with the staleness guard where that matters.

use crate::OracleClient;
use crate::{DataKey, Oracle};
use soroban_sdk::{contractimpl, Address, Env};

#[contractimpl]
impl Oracle {
    /// Read the latest price for `asset`, returning `None` if it is unset.
    pub fn get_price_optional(env: Env, asset: Address) -> Option<i128> {
        env.storage()
            .persistent()
            .get::<_, crate::PriceData>(&DataKey::Price(asset))
            .map(|d| d.price)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup(env: &Env) -> (OracleClient, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn known_asset_returns_some() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &2_5000000);
        assert_eq!(oracle.get_price_optional(&xlm), Some(2_5000000));
    }

    #[test]
    fn unknown_asset_returns_none() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let unknown = Address::generate(&env);
        assert_eq!(oracle.get_price_optional(&unknown), None);
    }
}
