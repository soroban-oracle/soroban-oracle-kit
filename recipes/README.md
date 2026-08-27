# Recipes

Practical patterns for consuming `soroban-oracle-kit` from another Soroban
contract.

## Rejecting stale prices

A consumer should never act on a price that might be too old. An admin who
has gone offline and a writer who has simply stopped publishing both look
identical from the consumer's side: `get_price` keeps returning a value, it's
just an increasingly old one. Use the shipped staleness guard,
[`get_price_no_older_than`](../crates/oracle/src/staleness.rs), instead of
`get_price` anywhere a stale value would be dangerous (liquidations,
valuations, anything gating a transfer of value).

```rust
use oracle::OracleClient;
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct Consumer;

#[contractimpl]
impl Consumer {
    /// Reads `asset`'s price from the oracle at `oracle_id`, rejecting
    /// anything older than `max_age_secs`.
    ///
    /// # Panics
    /// - `"price too stale"` if the guard trips (see
    ///   [`Oracle::get_price_no_older_than`]).
    pub fn read_fresh_price(
        env: Env,
        oracle_id: Address,
        asset: Address,
        max_age_secs: u64,
    ) -> i128 {
        let oracle = OracleClient::new(&env, &oracle_id);
        oracle.get_price_no_older_than(&asset, &max_age_secs)
    }
}
```

### Testing it

```rust
#[test]
fn rejects_a_stale_price() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy the oracle and publish a price.
    let admin = Address::generate(&env);
    let oracle_id = env.register_contract(None, oracle::Oracle);
    let oracle = OracleClient::new(&env, &oracle_id);
    oracle.init(&admin);

    let xlm = Address::generate(&env);
    env.ledger().set_timestamp(1_000);
    oracle.set_price(&xlm, &1_0000000);

    // Deploy the consumer and read through it while the price is fresh.
    let consumer_id = env.register_contract(None, Consumer);
    let consumer = ConsumerClient::new(&env, &consumer_id);
    assert_eq!(consumer.read_fresh_price(&oracle_id, &xlm, &60), 1_0000000);

    // Advance past the freshness window; the same call now panics.
    env.ledger().set_timestamp(1_061);
    let res = consumer.try_read_fresh_price(&oracle_id, &xlm, &60);
    assert!(res.is_err());
}
```

### Choosing `max_age_secs`

There's no universally correct value — it trades off availability (a brief
writer hiccup shouldn't halt your protocol) against safety (an old price is a
manipulation and bad-liquidation risk). A reasonable starting point is 2-3x
the oracle's expected update interval. If the asset has a configured
heartbeat, prefer reading it directly over hardcoding a threshold — see
[`get_heartbeat`](../crates/oracle/src/heartbeat.rs) and
[`is_heartbeat_satisfied`](../crates/oracle/src/heartbeat.rs).
