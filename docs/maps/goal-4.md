# Goal 4 — Brush and Transform Feel

*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

## Existing code

### Brush engine

- **Tip shape and dynamics model** — `crates/paint/src/brush.rs` defines `TipShape` (an enum: `Round` computed tip, `Sampled` bitmap tip, `Stored` tip loaded on demand; `brush.rs:85`), the per-parameter `Dynamic` (jitter, `control`, `fade_steps`, `minimum`, its own response `curve`; `brush.rs:47`), and the section structs built from it: `ShapeDynamics` (`:140`), `Scattering` (`:162`), `ColorDynamics` (`:361`) and `Transfer` (`:384`). There is no single `Dynamics` type. `BrushSettings` (`brush.rs:472`, `#[serde(default)]`) holds size, hardness, spacing, opacity, flow, colour and the sections.
- **Smoothing** — `brush.rs:448` declares `Smoothing` with `enabled`, `amount`, `pulled_string`, `catch_up`, `catch_up_on_end`, `adjust_for_zoom`. The default enables smoothing with `amount: 0.0`, `pulled_string` off and the other three flags on. The comment reads: "Lazy-mouse: paint only when the pointer pulls a string of `amount × 100 px` taut."
- **Dynamics execution** — `crates/paint/src/dynamics.rs` implements per-dab jitter, scattering, spacing, airbrush catch-up, and smoothing interpolation. Brush Pose is its own struct, `Pose` (`brush.rs:416`): `tilt_x`, `tilt_y`, `rotation`, `pressure` plus `override_*` flags. Pen rotation already drives dynamics through `Control::Rotation` (`dynamics.rs:355` value, `dynamics.rs:384` angle).
- **Brush UI panel** — `crates/ui-egui/src/brush_sections.rs` renders the Brush Settings panel with control pop-ups, mask modes, and pattern styles. Recent work (#4283) keeps open controls selected when toggling checkboxes; Smoothing can be disabled without discarding its amount or mode.

### Transform

- **Free Transform session** — `crates/ui-egui/src/transform_tool.rs` implements the bounding-box transform with corner/edge handles, pivot drag, arrow-key nudging, and a session-local `Steps` undo stack (`transform_tool.rs:143`). Handles have a 12 px grab radius (`HANDLE_PX` at `transform_tool.rs:114`).
- **Commit/cancel flow** — The session supports Esc cancel, Enter commit, and background commit jobs (`transform_tool.rs:89`). Right-click menu during transform (TOOL-214-5), options-bar buttons (#2442), and H/V skew fields are done.
- **Modifier gestures** — Shift-constrained axes, Alt-centered scaling, ⌘-drag temporary Move with box and snapping (#2684), and ⌘-arrow layer nudge (#2474) are implemented.

## Concrete gaps

### G4 — Brush feel (from `docs/gaps.md:162`)

- **Lag with big brushes** (#2619, #1994) and under the cursor (#2566, #1343): brush strokes stall on large-diameter brushes or when the pointer is stationary, producing visible lag between pen movement and paint appearance.
- **Jumpy pressure on Windows** (#2046) and tap sensitivity (#1933): pressure values fluctuate erratically on Windows tablets; light taps register as full-pressure dabs.
- **Stair-stepped line weights** (#1776): brush strokes produce visibly quantised edges rather than smooth anti-aliased boundaries, especially at small diameters.
- **Interpolation and smoothing problems** (#2138): existing `Smoothing` struct (`brush.rs:448`) has many options but stroke quality under fast pen motion still shows jitter and discontinuities.
- **Edge falloff** (#2149): brush edges do not fade naturally at stroke termini, producing hard stops instead of Photoshop-like taper.
- **Pixelated cursor** (#1433): brush cursor does not scale smoothly with zoom, unlike Photoshop's vector cursor.
- **Live preview only for Brush and Eraser** (TOOL-213-7): other tools lack transform-style live preview.
- **No image file as a brush tip** (TOOL-213-12): users cannot load arbitrary images as custom brush tips.
- **Per-control pressure curves** (TOOL-213-11): only a subset of dynamics sub-structs support Control curves; not every pressure-driven parameter is curve-editable.
- **4 of Photoshop's symmetry modes**: symmetry/drawing-guide modes are missing.
- **Minimum field interaction** (partial, #3825): Shape Dynamics Minimum Diameter/Roundness and Transfer four Minimum rows disable editing when Control is Off but retain configured values.

### G5 — Transform, handles and modifiers (from `docs/gaps.md:181`)

- **Full interpolation list**: the blend/interpolation mode dropdown during transform is incomplete.
- **Blend mode/opacity edits during transform** (#1384): users cannot change layer blend mode or opacity while the Free Transform box is open.
- **Path-point nudge**: no arrow-key or numeric nudge for vector path points during transform.
- **Autoscroll at the edge while selecting** (#2231): dragging near viewport edges does not auto-scroll the document.
- **Wheel and pinch behaviour** (#2485, #2463, #1989): mouse wheel and trackpad pinch gestures for zoom during transform are inconsistent with Photoshop.
- **Toolbar press-and-hold flyouts** (#2301): long-press on toolbar tools does not show the tool flyout panel.

## Small first steps

1. **Smoothing quality audit** — Run `cargo xtask perf` on a 24 MP image with a large `Smoothing::amount` (e.g. 0.8) and fast pen-motion input in `dynamics.rs`. Record before/after timings and compare stroke continuity against Photoshop reference. File the measured results in the dev log.
2. **Transform blend-mode/opacity editor** — Add a small combo box in the Free Transform options bar (next to existing H/V skew fields in `transform_tool.rs`) that reads/writes the layer's blend mode and opacity during the session, committing with Enter.
3. **Brush cursor smooth scaling** — Replace the pixelated cursor bitmap with a vector-drawn circle in the wgpu renderer, scaled by `zoom × size` in document pixels. Verify at 25 %, 100 %, and 400 % zoom.
4. **Edge falloff at stroke termini** — In `dynamics.rs`, add a taper factor that interpolates opacity from the full value at stroke midpoint to zero in the last N dabs. Tune N from `Smoothing::catch_up_on_end` behavior.
