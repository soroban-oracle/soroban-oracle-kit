# Seed Issue Drafts

These issue drafts back the open, point-tagged issues on the [GitHub issue
tracker](https://github.com/soroban-oracle/soroban-oracle-kit/issues). They
are intentionally small, single-file, and merge-safe.

> Staleness-guarded reads, median aggregation, multi-writer roles, and
> cross-decimals normalization have already shipped and are no longer listed
> here — see the "What's inside" table in [`README.md`](./README.md).

## Merge-Safe Maintainer Rule

To reduce merge conflicts, each issue below has a strict ownership boundary:

- one contributor per issue
- one primary file per issue
- contributors should not expand scope outside the listed file unless a
  maintainer asks

This makes it much easier to review and merge issues in any order.

## Point Tagging

Each issue carries a suggested point value reflecting scope/complexity.

- **2–3 pts** — beginner, single new file or function, no cross-module coupling
- **4–5 pts** — medium, a new module with its own tests and trust assumptions
- **8 pts** — advanced, manipulation-resistance design work

> **Oracle-specific reviewer note:** because price feeds are security-sensitive,
> every PR must state the trust assumptions and manipulation risks it
> introduces. Point values are set a little higher than a pure-tooling repo to
> reflect the extra care required.

## Contributor ETA Policy

Add this note to every issue:

> Contributor note: Please comment with your ETA before starting work. ETA must
> not be more than 2 days. If no ETA is added, or if the ETA exceeds 2 days, the
> issue may be unassigned.

---

## Beginner Issues

### [#1 Add a Consumer Recipe: Rejecting Stale Prices — 2 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/1)

**Why this is a good beginner issue**
Documentation-only. Shows a downstream contract how to consume the oracle
safely.

**Primary file**
- `recipes/README.md` (new)

**Scope**
- Create a `recipes/` folder with a README
- Include one worked example: a consumer contract that reads a price and rejects
  data older than a threshold

**Acceptance criteria**
- `recipes/README.md` exists with one complete, runnable recipe referencing only
  shipped APIs (including the staleness guard in `crates/oracle/src/staleness.rs`)

**Merge-safety note**
Keep changes limited to `recipes/README.md`.

### [#2 Add a SECURITY.md Documenting Oracle Trust Assumptions — 2 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/2)

**Why this is a good beginner issue**
Captures the project's threat model in one place — valuable and code-free.

**Primary file**
- `SECURITY.md` (new)

**Scope**
- Document the trust model (admin-curated prices), manipulation risks, and why
  TWAP/staleness matter
- Add responsible-disclosure contact instructions

**Acceptance criteria**
- `SECURITY.md` exists and clearly states trust assumptions and disclosure steps

**Merge-safety note**
Keep changes limited to `SECURITY.md`.

---

## Medium Issues

### [#3 Add an AMM Spot-Price Adapter (read-only) — 5 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/3)

**Why this is a medium issue**
Derives a spot price from `soroban-amm` pool reserves. Spot only (clearly marked
unsafe for direct pricing) — the safe TWAP version is issue #6.

**Primary file**
- `crates/oracle/src/adapters/amm_spot.rs` (new)

**Scope**
- Add a helper that, given a pool's reserves, computes the spot price of one
  token in terms of the other at `DECIMALS` precision
- Loudly document that spot prices are manipulable and must not be used directly
  for liquidations
- Register via `pub mod amm_spot;` in `crates/oracle/src/adapters/mod.rs`;
  include tests

**Acceptance criteria**
- Spot price computed correctly from sample reserves
- Manipulation warning present in docs; tests pass

**Merge-safety note**
Keep changes limited to the new `adapters/amm_spot.rs` file plus the module
registration line.

### [#4 Add an Integration Recipe: Pricing for soroban-amm — 4 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/4)

**Why this is a medium issue**
Documents how the AMM repo would consume this oracle end-to-end.

**Primary file**
- `recipes/integration-soroban-amm.md` (new)

**Scope**
- Walk through configuring the oracle, the AMM reading prices via a staleness
  guard, and why TWAP is preferred over the spot adapter

**Acceptance criteria**
- Recipe is accurate against shipped + issue APIs and self-contained

**Merge-safety note**
Keep changes limited to `recipes/integration-soroban-amm.md`.

---

## Advanced Issues

### [#5 Add a TWAP Accumulator — 8 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/5)

**Why this is an advanced issue**
The core manipulation defense: maintain a cumulative price·time accumulator on
each price update so consumers can compute a time-weighted average between two
observations. Requires careful overflow-safe arithmetic and timestamp handling.

**Primary file**
- `crates/oracle/src/twap.rs` (new)

**Scope**
- On each price write, update a per-asset cumulative `price * elapsed_seconds`
  accumulator plus last-update timestamp
- Expose the accumulator and last price/timestamp for consumers
- Register via a single `pub mod twap;` line; include tests advancing the ledger
  timestamp across multiple updates

**Acceptance criteria**
- Accumulator increases correctly across timestamped updates
- Overflow-safe; tests pass; trust/precision assumptions documented

**Merge-safety note**
Keep changes limited to `crates/oracle/src/twap.rs` plus the single `pub mod`
line.

### [#6 Add a Windowed TWAP Query — 8 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/6)

**Why this is an advanced issue**
Builds on #5 to answer "what was the average price over the last N seconds,"
the value consumers actually want. Depends on the accumulator design.

**Primary file**
- `crates/oracle/src/twap_query.rs` (new)

**Scope**
- Add `twap(asset, window_secs) -> i128` computing the time-weighted average
  from two accumulator observations
- Define behavior when the window exceeds available history (documented)
- Register via a single `pub mod twap_query;` line; include tests

**Acceptance criteria**
- TWAP matches a hand-computed average over a known series
- Edge cases (insufficient history) handled and documented; tests pass

**Merge-safety note**
Keep changes limited to `crates/oracle/src/twap_query.rs` plus the single
`pub mod` line.

### [#7 Add a Price-Manipulation Resistance Test Suite — 8 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/7)

**Why this is an advanced issue**
Demonstrates that TWAP resists a single-block spike that spot pricing would
fall for — the headline safety claim of the project.

**Primary file**
- `crates/oracle/tests/manipulation.rs` (new integration test)

**Scope**
- Simulate a sharp one-update price spike and show spot reflects it while a TWAP
  over a window barely moves
- Document the scenario and the defense in test comments

**Acceptance criteria**
- Test concretely shows TWAP dampening vs spot under a spike
- Passes under `cargo test`

**Merge-safety note**
Keep changes limited to `crates/oracle/tests/manipulation.rs`.

---

## Maintenance / Tooling Issues

### [#8 Add a Code-Coverage Job — 3 pts](https://github.com/soroban-oracle/soroban-oracle-kit/issues/8)

**Why this is a good issue**
Improves project health and is isolated to a new workflow file.

**Primary file**
- `.github/workflows/coverage.yml` (new)

**Scope**
- Add a coverage job (e.g. `cargo llvm-cov`) that uploads a report artifact
- Do not modify the existing `ci.yml`

**Acceptance criteria**
- Coverage job runs on PRs and produces a report; existing CI untouched

**Merge-safety note**
Keep changes limited to `.github/workflows/coverage.yml`.

---

## Recommended Merge Order

Each issue owns a separate file, so they can merge in any order. A comfortable
order if you want one:

1. Beginner issues (#1–#2) — recipe, security docs
2. Medium issues (#3–#4) — spot adapter, AMM recipe
3. Advanced issues (#5–#7) — TWAP accumulator, windowed query, manipulation tests
4. Maintenance (#8) — coverage
