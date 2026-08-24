//! Min/max sanity-bounds check on write.
//!
//! A fat-finger or buggy writer can publish a wildly wrong number (a misplaced
//! decimal, an unscaled raw value). This module provides a cheap pre-storage
//! guard: reject any candidate price outside a sane absolute range.
//!
//! ## Trust assumptions
//!
//! The caller supplies the bounds, so they are only as good as the policy
//! choosing them. This is a guard against *accidental* extreme values, not a
//! defence against a malicious in-range price. Bounds are inclusive.

use crate::Oracle;
use crate::OracleClient;
use soroban_sdk::{contractimpl, Env};

#[contractimpl]
impl Oracle {
    /// Validate `price` against an inclusive `[min, max]` absolute range.
    ///
    /// Returns `price` unchanged when in range so it can be used inline before a
    /// write.
    ///
    /// # Panics
    /// - `"min must be <= max"` if the bounds are inverted.
    /// - `"price below min bound"` / `"price above max bound"` on violation.
    pub fn check_price_bounds(_env: Env, price: i128, min: i128, max: i128) -> i128 {
        assert!(min <= max, "min must be <= max");
        assert!(price >= min, "price below min bound");
        assert!(price <= max, "price above max bound");
        price
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup(env: &Env) -> OracleClient {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        client
    }

    #[test]
    fn in_range_passes() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        assert_eq!(
            oracle.check_price_bounds(&5_0000000, &1, &10_0000000),
            5_0000000
        );
    }

    #[test]
    fn boundaries_are_inclusive() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        assert_eq!(oracle.check_price_bounds(&1, &1, &10), 1);
        assert_eq!(oracle.check_price_bounds(&10, &1, &10), 10);
    }

    #[test]
    #[should_panic(expected = "price below min bound")]
    fn below_min_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        oracle.check_price_bounds(&0, &1, &10);
    }

    #[test]
    #[should_panic(expected = "price above max bound")]
    fn above_max_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        oracle.check_price_bounds(&11, &1, &10);
    }
}
