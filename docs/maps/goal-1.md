# Goal 1: PSD that Photoshop trusts

*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

Save As corruption, Photoshop re-open check, and raising the Photoshop-oracle pass floor from 188 to 200+ (258 is the separate export round-trip floor).

## Goal description

PhotoCraft writes PSD/PSB files that Photoshop 26.6+ sometimes refuses to reopen (#1281, #2469; text-layer case #2374 closed). Goal 1 closes every gap that causes Photoshop to reject, mis-render, or silently corrupt our exports. It adds a **Photoshop re-open check** (FILE-216-4) — a test that opens every corpus export in a third-party editor — and raises the Photoshop-oracle pass floor from 188 to 200+ by fixing the remaining tagged-block, layer-record, and adjustment-composite issues. The work touches `photocraft_io` (PSD export/import), `photocraft_psd` (binary structures), `photocraft_engine` (file commands), and the test harness in `crates/io/tests/corpus.rs`.

## Existing crates, modules, and functions

| Crate / Module | Key symbols | Role |
|---|---|---|
| `crates/io/src/psd_export.rs` | `document_to_psd_with`, `PsdExportOptions`, `psd_background`, `SmartOut` | Converts `Document` → PSD/PSB bytes; handles layer records, smart objects, background detection, maximized compatibility (resource 1057), guides, layer comps, patterns |
| `crates/io/src/psd_import.rs` | `import_psd`, `Ctx`, `MAPPED_RESOURCES`, `REGENERATED` | Converts PSD/PSB bytes → `Document`; maps 19 resource IDs, regenerates 9 block types (`luni`, `lyid`, `lsct`, `lsdk`, `iOpa`, `lspf`, `lclr`, `lfx2`, `lfxs`), parses `Txt2`, smart-filter caches (`FEid`/`FXid`), placed layers |
| `crates/io/src/lib.rs` | `export`, `import`, `apply_file_encoding` | Public entry points; dispatches by file extension |
| `crates/engine/src/file_cmds.rs` | `save_doc`, `encode`, `save_a_copy`, `open_bytes_as`, `revert`, `SaveOpts` | File-menu commands: save, saveAs, saveACopy, open, revert; calls `photocraft_io::export` and `import` |
| `crates/psd/src/` | `PsdFile`, `LayerRecord`, `TaggedBlock`, `Compression`, `Header`, `Version` | Low-level PSD/PSB binary structures: headers, layer nodes, tagged blocks, compression, blend modes, color modes |
| `crates/psd/src/tagged.rs` | `BlockData`, `TaggedBlock` parsing | Tagged-block read/write, padding handling (Photoshop writes padding to 4-byte boundaries) |
| `crates/psd/src/resources.rs` | `ImageResource`, `ResolutionInfo`, `ids`, version_info_resource | Image resource blocks: ICC profile (1003), XMP (1060), thumbnail (1005), layer state (1040), unicode alpha names (1045), global angle/altitude (1027/1028) |
| `crates/psd/src/layer.rs` | `LayerFlags`, `BlendingRanges`, `LayerMask`, `MaskData`, `MaskParameters` | Per-layer metadata: clipping, blending, masks, real user masks (-3 channel), channel data |
| `crates/io/tests/corpus.rs` | `photoshop_oracle_corpus`, `Source { pass_floor, roundtrip_floor }`, `PHOTOSHOP` | Real-file oracle tests: parse → document → flatten vs. Photoshop's merged image; export → re-import → flatten round-trip; floors: `pass_floor: 188`, `roundtrip_floor: 258` for `corpus/photoshop` |
| `crates/engine/tests/photoshop_oracles.rs` | `photoshop_oracles_rerendered`, `PASS_FLOOR` | Re-renders Photoshop-authored smart-filter and type oracles with PhotoCraft's engine; checks against cached Photoshop pixels |
| `scorecard/file_compat.toml` | `FILE-216-1` through `FILE-216-6` | Trackable PSD infrastructure tasks: corpus CI, large fixtures, TIFF/RAW/EXR, third-party re-open, save-path tests, end-to-end workflows |

## Concrete gaps (code locations)

### Gap 1: No third-party re-open test (FILE-216-4) — `crates/io/tests/corpus.rs`
The corpus test only validates **our** export → **our** import round-trip. There is no test that opens PhotoCraft's exports in Photoshop or another third-party editor. The scorecard row `FILE-216-4` is marked "missing". A failing export that Photoshop rejects would be caught here.

> `crates/io/tests/corpus.rs:159` — `PHOTOSHOP` source defines `pass_floor: 188, roundtrip_floor: 258`. The test asserts `pass >= src.pass_floor` (line 435) but never invokes an external editor.

### Gap 2: Tagged-block padding and unknown blocks — `crates/psd/src/tagged.rs`
Photoshop writes tagged-block data padded to 4-byte boundaries. Unknown blocks may carry metadata PhotoCraft ignores. The import path keeps unknown blocks verbatim (AGENTS.md rule: "keep unknown PSD blocks verbatim"), but the export path may not re-serialize them identically, causing round-trip mismatches.

> `crates/io/src/psd_export.rs:19` — `RASTER_STALE_KEYS` lists 8 block types excluded from raster layers; any mismatch between what is written and what Photoshop expects could cause silent corruption.

### Gap 3: Smart object nesting and filter masks — `crates/io/src/psd_export.rs`
Smart objects have a hard nesting limit (`MAX_NESTING = 8`). Filter masks (`FEid`/`FXid` blocks) were fixed 2026-10-14 (oracle floor 185 → 188), but edge cases remain: placed layers with perspective transforms flattened (#579, fixed by #426), PSB smart objects rendering blank (#1764), and CS2-era smart objects (#2465, fixed). Remaining smart-object oracles may still fail.

> `crates/io/src/psd_export.rs:79` — `smart: SmartOut` handles embedded files and filter caches. `crates/io/src/psd_import.rs:58-63` — `filter_effects` and `plld_sizes` manage smart-object metadata.

### Gap 4: Background layer detection — `crates/io/src/psd_export.rs`
The `psd_background` function (lines 86-100) decides which layer becomes Photoshop's Background. It requires the bottom layer to be a locked (transparency + position), unclipped, unmasked, opaque raster with Normal blend. If the detection is too strict or too loose, Photoshop may open the file but misidentify the Background layer.

> `crates/io/src/psd_export.rs:88-91` — "Photoshop recognises a Background by the bottom layer having no transparency channel (measured on Photoshop 27.11: with the `-1` channel written, 317 of 427 re-exported corpus files reopened with the Background as an ordinary layer)."

### Gap 5: Adjustment layer compositing in CMYK/Lab/32-bit — `crates/io/src/psd_export.rs`, `crates/ops/`
CMYK documents must composite adjustments on their inks, not display RGB. Lab documents must composite on stored L, a, b. 32-bit adjustments must be unclipped. These were largely fixed 2026-10-12/13 (oracle floor 145 → 188), but remaining oracles may still fail on edge cases like pass-through clipping bases, gradient maps, or mask feathers.

> `docs/gaps.md:46` — "CMYK and Lab adjustment layers, 32-bit unclipped adjustments, Levels, Threshold, Posterize and mask feathers 2026-10-13"

## First steps (small, verifiable)

1. **Add a third-party re-open stub**: export a corpus file to a temp path, assert the file exists and is non-empty, and log a TODO comment invoking Photoshop or a headless oracle. This makes FILE-216-4 "partial" and visible in CI. (~2 h)

2. **Audit tagged-block padding in export** (`crates/io/src/psd_export.rs`): add a test that exports a document with 5+ tagged blocks of varying sizes, re-imports, and asserts each block's data length is a multiple of 4. If padding is missing, fix it in the `TaggedBlock` serialization path. (~4 h)

3. **Raise the Photoshop-oracle pass floor to 192**: run `cargo xtask test-corpus` with `PHOTOCRAFT_CORPUS_STRICT=1`, identify the 4 failing oracles, trace each to a specific block or layer, and fix the export/import path. Update the `pass_floor` at `corpus.rs:159`. (~8 h)

4. **Document the Background layer decision** (`crates/io/src/psd_export.rs`): add a comment block at `psd_background` explaining the detection criteria and linking to the measurement (317/427 corpus files), plus a test that constructs a 4-layer doc where only the bottom qualifies as Background and asserts the exported layer node has the correct flags. (~3 h)

5. **Smart object filter-mask regression test**: create a synthetic smart object with a Gaussian Blur filter, export to PSD, re-import, and assert the `FEid`/`FXid` blocks round-trip and the filtered pixels match. (~3 h)

**Total estimate for first steps: ~20 hours** (well within the 150-250 h budget for G1).

## Related tracking

- `docs/gaps.md` → G1 (PSD fidelity and Photoshop acceptance)
- `docs/roadmap.md` → Current focus item 1, FILE-216 tasks
- Saves are already crash-safe (temp file + fsync + rename): `crates/engine/src/file_cmds.rs:91-94` `write_file` calls `photocraft_format::atomic_write` (`crates/format/src/atomic.rs`), so atomic save is not a gap.
- `scorecard/file_compat.toml` → FILE-216-1 through FILE-216-6
- `ROADMAP.md` → Upcoming milestone "0.7 files you can trust"
- GitHub issues: #1281, #2469, #2374 (closed), #1764, #2465, #579, #375
