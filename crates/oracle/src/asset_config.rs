//! Per-asset config store.
//!
//! Later safety features (heartbeat, deviation breaker, decimals normalization)
//! all need per-asset settings. Scattering them across ad-hoc storage keys
//! invites drift; this module centralizes them in one admin-gated `AssetConfig`.
//!
//! ## Trust assumptions
//!
//! Only the admin can write config (`require_auth`). Config values are policy
//! parameters, not prices — a wrong heartbeat or deviation bound weakens a guard
//! but does not by itself move the feed. Reads are open.

use crate::{DataKey, Oracle};
use crate::OracleClient;
use soroban_sdk::{contractimpl, contracttype, Address, Env};

/// Per-asset safety/precision settings.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AssetConfig {
    /// Maximum allowed seconds between updates before the asset is "lapsed".
    pub heartbeat_secs: u64,
    /// Maximum allowed single-update move, in basis points (1% = 100 bps).
    pub max_deviation_bps: u32,
    /// Source precision for incoming prices, used by normalization.
    pub decimals: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum ConfigKey {
    Config(Address),
}

#[contractimpl]
impl Oracle {
    /// Store the `config` for `asset`. Admin-only.
    pub fn set_asset_config(env: Env, asset: Address, config: AssetConfig) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&ConfigKey::Config(asset), &config);
    }

    /// Read the config for `asset`.
    ///
    /// # Panics
    /// - `"no config for asset"` if none was set.
    pub fn get_asset_config(env: Env, asset: Address) -> AssetConfig {
        env.storage()
            .persistent()
            .get(&ConfigKey::Config(asset))
            .expect("no config for asset")
    }

    /// Read the config for `asset`, or `None` if unset.
    pub fn get_asset_config_optional(env: Env, asset: Address) -> Option<AssetConfig> {
        env.storage().persistent().get(&ConfigKey::Config(asset))
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

    fn cfg() -> AssetConfig {
        AssetConfig {
            heartbeat_secs: 3_600,
            max_deviation_bps: 500,
            decimals: 8,
        }
    }

    #[test]
    fn config_persists_per_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_asset_config(&xlm, &cfg());
        assert_eq!(oracle.get_asset_config(&xlm), cfg());
    }

    #[test]
    fn missing_config_returns_none() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let xlm = Address::generate(&env);
        assert_eq!(oracle.get_asset_config_optional(&xlm), None);
    }

    #[test]
    fn config_is_isolated_per_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let a = Address::generate(&env);
        let b = Address::generate(&env);
        oracle.set_asset_config(&a, &cfg());
        assert_eq!(oracle.get_asset_config_optional(&b), None);
    }
}
