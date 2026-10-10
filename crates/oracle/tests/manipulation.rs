/* Authorized Protocol Quality Assurance & Formal Verification Test Suite */

//! Price-Manipulation Resistance Test Suite.
//!
//! # Objective
//! Concretely verify that the Time-Weighted Average Price (TWAP) accumulator
//! resists adverse single-block price spikes and flash distortions that spot
//! pricing directly falls for.
//!
//! # Threat Model & Invariant Formal Verification
//! In decentralized protocols (lending, synthetic assets, liquidations), relying
//! directly on instantaneous spot prices exposes the system to adverse liquidity
//! distortions (e.g., flash-loan imbalances, low-liquidity pool draining, or
//! anomalous transaction sequencing).
//!
//! By accumulating `price * elapsed_time`, the TWAP accumulator ensures that a
//! single-block price distortion has negligible mathematical weight when
//! evaluated across a time window (e.g., 30 minutes / 1800 seconds).
//!
//! ## Invariant
//! Let $W$ be the query window duration, $\Delta t_{spike}$ be the spike duration
//! (typically 1 block, ~5 seconds), $P_{fair}$ be the fair market price, and
//! $P_{spike}$ be the manipulated price.
//!
//! $$\Delta P_{twap} = \frac{\Delta t_{spike}}{W} \times (P_{spike} - P_{fair})$$
//!
//! For $\Delta t_{spike} = 5\text{s}$ and $W = 1800\text{s}$:
//! The effective price impact is dampened by a factor of $\frac{1800}{5} = 360\times$.

use oracle::{Oracle, OracleClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env,
};

/// Implied decimal precision (7 decimals matching Stellar / Soroban standard).
const DECIMALS: i128 = 10_000_000;

/// Helper function to compute the windowed TWAP between two accumulator snapshots.
///
/// In Uniswap V2 / Soroban TWAP architecture:
/// $$\text{TWAP} = \frac{A_{end} - A_{start}}{t_{end} - t_{start}}$$
fn compute_windowed_twap(start_acc: i128, start_time: u64, end_acc: i128, end_time: u64) -> i128 {
    let elapsed = (end_time - start_time) as i128;
    assert!(elapsed > 0, "Window duration must be strictly positive");
    (end_acc - start_acc) / elapsed
}

fn setup_oracle(env: &Env) -> (OracleClient<'_>, Address) {
    let admin = Address::generate(env);
    let id = env.register_contract(None, Oracle);
    let client = OracleClient::new(env, &id);
    client.init(&admin);
    (client, admin)
}

