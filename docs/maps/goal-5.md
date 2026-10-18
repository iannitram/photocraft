# Goal 5 — Missing Photoshop Tools Map

*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

> Generated: 2026-10-16
> Scope: PhotoCraft (photocraft) — native `egui`/`eframe` desktop + WASM web build
> Source of parity numbers: `docs/target-app-parity.md`, `docs/ui-parity.md`

## 1. Tool inventory

PhotoCraft defines every toolbar tool as a variant of the `Tool` enum in
`crates/ui-egui/src/state.rs:64`. The enum has **67 variants**; `Tool::ALL`
(`state.rs:141`) is a `const [Tool; 67]` array that the menu system and
keyboard-shortcut registry iterate. Photoshop 2026 ships **68 tools** in the
toolbar flyouts.

### 1.1 Implemented tools (67 / 68)

| # | Photoshop tool | PhotoCraft variant | Wiring locations |
|---|---|---|---|
| 1 | Move | `Tool::Move` | `state.rs:65`, `state.rs:142` |
| 2 | Artboard | `Tool::Artboard` | `state.rs:66`, `state.rs:143` |
| 3 | Rectangular Marquee | `Tool::RectMarquee` | `state.rs:67`, `state.rs:144` |
| 4 | Elliptical Marquee | `Tool::EllipseMarquee` | `state.rs:68`, `state.rs:145` |
| 5 | Single Row Marquee | `Tool::SingleRowMarquee` | `state.rs:69`, `state.rs:146` |
| 6 | Single Column Marquee | `Tool::SingleColumnMarquee` | `state.rs:73`, `state.rs:147` |
| 7 | Lasso | `Tool::Lasso` | `state.rs:74`, `state.rs:148` |
| 8 | Polygonal Lasso | `Tool::PolygonLasso` | `state.rs:75`, `state.rs:149` |
| 9 | Magnetic Lasso | `Tool::MagneticLasso` | `state.rs:76`, `state.rs:150` |
| 10 | Magic Wand | `Tool::MagicWand` | `state.rs:77`, `state.rs:151` |
| 11 | Crop | `Tool::Crop` | `state.rs:78`, `state.rs:152` |
| 12 | Perspective Crop | `Tool::PerspectiveCrop` | `state.rs:79`, `state.rs:153` |
| 13 | Eyedropper | `Tool::Eyedropper` | `state.rs:80`, `state.rs:154` |
| 14 | Color Sampler | `Tool::ColorSampler` | `state.rs:81`, `state.rs:155` |
| 15 | Ruler | `Tool::Ruler` | `state.rs:82`, `state.rs:156` |
| 16 | Note | `Tool::Note` | `state.rs:83`, `state.rs:157` |
| 17 | Count | `Tool::Count` | `state.rs:84`, `state.rs:158` |
| 18 | Brush | `Tool::Brush` | `state.rs:85`, `state.rs:159` |
| 19 | Pencil | `Tool::Pencil` | `state.rs:86`, `state.rs:160` |
| 20 | Mixer Brush | `Tool::MixerBrush` | `state.rs:87`, `state.rs:161` |
| 21 | Color Replacement | `Tool::ColorReplacement` | `state.rs:88`, `state.rs:162` |
| 22 | Eraser | `Tool::Eraser` | `state.rs:89`, `state.rs:163` |
| 23 | Background Eraser | `Tool::BackgroundEraser` | `state.rs:90`, `state.rs:164` |
| 24 | Magic Eraser | `Tool::MagicEraser` | `state.rs:91`, `state.rs:165` |
| 25 | Gradient | `Tool::Gradient` | `state.rs:92`, `state.rs:166` |
| 26 | Paint Bucket | `Tool::PaintBucket` | `state.rs:167` |
| 27 | Horizontal Type | `Tool::Type` | `state.rs:94`, `state.rs:168` |
| 28 | Vertical Type | `Tool::VerticalType` | `state.rs:95`, `state.rs:169` |
| 29 | Horizontal Type Mask | `Tool::HorizontalTypeMask` | `state.rs:97`, `state.rs:170` |
| 30 | Vertical Type Mask | `Tool::VerticalTypeMask` | `state.rs:98`, `state.rs:171` |
| 31 | Hand | `Tool::Hand` | `state.rs:99`, `state.rs:172` |
| 32 | Rotate View | `Tool::RotateView` | `state.rs:100`, `state.rs:173` |
| 33 | Zoom | `Tool::Zoom` | `state.rs:101`, `state.rs:174` |
| 34 | Remove | `Tool::Remove` | `state.rs:102`, `state.rs:175` |
| 35 | Spot Healing Brush | `Tool::SpotHealing` | `state.rs:103`, `state.rs:176` |
| 36 | Healing Brush | `Tool::Healing` | `state.rs:104`, `state.rs:177` |
| 37 | Patch | `Tool::Patch` | `state.rs:105`, `state.rs:178` |
| 38 | Content-Aware Move | `Tool::ContentAwareMove` | `state.rs:106`, `state.rs:179` |
| 39 | Red Eye | `Tool::RedEye` | `state.rs:107`, `state.rs:180` |
| 40 | Clone Stamp | `Tool::CloneStamp` | `state.rs:108`, `state.rs:181` |
| 41 | Pattern Stamp | `Tool::PatternStamp` | `state.rs:109`, `state.rs:182` |
| 42 | History Brush | `Tool::HistoryBrush` | `state.rs:110`, `state.rs:183` |
| 43 | Blur | `Tool::Blur` | `state.rs:111`, `state.rs:184` |
| 44 | Sharpen | `Tool::Sharpen` | `state.rs:112`, `state.rs:185` |
| 45 | Smudge | `Tool::Smudge` | `state.rs:113`, `state.rs:186` |
| 46 | Dodge | `Tool::Dodge` | `state.rs:114`, `state.rs:187` |
| 47 | Burn | `Tool::Burn` | `state.rs:115`, `state.rs:188` |
| 48 | Sponge | `Tool::Sponge` | `state.rs:116`, `state.rs:189` |
| 49 | Quick Selection | `Tool::QuickSelection` | `state.rs:117`, `state.rs:190` |
| 50 | Object Selection | `Tool::ObjectSelection` | `state.rs:118`, `state.rs:191` |
| 51 | Pen | `Tool::Pen` | `state.rs:119`, `state.rs:192` |
| 52 | Freeform Pen | `Tool::FreeformPen` | `state.rs:120`, `state.rs:193` |
| 53 | Curvature Pen | `Tool::CurvaturePen` | `state.rs:121`, `state.rs:194` |
| 54 | Add Anchor Point | `Tool::AddAnchorPoint` | `state.rs:122`, `state.rs:195` |
| 55 | Delete Anchor Point | `Tool::DeleteAnchorPoint` | `state.rs:123`, `state.rs:196` |
| 56 | Convert Point | `Tool::ConvertPoint` | `state.rs:126`, `state.rs:197` |
| 57 | Path Selection | `Tool::PathSelection` | `state.rs:127`, `state.rs:198` |
| 58 | Direct Selection | `Tool::DirectSelection` | `state.rs:128`, `state.rs:199` |
| 59 | Rectangle | `Tool::Rectangle` | `state.rs:129`, `state.rs:200` |
| 60 | Ellipse | `Tool::EllipseShape` | `state.rs:130`, `state.rs:201` |
| 61 | Triangle | `Tool::Triangle` | `state.rs:131`, `state.rs:202` |
| 62 | Polygon | `Tool::Polygon` | `state.rs:132`, `state.rs:203` |
| 63 | Line | `Tool::Line` | `state.rs:133`, `state.rs:204` |
| 64 | Custom Shape | `Tool::CustomShape` | `state.rs:134`, `state.rs:205` |
| 65 | Slice | `Tool::Slice` | `state.rs:135`, `state.rs:206` |
| 66 | Slice Select | `Tool::SliceSelect` | `state.rs:136`, `state.rs:207` |
| 67 | Frame | `Tool::Frame` | `state.rs:137`, `state.rs:208` |

