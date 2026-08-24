//! Price-age helper.
//!
//! A tiny read-only convenience so consumers can reason about freshness without
//! re-deriving `now - timestamp` from raw [`PriceData`]. Pairs naturally with
//! the staleness guard.
//!
//! ## Trust assumptions
//!
//! The reported age is only as honest as the timestamp the writer recorded. The
//! admin is trusted to write prices at the true ledger time; the helper merely
//! exposes the elapsed seconds since that recorded write.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, Address, Env};

#[contractimpl]
impl Oracle {
    /// Seconds elapsed since the stored price for `asset` was written.
    ///
    /// Returns `0` for a price written at the current ledger timestamp and
    /// grows as the ledger advances. Saturates at `0` in the unusual case the
    /// stored timestamp is in the future relative to `now`.
    ///
    /// # Panics
    /// - `"no price for asset"` if no price was ever written.
    pub fn price_age_secs(env: Env, asset: Address) -> u64 {
        let data: PriceData = env
            .storage()
            .persistent()
            .get(&DataKey::Price(asset))
            .expect("no price for asset");
        env.ledger().timestamp().saturating_sub(data.timestamp)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Env,
    };

    fn setup(env: &Env) -> (OracleClient<'_>, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn fresh_price_has_zero_age() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(500);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &1_0000000);
        assert_eq!(oracle.price_age_secs(&xlm), 0);
    }

    #[test]
    fn age_grows_with_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(500);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &1_0000000);

        env.ledger().set_timestamp(575);
        assert_eq!(oracle.price_age_secs(&xlm), 75);
    }

    #[test]
    #[should_panic(expected = "no price for asset")]
    fn unknown_asset_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let unknown = Address::generate(&env);
        oracle.price_age_secs(&unknown);
    }
}
