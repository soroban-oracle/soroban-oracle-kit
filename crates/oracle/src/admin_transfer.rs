//! Two-step admin transfer.
//!
//! A one-shot admin handoff is dangerous: a typo'd or dead address permanently
//! bricks control of the feed. This module replaces it with a propose/accept
//! flow — the current admin nominates a successor, and the transfer completes
//! only when that successor actively accepts.
//!
//! ## Trust assumptions
//!
//! The current admin is trusted until acceptance; afterwards the new admin holds
//! full write/control authority. Both steps require the respective party's auth,
//! so neither the proposal nor the acceptance can be forged. A pending proposal
//! can be overwritten by the current admin before acceptance.

use crate::OracleClient;
use crate::{DataKey, Oracle};
use soroban_sdk::{contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum AdminTransferKey {
    PendingAdmin,
}

#[contractimpl]
impl Oracle {
    /// Nominate `new_admin` as the pending successor. Current-admin-only.
    pub fn propose_admin(env: Env, new_admin: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
        env.storage()
            .instance()
            .set(&AdminTransferKey::PendingAdmin, &new_admin);
    }

    /// Accept a pending nomination, becoming the new admin. Pending-admin-only.
    ///
    /// # Panics
    /// - `"no pending admin"` if no proposal is outstanding.
    pub fn accept_admin(env: Env) {
        let pending: Address = env
            .storage()
            .instance()
            .get(&AdminTransferKey::PendingAdmin)
            .expect("no pending admin");
        pending.require_auth();
        env.storage().instance().set(&DataKey::Admin, &pending);
        env.storage()
            .instance()
            .remove(&AdminTransferKey::PendingAdmin);
    }

    /// The currently pending successor, if any.
    pub fn pending_admin(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&AdminTransferKey::PendingAdmin)
    }

    /// The current admin.
    pub fn current_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized")
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
    fn transfer_completes_after_acceptance() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, admin) = setup(&env);

        let new_admin = Address::generate(&env);
        oracle.propose_admin(&new_admin);
        assert_eq!(oracle.pending_admin(), Some(new_admin.clone()));
        assert_eq!(oracle.current_admin(), admin);

        oracle.accept_admin();
        assert_eq!(oracle.current_admin(), new_admin);
        assert_eq!(oracle.pending_admin(), None);
    }

    #[test]
    fn accept_without_proposal_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let res = oracle.try_accept_admin();
        assert!(res.is_err());
    }

    #[test]
    fn new_admin_can_set_price_after_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let new_admin = Address::generate(&env);
        oracle.propose_admin(&new_admin);
        oracle.accept_admin();

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &1_0000000);
        assert_eq!(oracle.get_price(&xlm), 1_0000000);
    }
}
