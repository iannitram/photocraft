# PhotoCraft (fork): spec for 1.0

**Approval:** 2026-10-26, the owner, in the Project Manager session's question tool: "Both (Recommended)" (photocraft normalized like the other forks: spec = upstream plan + our delta, R18). A changed Must or Won't sets this back to "not yet".

## 1.0 in one sentence

The maintainers make a real YouTube thumbnail from one of their layered PSD templates in PhotoCraft (photo in a Smart Object, title type with strokes and glows, masks, Select Subject) and export it as JPG or PNG, on Linux, Windows and Mac, with no Photoshop needed, and the PSD saved back still opens in Photoshop.

## The spec is upstream's plan plus our delta

- **Upstream plan:** storytold/photocraft's [ROADMAP.md](https://github.com/storytold/photocraft/blob/main/ROADMAP.md), [docs/roadmap.md](https://github.com/storytold/photocraft/blob/main/docs/roadmap.md) and [docs/gaps.md](https://github.com/storytold/photocraft/blob/main/docs/gaps.md) (G1-G17), with its milestone "PSD that Photoshop trusts".
- **Our delta:** the goals and standing rules in the pinned issue #37 and the "95% list" pipeline in #29 (set 2026-10-11 and 2026-10-19). Our 70 commits on `ours` that upstream lacks are: self-logging (craft-log shared with FilmCraft), missing-font prompts, Linux/Wayland handheld window fixes and Wayland file drop, Select Subject quality and its graph-cut step budget, View > Snap behaviour, Replace Contents size notice, PSD/PSB save with a timeline, Camera Raw on small screens, and zh-hans translations.

| Delta | Upstream? |
|---|---|
| Fixes listed as "solid" in #35 (PSD export ids, import fallback allocation, linked-read hang, substituted-font gamma, stroke opacity, Select and Mask feather) | send upstream (policy in #35) |
| Select Subject changes (#11-#14) | send upstream once measured and solid |
| Self-logging, missing-font prompts, Wayland handheld fixes | keep on our fork |
| zh-hans translation rows | send upstream |

## Musts

| # | Must | Done looks like | Check | Issues |
|---|---|---|---|---|
| M1 | Every existing PSD opens, edits and saves back | All files in the maintainers' private PSD library open, render, save and reopen; a type layer survives a text edit; Photoshop opens the saved file | The PSD baseline run (counts only; nothing private leaves the machine) | #22, #28 |
| M2 | Rendering matches Photoshop on our test documents | The layer effects the templates use (Stroke, Outer/Inner Glow, Drop Shadow, Photo Filter, Brightness/Contrast, Posterize, masks, clipped adjustments) match Photoshop's saved image within the measured tolerance | Photoshop-made test files (rule 1 of #37) | #16-#20 |
| M3 | Select and Mask > Select Subject works on photos of people | Head-and-shoulders photos keep the body; the cut finishes within its step budget | The Select Subject test set | #11, #13, #14 |
| M4 | The 95% list is built | The list exists (#29), and every must-have item on it is proven on each platform | Each item's "done looks like" line passes per platform | #29 and its items |
| M5 | Performance on the maintainers' machines | Upstream's perf budgets met or explained on the AMD Strix Halo handheld and the 8 GB Apple M1 | `cargo xtask` perf rows | #30 |
| M6 | One real thumbnail job a day | A full thumbnail from a template, start to export, as stability evidence | The daily job log | #29 |
| M7 | Builds and tests are trusted | The corpus sets, GPU tests and window tests pass on `ours`; known failures are upstream's | #33's gates | #33, #40 |
| M8 | Platform parity | The above holds on Linux, Windows and Mac (droplets run on Mac) | Per-platform runs | #27, #31 |

## Won'ts (after 1.0)

Upstream's gaps outside the maintainers' 95% list (the rest of G1-G17: AI/generative features, plug-in compatibility, pen-tablet depth, the missing tools nobody here uses), round-tripping into the other Craft apps beyond what M1 needs (#28), and a local segmentation model unless #12 decides it is needed.

## Platforms

Linux (the AMD handheld, Wayland and gamescope), Windows and macOS (Apple M1, 8 GB): R04, see `docs/code-map.md`.
