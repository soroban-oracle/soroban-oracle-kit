# Security

`soroban-oracle-kit` is a price-feed oracle. Oracles are one of the most
attacked components in DeFi, so this document exists to make the trust model
explicit rather than implicit in scattered doc comments.

## Reporting a vulnerability

Please report suspected vulnerabilities **privately** rather than opening a
public issue: use GitHub's
[private vulnerability reporting](https://github.com/soroban-oracle/soroban-oracle-kit/security/advisories/new)
(Security tab → "Report a vulnerability"). This lets us assess and fix the
issue before it's publicly visible.

## Trust model

The shipped contract is **admin-curated**: one address (`Admin`) is trusted to
publish honest prices at honest timestamps via `set_price`/`set_prices`
(`require_auth`-gated). Every safety module in this kit builds on top of that
base trust — none of them make the feed trustless, they narrow *what kind* of
bad behavior or bad data a consumer is exposed to.

| Module | Defends against | Does **not** defend against |
|---|---|---|
| [`staleness`](./crates/oracle/src/staleness.rs) | An absent/stopped writer (old data) | A malicious writer publishing fresh, wrong data |
| [`pause`](./crates/oracle/src/pause.rs) | Serving reads during a known-bad incident | The admin failing to pause, or pausing dishonestly |
| [`sanity_bounds`](./crates/oracle/src/sanity_bounds.rs) | Accidental fat-finger / out-of-range writes | A malicious but in-range price |
| [`nonzero`](./crates/oracle/src/nonzero.rs) | An accidental/degenerate zero price | Any other wrong value |
| [`deviation_breaker`](./crates/oracle/src/deviation_breaker.rs) | A single large jump in one write | A patient attacker walking the price in many within-threshold steps |
| [`median`](./crates/oracle/src/median.rs) | A minority of bad values in one submission set | The admin assembling a dishonest majority |
| [`roles`](./crates/oracle/src/roles.rs) (multi-writer) | Nothing by itself | Every added writer is *fully* trusted — this widens the trusted surface and must be vetted by the admin |
| [`admin_transfer`](./crates/oracle/src/admin_transfer.rs) | A typo'd/dead successor address bricking control | A compromised current or pending admin |
| [`heartbeat`](./crates/oracle/src/heartbeat.rs) | Detecting a lapsed feed (liveness only) | The *value* of the last price being correct |
| [`ring_buffer`](./crates/oracle/src/ring_buffer.rs) | Nothing by itself | A ring of spot samples is only as manipulation-resistant as whatever aggregation is applied on top |
| [`decimals`](./crates/oracle/src/decimals.rs), [`inverse`](./crates/oracle/src/inverse.rs) | N/A — pure arithmetic over an already-trusted price | N/A |

## Known gaps

The following are **not yet shipped** — do not assume the protection they'd
provide until the corresponding issue lands:

- **TWAP** ([#5](https://github.com/soroban-oracle/soroban-oracle-kit/issues/5),
  [#6](https://github.com/soroban-oracle/soroban-oracle-kit/issues/6)) — the
  main defense against single-block price manipulation. Until it ships, treat
  every read as a spot price.
- **AMM spot adapter**
  ([#3](https://github.com/soroban-oracle/soroban-oracle-kit/issues/3)) — when
  it lands, it derives a *spot* price from pool reserves and will be
  explicitly documented as unsafe for direct use in liquidations (trivially
  manipulable within one transaction via a flash loan). The safe path is the
  windowed TWAP query in #6.
- **Manipulation-resistance test suite**
  ([#7](https://github.com/soroban-oracle/soroban-oracle-kit/issues/7)) —
  until this lands, the manipulation-resistance claims above are documented
  but not yet demonstrated by a test.

## Guidance for new contributions

Every PR that adds or changes oracle behavior must state, in the PR
description, what it does and does not defend against — see
[`CONTRIBUTING.md`](./CONTRIBUTING.md). If you're unsure how to phrase it, use
the two-column shape from the table above: "defends against X" / "does not
defend against Y."
