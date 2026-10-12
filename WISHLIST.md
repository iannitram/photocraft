# PhotoCraft: our wishlist

Our priorities for this fork, in order. **Ian owns this file.** The nightly agent reads it at the start of every run and never edits it; it suggests changes in its log under "Needs Ian". Upstream's own roadmap is in the code branch (`ROADMAP.md`, `docs/roadmap.md`, `docs/gaps.md`), not here.

Use: YouTube thumbnail production. Hardware: AMD Strix Halo laptop (Ryzen AI Max+ 395, Radeon 8060S, Linux, RADV Vulkan) and an Apple-Silicon Mac.

*Last changed: 2026-10-12 (real PSDs allowed for item 1).*

## 1. Open and edit real Photoshop thumbnail PSDs

Layered thumbnail PSDs made in Photoshop must open, edit and save correctly: layers, masks, smart objects, text layers, layer effects, adjustment layers. This matches upstream's #1 milestone.

- Upstream issues: #1281, #2469 (Photoshop accepting PSDs we write)
- Code areas: `crates/psd`, and the io/format code that reads and writes PSD
- Real files: the nightly agent may test against Ian's own thumbnail PSDs (read-only, via the Dropbox connector). They're private: refer to them only by label (`psd-xxxxxx`), never commit them or anything derived from them, and reproduce any bug they expose with a synthetic file.

## 2. Select and Mask → Select Subject that works on people

Ian used Photoshop's Select Subject inside Select and Mask constantly: people, faces, hair. Today it's a classical heuristic, too slow and weak on hair.

- Upstream issues: #2788 (local segmentation model, BiRefNet / SAM 2, heuristic as fallback; unassigned), #211 (Select Subject 559 ms vs a 100 ms budget), #2671 (no Refine Hair), #214 (Select and Mask is a dialog, not the workspace)
- Code areas: `crates/algo/src/segment/`, `crates/engine/src/smartselect_cmds.rs`, `crates/engine/src/cutout_cmds.rs`
- Larger work (the #2788 model backend) can span several nights: design note in `STATE.md`, reviewable increments.

## 3. AMD / Strix Halo performance

Make PhotoCraft fast on Ian's AMD iGPU with unified memory.

- Upstream issues: #49 (tiling above the GPU texture limit), the over-budget scenarios in `perf/budgets.toml` / `docs/scorecard.md`
- Code areas: GPU compose and effects (wgpu), wgpu startup, CPU-only filters or fallbacks that could run on the GPU, multi-core CPU paths
- Anything that needs the real GPU goes under "Needs Z13 testing" with exact steps.

## 4. Help upstream's goals

Speed (#211 and the perf budgets), the missing tools (`docs/parity-checklist.md`, `docs/gaps.md`), and PSD compatibility.
