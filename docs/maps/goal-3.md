# Goal 3: User bug backlog to under 100 open

*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

## Summary

- **Goal:** Reduce open GitHub issues from 579 (2026-10-10) to under 100.
- **Estimate:** 250–400 Opus 5.5 agent-hours, counted once per fix across gaps.
- **Impact:** Blocks beta; each issue is a place where a Photoshop user hits unexpected behaviour. Together they explain why ready-for-real-work is ~45% while breadth is ~76%.
- **Source:** [gaps.md G3](../gaps.md#g3-user-reported-bug-backlog), [target-app-parity](../target-app-parity.md).
- **Current state:** 579 open issues, of which ~244 are on core paths (layers, selection, brush, transform, type, save/open, performance). 136 are feature requests, 443 are bugs or reports.

## Bug categories (by title, ~244 core-path issues)

| Category | Count | Typical issue numbers |
|---|---:|---|
| UI / panels / menus / shortcuts | ~150 | spring-loaded tools (#2301), flyouts (#2301), autoscroll (#2231), 27 prefs do nothing (scorecard) |
| File / format | ~94 | PSD refused by Photoshop (#1281, #2469), text-layer re-save (#2374, fixed), PSB > 4 GB Win (#375) |
| Layers | ~87 | locked-layer fill (#398), paste/clipboard, layer styles, smart objects |
| Painting | ~54 | brush lag (#2619, #1994), pressure jumps (#2046), pixelated cursor (#1433), big brushes (#2566) |
| Selection | ~54 | marquee, lasso, magic wand, object selection edge cases |
| Type | ~46 | freeze on macOS (#2518, 60 s), type on a path, manual kerning, RTL |
| Performance | ~41 | layer move with styles 321 ms, opacity 276 ms, paste 12 MP 1.2 s, 150-layer GPU crash (P12) |
| Transform / move | ~34 | Free Transform H/V skew fields, blend/opacity during transform (#1384), nudge |
| Linux / Wayland | ~25 | Wayland tablet-v2 broken (#2622, #2297), X11 XInput2 |
| Windows | ~24 | WinTab tilt/rotation, pen tap sensitivity (#1933) |
| macOS | ~20 | monitor profile detection, HDR/EDR, Touch Bar (Intel only, low value) |

## Existing crates, modules and functions

### Engine command registry (all fixes go through commands)

| Path | What it does |
|---|---|
| `crates/engine/src/commands.rs` | `CommandSpec` struct: id, label, menu path, shortcut, params, `enabled`, `run` |
| `crates/engine/src/lib.rs:640` | `Session::execute(id, params)` — the single dispatch entry point |
| `crates/engine/src/lib.rs:719` | `Session::is_enabled(id)` — command enablement gate |
| `crates/engine/src/lib.rs:731` | `Session::disabled_reason(id)` — human-readable disable reason |
| `crates/engine/src/lib.rs:742` | `Session::precondition(spec, params)` (private) — validates params before dispatch |
| `crates/engine/src/lib.rs:781` | `Session::edit(label, f)` — document edit with undo |

### Command modules (one `*_cmds.rs` per feature area, each with a `specs()` function)

| Path | Feature area |
|---|---|
| `crates/engine/src/layer_multi_cmds.rs` | Multi-layer ops (duplicate, paste, delete, undo/redo damage reporting) |
| `crates/engine/src/layer_list_cmds.rs` | Layer list panel operations |
| `crates/engine/src/file_cmds.rs` | Open, save, export, batch (atomic saves, #1547, #2579) |
| `crates/engine/src/edit_cmds.rs` | Edit menu: fill, stroke, transform, preferences |
| `crates/engine/src/adjust_cmds.rs` | Image › Adjustments (brightness, levels, curves, hue/sat, etc.) |
| `crates/engine/src/brush_cmds.rs` | Brush tool state and stroke dispatch |
| `crates/engine/src/transform_cmds.rs` | Free Transform, affine transforms, handle widgets |
| `crates/engine/src/type_cmds.rs` | Type tool, text layers, font selection |
| `crates/engine/src/selection_cmds.rs` | Marquee, lasso, magic wand, select/deselect/invert |
| `crates/engine/src/color_cmds.rs` | Color management, profile conversions, color settings |
| `crates/engine/src/actions_cmds.rs` | Actions playback, stop-on-failure reporting |
| `crates/engine/src/automate_cmds.rs` | Automate menu commands |
| `crates/engine/src/align_cmds.rs` | Layer alignment commands |
| `crates/engine/src/artboard_cmds.rs` | Artboard operations |
| `crates/engine/src/atn_cmds.rs` | `.atn` Actions load/save (112 events) |
| `crates/engine/src/background_cmds.rs` | Background layer operations |
| `crates/engine/src/channel_cmds.rs` | Channel operations |
| `crates/engine/src/color_cmds.rs` | Color picker, swatches, profiles |
| `crates/engine/src/comps_cmds.rs` | Layer Comps (Window › Layer Comps, Export › Layer Comps to Files) |
| `crates/engine/src/distort_cmds.rs` | Distort/warp filters |
| `crates/engine/src/document_preset_cmds.rs` | Document presets |
| `crates/engine/src/paint_cmds.rs` | Paint tool dispatch |
| `crates/engine/src/retouch_cmds.rs` | Retouch tools (healing, clone, content-aware) |
| `crates/ui-egui/src/view_cmds.rs` | View commands (zoom, canvas size, screen mode) |
| `crates/engine/src/video_cmds.rs` | Video/timeline operations |
| `crates/engine/src/wia_cmds.rs` | File › Import › WIA Support (Windows Image Acquisition) |
| `crates/engine/src/inspect.rs` | Document/layer inspection, damage reporting |

### UI shell (menu wiring, panels, state)

| Path | What it does |
|---|---|
| `crates/ui-egui/src/menus.rs` | Menu bar from command registry + `UI_COMMANDS`, `modal_allows()` gate |
| `crates/ui-egui/src/menus.rs:20` | `TOP_MENUS` — 10 menus in Photoshop order |
| `crates/ui-egui/src/menus.rs:25` | `UI_COMMANDS` — pure view/window state commands (panels, zoom) |
| `crates/ui-egui/src/menu_catalog.rs` | 631 menu items (2026-10-12), the denominator for parity |
| `crates/ui-egui/src/state.rs` | `AppState` — serde-serialisable UI state (panels, zoom, tools) |
| `crates/ui-egui/src/panels.rs:18` | Panel sections, `TOOL_SECTIONS` (67 of Photoshop's 68 tools; Art History Brush missing) |
| `crates/ui-egui/src/tool_shortcuts.rs` | Toolbar shortcut reassignment, saved bindings (#4086) |
| `crates/ui-egui/src/canvas.rs` | Canvas rendering, readout, cursor badge (`tool_feedback`) |
| `crates/ui-egui/src/work_area.rs` | Work area layout, monitor orientation |
| `crates/ui-egui/src/prefs_ui.rs` | Preferences UI, auto theme, modifier key handling |
| `crates/ui-egui/src/layer_tree_ui.rs` | Layer panel UI (selection, visibility, locking) |
| `crates/ui-egui/src/brush_panel.rs` | Brush Settings panel UI |
| `crates/ui-egui/src/panels.rs:908` | Options bar dispatcher (`options_bar`) |
| `crates/ui-egui/src/control.rs` | Control channel — automation API (inspect, screenshot, execute) |

### Robustness and testing

| Path | What it does |
|---|---|
| `crates/engine/tests/panic_hunt.rs` | Fuzz test: runs every command with adversarial params on a worker thread; panics caught, hangs reported (Rule 9) |
| `crates/engine/src/allocation.rs` | Allocation failure handling, `allocation_failure_is_reported_without_panicking` |
| `crates/ui-egui/src/parity.rs` | Parity floor `FLOOR` — raised when parity numbers improve |
| `crates/ui-egui/src/theme.rs` | `theme::Tokens` — design tokens, colours, radii (never hard-coded) |

### Issue tracking and docs

| Path | What it does |
|---|---|
| `docs/agents/issue-tracker.md` | Agent skill: reports as markdown files in Software Depot `apps/<App>/reports/ (new)`, status flow `new→triaged→in-progress→fixed→verified` |
| `docs/gaps.md` | Ranked gaps; G3 section lists all 579 issues by category |
| `docs/target-app-parity.md` | Parity assessment; user evidence section (579 open, 244 core-path) |
| `docs/scorecard.md` | Generated scorecard with GitHub issue links for bugs |
| `docs/parity-checklist.md` | Generated: every Photoshop menu item, live or missing |
| `ROADMAP.md` | Milestone 3: "User bug backlog to under 100 open" (250–400 h) |

## Concrete gaps (where fixes live)

### G3.1 — UI: spring-loaded tools and flyouts (~30 issues)
- **Missing:** toolbar press-and-hold flyouts (#2301), spring-loaded key tools (#2672)
- **Where to fix:** `crates/ui-egui/src/tool_shortcuts.rs` (already handles shortcut reassignment), `crates/ui-egui/src/menus.rs` (menu wiring), `crates/ui-egui/src/panels.rs` (options bar)
- **Approach:** Add hold-key state machine in UI shell; register as `UI_COMMANDS` entries since they are pure view state

### G3.2 — UI: 27 preferences that do nothing (scorecard prefs audit)
- **Missing:** 27 of 156 preferences are dead code
- **Where to fix:** `crates/ui-egui/src/prefs_ui.rs` (preferences panels), `crates/engine/src/` (engine-side prefs)
- **Approach:** For each dead pref: either wire it to a real setting or remove the UI + engine code. Measure impact per pref.

### G3.3 — File: PSD round-trip failures (~20 issues)
- **Missing:** Photoshop refuses PSDs we write (#1281, #2469), text-layer re-save (#2374, fixed), PSB > 4 GB on Windows (#375)
- **Where to fix:** `crates/io/` (PSD import/export), `crates/compose/` (compositor), `crates/codecs/` (PSD codec), `crates/engine/src/file_cmds.rs` (save/save-as/export), corpus/ (new)
- **Approach:** Corpus oracle comparison (Photoshop-authored vs our output), fix writer path by path

### G3.4 — Performance: budget violations (~41 issues)
- **Missing:** 3 of 25 budgeted scenarios meet budget; layer move with styles 321 ms (25 ms target), opacity 276 ms (20 ms), paste 12 MP 1.2 s (100 ms), 150-layer GPU crash (P12)
- **Where to fix:** `crates/compose/` (CPU compositor), GPU compositor path, `crates/engine/src/layer_multi_cmds.rs` (damage reporting), `crates/raster/` (tile operations)
- **Approach:** Profile each scenario, cache per-revision (never scan full surface per frame), parallelise work per tile

### G3.5 — Type tool freeze and depth (~46 issues)
- **Missing:** Type tool freeze on macOS (#2518, 60 s), type on a path, manual kerning, RTL paragraphs, full font fallback
- **Where to fix:** `crates/text/` (type engine, font serving via `CRAFT_FONTS_DIR`), `crates/engine/src/type_cmds.rs` (type commands)
- **Approach:** Profile freeze path (font loading? text layout?), fix blocking I/O, add missing type features

### G3.6 — Platform-specific: Wayland, Windows, macOS (~69 issues)
- **Missing:** Wayland tablet-v2 (#2622, #2297), WinTab tilt/rotation, monitor profile detection off-macOS, HDR/EDR
- **Where to fix:** `crates/tablet/` (isolated unsafe crate, `crates/tablet/src/macos.rs` for AppKit), `crates/ui-egui/src/monitor_status.rs` (monitor profiles), `crates/ui-egui/src/` (platform event handling)
- **Approach:** Platform-specific fixes, each isolated in `cfg()` blocks for wasm safety

## First steps (pick 3–6 to start)

1. **Audit 27 dead preferences** (G3.2). Read `crates/ui-egui/src/prefs_ui.rs`, cross-reference with scorecard prefs audit. For each: wire to a real setting or remove dead code. Expected: 2–4 h.
2. **Fix spring-loaded tools** (G3.1). Add hold-key state machine in `crates/ui-egui/src/tool_shortcuts.rs` and register spring-loaded tool commands in `UI_COMMANDS` in `menus.rs`. Expected: 4–8 h.
3. **Fix PSD round-trip corpus failures** (G3.3). Run `cargo xtask test-corpus`, compare our PSD output against Photoshop oracles in `corpus/ (new)`. Fix writer path per failing oracle. Expected: 20–40 h.
4. **Fix one performance budget** (G3.4). Pick the lowest-hanging: layer move with styles (321 ms → 25 ms). Profile in `crates/compose/`, cache per-revision, parallelise. Expected: 8–16 h.
5. **Fix type tool freeze** (G3.5). Profile the 60 s freeze path in `crates/text/` and `crates/engine/src/type_cmds.rs`. Fix blocking I/O or unnecessary re-layout. Expected: 4–8 h.
6. **Fix one Wayland tablet issue** (G3.6). Add Wayland tablet-v2 support in `crates/tablet/` (safe code path). Expected: 8–16 h.

## Progress tracking

- Each fix: file a report in `apps/photocraft/reports/ (new)` (via issue-tracker skill), link the GitHub issue number.
- Update `docs/gaps.md` G3 section with the date and count reduction.
- Update `docs/scorecard.md` if performance budgets are met.
- Raise `FLOOR` in `crates/ui-egui/src/parity.rs` when parity numbers improve.
- Commit the regenerated `docs/parity-checklist.md` after `cargo xtask parity`.
