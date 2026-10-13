# State

## Review cursor

- Done (2026-10-12): `crates/psd/src/*` and the PSD parts of `crates/io` (`psd_import.rs`, `psd_export.rs`, `pixels.rs`). Findings are in `nightly/2026-10-12.md`.
- Done (2026-10-12, 07:22 run, read-only subagent review): `crates/io/src/psd_export.rs`, `crates/format`, `crates/algo/src/segment/`, `crates/engine/src/smartselect_cmds.rs`, `crates/engine/src/cutout_cmds.rs`. Findings R1-R7 are in `nightly/2026-10-12.md` (07:22 section).
- Done (2026-10-12, 07:01 run, guided by the real PSDs): precise inner glow in `crates/compose/src/effects.rs`, GrabCut / max-flow choice in `crates/algo/src/segment/`, Select and Mask feather in `crates/algo/src/matting.rs`, linked-file embedding in `psd_export.rs`.
- **Next:**
  1. The text-stroke edge difference on 4 real files (finding N5 in the 07:01 log: outside Stroke under a half-covered glyph pixel, `effects.rs` around the outside-stroke share).
  2. Select Subject: the 68k-node low-resolution cut still costs ~330 ms (N2b). Ideas: reuse the flow between GrabCut iterations, fewer iterations once labels settle, a coarser working size for thumbnails.
  3. Item 3: GPU compose and effects (`crates/gpu`), and the over-budget rows in `docs/scorecard.md`.
  4. Then the rest of the workspace, crate by crate.

## Open threads

- **Fork layout (decided 2026-10-12)**: `main` stays an exact mirror of upstream (fast-forward only, never our commits). Each change is one commit on its own branch. **`ours`** is `main` plus every branch marked "in `ours`" in the Branches table, merged with `--no-ff` in table order; it is the build Ian runs. Rebuild `ours` from scratch whenever `main` moves or a branch changes (push with `--force-with-lease`). `wip/merged-2026-10-12` is retired: don't rebuild it or build on it (it is still on the fork until Ian deletes it; everything in it is on the other branches).
- **Disk (~40 GB)**: `cargo test --workspace` doesn't fit in one go. Use the crate-by-crate script described in SETUP.md (delete `target/debug/deps` executables, `target/debug/examples` and `incremental` after each crate). Works; about 38 min in total.
- **Corpus**: the download is solved by `fix/corpus-git-fallback` (needed until it lands upstream: run `cargo xtask corpus --all` from that branch, or merge it into the branch under test). The *test* run (`cargo xtask test-corpus`, release + thin LTO + `codegen-units=1`) for just psd+io did not finish in ~45 min here. Next night, start it early, right after the checks, or ask Ian about a dev-profile option.
- Low-priority findings not yet fixed: ZIP merged image fully inflated at open just to validate (`crates/psd/src/image_data.rs:49`); double decode of the merged image for flat files (`psd_import.rs`); odd-length mask records with parameters (`psd_export.rs:350`, needs a Photoshop check); unchecked `index * plane` (`image_data.rs:43`).
- Open review findings from the 07:22 run (details in its log): R2 Select Subject re-renders the composite per strip and per refine tile (high perf; `crates/algo/src/segment/mod.rs:121-152`, `:609-611`), R2b huge-canvas strip OOM, R4 linked smart object read on PSD export has no size cap or authorize check (`psd_export.rs:807`), R5 global blocks copied on save (`psd_export.rs:1529`), R6 .pcraft writer lacks ZIP64 (`format/src/zip.rs:39-43`), R7 duplicate manifest ids (`format/src/convert.rs:373`).
- Upstream PR #2580 rewrites `interleave` like `perf/psd-interleave-in-place`. Decided 2026-10-12: ours is dropped from `ours` and not offered upstream; the branch stays (un-rebased, on dbca08cd) until #2580 lands, then delete it. Don't rebase it or build on it.
- 2026-10-12: two runs (07:01 and 07:22) ran at once. The 07:01 run did the real-PSD pass (see its section for the table); the 07:22 run wrapped up early.
- Rebase state (2026-10-12, daytime session): `main` = upstream b3309e27. All seven branches in `ours` are rebased onto it (one commit each, no conflicts) and `ours` is rebuilt on top.
- Upstream #3651 (budget check inside `composite_rgba8`) merged on 10-12. `fix/psd-import-fallback-alloc` is still needed: the importer's own fallback preview is not covered by it.
- Open from the 07:01 run: N3b (authorize gate not asked for linked files at save), N5 (text stroke edges), the inner-glow trade-off on `inner-glow-center-precise-choke30.psd` (Needs Ian). Other #214 audit points not done: whole-pixel fields in Select and Mask, existing mask metadata reset on output.
- nightly-notes was force-pushed once (2026-10-12 ~11:30, lease-protected) to drop a log line that described one of Ian's files; the old commit may stay in GitHub's cache for a while.

