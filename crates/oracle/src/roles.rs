//! Multi-writer role support.
//!
//! The shipped feed has a single writer (the admin). Real deployments often want
//! several authorized publishers (redundant data providers, a rotation of bots)
//! while keeping membership under admin control. This module adds an admin-
//! managed writer set and a check usable by write paths.
//!
//! ## Trust assumptions
//!
//! Every writer in the set is *fully* trusted to publish correct prices — adding
//! a writer widens the trusted surface, so the admin must vet each one. Only the
//! admin can add or remove writers (`require_auth`). This module does not by
//! itself aggregate or cross-check writer submissions; combine with median
//! aggregation for defence against a single bad writer.

use crate::OracleClient;
use crate::{DataKey, Oracle};
use soroban_sdk::{contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum RoleKey {
    Writer(Address),
}

#[contractimpl]
impl Oracle {
    /// Authorize `writer` to publish prices. Admin-only. Idempotent.
    pub fn add_writer(env: Env, writer: Address) {
        Self::require_admin_role(&env);
        env.storage()
            .persistent()
            .set(&RoleKey::Writer(writer), &true);
    }

    /// Revoke `writer`'s publish authorization. Admin-only. Idempotent.
    pub fn remove_writer(env: Env, writer: Address) {
        Self::require_admin_role(&env);
        env.storage().persistent().remove(&RoleKey::Writer(writer));
    }

    /// Whether `writer` is currently authorized to publish.
    pub fn is_writer(env: Env, writer: Address) -> bool {
        env.storage()
            .persistent()
            .get(&RoleKey::Writer(writer))
            .unwrap_or(false)
    }

    /// Require that `writer` is authorized *and* has signed this call.
    ///
    /// # Panics
    /// - `"not an authorized writer"` if `writer` is not in the set.
    pub fn require_writer(env: Env, writer: Address) {
        assert!(
            Self::is_writer(env.clone(), writer.clone()),
            "not an authorized writer"
        );
        writer.require_auth();
    }

    fn require_admin_role(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
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
    fn add_and_check_writer() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let w = Address::generate(&env);
        assert!(!oracle.is_writer(&w));
        oracle.add_writer(&w);
        assert!(oracle.is_writer(&w));
    }

    #[test]
    fn remove_revokes_writer() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let w = Address::generate(&env);
        oracle.add_writer(&w);
        oracle.remove_writer(&w);
        assert!(!oracle.is_writer(&w));
    }

    #[test]
    fn require_writer_passes_for_authorized() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let w = Address::generate(&env);
        oracle.add_writer(&w);
        oracle.require_writer(&w); // does not panic
    }

    #[test]
    fn require_writer_rejects_unauthorized() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let stranger = Address::generate(&env);
        let res = oracle.try_require_writer(&stranger);
        assert!(res.is_err());
    }
}
