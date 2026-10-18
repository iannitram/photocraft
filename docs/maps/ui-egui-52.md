# ui-egui-52 — 11 top-level source files in `crates/ui-egui/src/`

*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

Maps 11 files so a coding assistant can navigate them without reading full source. Each entry shows the file's role, public API, and who calls it.

## Overview

`crates/ui-egui/src/` is the egui shell: it renders the UI, dispatches commands through the engine, manages document state, and exposes a JSON control protocol for agents. The 11 files fall into three groups:

- **Parity & testing** — `parity.rs` (menu coverage), `path_selection_tests.rs`, `pen_draw_tests.rs`, `pen_variants_tests.rs`, `pencil_tests.rs`, `pattern_fill_ui_tests.rs` (tool/dialog tests).
- **Pattern Fill / Pattern Preview** — `pattern_fill_ui.rs` (dialog), `pattern_fill_ui_tests.rs` (included via `#[path]` in `pattern_fill_ui.rs`), `pattern_preview.rs` (View › Pattern Preview tile math).
- **Patch, Perspective, Pen** — `patch_preview.rs` (live solve preview), `perspective_crop_ui.rs` (drag states, overlay), `perspective_ui.rs` (warp mode planes & homography).

## File maps

| File | Description | Key public items |
|---|---|---|
| `parity.rs` | Photoshop menu parity: counts live vs. missing items from `menu_catalog::CATALOG`, keeps a floor. | `FLOOR` (const, 631), `MenuParity`, `Parity`, `compute()`, `Parity::percent()`, `Parity::to_markdown()` |
| `patch_preview.rs` | Live coarse-solve preview for Patch Tool drags: computes an overlay tile, stores it in `App`, and draws it on the canvas. | `PatchPreview`, `take(app)`, `committed(app, preview, off)`, `display_doc(app, idx)`, `showing(app)`, `damage(app, doc, revision, seen, now)` |
| `path_selection_tests.rs` | Tests for loading paths as selections via keyboard shortcut (⌘↩ / Ctrl+Return). Included in `vector_ui.rs` via `#[path = "path_selection_tests.rs"]` under `mod path_selection_tests`. | `cmd_enter_with_the_pen_loads_the_work_path`, `cmd_enter_loads_the_path_selected_in_the_paths_panel`, `without_a_path_it_says_why_and_the_key_can_be_rebound`, `thumbnail_selection_ctrl_click_*` (4), `path_overlay_drawn_for_non_vector_tools` |
| `pattern_fill_ui.rs` | Pattern Fill dialog: field mapping (scale, angle, opacity, blend mode), live tile preview, and commit to engine. | `Preview`, `owns(&fields)`, `body(app, ui, &mut fields)`, `open(app)`, `open_new(app)`, `confirm(app, &fields)`, `display_doc(app, idx)` |
| `pattern_fill_ui_tests.rs` | Tests for Pattern Fill dialog opening, preview rendering, and commit. Included in `pattern_fill_ui.rs` via `#[path = "pattern_fill_ui_tests.rs"]` under `#[cfg(test)] mod pattern_fill_ui_tests`. | `new_pattern_fill_opens_the_dialog_previews_and_ok_is_one_step`, `cancelling_a_new_pattern_fill_adds_nothing`, `content_options_edit_the_layer_and_cancel_restores`, `double_clicking_the_thumbnail_opens_the_dialog_and_a_picked_pattern_is_applied` |
| `pattern_preview.rs` | Tile calculation for View › Pattern Preview: computes how many copies of a tile fit a viewport and per-tile offsets. | `copies(viewport, size)`, `offset(cell, doc_size)` |
| `pen_draw_tests.rs` | Tests for Pen tool modifier gestures (Alt/Option to break angle, drag to close subpath). Included in `vector_ui.rs` via `#[path = "pen_draw_tests.rs"]` under `mod pen_draw_tests`. | `alt_breaks_the_handle_being_dragged_and_grabs_the_last_outgoing_handle`, `dragging_on_the_first_anchor_closes_with_its_handles_in_one_step`, `opening_a_new_document_finishes_the_previous_pen_path`, `delete_with_the_pen_deletes_the_finished_work_path_not_the_pixels` |
| `pen_variants_tests.rs` | Tests for Freeform Pen and Curvature Pen gesture paths. Included in `lib.rs` via `mod pen_variants_tests`. | `freeform_pen_drag_is_fitted_into_one_work_path_or_shape`, `freeform_pen_alt_clicks_draw_straight_segments_between_freehand_runs`, `curvature_pen_clicks_make_a_smooth_path_through_every_point`, `new_pens_with_command_edit_paths_as_direct_selection` |
| `pencil_tests.rs` | Tests for Pencil tool features (auto-erase, Shift-straight lines, resizing mid-stroke). Included in `lib.rs` via `mod pencil_tests`. | `pencil_strokes_are_aliased_live_and_one_undo_step`, `auto_erase_paints_the_background_colour_from_foreground_pixels`, `shift_click_draws_a_pencil_line`, `ctrl_alt_drag_resizes_the_pencil_without_painting`, cursor and eraser-mode tests |
| `perspective_crop_ui.rs` | Perspective Crop tool: drag states (`Drag`), overlay rendering, keyboard shortcuts, and commit to engine. | `Drag`, `PerspectiveCrop`, `options_bar(app, ui)`, `keys(app, ctx)`, `draw_overlay(app, painter, xf)`, `pointer(app, ev)`, `commit(app)` |
| `perspective_ui.rs` | Perspective Warp: `PerspMode` (Layout/Warp), `PerspSession` with planes, pointer handling, homography preview rendering, and options bar. | `PerspMode`, `PerspSession`, `begin(app, ctx, command: &str)`, `control(app, ui: &Value)`, `pointer(app, ev, mods)`, `commit(app)`, `draw(p, painter, xf)`, `options_bar(app, ui)`, `PerspSession::describe()` |

