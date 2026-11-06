Stage: Parked
Since: 2026-11-06
Kind: fork (upstream: storytold/photocraft)
Spec: SPEC.md   Roadmap: upstream ROADMAP.md, docs/roadmap.md and docs/gaps.md, plus our 95% list (#29)   Code map: docs/code-map.md
Platforms: linux, windows, mac
Notes: Parked 2026-11-06 on my call (every Craft app except WavCraft); no satellite takes issues here until I unpark it (R26). Before parking: Integration branch `ours` (main is an exact mirror of upstream; never commit to it). Upstream is about 723 commits ahead of ours (2026-10-28; the 2026-10-26 trial merge conflicted in 20 files): #49 (upstream-sync, Hard) is taken before anything else (R21), and upstream-gap build issues wait until it lands, since a gap may already be closed upstream. Agent rules for our pipeline: AGENTS.fork.md. 95% list work: #50, #51, #52. Merge-review findings on the sync merges already in `ours` (PRs #45, #46): #48 (`--version` lost its commit text; linked-PSD embed test); #47 is the same fault on `main`. No open Easy issues; open sized: 9 Hard, 16 Medium.

Stages: Proposal -> Spec -> Roadmap -> Dev Loop -> Prototype / Beta.
What each stage means and who moves a project: the pipeline repo (private), PIPELINE.md.
The Project Manager updates this file when the stage changes and adds a CHANGELOG line.
