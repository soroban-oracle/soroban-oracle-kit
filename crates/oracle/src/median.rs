//! Median aggregation of submitted prices.
//!
//! Relying on a single submitted number means a single mistake or compromise
//! moves the feed. Taking the median of several submissions makes the published
//! price robust to a minority of bad inputs: an attacker must corrupt more than
//! half the submitters to move it.
//!
//! Submissions are admin-curated here (the admin submits the candidate set);
//! the value of the median is the *aggregation* logic, reusable once multi-
//! writer roles land.
//!
//! ## Trust assumptions
//!
//! The admin is trusted to assemble an honest candidate set. The median dilutes
//! a minority of bad values but cannot help if the majority is wrong. Even
//! counts average the two central values, which can round down by one ulp.

use crate::OracleClient;
use crate::{DataKey, Oracle, PriceData};
use soroban_sdk::{contractimpl, Address, Env, Vec};

#[contractimpl]
impl Oracle {
    /// Aggregate `submissions` into their median and store it as the price for
    /// `asset` at the current ledger timestamp. Admin-only.
    ///
    /// For an odd count the middle element is used; for an even count the two
    /// central elements are averaged (toward negative infinity by one ulp at
    /// most). The submission vector is sorted internally and is not mutated for
    /// the caller.
    ///
    /// # Panics
    /// - `"no submissions"` if the vector is empty.
    pub fn set_price_median(env: Env, asset: Address, submissions: Vec<i128>) -> i128 {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();

        let median = compute_median(&env, &submissions);

        if !Self::has_price(env.clone(), asset.clone()) {
            let mut assets: Vec<Address> = env
                .storage()
                .instance()
                .get(&DataKey::Assets)
                .unwrap_or_else(|| Vec::new(&env));
            assets.push_back(asset.clone());
            env.storage().instance().set(&DataKey::Assets, &assets);
        }

        let data = PriceData {
            price: median,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Price(asset), &data);
        median
    }
}

/// Pure median of a price vector (does not touch storage).
///
/// # Panics
/// - `"no submissions"` if the vector is empty.
pub fn compute_median(env: &Env, submissions: &Vec<i128>) -> i128 {
    let n = submissions.len();
    assert!(n > 0, "no submissions");

    // Insertion sort into a fresh Vec; n is small (a handful of writers).
    let mut sorted: Vec<i128> = Vec::new(env);
    for v in submissions.iter() {
        let mut inserted = false;
        for i in 0..sorted.len() {
            if v < sorted.get(i).unwrap() {
                sorted.insert(i, v);
                inserted = true;
                break;
            }
        }
        if !inserted {
            sorted.push_back(v);
        }
    }

    let mid = n / 2;
    if n % 2 == 1 {
        sorted.get(mid).unwrap()
    } else {
        let lo = sorted.get(mid - 1).unwrap();
        let hi = sorted.get(mid).unwrap();
        // Overflow-safe average of two i128 values.
        lo.checked_add(hi)
            .expect("median sum overflow")
            .checked_div(2)
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env};

    fn setup(env: &Env) -> (OracleClient, Address) {
        let admin = Address::generate(env);
        let id = env.register_contract(None, Oracle);
        let client = OracleClient::new(env, &id);
        client.init(&admin);
        (client, admin)
    }

    #[test]
    fn odd_count_picks_middle() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let xlm = Address::generate(&env);
        let m = oracle.set_price_median(&xlm, &vec![&env, 3_0000000, 1_0000000, 2_0000000]);
        assert_eq!(m, 2_0000000);
        assert_eq!(oracle.get_price(&xlm), 2_0000000);
    }

    #[test]
    fn even_count_averages_central_two() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let xlm = Address::generate(&env);
        let m = oracle.set_price_median(
            &xlm,
            &vec![&env, 1_0000000, 2_0000000, 3_0000000, 4_0000000],
        );
        assert_eq!(m, 2_5000000);
    }

    #[test]
    fn single_submission_is_itself() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let xlm = Address::generate(&env);
        assert_eq!(
            oracle.set_price_median(&xlm, &vec![&env, 9_0000000]),
            9_0000000
        );
    }

    #[test]
    #[should_panic(expected = "no submissions")]
    fn empty_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (oracle, _admin) = setup(&env);
        let xlm = Address::generate(&env);
        oracle.set_price_median(&xlm, &vec![&env]);
    }
}
