# soroban-oracle-kit

[![CI](https://github.com/your-org/soroban-oracle-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/soroban-oracle-kit/actions/workflows/ci.yml)

**A price-feed oracle for Soroban DeFi** — an on-chain oracle contract plus the
freshness, aggregation, and adapter machinery that DeFi contracts need to price
assets safely.

DeFi on Stellar has AMMs, vaults, and lending designs in flight — but no shared,
open-source oracle. Every protocol either trusts a single admin-set price with
no staleness checks, or rolls its own feed. `soroban-oracle-kit` provides a
reusable, auditable oracle with the safety features that matter: timestamped
prices, staleness guards, time-weighted averages (TWAP) to resist manipulation,
and adapters that derive prices from on-chain sources like AMM pools.

---

## What's inside

| Component | Status | Provides |
|---|---|---|
| `oracle` contract | ✅ shipped | Admin-curated, timestamped price feed (`set_price` / `get_price` / `get_price_data` / `assets`) |
| Staleness guard | 🚧 seed issue | `get_price_no_older_than(asset, max_age)` |
| TWAP accumulator | 🚧 seed issue | Cumulative price·time accumulator + windowed average |
| Median aggregation | 🚧 seed issue | Combine multiple submitted prices into a median |
| AMM pool adapter | 🚧 seed issue | Derive a price from `soroban-amm` reserves |
| Multi-writer roles | 🚧 seed issue | More than one authorized publisher |
| Consumer examples | 🚧 seed issue | Reference integrations |

The 🚧 items are scoped issues — see [`OPEN_SOURCE_ISSUES.md`](./OPEN_SOURCE_ISSUES.md).

---

## Pricing convention

Prices are `i128` fixed-point with **7 implied decimals** (matching Stellar's
native precision). `1_0000000` means `1.0` quote units per priced unit. Each
stored price carries the ledger timestamp at which it was written, so consumers
can enforce freshness.

---

## Usage

```rust
use soroban_sdk::{testutils::Address as _, Address, Env};
use oracle::{Oracle, OracleClient};

let env = Env::default();
env.mock_all_auths();

let admin = Address::generate(&env);
let id = env.register_contract(None, Oracle);
let oracle = OracleClient::new(&env, &id);
oracle.init(&admin);

let xlm = Address::generate(&env);
oracle.set_price(&xlm, &1_0000000);          // 1.0
assert_eq!(oracle.get_price(&xlm), 1_0000000);

let data = oracle.get_price_data(&xlm);
// data.price, data.timestamp  ← consumer can reject stale prices
```

---

## Building & testing

```sh
rustup target add wasm32-unknown-unknown   # one-time
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown
```

Pinned to `soroban-sdk 21.7.7` to stay in lockstep with the rest of the
ecosystem (`soroban-amm`, `soroban-test-kit`).

---

## Security note

Oracles are the most attacked component in DeFi. This kit favors **TWAP and
staleness checks over raw spot prices**, and every new feature must document its
trust assumptions. See [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## Contributing

This repository participates in
**[Drips Wave](https://docs.drips.network/wave/)**. See
[`OPEN_SOURCE_ISSUES.md`](./OPEN_SOURCE_ISSUES.md) to get started.

## License

[MIT](./LICENSE).