## Done locally (daytime sessions; don't redo)

- **Overnight run, second leg (local, Hyperion, no GPU)**: `ours` is `main` b3309e27 + ten branches (41c35114). Corpus sets `psd` (384 tests) and `codecs` (678) pass on 3c58ce16; `io` and `engine` not run yet (about 50 release test programs each). A live shape's 1 px centre stroke at a fractional position: Photoshop draws it on whole pixels (band start `floor(edge - 0.5)` fits one real file; unproven, nothing changed).
- **Overnight run 2026-10-12/13 (local, Hyperion)**: baseline over 162 real PSDs (161 open, render, save, reopen; 132 of 160 matched Photoshop's merged image before the two fixes above). `ours` is now `main` b3309e27 + nine branches (3c58ce16). Not yet run on it: `cargo xtask test-corpus` (one public file, psd-tools/masks.psb, has substituted type layers).
2026-10-12, Windows desktop (RTX 3090 Ti, Rust 1.95):

- Every branch below was read against the surrounding code and upstream's open PRs (476 scanned). Only `perf/psd-interleave-in-place` duplicates one (#2580).
- `perf/grabcut-bk-maxflow`: the "identical selections" claim was checked on 240 random graphs (flat images, quantised noise, tied data costs, random hard seeds) with both solvers: zero pixels differ. Bench (`bench_smartselect`, 6016 px, release): Object Selection 413 -> 206 ms, Select Subject 327 -> 225 ms, same IoU.
- `fix/psd-export-linked-read`: the first version did not hold on Windows. A named pipe reports itself as a regular file there, so the check let it through and the read blocked forever. Fixed on the branch (device-namespace paths and server pipes are refused by name; a verbatim drive path is still embedded; two Windows tests). Still open from R4 / N3b: no authorize check and no size cap.
- `fix/precise-inner-glow-soft-edges`: one real file goes 0.149 -> 0.008 against Photoshop's merged image; three others unchanged. Caution for anyone building on it: the model is fitted on one file, and it moves the glow about half a pixel outward on every anti-aliased edge, not only on soft content.
- `fix/select-and-mask-feather-sigma`: not measured against Photoshop by anyone. Upstream's #214 audit asks for a Photoshop-made oracle before the formula changes.
- `fix/psd-export-ids`: both new tests were run against upstream's exporter (main b3309e27) and fail there, as the commit says; they pass with the fix.
- Gates on `ours` 9478bcf6 (= main b3309e27 + the seven branches): fmt, psd / io / algo / compose / xtask tests, `xtask layers`, `xtask wasm`, GPU crate tests on a real GPU and the engine lib tests all pass. Clippy 1.95 stops on upstream's own code (`photocraft-affinity`); nothing is reported in the lines our branches touch.
- Corpus tests on `ours` 9478bcf6: `cargo xtask test-corpus` (psd, codecs, io, engine; release) passes, 3,170 tests, 0 failed. 2 h 2 min on a 24-thread desktop with `CARGO_BUILD_JOBS=6`, so it does not fit a cloud night: leave it to daytime sessions.
- Four real files on the new build: renders are byte-identical to the previous `ours`, also after save and reopen; against Photoshop's merged image (checked with a separate parser) psd-b0a2f2 is 0.008, psd-27a113 and psd-353a73 0.004, psd-313e15 0.169 (finding N5).
- The 5 GPU-adapter tests: 4 pass on a real GPU. `prefs_ui::shortcut_capture_tests::ctrl_k_is_recorded_as_ctrl_k_not_the_control_key` hung for 39 minutes and was killed (upstream's test; it seems to wait for a real key press).
- No upstream PR has been opened for any branch.

## Design notes

- Wishlist item 2's model backend is claimed upstream by #2788 / #3826 / #2995 / #1199, so no design was started.
- **Precise Inner Glow model** (fitted on one real file, 2026-10-12): Photoshop measures the inside distance from every pixel below full opacity, each at `a − ½` from the edge; opaque pixels add their nearest such pixel's coverage. With #3578's ramp (0.5 at the edge to 0 at size + 1, then Range) this matches the file to 0.007. Still open: a curved anti-aliased centre-glow oracle gets more pixels over 2/255 (see the 07:01 log).
- **Max-flow choice**: push/relabel wins on narrow band cuts (Quick Selection, finish_region, Content-Aware Fill seams); Boykov-Kolmogorov wins on GrabCut's whole-image cuts (measured 2026-10-12).

## Real PSDs (by label; format facts only)

Labels are `psd-` + the first 6 hex characters of sha256 of the Dropbox file id. The label → file map is not kept here (the fork is public); recompute it from a folder listing. Baseline 2026-10-12 on main b564f2ca (cloud CPU). Vs Photoshop: largest error / pixels over 2/255, our render against the file's merged image (`oracle_diff`).

| Label | Size | Layers | Opens | Round trip | Edit+save+reopen | vs Photoshop (main) | Select Subject ms (main / BK branch) |
|---|---|---|---|---|---|---|---|
| psd-2025e2 | 1494x840 8-bit RGB | 10 | yes | yes | yes | 0.004 PASS | 958 / 941 |
| psd-217bbc | 1494x840 | 4 | yes | yes | yes (no type layer) | 0.006 PASS | 2380 / 1470 |
| psd-27a113 | 1920x1080 | 12 | yes | yes | yes | 0.005 PASS | 1072 / 793 |
| psd-313e15 | 1494x840 | 9 | yes | yes | yes | 0.169 / 0.30 % | 1147 / 853 |
| psd-353a73 | 1920x1080 | 11 | yes | yes | yes | 0.006 PASS | 2170 / 1380 |
| psd-4a3295 | 1440x1440 | 14 | yes | yes | yes | 0.002 PASS | 816 / 289 |
| psd-5b98bb | 1494x840 | 21 | yes | yes | yes | 0.005 PASS | 1421 / 859 |
| psd-6b50ac | 2560x1440 | 5 | yes | yes | yes | 0.155 / 0.24 % | 534 / 455 |
| psd-6ec13a | 1920x1080 | 9 | yes | yes | yes | 0.005 PASS | 2005 / 1756 |
| psd-84df1b | 1494x840 | 7 | yes | yes | yes | 0.153 / 0.52 % | 461 / 379 |
| psd-9d2240 | 1494x840 | 7 | yes | yes | yes | 0.153 / 0.29 % | 2161 / 1372 |
| psd-a4ec8f | 2560x1440 | 16 | yes | yes | yes | 0.018 / 0.02 % | 1051 / 551 |
| psd-b0a2f2 | 1494x840 | 8 | yes | yes | yes | 0.149 / 3.07 % (0.007 PASS with the glow fix) | 2273 / 1268 |

All 8-bit RGB; no vector masks, clipping or adjustment layers; blends Normal / Pass Through; effects Stroke, Drop Shadow, Color Overlay, Inner Shadow, Inner Glow. About 30 more PSDs in the folder are untested.

## Branches

`main` = upstream b3309e27 (2026-10-12). `ours` = `main` + the "yes" rows, merged in this order.

| Branch | Purpose | In `ours` | Status |
|---|---|---|---|
| `fix/psd-import-fallback-alloc` | Huge bitmap/multichannel PSD no longer aborts on open; wasm overflow; resource padding | yes | reviewed; ready for an upstream PR (Ian decides) |
| `fix/corpus-git-fallback` | `cargo xtask corpus` works where codeload.github.com is refused | yes | reviewed; ready for an upstream PR (Ian decides). The nightly needs it to fetch the corpus |
| `fix/psd-export-ids` | Imported type layers keep their Txt2 slot when a new text layer is added; no lyid overflow (review R1, R3) | yes | reviewed; ready for an upstream PR (Ian decides) |
| `fix/psd-export-linked-read` | Linked smart-object file embedded only if a regular file (no /dev/zero hang) | yes | reviewed; ready for an upstream PR (Ian decides). Authorize gate and size cap still open |
| `fix/precise-inner-glow-soft-edges` | Precise Inner Glow measured from every not-fully-opaque pixel (real-file fidelity) | yes | **held**: needs a shareable Photoshop-made test file; one public oracle gets worse on one measure |
| `perf/grabcut-bk-maxflow` | GrabCut cuts use Boykov-Kolmogorov: Select Subject 1.1-2.8x faster, same masks | yes | reviewed; ready for an upstream PR (Ian decides) |
| `fix/select-and-mask-feather-sigma` | Select and Mask Feather sigma = radius | yes | **held**: unverified against Photoshop (#214 audit) |
| `fix/type-substituted-font-gamma` | A type layer Photoshop saved with `substitutesUsed` (a missing font) is mixed without the text gamma, as Photoshop does; editing it makes it live type again | yes | measured on 59 type layers of 20 real files (no exception); unit tests in doc, compose, engine; ready for an upstream PR (Ian decides) |
| `fix/stroke-opacity-replaces-fill` | A Normal-mode Stroke effect below 100 % opacity shows the backdrop through its inside part, not the layer's fill (CPU `stroke_replace`, GPU `F_REPLACE`) | yes | largest error on real files 0.749 -> 0.200; still open: the drop shadow shows through the see-through band; ready for an upstream PR (Ian decides) |
| `feat/missing-font-warning` | The window warns before a missing font is substituted: a prompt before an edit redraws a PSD type layer's saved pixels (Cancel keeps them), a notice when another `type.*` command substitutes a font, a notice on open, a caution mark on the layer thumbnail. `photocraft-ui-egui` only | yes | 9 tests (one paints the whole window frame by frame without a renderer); the crate's lib suite without its 16 GPU-rendering tests passes except 3 upstream failures (Turkish catalog gaps, a Windows path assertion); not yet seen in a window; ready for an upstream PR (Ian decides) |
| `wip/stroke-band-knocks-out-shadow` | Keep the drop shadow and outer glow out of the band a stroke below 100 % opens (on top of `fix/stroke-opacity-replaces-fill`) | **no** | CPU half only; measured on two real files: both forms (a share of the shadow left, none left) take the largest error from 0.200 to 0.094 and neither matches; the 0.094 is the stroke's own half-covered edge pixel (Photoshop's fill share there is `w - k w (1 - w)`, k about 0.36). Its test still fails on purpose. Needs the edge-pixel coverage first, then a GPU half and the GPU tests |
| `wip/type-effects-text-gamma` | Experiment: a type layer's exterior effects mixed at the text gamma | **no** | **do not merge**: only files saved by Photoshop CC 2019 want it; 51 of 87 real files get worse. Kept as a record |
| `perf/psd-interleave-in-place` | PSD channel interleave in place | **no** | dropped: duplicates upstream PR #2580. Delete once #2580 lands |
