//! Heartbeat / required-update-interval tracking.
//!
//! Staleness is a consumer-side judgement; a *heartbeat* is the feed's own
//! promise of how often an asset will be refreshed. This module stores a per-
//! asset required update interval and answers whether the last write still
//! honours it — a clean liveness signal independent of any caller's threshold.
//!
//! ## Trust assumptions
//!
//! The admin sets each heartbeat (`require_auth`) and is trusted to write prices
//! within it. The check is a liveness indicator only: a satisfied heartbeat says
//! the data is recent, not that its *value* is correct.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum HeartbeatKey {
    Heartbeat(Address),
}

#[contractimpl]
impl Oracle {
    /// Set the required update interval (seconds) for `asset`. Admin-only.
    pub fn set_heartbeat(env: Env, asset: Address, interval_secs: u64) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&HeartbeatKey::Heartbeat(asset), &interval_secs);
    }

    /// The configured heartbeat interval for `asset`, if any.
    pub fn get_heartbeat(env: Env, asset: Address) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&HeartbeatKey::Heartbeat(asset))
    }

    /// Whether `asset`'s latest price is within its heartbeat interval.
    ///
    /// Returns `false` if either no price or no heartbeat is configured — an
    /// unconfigured or never-written asset cannot be considered live.
    pub fn is_heartbeat_satisfied(env: Env, asset: Address) -> bool {
        let interval: u64 = match env
            .storage()
            .persistent()
            .get(&HeartbeatKey::Heartbeat(asset.clone()))
        {
            Some(v) => v,
            None => return false,
        };
        let data: PriceData = match env.storage().persistent().get(&DataKey::Price(asset)) {
            Some(d) => d,
            None => return false,
        };
        let age = env.ledger().timestamp().saturating_sub(data.timestamp);
        age <= interval
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
    fn satisfied_within_interval() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_heartbeat(&xlm, &3_600);
        oracle.set_price(&xlm, &1_0000000);

        env.ledger().set_timestamp(1_000 + 3_000);
        assert!(oracle.is_heartbeat_satisfied(&xlm));
    }

    #[test]
    fn unsatisfied_once_lapsed() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_heartbeat(&xlm, &3_600);
        oracle.set_price(&xlm, &1_0000000);

        env.ledger().set_timestamp(1_000 + 3_601);
        assert!(!oracle.is_heartbeat_satisfied(&xlm));
    }

    #[test]
    fn false_without_config_or_price() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        assert!(!oracle.is_heartbeat_satisfied(&xlm));
        oracle.set_heartbeat(&xlm, &60);
        assert!(!oracle.is_heartbeat_satisfied(&xlm)); // still no price
    }
}
