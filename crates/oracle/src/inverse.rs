//! Inverse-price helper.
//!
//! Given a price for pair A/B (units of B per A), consumers frequently need the
//! reciprocal B/A. Doing this by hand invites decimal-scaling bugs, so this
//! module centralizes one overflow-safe fixed-point inversion.
//!
//! For a price `p` carrying `DECIMALS` implied decimals, the inverse is
//! `10^(2*DECIMALS) / p`, which itself carries `DECIMALS` implied decimals.
//!
//! ## Trust assumptions
//!
//! Pure arithmetic over an already-trusted price; introduces no new trust. The
//! only failure modes are a zero input (undefined reciprocal) and integer
//! truncation, both handled explicitly.

use crate::OracleClient;
use crate::{Oracle, DECIMALS};
use soroban_sdk::{contractimpl, Env};

/// `10^(2 * DECIMALS)` as the fixed-point numerator for inversion.
const SCALE_SQUARED: i128 = 100_000_000_000_000; // 10^14 for DECIMALS = 7

/// Compile-time guard tying [`SCALE_SQUARED`] to the kit's [`DECIMALS`].
const _: () = assert!(DECIMALS == 7, "SCALE_SQUARED assumes DECIMALS == 7");

#[contractimpl]
impl Oracle {
    /// Compute the reciprocal of a `DECIMALS`-scaled `price`.
    ///
    /// The result is truncated toward zero (integer division), so a round-trip
    /// inversion is accurate only to within one unit in the last place.
    ///
    /// # Panics
    /// - `"cannot invert zero price"` if `price == 0`.
    pub fn inverse_price(_env: Env, price: i128) -> i128 {
        assert!(price != 0, "cannot invert zero price");
        // 10^(2*DECIMALS) fits in i128 with room to spare; the division cannot
        // overflow because |result| <= SCALE_SQUARED.
        SCALE_SQUARED
            .checked_div(price)
            .expect("inverse division overflow")
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
    fn inverse_of_one_is_one() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        // 1.0 -> 1.0
        assert_eq!(oracle.inverse_price(&1_0000000), 1_0000000);
    }

    #[test]
    fn inverse_of_two_is_half() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        // 2.0 -> 0.5
        assert_eq!(oracle.inverse_price(&2_0000000), 0_5000000);
    }

    #[test]
    fn round_trip_within_precision() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        let p = 3_3333333; // ~3.3333333
        let back = oracle.inverse_price(&oracle.inverse_price(&p));
        // Round-trip should be within 1 ulp of the original.
        assert!((back - p).abs() <= 1);
    }

    #[test]
    #[should_panic(expected = "cannot invert zero price")]
    fn zero_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        oracle.inverse_price(&0);
    }
}
