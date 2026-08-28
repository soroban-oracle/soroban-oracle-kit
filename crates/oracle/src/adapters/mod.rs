//! Price-source adapters.
//!
//! Adapters present heterogeneous price origins — a fixed test constant, the
//! oracle's own stored feed, pool reserves, external contracts — behind one
//! uniform shape so consumers can swap sources without rewriting call sites.
//!
//! Every adapter documents its own trust assumptions and manipulation profile;
//! some (e.g. AMM spot) are explicitly *not* safe for direct use in
//! liquidations. See the individual modules.

pub mod amm_spot;
pub mod fixed;
pub mod identity;
pub mod source;