### 1.2 Missing tool (1 / 68)

| Photoshop tool | PhotoCraft status | Evidence |
|---|---|---|
| **Art History Brush** | **Missing** | Not in `Tool` enum (`state.rs:64-138`); `grep -r "ArtHistoryBrush" crates/` returns no matches; `docs/ui-parity.md` lists it as missing in the History group; `docs/target-app-parity.md:447` confirms it is the only missing tool. |

## 2. Tool flyout layout (TOOL_SECTIONS)

The toolbar flyout groups are defined by `TOOL_SECTIONS` at `crates/ui-egui/src/panels.rs:18`
(not in `state.rs`): five toolbar sections, 21 flyout slots in all. The slot Art History Brush
belongs in is `&[Tool::CloneStamp, Tool::PatternStamp]` or, as in Photoshop, next to
`&[Tool::HistoryBrush]` (Photoshop's Y flyout is History Brush / Art History Brush).

| Section | Slots (each `&[...]` is one flyout) |
|---|---|
| 1 | Move, Artboard |
| 2 | Marquees (4) · Lassos (3) · Object Selection, Quick Selection, Magic Wand · Crop, Perspective Crop, Slice, Slice Select · Frame · Eyedropper, Color Sampler, Ruler, Note, Count |
| 3 | Remove, Spot Healing, Healing, Patch, Content-Aware Move, Red Eye · Brush, Pencil, Mixer Brush, Color Replacement · Clone Stamp, Pattern Stamp · History Brush · Eraser, Background Eraser, Magic Eraser · Gradient, Paint Bucket · Blur, Sharpen, Smudge · Dodge, Burn, Sponge |
| 4 | Pen, Freeform Pen, Curvature Pen, Add/Delete Anchor Point, Convert Point · Type (4) · Path Selection, Direct Selection · Shapes (6) |
| 5 | Hand, Rotate View · Zoom |

**Stale doc:** `ROADMAP.md:5` ("toolbox count 56 of 68, 12 missing") and `ROADMAP.md:117`
("The 12 missing tools") predate the 2026-10-13 correction in `docs/gaps.md:57`,
`docs/ui-parity.md:39` and `docs/target-app-parity.md:447`: 67 of 68, only Art History Brush missing.

## 3. Art History Brush — gap analysis

### 3.1 What exists (reference: History Brush)

`HistoryBrush` (`Tool::HistoryBrush`) is fully wired and serves as a template:

| Component | File | Line(s) |
|---|---|---|
| Enum variant | `state.rs` | `110` |
| ALL array entry | `state.rs` | `183` |
| Label | `state.rs` | `257` |
| Icon registration | `icons.rs` | (search `history_brush`) |
| Options-bar UI | `retouch_ui.rs` | (search `HistoryBrush`) |
| Canvas event handling | `canvas.rs` | (search `HistoryBrush`) |
| Tool reset | `tool_reset.rs` | (search `HistoryBrush`) |
| Panel integration | `panels.rs` | (search `history`) |

### 3.2 What Art History Brush needs

*General Photoshop knowledge, not checked against PhotoCraft code (nothing exists yet):*
Art History Brush paints from the same History source as the History Brush (the state or
snapshot marked in the History panel), but lays down stylised, automatically generated strokes
instead of copying the source directly. Its options-bar settings are:

1. **Brush, Mode, Opacity** — as for the History Brush
2. **Style** — the stroke shape (Tight Short, Tight Medium, Loose Long, Dab, Tight Curl, …)
3. **Area** — the diameter the generated strokes cover
4. **Tolerance** — how much a pixel must differ from the source before strokes are painted there

Implementation plan (following History Brush pattern):

1. Add `ArtHistoryBrush` variant to `Tool` enum (`state.rs:110`, after `HistoryBrush`)
2. Add to `Tool::ALL` array (`state.rs:183`, after `HistoryBrush`)
3. Add label in `Tool::label()` (`state.rs:257`, after `HistoryBrush`)
4. Register icon in `icons.rs`
5. Add options-bar UI in `retouch_ui.rs` (Mode, Opacity, Style, Area, Tolerance)
6. Add canvas event handling in `canvas.rs`
7. Add tool reset in `tool_reset.rs`
8. Add it to the History Brush flyout in `TOOL_SECTIONS` (`panels.rs:18`)
9. Register engine command in `crates/engine/src/brush_cmds.rs` (history-aware paint operation)
10. Update `docs/parity-checklist.md` via `cargo xtask parity`
11. Update `docs/target-app-parity.md:447` to reflect 68/68

## 4. Verification

```bash
# Confirm no ArtHistoryBrush variant exists
grep -r "ArtHistoryBrush\|Art History Brush" crates/

# Verify 67 tools in ALL array
grep "Tool::ALL" crates/ui-egui/src/state.rs
grep -c "Tool::" crates/ui-egui/src/state.rs | head -1

# Run parity check after adding the tool
cargo xtask parity
```

## 5. Related documents

- `docs/ui-parity.md` — UI parity checklist (Art History Brush listed as missing)
- `docs/target-app-parity.md:447` — Tools parity assessment
- `docs/roadmap.md` — Milestone tracking
- `docs/gaps.md` — Known shortfalls ranked
- `docs/scorecard.md` — Per-area parity scorecard
- `crates/ui-egui/src/state.rs` — Tool enum, labels, ALL array
- `crates/ui-egui/src/panels.rs:18` — `TOOL_SECTIONS` (toolbar flyouts)
- `crates/ui-egui/src/icons.rs` — Tool icon registry
- `crates/ui-egui/src/retouch_ui.rs` — Retouch tool options-bar UI
- `crates/ui-egui/src/canvas.rs` — Canvas event dispatch per tool
- `crates/ui-egui/src/tool_reset.rs` — Tool state reset logic
- `crates/engine/src/commands.rs` — Command registry

## 6. Summary

| Metric | Value |
|---|---|
| Photoshop toolbar tools | 68 |
| PhotoCraft implemented | 67 |
| Missing | Art History Brush (1) |
| Flyout slots | 21, in 5 sections (`panels.rs:18`) |
| Implementation template | History Brush (`Tool::HistoryBrush`) |
| Estimated effort | Medium — follows History Brush pattern; requires history-state-as-brush-source logic |
