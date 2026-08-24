//! Price-deviation circuit breaker.
//!
//! A single write that jumps the price far from its previous value is the
//! signature of manipulation or a bad data point. This breaker compares a
//! candidate against the last stored price and rejects moves beyond a configured
//! basis-point threshold, blunting single-write manipulation.
//!
//! ## Trust assumptions
//!
//! The threshold is a policy parameter the admin chooses; too tight blocks
//! legitimate volatility, too loose lets manipulation through. The breaker only
//! constrains *change* — it cannot validate the first price, and a patient
//! attacker could still walk the price in within-threshold steps. Combine with
//! TWAP/median for stronger defence.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, Address, Env};

const BPS_DENOMINATOR: i128 = 10_000;

#[contractimpl]
impl Oracle {
    /// Whether moving from `old_price` to `new_price` stays within
    /// `max_deviation_bps` basis points of `old_price`.
    ///
    /// A non-positive `old_price` (no meaningful baseline) is treated as always
    /// within tolerance. Uses checked arithmetic on the absolute delta.
    pub fn is_within_deviation(
        _env: Env,
        old_price: i128,
        new_price: i128,
        max_deviation_bps: u32,
    ) -> bool {
        if old_price <= 0 {
            return true;
        }
        let delta = (new_price - old_price).unsigned_abs() as i128;
        // delta / old_price <= bps / 10_000  <=>  delta * 10_000 <= bps * old_price
        let lhs = delta
            .checked_mul(BPS_DENOMINATOR)
            .expect("deviation overflow");
        let rhs = (max_deviation_bps as i128)
            .checked_mul(old_price)
            .expect("deviation overflow");
        lhs <= rhs
    }

    /// Publish `price` for `asset` only if it is within `max_deviation_bps` of
    /// the currently stored price. Admin-only.
    ///
    /// If no price exists yet, the write is accepted unconditionally (there is
    /// no baseline to deviate from).
    ///
    /// # Panics
    /// - `"deviation too large"` if the move exceeds the threshold.
    pub fn set_price_guarded(env: Env, asset: Address, price: i128, max_deviation_bps: u32) {
        assert!(price >= 0, "price must be non-negative");
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        if let Some(prev) = env
            .storage()
            .persistent()
            .get::<_, PriceData>(&DataKey::Price(asset.clone()))
        {
            assert!(
                Self::is_within_deviation(env.clone(), prev.price, price, max_deviation_bps),
                "deviation too large"
            );
        } else {
            // First price for this asset: register it in the asset set.
            let mut assets: soroban_sdk::Vec<Address> = env
                .storage()
                .instance()
                .get(&DataKey::Assets)
                .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
            assets.push_back(asset.clone());
            env.storage().instance().set(&DataKey::Assets, &assets);
        }

        // Write inline rather than delegating to `set_price`, which would
        // re-authorize the admin in the same frame and trip the host.
        let data = PriceData {
            price,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Price(asset), &data);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup(env: &Env) -> (OracleClient<'_>, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn within_threshold_passes() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        // 4% move, 5% (500 bps) threshold.
        assert!(oracle.is_within_deviation(&1_0000000, &1_0400000, &500));
    }

    #[test]
    fn over_threshold_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        // 6% move, 5% threshold.
        assert!(!oracle.is_within_deviation(&1_0000000, &1_0600000, &500));
    }

    #[test]
    fn guarded_write_rejects_jump() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price_guarded(&xlm, &1_0000000, &500); // first write, no baseline
        oracle.set_price_guarded(&xlm, &1_0400000, &500); // 4%, ok
        assert_eq!(oracle.get_price(&xlm), 1_0400000);

        let res = oracle.try_set_price_guarded(&xlm, &2_0000000, &500); // huge jump
        assert!(res.is_err());
        assert_eq!(oracle.get_price(&xlm), 1_0400000); // unchanged
    }

    #[test]
    fn no_baseline_accepts_first_write() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let xlm = Address::generate(&env);
        oracle.set_price_guarded(&xlm, &123_0000000, &10);
        assert_eq!(oracle.get_price(&xlm), 123_0000000);
    }
}
