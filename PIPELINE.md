Stage: Dev Loop
Since: 2026-10-26
Kind: fork (upstream: storytold/photocraft)
Spec: SPEC.md   Roadmap: upstream ROADMAP.md, docs/roadmap.md and docs/gaps.md, plus our 95% list (#29)   Code map: docs/code-map.md
Platforms: linux, windows, mac
Notes: Integration branch `ours` (main is an exact mirror of upstream; never commit to it). Upstream is 279 commits ahead of ours and the trial merge conflicts in 20 files: #49 (upstream-sync, Hard) is taken before anything else (R21), and upstream-gap build issues wait until it lands, since a gap may already be closed upstream. Agent rules for our pipeline: AGENTS.fork.md. 95% list work: #50, #51, #52.

Stages: Proposal -> Spec -> Roadmap -> Dev Loop -> Prototype / Beta.
What each stage means and who moves a project: the pipeline repo (private), PIPELINE.md.
The Project Manager updates this file when the stage changes and adds a CHANGELOG line.
