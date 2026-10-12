# State

## Review cursor

- Done (2026-10-12): `crates/psd/src/*` and the PSD parts of `crates/io` (`psd_import.rs`, `psd_export.rs`, `pixels.rs`). Findings are in `nightly/2026-10-12.md`.
- **Next:**
  1. The rest of `crates/io`'s PSD export path (`psd_export.rs`), checked against the open Txt2 work (#2469/#3514), plus `crates/format`.
  2. Wishlist item 2 code areas: `crates/algo/src/segment/`, `crates/engine/src/smartselect_cmds.rs`, `crates/engine/src/cutout_cmds.rs`. Bugs and perf only; #2788 is claimed upstream.
  3. Item 3: GPU compose and effects (`crates/gpu`), and the over-budget rows in `docs/scorecard.md`.

## Open threads

- **Fork `main` vs "merge"**: Ian asked on 2026-10-12 to "merge and commit". The work was merged on `wip/merged-2026-10-12`, not on fork `main`, because the sync rule fast-forwards `main` to upstream only. Waiting for Ian. Until he answers, keep fast-forwarding `main` and keep work on branches.
- **Disk (~40 GB)**: `cargo test --workspace` doesn't fit in one go. Use the crate-by-crate script described in SETUP.md (delete `target/debug/deps` executables, `target/debug/examples` and `incremental` after each crate). Works; about 38 min in total.
- **Corpus**: the download is solved by `fix/corpus-git-fallback` (needed until it lands upstream: run `cargo xtask corpus --all` from that branch, or merge it into the branch under test). The *test* run (`cargo xtask test-corpus`, release + thin LTO + `codegen-units=1`) for just psd+io did not finish in ~45 min here. Next night, start it early, right after the checks, or ask Ian about a dev-profile option.
- Low-priority findings not yet fixed: ZIP merged image fully inflated at open just to validate (`crates/psd/src/image_data.rs:49`); double decode of the merged image for flat files (`psd_import.rs`); odd-length mask records with parameters (`psd_export.rs:350`, needs a Photoshop check); unchecked `index * plane` (`image_data.rs:43`).
- No Dropbox connector in the 2026-10-12 session, so Ian's real PSDs were not tested.

## Design notes

(none yet; wishlist item 2's model backend is claimed upstream by #2788 / #3826 / #2995 / #1199, so no design was started)

## Branches

| Branch | Purpose | Status |
|---|---|---|
| `fix/psd-import-fallback-alloc` | Huge bitmap/multichannel PSD no longer aborts on open; wasm overflow; resource padding | ready for Ian (after test-corpus on Z13) |
| `perf/psd-interleave-in-place` | PSD channel interleave in place, 8-bit ~6x | needs Z13 (numbers + test-corpus) |
| `fix/corpus-git-fallback` | `cargo xtask corpus` works where codeload.github.com is refused | ready for Ian |
| `wip/merged-2026-10-12` | main 7e5d2ce6 + the three branches above (Ian's "merge" request) | active; rebuild from the branches if main moves |
