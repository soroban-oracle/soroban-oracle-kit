//! AMM spot-price adapter (read-only).
//!
//! Derives a *spot* price for a constant-product pool from its two reserve
//! balances: the price of the base token in quote units is
//! `reserve_quote / reserve_base` — the textbook `x*y=k` spot price. It is
//! cheap to compute but trivially manipulable within a single transaction
//! (a flash loan can move reserves and restore them before the block ends).
//!
//! **Do not use this for liquidations or any valuation where a momentarily
//! manipulated price causes real loss.** Prefer a TWAP over the pool once
//! available; this adapter exists for callers that explicitly want a spot
//! read (e.g. as one input into a TWAP) with eyes open to the risk.
//!
//! ## Trust assumptions
//!
//! Reserves are supplied by the caller — typically read from a `soroban-amm`
//! pool contract — and are trusted at face value; this adapter performs no
//! validation beyond rejecting a non-positive base reserve. It inherits
//! whatever manipulation risk the underlying pool has at the moment the
//! reserves were read.

use crate::adapters::source::PriceSource;
use soroban_sdk::{Address, Env};

/// `10^DECIMALS` as the fixed-point scale factor.
const SCALE: i128 = 10_000_000; // 10^7 for DECIMALS = 7

/// Compile-time guard tying [`SCALE`] to the kit's [`crate::DECIMALS`].
const _: () = assert!(crate::DECIMALS == 7, "SCALE assumes DECIMALS == 7");

/// A [`PriceSource`] that derives a constant-product pool's spot price from
/// its two reserve balances.
///
/// **Unsafe for direct use in liquidations or valuations** — see the module
/// docs.
pub struct AmmSpotSource {
    /// Reserve of the priced (base) token.
    pub reserve_base: i128,
    /// Reserve of the quote token.
    pub reserve_quote: i128,
}

impl AmmSpotSource {
    /// Construct a spot-price source over a pool's current reserves.
    pub fn new(reserve_base: i128, reserve_quote: i128) -> Self {
        AmmSpotSource {
            reserve_base,
            reserve_quote,
        }
    }
}

impl PriceSource for AmmSpotSource {
    /// Spot price of the base token in quote units, at `DECIMALS` precision.
    ///
    /// # Panics
    /// - `"empty pool"` if `reserve_base <= 0`.
    /// - `"amm spot overflow"` if the scaled multiplication overflows `i128`.
    fn price(&self, _env: &Env, _asset: &Address) -> i128 {
        assert!(self.reserve_base > 0, "empty pool");
        self.reserve_quote
            .checked_mul(SCALE)
            .expect("amm spot overflow")
            / self.reserve_base
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn computes_spot_price_from_reserves() {
        let env = Env::default();
        let asset = Address::generate(&env);
        // 10 base : 50 quote -> 5.0 quote per base.
        let src = AmmSpotSource::new(10, 50);
        assert_eq!(src.price(&env, &asset), 5_0000000);
    }

    #[test]
    fn handles_fractional_price() {
        let env = Env::default();
        let asset = Address::generate(&env);
        // 100 base : 200 quote -> 2.0 quote per base.
        let src = AmmSpotSource::new(100, 200);
        assert_eq!(src.price(&env, &asset), 2_0000000);
    }

    #[test]
    #[should_panic(expected = "empty pool")]
    fn empty_base_reserve_panics() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let src = AmmSpotSource::new(0, 100);
        src.price(&env, &asset);
    }

    #[test]
    #[should_panic(expected = "amm spot overflow")]
    fn overflow_panics() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let src = AmmSpotSource::new(1, i128::MAX);
        src.price(&env, &asset);
    }
}
