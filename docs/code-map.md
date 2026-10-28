# Code map: photocraft (fork)

- Repo: iannitram/photocraft, a fork of storytold/photocraft (Rust, edition 2024, MIT OR Apache-2.0).
- Kind: native layered image editor (Photoshop-style, clean-room) as a Cargo workspace, version 0.16.0, early alpha. Desktop, CLI, MCP server and browser builds from one engine.
- Integration branch: `ours` (the default branch). `main` is a mirror of upstream and is not our work branch.
- Describes commit: 311154f8f (2026-10-28)
- Upstream: `storytold/photocraft` (`upstream/main`). Nothing is pushed or posted there. `docs/maps/` holds goal maps (and one `ui-egui` map), not a code map, so this file is `docs/code-map.md`. Upstream owns every crate and file not listed under "Where our work is".
- Longer references: `AGENTS.md` (workspace map and rules), `docs/architecture.md`, `docs/development.md`.

## Layout

Workspace members: `crates/*`, `apps/*`, `xtask` (30 crates, 4 apps; the default members leave out the two web apps). Layers are enforced by `cargo xtask layers` (`xtask/src/layers.rs`); a crate may depend only on lower layers.

| Path | What it owns |
|---|---|
| `crates/geom`, `cms`, `color`, `raster` | L0 foundation: geometry; pure-Rust ICC v2/v4 colour management; pixel formats and blend math; sparse copy-on-write tiled surfaces. |
| `crates/psd`, `codecs` | L0 standalone format crates: lossless PSD/PSB reader-writer; flat raster codecs. No workspace deps. |
| `crates/raw`, `heif`, `affinity` | Camera raw decode and develop (DNG, CR2 and others); optional HEIF/HEIC decoder; Affinity document reader. |
| `crates/tablet` | Pen pressure/tilt input (AppKit, X11 XInput2, Windows barrel button). Isolated `unsafe` (`deny`, not the workspace's `forbid`); `wayland-drop` is the other crate that opts in. |
| `crates/doc` | L1 document model: layers, masks, adjustments, effects, smart objects. Pure data. |
| `crates/ops`, `paint`, `algo`, `text`, `vector` | L2: undo history; brush engine; tile-aware CPU filters and selection (`algo/src/segment`); font and text layout (parley); paths and shapes. |
| `crates/compose`, `gpu`, `format` | L3: CPU reference compositor (the oracle); wgpu compositor; native `.pcraft` bundle (zstd tiles). |
| `crates/io`, `plugins` | L4: document to PSD and flat-format mapping; sandboxed WebAssembly filter plug-ins (wasmi). |
| `crates/engine` | L5: sessions, history and the command registry (about 700 commands; every action is a command id). |
| `crates/ui-egui` | L6: egui/eframe shell, menus, dialogs, tools, `i18n/` translations. Thin; all actions go through the engine. |
| `crates/automation` | MCP server (rmcp) over the engine, plus a bridge to a running GUI. |
| `crates/ipp` | Small IPP client for CUPS printing. |
| `crates/testkit` | Test helpers. |
| `crates/craft-log`, `craft-stage` | Self-logging (log files, crash and stall reports) and named long-operation stages. Ours, not upstream; `craft-stage` is a standalone crate, `craft-log` sits at L5. |
| `crates/wayland-drop` | Files dropped on a winit window under Wayland (a prototype). Ours; `unsafe_code` is `deny` with the FFI isolated here, as in `tablet`. |
| `apps/photocraft` | Desktop app (eframe/wgpu) with a TCP control server. |
| `apps/photocraft-cli` | Headless CLI: convert, info, run, batch, commands, mcp. |
| `apps/photocraft-web`, `photocraft-web-boot` | Browser build (trunk, wasm-bindgen, WebGPU with WebGL2 fallback) and its compressed-parts bootstrap. |
| `xtask/` | `cargo xtask` commands: ci, test, layers, wasm, stats, corpus, parity, perf, scorecard, i18n coverage. |
| `examples/plugins/invert-rs` | Example wasm filter plug-in. |
| `perf/`, `scripts/` | Performance budgets and baseline; corpus fetch and fixture scripts. |
| `packaging/` | Linux, macOS (dmg, notarization scripts), Windows, FreeBSD and web packaging. |
| `docs/`, `book/` | Architecture, parity, per-feature notes, performance reports, generated `scorecard.md`; `docs/maps/` goal maps; `docs/agents/` agent notes (ours). |
| `assets/`, `contributors/`, `scorecard/` | Icons and brand; contributor data; scorecard inputs. |

## How it fits together

```mermaid
flowchart TD
    app["apps/photocraft (GUI)"] --> ui["crates/ui-egui"]
    web["apps/photocraft-web"] --> ui
    ui --> eng["crates/engine (command registry)"]
    cli["apps/photocraft-cli"] --> eng
    mcp["crates/automation (MCP)"] --> eng
    eng --> io["io / plugins"] --> fmt["psd, codecs, raw, heif, affinity, format"]
    eng --> ops["ops, paint, algo, text, vector"] --> doc["doc"] --> ras["raster, color, geom, cms"]
    eng --> comp["compose (CPU) / gpu (wgpu)"]
```

- Everything is a command: menu, palette, shortcuts, CLI, JSON control channel and MCP all dispatch by command id into `crates/engine` (`CommandSpec` in `commands.rs`).
- The UI is data-driven; state lives in `ui-egui/src/state.rs` so the control channel can read and drive it. The core compiles to wasm32, so the browser build shares the engine.
- Each algorithm has a CPU reference; some also have a WGSL version reading the same `bytemuck` parameter struct, checked against the CPU result.
- Entry points: `apps/photocraft/src/main.rs`, `apps/photocraft-cli/src/main.rs`, `apps/photocraft-web/src`, `xtask/src/main.rs`.

## Where our work is

`git log upstream/main..HEAD`: 120 commits (some are sync merges); `git diff upstream/main...HEAD` is 123 files, +8,967 / -99. Upstream is 723 commits ahead of our merge base at this snapshot; an upstream sync has not landed on `ours`. Areas:

| Area | Where |
|---|---|
| Self-logging | New `crates/craft-log` and `craft-stage`; wiring in `apps/photocraft/src/self_logging.rs`, `apps/photocraft-cli/src/selflog.rs`, `crates/ui-egui/src/self_logging.rs`; tests in `apps/*/tests/selflog*.rs` |
| Linux / Wayland | `crates/wayland-drop`, `apps/photocraft/src/wayland_drop.rs`, `wayland_outputs.rs` (gamescope guard, sideways panels, maximize a tall window) |
| `crates/ui-egui` | Missing-font prompt (`missing_font_prompt*.rs`), snap and smart-guide behaviour, Camera Raw window sizing, open-file handling, and translations in `i18n/` (zh-hans: 361 changed rows; Serbian and Hungarian strings; a few rows in every language) |
| `crates/algo/src/segment/` | GrabCut and Boykov-Kolmogorov cancel and step-budget fixes (`budget_tests.rs`); Select Subject edge handling in `subject.rs`; Select and Mask feather in `matting.rs` |
| `crates/engine`, `compose`, `io`, `psd`, `doc`, `gpu` | Replace Contents size note (`smart_cmds.rs`), layer-style `edit` (`layer_style.rs`), stroke effect opacity and glow fixes (`compose/src/effects.rs`, `compose.wgsl`), PSD/PSB save refusing a lossy timeline save, PSD export fixes and tests (`io/tests/it/psd_export_*.rs`) |
| Docs | `docs/maps/` goal and `ui-egui` maps, `docs/agents/`, `SPEC.md`, `PIPELINE.md`, `AGENTS.fork.md`, `CHANGELOG.md`, regenerated `docs/scorecard.md` |

Several commits are sync-merge follow-ups (adapting our tests and zh-hans rows to upstream changes). Ours-only crates are marked "never offered upstream" in their headers.

## Build and test

Commands as `docs/development.md` states them (Rust stable 1.95+, `rustup target add wasm32-unknown-unknown` for web):

```sh
cargo run --release -p photocraft -- path/to/image.psd   # desktop app
cargo test --workspace
cargo xtask test      # nextest when installed, plus doctests
cargo xtask ci        # fmt, clippy, layers, scorecard, tests, panic_hunt, doctests, wasm, i18n report
cargo xtask layers    # dependency layering check
```

CI (`.github/workflows/ci.yml`): `platforms` selects legs; `gate` (shared workflow, Ubuntu, runs `cargo xtask ci --locked --features heif`); `windows` (windows-latest, unit and integration legs; for pull requests only when platform-sensitive code changes or the `full-ci` label is on); `test-without-heif`; `corpus` (real-file oracles); `web` (wasm build, when web-relevant files change); `fuzz` (manual). Other workflows: `freebsd`, `nix`, `windows-arm64`, `packaging-lint`, `docs`, `perf-nightly`, `release`, `sonarcloud`. The push triggers in these files name `main`, not `ours`: a plain push to `ours` starts no push run, so changes to `ours` are checked by their pull request, and the scheduled runs (FreeBSD, Windows ARM64, perf) use the default branch, `ours`.

## Mac / PC / Linux

- All three desktops are targets, plus FreeBSD and the browser. Linux build needs libxkbcommon, wayland, x11, xrandr, xi, GL and GTK3 dev packages.
- Platform-specific code: `crates/tablet` (AppKit, X11, Windows), `crates/wayland-drop` and `apps/photocraft/src/wayland_*.rs` (Linux only), `crates/ipp` (CUPS printing), macOS shell code in `apps/photocraft/src/mac_menu.rs`, `mac_window.rs` and `apple_events.rs`, `packaging/{macos,windows,linux,freebsd,web}`.
- CI coverage (R04): Linux on every pull request, Windows on platform-sensitive changes or `full-ci`, FreeBSD and Windows on ARM64 on a schedule, a Nix check, wasm32 built. There is no macOS job in `ci.yml` (its own comment: macOS is tested only by `release.yml`, which calls a shared workflow on pushes to a `release` branch; our fork has no `release` branch). A change that touches macOS-only code (tablet, `mac_*.rs`, packaging) is therefore not tested by CI here and needs a run on a Mac.

## Open-source pieces

egui/eframe 0.37 on wgpu (UI), winit, rfd (dialogs), parley (text shaping), rayon (parallel CPU filters), image and zune-jpeg (codec support), ruzstd and flate2 (tiles and compression), wasmi (plug-in sandbox), rmcp (MCP server), clap (CLI), chrono (all date handling), proptest, criterion, egui_kittest (UI tests), trunk and wasm-bindgen (web). Suite libraries `craft-ui`, `craft-print`, `craft-docpath`, `craft-jpeg`, `craft-update` come from `storytold/craft-libs` at a pinned revision. Optional `craft-fonts` input is used by `crates/text/build.rs`.
