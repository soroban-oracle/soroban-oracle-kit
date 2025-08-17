//! Asset-count convenience read.
//!
//! Consumers (dashboards, iterators) often want only the *number* of tracked
//! assets, not the whole vector. This read avoids materializing and transferring
//! the full asset set just to call `.len()`.
//!
//! ## Trust assumptions
//!
//! Read-only over admin-curated state; introduces no new trust beyond the base
//! feed.

use crate::{DataKey, Oracle};
use crate::OracleClient;
use soroban_sdk::{contractimpl, Address, Env, Vec};

#[contractimpl]
impl Oracle {
    /// Number of distinct assets that have ever been priced.
    pub fn asset_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Assets)
            .unwrap_or_else(|| Vec::<Address>::new(&env))
            .len()
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
    fn starts_at_zero() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        assert_eq!(oracle.asset_count(), 0);
    }

    #[test]
    fn counts_distinct_assets() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let a = Address::generate(&env);
        let b = Address::generate(&env);
        oracle.set_price(&a, &10);
        oracle.set_price(&b, &20);
        oracle.set_price(&a, &11); // update, not a new asset
        assert_eq!(oracle.asset_count(), 2);
    }
}
