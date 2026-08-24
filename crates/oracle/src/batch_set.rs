//! Batch `set_price`.
//!
//! Publishing many assets in separate transactions is costly and produces an
//! inconsistent snapshot (different assets carry different timestamps). This
//! module lets a writer publish a whole set in one call, all sharing a single
//! timestamp, so consumers reading the batch see a coherent moment in time.
//!
//! ## Trust assumptions
//!
//! Same writer trust as the base feed — the admin signs the batch. Atomicity is
//! provided by the host: if the call panics partway, no entry is committed. A
//! single shared timestamp means a slow batch does not make later entries look
//! fresher than earlier ones.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, Address, Env, Vec};

#[contractimpl]
impl Oracle {
    /// Publish each `(asset, price)` in `entries` with one shared timestamp.
    /// Admin-only.
    ///
    /// # Panics
    /// - `"price must be non-negative"` if any price is negative (nothing is
    ///   written because the host rolls back the call).
    pub fn set_prices(env: Env, entries: Vec<(Address, i128)>) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        let timestamp = env.ledger().timestamp();
        let mut assets: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Assets)
            .unwrap_or_else(|| Vec::new(&env));

        for entry in entries.iter() {
            let (asset, price) = entry;
            assert!(price >= 0, "price must be non-negative");

            if !env
                .storage()
                .persistent()
                .has(&DataKey::Price(asset.clone()))
            {
                assets.push_back(asset.clone());
            }

            let data = PriceData { price, timestamp };
            env.storage()
                .persistent()
                .set(&DataKey::Price(asset), &data);
        }

        env.storage().instance().set(&DataKey::Assets, &assets);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        vec, Env,
    };

    fn setup(env: &Env) -> (OracleClient, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn writes_all_entries_with_shared_timestamp() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(4_242);
        let (oracle, _admin) = setup(&env);

        let a = Address::generate(&env);
        let b = Address::generate(&env);
        oracle.set_prices(&vec![&env, (a.clone(), 1_0000000), (b.clone(), 2_0000000)]);

        assert_eq!(oracle.get_price(&a), 1_0000000);
        assert_eq!(oracle.get_price(&b), 2_0000000);
        assert_eq!(oracle.get_price_data(&a).timestamp, 4_242);
        assert_eq!(oracle.get_price_data(&b).timestamp, 4_242);
        assert_eq!(oracle.assets().len(), 2);
    }

    #[test]
    fn batch_updates_do_not_duplicate_assets() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let a = Address::generate(&env);
        oracle.set_price(&a, &1);
        oracle.set_prices(&vec![&env, (a.clone(), 9_0000000)]);
        assert_eq!(oracle.assets().len(), 1);
        assert_eq!(oracle.get_price(&a), 9_0000000);
    }

    #[test]
    fn negative_price_rejected_atomically() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let a = Address::generate(&env);
        let b = Address::generate(&env);
        let res = oracle.try_set_prices(&vec![&env, (a.clone(), 1_0000000), (b.clone(), -5)]);
        assert!(res.is_err());
        // Rolled back: neither asset was committed.
        assert!(!oracle.has_price(&a));
        assert!(!oracle.has_price(&b));
    }
}
