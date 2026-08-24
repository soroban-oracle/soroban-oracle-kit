//! Staleness-guarded price read.
//!
//! Reading a raw price says nothing about *when* it was written. A price that
//! is hours old can be as dangerous as a wrong one — downstream liquidations
//! and valuations must refuse to act on data that has gone stale. This module
//! adds a single safety primitive: read a price only if it is fresh enough.
//!
//! ## Trust assumptions
//!
//! The admin (the only writer in the shipped [`Oracle`]) is trusted to publish
//! correct prices with honest timestamps. This guard does *not* defend against
//! a malicious writer; it defends consumers against an *absent* writer whose
//! last price has aged past a safe bound.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, Address, Env};

#[contractimpl]
impl Oracle {
    /// Read the latest price for `asset`, panicking if it is older than
    /// `max_age_secs` seconds.
    ///
    /// Freshness is measured against the current ledger timestamp. A price
    /// written exactly `max_age_secs` ago is still considered fresh; only a
    /// strictly older price is rejected.
    ///
    /// # Panics
    /// - `"no price for asset"` if no price was ever written.
    /// - `"price too stale"` if `now - timestamp > max_age_secs`.
    pub fn get_price_no_older_than(env: Env, asset: Address, max_age_secs: u64) -> i128 {
        let data: PriceData = env
            .storage()
            .persistent()
            .get(&DataKey::Price(asset))
            .expect("no price for asset");

        let now = env.ledger().timestamp();
        let age = now.saturating_sub(data.timestamp);
        assert!(age <= max_age_secs, "price too stale");
        data.price
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Env,
    };

    fn setup(env: &Env) -> (OracleClient, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn fresh_price_is_returned() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &1_0000000);

        env.ledger().set_timestamp(1_030);
        assert_eq!(oracle.get_price_no_older_than(&xlm, &60), 1_0000000);
    }

    #[test]
    fn exact_age_boundary_is_fresh() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &5_0000000);

        env.ledger().set_timestamp(1_060);
        assert_eq!(oracle.get_price_no_older_than(&xlm, &60), 5_0000000);
    }

    #[test]
    #[should_panic(expected = "price too stale")]
    fn stale_price_panics() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &1_0000000);

        env.ledger().set_timestamp(1_061);
        oracle.get_price_no_older_than(&xlm, &60);
    }

    #[test]
    #[should_panic(expected = "no price for asset")]
    fn unknown_asset_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let unknown = Address::generate(&env);
        oracle.get_price_no_older_than(&unknown, &60);
    }
}