#[test]
fn single_block_price_spike_demonstrates_twap_dampening() {
    let env = Env::default();
    env.mock_all_auths();
    let (oracle, _admin) = setup_oracle(&env);
    let asset = Address::generate(&env);

    // Baseline: Normal fair market price is 100.00 XLM/USD (100 * 10^7)
    let fair_price = 100 * DECIMALS;
    let window_duration: u64 = 1_800; // 30-minute window (1,800 seconds)
    let block_time: u64 = 5; // 1 Stellar ledger/block duration (~5 seconds)

    // 1. Initial observation at t = 1,000
    let t_start = 1_000;
    env.ledger().set_timestamp(t_start);
    oracle.set_price(&asset, &fair_price);
    let initial_acc = oracle.record_twap_observation(&asset, &fair_price);
    assert_eq!(initial_acc, 0, "First observation accumulator starts at 0");

    // 2. Steady state trading for 1,795 seconds (up to t = 2,795)
    let t_spike_start = t_start + window_duration - block_time; // 2,795
    env.ledger().set_timestamp(t_spike_start);
    // Price remained at fair_price throughout this interval
    oracle.set_price(&asset, &fair_price);

    // 3. Adverse flash distortion: 10x single-block spike (100.00 -> 1000.00 XLM/USD)
    // A +900% price spike is injected into the feed
    let spike_price = 1_000 * DECIMALS;
    oracle.set_price(&asset, &spike_price);
    oracle.record_twap_observation(&asset, &spike_price);

    // Spot price immediately reflects the manipulated price (10x jump)
    let spot_during_spike = oracle.get_price(&asset);
    assert_eq!(
        spot_during_spike, spike_price,
        "Spot price immediately collapses to the manipulated spike"
    );

    // 4. Exactly 1 block later (t = 2,800), market arbitrage / correction restores fair price
    let t_end = t_start + window_duration; // 2,800
    env.ledger().set_timestamp(t_end);
    oracle.set_price(&asset, &fair_price);
    let end_acc = oracle.record_twap_observation(&asset, &fair_price);

    // 5. Measure TWAP across the 1,800-second window
    let twap = compute_windowed_twap(initial_acc, t_start, end_acc, t_end);

    // Expected Accumulator Calculation:
    // Interval 1: [1000, 2795] = 1,795 seconds at 100.00 = 179,500 * 10^7
    // Interval 2: [2795, 2800] = 5 seconds at 1,000.00  = 5,000 * 10^7
    // Total accumulated = 184,500 * 10^7
    // TWAP = 184,500 * 10^7 / 1,800 = 102.50 * 10^7 (102.5000000)
    let expected_twap = 102 * DECIMALS + (DECIMALS / 2); // 102.50
    assert_eq!(twap, expected_twap);

    // Spot price jumped by +900% (from 100.00 to 1,000.00)
    let spot_percent_increase = ((spot_during_spike - fair_price) * 100) / fair_price;
    assert_eq!(spot_percent_increase, 900);

    // TWAP only moved by +2.5% (from 100.00 to 102.50)
    let twap_delta = twap - fair_price;
    let expected_twap_delta = 2 * DECIMALS + (DECIMALS / 2); // +2.50
    assert_eq!(twap_delta, expected_twap_delta);

    // Verification of dampening factor:
    // (1000 - 100) / (102.50 - 100) = 900 / 2.50 = 360x dampening
    let spot_delta = spot_during_spike - fair_price;
    let dampening_factor = spot_delta / twap_delta;
    assert_eq!(
        dampening_factor, 360,
        "TWAP must dampen single-block price distortion by exactly 360x over 30min"
    );
}

#[test]
fn instant_intra_block_manipulation_zero_divergence() {
    let env = Env::default();
    env.mock_all_auths();
    let (oracle, _admin) = setup_oracle(&env);
    let asset = Address::generate(&env);

    let base_price = 50 * DECIMALS;

    // Initialize at t = 0
    env.ledger().set_timestamp(0);
    oracle.set_price(&asset, &base_price);
    let acc_0 = oracle.record_twap_observation(&asset, &base_price);

    // Advance to t = 600 (10 minutes)
    env.ledger().set_timestamp(600);

    // Adverse actor attempts a massive 100x instant price spike in the current block
    let extreme_spike = 5_000 * DECIMALS; // 100x spike
    oracle.set_price(&asset, &extreme_spike);
    let acc_600 = oracle.record_twap_observation(&asset, &extreme_spike);

    // Spot price has already been corrupted to 5,000.00
    assert_eq!(oracle.get_price(&asset), extreme_spike);

    // But TWAP over [0, 600] accumulated the price BEFORE the spike:
    // Elapsed duration at spike price is 0 seconds in this observation.
    let twap_at_spike = compute_windowed_twap(acc_0, 0, acc_600, 600);

    // Invariant: TWAP has exactly 0% divergence at the moment of manipulation
    assert_eq!(
        twap_at_spike, base_price,
        "TWAP must exhibit zero divergence within the spike block"
    );
}

