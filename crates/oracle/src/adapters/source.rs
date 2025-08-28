//! Adapter trait definition.
//!
//! [`PriceSource`] is the common shape every adapter implements: given the
//! environment and an asset, produce a `DECIMALS`-scaled price. Keeping the
//! family behind one trait lets wrappers (staleness filter, bounds clamp,
//! median-of-sources) compose sources uniformly.
//!
//! ## Freshness contract
//!
//! A `PriceSource` returns the *most recent price it knows*; it makes **no**
//! freshness promise on its own. Callers that require fresh data must wrap the
//! source in a staleness-enforcing adapter or check age separately. Returning a
//! stale price is allowed; returning a knowingly-wrong price is not.
//!
//! ## Trust assumptions
//!
//! Each implementation inherits the trust of its underlying origin and must
//! document it. The trait itself adds no trust — it only standardizes the call.
//!
//! ```ignore
//! use oracle::adapters::source::PriceSource;
//! use soroban_sdk::{Address, Env};
//!
//! struct AlwaysOne;
//! impl PriceSource for AlwaysOne {
//!     fn price(&self, _env: &Env, _asset: &Address) -> i128 {
//!         1_0000000 // 1.0 at DECIMALS = 7
//!     }
//! }
//! ```

use soroban_sdk::{Address, Env};

/// A uniform read interface over a single price origin.
pub trait PriceSource {
    /// Latest known `DECIMALS`-scaled price for `asset`.
    ///
    /// Implementations may panic if they cannot produce a price (e.g. the
    /// asset is unknown to the underlying origin). They make no freshness
    /// guarantee — see the module docs.
    fn price(&self, env: &Env, asset: &Address) -> i128;
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    struct Constant(i128);
    impl PriceSource for Constant {
        fn price(&self, _env: &Env, _asset: &Address) -> i128 {
            self.0
        }
    }

    #[test]
    fn trait_is_callable() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let src = Constant(7_0000000);
        assert_eq!(src.price(&env, &asset), 7_0000000);
    }

    #[test]
    fn can_be_used_via_dyn() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let src = Constant(3_0000000);
        let dyn_src: &dyn PriceSource = &src;
        assert_eq!(dyn_src.price(&env, &asset), 3_0000000);
    }
}
