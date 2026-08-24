//! Reject zero price on write.
//!
//! A zero price silently breaks downstream valuations: any consumer dividing by
//! it traps, and any ratio against it explodes. Rejecting `0` at the boundary is
//! a tiny, high-value guard.
//!
//! ## Trust assumptions
//!
//! This guards against an accidental or degenerate zero only. A non-zero but
//! wrong price is out of scope — combine with sanity bounds and deviation checks
//! for fuller protection.

use crate::Oracle;
use crate::OracleClient;
use soroban_sdk::{contractimpl, Env};

#[contractimpl]
impl Oracle {
    /// Return `price` unchanged, panicking if it is zero.
    ///
    /// # Panics
    /// - `"price must be non-zero"` if `price == 0`.
    pub fn require_nonzero_price(_env: Env, price: i128) -> i128 {
        assert!(price != 0, "price must be non-zero");
        price
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup(env: &Env) -> OracleClient<'_> {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        client
    }

    #[test]
    fn positive_price_passes() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        assert_eq!(oracle.require_nonzero_price(&1), 1);
        assert_eq!(oracle.require_nonzero_price(&9_9999999), 9_9999999);
    }

    #[test]
    #[should_panic(expected = "price must be non-zero")]
    fn zero_price_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        oracle.require_nonzero_price(&0);
    }
}
