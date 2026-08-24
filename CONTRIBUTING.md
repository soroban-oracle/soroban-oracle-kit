# Contributing to soroban-oracle-kit

Thanks for contributing! This repository participates in
[Drips Wave](https://docs.drips.network/wave/), so issues are scoped to be
small, single-file, and point-tagged.

## Ground rules

- **One contributor per issue, one primary file per issue.** Don't expand scope
  outside the file listed in the issue unless a maintainer asks.
- **Comment your ETA before starting.** ETA must be ≤ 2 days, or the issue may
  be unassigned.
- **Every new behavior needs a test.** Tests live in a `#[cfg(test)]` module
  alongside the code.
- **Oracles are security-sensitive.** Price manipulation and stale-data risks
  are real. Call out the trust assumptions of any new feature in its PR.

## Development workflow

1. Fork and branch from `main`:
   ```sh
   git checkout -b feat/short-description
   ```
   Prefixes: `feat/`, `fix/`, `refactor/`, `test/`, `docs/`, `chore/`.

2. Make sure the toolchain is set up:
   ```sh
   rustup target add wasm32-unknown-unknown
   ```

3. Before opening a PR, confirm all of these pass:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --workspace
   cargo build --release --target wasm32-unknown-unknown
   ```

4. Open a PR referencing the issue with `Closes #<issue>`. Explain what changed,
   why, and any trust assumptions it introduces.

## Code style

- Run `cargo fmt` before committing (default `rustfmt` settings).
- No `unsafe`.
- Prefer explicit, overflow-checked arithmetic — price math must never wrap.
- Don't add dependencies without discussion — this is a lean, `no_std` crate.

## PR checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] WASM build succeeds
- [ ] New behavior is covered by tests
- [ ] Trust assumptions / manipulation risks documented in the PR
- [ ] Changes are limited to the issue's primary file (plus any one-line module
      registration the issue explicitly allows)
