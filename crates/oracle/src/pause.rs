//! Pause switch (read gate).
//!
//! An operational circuit breaker. During an incident — a suspected bad feed, a
//! compromised writer, a downstream emergency — the admin can flip a flag that
//! gated reads consult and refuse to serve.
//!
//! ## Trust assumptions
//!
//! Only the admin can toggle the pause, enforced via `require_auth`. The pause
//! is a *liveness* trade-off: it lets the admin halt a possibly-bad feed at the
//! cost of availability. Consumers that gate on `is_paused` trust the admin to
//! pause honestly and unpause promptly.

use crate::OracleClient;
use crate::{DataKey, Oracle};
use soroban_sdk::{contractimpl, contracttype, Env};

#[derive(Clone)]
#[contracttype]
pub enum PauseKey {
    Paused,
}

#[contractimpl]
impl Oracle {
    /// Halt gated reads. Admin-only.
    pub fn pause(env: Env) {
        Self::require_admin(&env);
        env.storage().instance().set(&PauseKey::Paused, &true);
    }

    /// Resume gated reads. Admin-only.
    pub fn unpause(env: Env) {
        Self::require_admin(&env);
        env.storage().instance().set(&PauseKey::Paused, &false);
    }

    /// Whether the oracle is currently paused. Defaults to `false`.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&PauseKey::Paused)
            .unwrap_or(false)
    }

    /// Read a price, panicking if the oracle is paused.
    ///
    /// # Panics
    /// - `"oracle is paused"` while paused.
    /// - `"no price for asset"` if no price was ever written.
    pub fn get_price_gated(env: Env, asset: soroban_sdk::Address) -> i128 {
        assert!(!Self::is_paused(env.clone()), "oracle is paused");
        Self::get_price(env, asset)
    }

    fn require_admin(env: &Env) {
        let admin: soroban_sdk::Address = env
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
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
        Address, Env, IntoVal,
    };

    fn setup(env: &Env) -> (OracleClient<'_>, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn defaults_unpaused() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        assert!(!oracle.is_paused());
    }

    #[test]
    fn admin_can_toggle_and_state_persists() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        oracle.pause();
        assert!(oracle.is_paused());
        oracle.unpause();
        assert!(!oracle.is_paused());
    }

    #[test]
    fn gated_read_blocks_while_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);

        let xlm = Address::generate(&env);
        oracle.set_price(&xlm, &1_0000000);
        assert_eq!(oracle.get_price_gated(&xlm), 1_0000000);

        oracle.pause();
        let res = oracle.try_get_price_gated(&xlm);
        assert!(res.is_err());
    }

    #[test]
    fn pause_requires_admin_auth() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, admin) = setup(&env);

        oracle.pause();

        // The recorded auth must be the admin invoking `pause`.
        let auths = env.auths();
        assert_eq!(auths.len(), 1);
        let (addr, invocation) = auths.first().unwrap().clone();
        assert_eq!(addr, admin);
        assert_eq!(
            invocation.function,
            AuthorizedFunction::Contract((
                oracle.address.clone(),
                soroban_sdk::Symbol::new(&env, "pause"),
                ().into_val(&env),
            ))
        );
        let _ = AuthorizedInvocation {
            function: invocation.function.clone(),
            sub_invocations: invocation.sub_invocations.clone(),
        };
    }
}
