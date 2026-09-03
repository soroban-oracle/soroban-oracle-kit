//! TWAP accumulator.
//!
//! The core defense against single-block price manipulation: on every
//! observation, accumulate `previous_price * elapsed_seconds` since the prior
//! observation. Dividing the difference between two accumulator snapshots by
//! the elapsed time between them (the windowed TWAP query, tracked
//! separately) yields a time-weighted average that a single manipulated write
//! barely moves, because that write only counts for the time it was actually
//! in effect. This is the same accumulator design used by Uniswap V2-style
//! oracles.
//!
//! ## Trust assumptions
//!
//! Same writer trust as the base feed: the admin is trusted to record honest
//! prices at honest timestamps. The accumulator itself performs no
//! validation — it faithfully integrates whatever it is given. Manipulation
//! resistance comes from *reading it back over a sufficiently long window*
//! (the windowed TWAP query), not from this module alone. A single dishonest
//! observation still corrupts every window that includes it.

use crate::OracleClient;
use crate::{DataKey, Oracle};
use soroban_sdk::{contractimpl, contracttype, Address, Env};

/// A per-asset TWAP accumulator snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct TwapObservation {
    /// Cumulative sum of `price * elapsed_seconds` since the first
    /// observation for this asset.
    pub accumulator: i128,
    /// The price recorded by the most recent observation.
    pub last_price: i128,
    /// Ledger timestamp (seconds) of the most recent observation.
    pub last_timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum TwapKey {
    Observation(Address),
}

#[contractimpl]
impl Oracle {
    /// Record a new price observation for `asset`, extending its TWAP
    /// accumulator by `last_price * elapsed_seconds` since the previous
    /// observation (zero on the first observation, since there is no prior
    /// interval to integrate). Admin-only. Returns the updated accumulator.
    ///
    /// # Panics
    /// - `"price must be non-negative"` if `price` is negative.
    /// - `"twap accumulator overflow"` if the running sum overflows `i128`.
    pub fn record_twap_observation(env: Env, asset: Address, price: i128) -> i128 {
        assert!(price >= 0, "price must be non-negative");
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        let now = env.ledger().timestamp();
        let prev: Option<TwapObservation> = env
            .storage()
            .persistent()
            .get(&TwapKey::Observation(asset.clone()));

        let accumulator = match prev {
            Some(p) => {
                let elapsed = now.saturating_sub(p.last_timestamp);
                let increment = p
                    .last_price
                    .checked_mul(elapsed as i128)
                    .expect("twap accumulator overflow");
                p.accumulator
                    .checked_add(increment)
                    .expect("twap accumulator overflow")
            }
            None => 0,
        };

        let observation = TwapObservation {
            accumulator,
            last_price: price,
            last_timestamp: now,
        };
        env.storage()
            .persistent()
            .set(&TwapKey::Observation(asset), &observation);
        accumulator
    }

    /// Read the current TWAP accumulator snapshot for `asset`.
    ///
    /// # Panics
    /// - `"no twap observation for asset"` if none was ever recorded.
    pub fn get_twap_observation(env: Env, asset: Address) -> TwapObservation {
        env.storage()
            .persistent()
            .get(&TwapKey::Observation(asset))
            .expect("no twap observation for asset")
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
    fn first_observation_starts_at_zero() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        let acc = oracle.record_twap_observation(&xlm, &1_0000000);
        assert_eq!(acc, 0);

        let obs = oracle.get_twap_observation(&xlm);
        assert_eq!(obs.accumulator, 0);
        assert_eq!(obs.last_price, 1_0000000);
        assert_eq!(obs.last_timestamp, 1_000);
    }

    #[test]
    fn accumulates_prior_price_over_elapsed_time() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.record_twap_observation(&xlm, &2_0000000);

        env.ledger().set_timestamp(1_100); // 100 seconds later
        let acc = oracle.record_twap_observation(&xlm, &3_0000000);

        // 100s at the *prior* price of 2.0.
        assert_eq!(acc, 2_0000000 * 100);
    }

    #[test]
    fn multiple_updates_accumulate_correctly() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(0);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.record_twap_observation(&xlm, &1_0000000); // t=0, price 1.0

        env.ledger().set_timestamp(10);
        oracle.record_twap_observation(&xlm, &2_0000000); // t=10, price 2.0

        env.ledger().set_timestamp(30);
        let acc = oracle.record_twap_observation(&xlm, &4_0000000); // t=30

        // [0,10) at 1.0 + [10,30) at 2.0
        let expected = 1_0000000 * 10 + 2_0000000 * 20;
        assert_eq!(acc, expected);
    }

    #[test]
    #[should_panic(expected = "no twap observation for asset")]
    fn unknown_asset_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let unknown = Address::generate(&env);
        oracle.get_twap_observation(&unknown);
    }

    #[test]
    #[should_panic(expected = "twap accumulator overflow")]
    fn overflow_panics() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(0);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.record_twap_observation(&xlm, &i128::MAX);

        env.ledger().set_timestamp(u64::MAX);
        oracle.record_twap_observation(&xlm, &1);
    }
}