## Data flow

**Pattern Fill** — User invokes via Edit › Fill › Pattern (menu triggers `pattern_fill_ui::open(app)` in `menus.rs`). Dialog opens as `DialogKind::Command` in `dialogs.rs` — `owns(&fields)` matches, `body()` renders fields + preview, `confirm()` commits. Canvas also calls `display_doc(app, idx)` to show a pattern-fill overlay. The control protocol exposes `open_new(app)` returning JSON.

**Pattern Preview** — View › Pattern Preview calls `pattern_preview::copies()` to tile the viewport and `pattern_preview::offset()` for per-cell positioning. Used in `canvas.rs` during tile rendering.

**Patch Tool** — `retouch_ui.rs` calls `patch_preview::take(app)` to grab the current preview, then `patch_preview::committed(app, preview, off)` on confirm. `canvas.rs` calls `display_doc()`, `showing()`, and `damage()` to render and cull the overlay. The preview struct is stored in `App.patch_preview` (`lib.rs`).

**Perspective Crop** — `panels.rs` renders the options bar via `perspective_crop_ui::options_bar()`. `shortcuts.rs` handles keyboard input via `keys()`. `canvas.rs` calls `draw_overlay()` and `pointer()` for mouse interaction.

**Perspective Warp** — `distort_ui.rs` is the central hub: `begin()` starts a session, `control()` applies control-protocol params, `pointer()` handles mouse, `commit()` applies, `draw()` renders planes, and `options_bar()` shows settings. The session lives in `App.distort.perspective`.

**Pen / Pencil tests** — All test modules (`path_selection_tests`, `pen_draw_tests`, `pen_variants_tests`, `pencil_tests`) are `#[cfg(test)]` modules included via `#[path]` or `mod` declarations in `vector_ui.rs` and `lib.rs`. They test the pen/pencil tool gesture logic defined in the vector editing subsystem.

**Parity** — `parity::compute()` iterates `menu_catalog::CATALOG`, checks each id against `menus::is_live()`, and builds `Parity` with per-menu live/total/missing lists. The test `parity_does_not_regress` asserts `live >= FLOOR` (631). `cargo xtask parity` writes `docs/parity-checklist.md` from `Parity::to_markdown()`.

## Control protocol

- `pattern_fill_ui::open_new(app)` — returns JSON for Pattern Fill dialog (dispatched via control protocol in `menus.rs`).
- `perspective_ui::begin()` — returns `Result<(), String>`; the dispatcher in `distort_ui.rs:121` answers with `describe()["perspective"]` JSON for the warp session state.
- `perspective_ui::control(app, &Value::Null)` — accepts control-protocol JSON to drive warp dialog controls.

## Warnings for newcomers

1. **Test files are not standalone modules.** `path_selection_tests.rs`, `pen_draw_tests.rs`, and `pattern_fill_ui_tests.rs` are included via `#[path = "..."]` attributes in their parent files (`vector_ui.rs` and `pattern_fill_ui.rs`). `pen_variants_tests.rs` and `pencil_tests.rs` are `mod` declarations in `lib.rs`. You won't find `mod xxx_tests` as a top-level file declaration.
2. **`pattern_preview.rs` has no `pub` struct — only free functions.** `copies()` and `offset()` are pure math helpers called from `canvas.rs` during tile rendering. No state lives here.
3. **`patch_preview.rs` state is owned by `App`.** The preview struct is stored in `App.patch_preview` and accessed via `take()` / `committed()` — never held by callers. `damage()` computes whether the overlay needs repainting based on document revision.
4. **`perspective_ui.rs` and `perspective_crop_ui.rs` are separate tools.** Perspective Warp (`perspective_ui`) is driven from `distort_ui.rs` (session in `App.distort.perspective`) and manages planes with homography. Perspective Crop (`perspective_crop_ui`) is a tool with drag states and a rectangular overlay. Don't confuse them.
5. **`parity.rs` FLOOR is a hard floor.** The test will fail if `live < 631`. When parity improves, raise `FLOOR` in the same change. Never lower it.
6. **`pattern_fill_ui::owns()` is a discriminant.** `dialogs.rs` uses it to match `DialogKind::Command` to the Pattern Fill handler. If you add fields to `PatternFields`, update `owns()` to check a stable subset (not transient preview state).
7. **Canvas calls into all preview modules.** `canvas.rs` lines ~1130–1270 and ~3000–4220 call `pattern_fill_ui::display_doc()`, `patch_preview::*`, `pattern_preview::*`, and `perspective_crop_ui::draw_overlay()`. This is the central render compositing point — if an overlay isn't showing, check here first.
