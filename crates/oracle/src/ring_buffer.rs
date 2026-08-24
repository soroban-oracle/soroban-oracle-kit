//! Last-N price ring buffer.
//!
//! A bounded history of recent prices per asset is the substrate that median,
//! SMA, EMA, and TWAP-over-samples all read from. This module stores a fixed-
//! capacity circular buffer of recent [`PriceData`] per asset, overwriting the
//! oldest entry once full.
//!
//! ## Trust assumptions
//!
//! Samples inherit the trust of whatever wrote them (admin/writer). A ring
//! buffer of spot samples is only as manipulation-resistant as the aggregation
//! applied on top — keeping history does not by itself defend against a writer
//! pushing crafted values.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, contracttype, Address, Env, Vec};

/// Maximum number of samples retained per asset.
pub const RING_CAPACITY: u32 = 8;

#[derive(Clone)]
#[contracttype]
pub enum RingKey {
    Ring(Address),
}

#[contractimpl]
impl Oracle {
    /// Append `price` (at the current ledger timestamp) to `asset`'s ring
    /// buffer, evicting the oldest sample once capacity is reached. Admin-only.
    pub fn push_sample(env: Env, asset: Address, price: i128) {
        assert!(price >= 0, "price must be non-negative");
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        let mut ring: Vec<PriceData> = env
            .storage()
            .persistent()
            .get(&RingKey::Ring(asset.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let sample = PriceData {
            price,
            timestamp: env.ledger().timestamp(),
        };
        ring.push_back(sample);
        // Evict from the front until within capacity (preserves chronological
        // order: index 0 is oldest, last is newest).
        while ring.len() > RING_CAPACITY {
            ring.remove(0);
        }
        env.storage().persistent().set(&RingKey::Ring(asset), &ring);
    }

    /// The retained samples for `asset`, oldest first. Empty if none.
    pub fn samples(env: Env, asset: Address) -> Vec<PriceData> {
        env.storage()
            .persistent()
            .get(&RingKey::Ring(asset))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Number of retained samples for `asset`.
    pub fn sample_count(env: Env, asset: Address) -> u32 {
        Self::samples(env, asset).len()
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
    fn pushes_preserve_order() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.push_sample(&xlm, &1);
        oracle.push_sample(&xlm, &2);
        oracle.push_sample(&xlm, &3);

        let s = oracle.samples(&xlm);
        assert_eq!(s.len(), 3);
        assert_eq!(s.get(0).unwrap().price, 1);
        assert_eq!(s.get(2).unwrap().price, 3);
    }

    #[test]
    fn wraps_at_capacity() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        for i in 0..(RING_CAPACITY + 3) {
            oracle.push_sample(&xlm, &(i as i128));
        }

        let s = oracle.samples(&xlm);
        assert_eq!(s.len(), RING_CAPACITY);
        // Oldest retained should be (total - capacity) = 3, newest = cap+2.
        assert_eq!(s.get(0).unwrap().price, 3);
        assert_eq!(
            s.get(RING_CAPACITY - 1).unwrap().price,
            (RING_CAPACITY + 2) as i128
        );
    }

    #[test]
    fn empty_for_unknown_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let unknown = Address::generate(&env);
        assert_eq!(oracle.sample_count(&unknown), 0);
    }
}
