# soroban-oracle-kit

[![CI](https://github.com/soroban-oracle/soroban-oracle-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/soroban-oracle/soroban-oracle-kit/actions/workflows/ci.yml)

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
| Staleness guard | ✅ shipped | `get_price_no_older_than(asset, max_age)` |
| Median aggregation | ✅ shipped | Combine multiple submitted prices into a median |
| Multi-writer roles | ✅ shipped | More than one authorized publisher |
| Cross-decimals normalization | ✅ shipped | Scale prices between arbitrary decimals and the kit's 7-decimal convention |
| TWAP accumulator | 🚧 open issue | Cumulative price·time accumulator + windowed average |
| AMM pool adapter | 🚧 open issue | Derive a price from `soroban-amm` reserves |
| Consumer examples | 🚧 open issue | Reference integrations |

The 🚧 items are tracked as [open issues](https://github.com/soroban-oracle/soroban-oracle-kit/issues) — see [`OPEN_SOURCE_ISSUES.md`](./OPEN_SOURCE_ISSUES.md) for scope details on each.

---

## Roadmap

**Near-term**, tracked as open, point-tagged issues:

- TWAP accumulator ([#5](https://github.com/soroban-oracle/soroban-oracle-kit/issues/5)) and windowed TWAP query ([#6](https://github.com/soroban-oracle/soroban-oracle-kit/issues/6)) — the core manipulation-resistance work
- A test suite demonstrating TWAP's manipulation resistance vs. spot pricing ([#7](https://github.com/soroban-oracle/soroban-oracle-kit/issues/7))
- An integration recipe for pricing `soroban-amm` pools end-to-end ([#4](https://github.com/soroban-oracle/soroban-oracle-kit/issues/4))
- A code-coverage CI job ([#8](https://github.com/soroban-oracle/soroban-oracle-kit/issues/8))

**Further out**, not yet scoped into issues:

- Wire the oracle into `soroban-amm` as a reference consumer
- Push-based feeds and a keeper script for automated price updates
- Publish to crates.io once the interface stabilizes

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

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the development workflow and
[`OPEN_SOURCE_ISSUES.md`](./OPEN_SOURCE_ISSUES.md) for scoped, point-tagged
issues to get started on.

## Questions & getting in touch

For general questions, ideas, or design discussion, use
[GitHub Discussions](https://github.com/soroban-oracle/soroban-oracle-kit/discussions)
rather than opening an issue. For suspected security vulnerabilities, see the
private reporting instructions in [`SECURITY.md`](./SECURITY.md) instead —
please don't discuss those in the open.

## License

[MIT](./LICENSE).
