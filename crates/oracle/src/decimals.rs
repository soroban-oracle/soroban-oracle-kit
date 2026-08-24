//! Cross-decimals price normalization.
//!
//! Source feeds report prices at their own precision (6, 8, 18 decimals…). To
//! store everything at the kit's `DECIMALS = 7` convention, prices must be
//! scaled up or down. Doing this inline is the classic off-by-`10^n` footgun, so
//! this module centralizes overflow-checked scaling.
//!
//! ## Trust assumptions
//!
//! Pure arithmetic over an already-trusted price; introduces no new trust.
//! Scaling *down* truncates toward zero and is therefore lossy below the target
//! precision — round-trips are exact only when the original precision is >= the
//! intermediate one used. Scaling *up* is checked against `i128` overflow.

use crate::OracleClient;
use crate::{Oracle, DECIMALS};
use soroban_sdk::{contractimpl, Env};

#[contractimpl]
impl Oracle {
    /// Normalize `price` expressed with `from_decimals` implied decimals to the
    /// kit's `DECIMALS` convention.
    ///
    /// # Panics
    /// - `"decimals too large"` if a difference exceeds 38 (beyond `i128` range).
    /// - `"normalize overflow"` if scaling up overflows `i128`.
    pub fn normalize_decimals(_env: Env, price: i128, from_decimals: u32) -> i128 {
        if from_decimals == DECIMALS {
            price
        } else if from_decimals < DECIMALS {
            let diff = DECIMALS - from_decimals;
            let factor = pow10(diff);
            price.checked_mul(factor).expect("normalize overflow")
        } else {
            let diff = from_decimals - DECIMALS;
            let factor = pow10(diff);
            price / factor
        }
    }

    /// Convert a `DECIMALS`-scaled `price` back to `to_decimals` precision.
    ///
    /// Inverse of [`Oracle::normalize_decimals`]. Scaling down truncates.
    pub fn denormalize_decimals(_env: Env, price: i128, to_decimals: u32) -> i128 {
        if to_decimals == DECIMALS {
            price
        } else if to_decimals > DECIMALS {
            let diff = to_decimals - DECIMALS;
            let factor = pow10(diff);
            price.checked_mul(factor).expect("normalize overflow")
        } else {
            let diff = DECIMALS - to_decimals;
            let factor = pow10(diff);
            price / factor
        }
    }
}

/// `10^exp` as an `i128`, panicking past the representable range.
fn pow10(exp: u32) -> i128 {
    assert!(exp <= 38, "decimals too large");
    let mut acc: i128 = 1;
    for _ in 0..exp {
        acc = acc.checked_mul(10).expect("normalize overflow");
    }
    acc
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
    fn same_decimals_is_identity() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        assert_eq!(oracle.normalize_decimals(&1_2345678, &7), 1_2345678);
    }

    #[test]
    fn scale_up_from_six() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        // 1.234567 at 6dp -> 1.2345670 at 7dp
        assert_eq!(oracle.normalize_decimals(&1_234567, &6), 1_2345670);
    }

    #[test]
    fn scale_down_from_eight_truncates() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        // 1.23456789 at 8dp -> 1.2345678 at 7dp (last digit dropped)
        assert_eq!(oracle.normalize_decimals(&1_23456789, &8), 1_2345678);
    }

    #[test]
    fn round_trip_at_or_above_target() {
        let env = Env::default();
        env.mock_all_auths();
        let oracle = setup(&env);
        let original = 5_000000; // 6dp
        let norm = oracle.normalize_decimals(&original, &6);
        let back = oracle.denormalize_decimals(&norm, &6);
        assert_eq!(back, original);
    }
}
