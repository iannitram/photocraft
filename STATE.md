# State

## Review cursor

Not started. First run: begin with the code areas of WISHLIST.md item 1.

## Open threads

- **Disk allowance (~40 GB) doesn't fit a workspace-wide `cargo test --workspace --no-run` or
  `cargo clippy --workspace --all-targets` run in one shot** — confirmed by actually running it on
  2026-10-12: it crashed with ENOSPC partway through `target/debug`. Needs a decision: test
  per-crate/per-area with cleanup between batches (works today, just slower to orchestrate), or
  ask whoever manages the environment for a bigger disk allowance. See `SETUP.md`.
- **`cargo xtask corpus --all` / `cargo xtask test-corpus` / the `corpus` feature can't run
  unattended**: every pinned corpus except PngSuite is fetched via `codeload.github.com`
  (`xtask/src/pinned.rs`), which 403s for any repo not individually attached to the session.
  Confirmed 2026-10-12. Needs either a durable repo allowlist for the nightly session, or a code
  change to fetch via plain `git clone`/`git archive` instead (plain git to those repos is not
  gated — only the codeload/API path is). Put to Ian before spending a night on it, since it's a
  real PR against `xtask`, not a local workaround.

## Design notes

(none)

## Branches

| Branch | Purpose | Status |
|---|---|---|
