# Rules for agents in this fork (our pipeline)

Upstream owns AGENTS.md, ROADMAP.md, docs/roadmap.md and docs/gaps.md: read them, never edit them for pipeline reasons. The full pipeline and rules live in the pipeline repo (private); this is the short copy. The goal and standing rules are the pinned issue #37.

1. **Find the stage** in `PIPELINE.md`. The spec is `SPEC.md`, the code map `docs/code-map.md`.
2. **Build on `ours`.** It is the integration branch; `main` mirrors upstream and is never committed to. Open PRs against `ours`.
3. **Work from an issue.** Take one that is not `in-progress` and not blocked; add `in-progress` before you start. Easy issues have a "Qwen brief". #49 (upstream sync) goes first.
4. **Your own platform's issues first.** Say which machine class you built on.
5. **Check that it builds**, then open the PR. CI and a review do the rest.
6. **Label the PR** `by:<machine>` and `model:<model>`, and write `Closes #N`.
7. **A failed job** gets a comment on the issue and the issue moves up one size.
8. **Measure Photoshop; do not guess it** (#37). No private PSDs, names or paths in code, commits, tests or PR text. Commit as the no-reply identity.
9. **Upstream offers** follow the "solid" policy in #35.