#[test]
fn flash_crash_downward_spike_resistance() {
    let env = Env::default();
    env.mock_all_auths();
    let (oracle, _admin) = setup_oracle(&env);
    let asset = Address::generate(&env);

    // Baseline: Normal collateral price is 200.00 (e.g., collateral token)
    let fair_price = 200 * DECIMALS;
    let window_duration: u64 = 1_800; // 30 minutes
    let block_time: u64 = 5; // 5-second block

    let t_start = 10_000;
    env.ledger().set_timestamp(t_start);
    oracle.set_price(&asset, &fair_price);
    let initial_acc = oracle.record_twap_observation(&asset, &fair_price);

    // Steady until single block before window end
    let t_crash = t_start + window_duration - block_time;
    env.ledger().set_timestamp(t_crash);

    // Flash-crash attack: price artificially dumped by 90% (200.00 -> 20.00)
    // In protocols relying on spot, this would trigger cascading unfair liquidations
    let crashed_price = 20 * DECIMALS;
    oracle.set_price(&asset, &crashed_price);
    oracle.record_twap_observation(&asset, &crashed_price);

    assert_eq!(
        oracle.get_price(&asset),
        crashed_price,
        "Spot price dropped by 90%"
    );

    // Restoration after 1 block
    let t_end = t_start + window_duration;
    env.ledger().set_timestamp(t_end);
    oracle.set_price(&asset, &fair_price);
    let end_acc = oracle.record_twap_observation(&asset, &fair_price);

    let twap = compute_windowed_twap(initial_acc, t_start, end_acc, t_end);

    // Expected Accumulator:
    // 1795s * 200.00 = 359,000 * 10^7
    // 5s * 20.00     = 100 * 10^7
    // Total = 359,100 * 10^7
    // TWAP = 359,100 / 1800 = 199.50 * 10^7 (199.50)
    let expected_twap = 199 * DECIMALS + (DECIMALS / 2); // 199.50
    assert_eq!(twap, expected_twap);

    // Spot dropped by 90%, but TWAP only dropped by 0.25% (0.50 out of 200)
    let twap_drop = fair_price - twap;
    let expected_drop = DECIMALS / 2; // 0.50
    assert_eq!(twap_drop, expected_drop);

    // Dampening factor: (200 - 20) / (200 - 199.50) = 180 / 0.50 = 360x
    let spot_drop = fair_price - crashed_price;
    assert_eq!(spot_drop / twap_drop, 360);
}

#[test]
fn multi_window_recovery_after_manipulation() {
    let env = Env::default();
    env.mock_all_auths();
    let (oracle, _admin) = setup_oracle(&env);
    let asset = Address::generate(&env);

    let fair_price = 100 * DECIMALS;
    let spike_price = 1_000 * DECIMALS;

    // Start at t = 0
    env.ledger().set_timestamp(0);
    oracle.set_price(&asset, &fair_price);
    let acc_0 = oracle.record_twap_observation(&asset, &fair_price);

    // Spike at t = 1795 for 5 seconds
    env.ledger().set_timestamp(1_795);
    oracle.set_price(&asset, &spike_price);
    oracle.record_twap_observation(&asset, &spike_price);

    // Price restored at t = 1800
    env.ledger().set_timestamp(1_800);
    oracle.set_price(&asset, &fair_price);
    let acc_1800 = oracle.record_twap_observation(&asset, &fair_price);

    let twap_1800 = compute_windowed_twap(acc_0, 0, acc_1800, 1_800);
    assert_eq!(twap_1800, 102 * DECIMALS + (DECIMALS / 2)); // 102.50

    // After another 1,800 seconds of fair trading (t = 3600)
    env.ledger().set_timestamp(3_600);
    let acc_3600 = oracle.record_twap_observation(&asset, &fair_price);

    // Window [0, 3600]: 1 hour window
    let twap_3600 = compute_windowed_twap(acc_0, 0, acc_3600, 3_600);

    // Accumulator at 3600:
    // 184,500 * 10^7 + 1800 * 100 * 10^7 = 364,500 * 10^7
    // TWAP = 364,500 / 3,600 = 101.25 * 10^7 (101.25)
    let expected_twap_3600 = 101 * DECIMALS + (DECIMALS / 4); // 101.25
    assert_eq!(twap_3600, expected_twap_3600);

    // Moving window [1800, 3600] has completely forgotten the spike:
    let moving_window_twap = compute_windowed_twap(acc_1800, 1_800, acc_3600, 3_600);
    assert_eq!(
        moving_window_twap, fair_price,
        "Subsequent non-overlapping window has zero residual impact from earlier spike"
    );
}
