---
name: language-wave
description: Adopt a new language in codespec, or implement one or more cir/1 features across several language adapters at once, in a single pass - one integration tree, one commit per unit, one fix pass, one review, one PR. Use when asked to add a reader or writer for a language, to raise reader or writer coverage, to make several adapters claim a feature, or to run a "reader wave" or "coverage wave".
---

# language-wave

One pass: plan, build every unit in one tree, regenerate once, review once, fix once, one PR.
Written on 2026-11-10 after a wave that took a night for six features (see "Why" at the end).

## 0. Decide whether to run at all

Read the reader and writer reports (`website/data/conformance/{reader,writer}-<lang>.json`,
`summary.claimed`, `summary.untested`). Write down per language: claimed now, and the features
that could be claimed (the language can express them and a kit case exists or can be written).
If the gap is under 3 features per language, say so to the operator and ask before starting: the
last features cost the most.

## 1. Plan once

- One story per unit in the planning store. A unit is one feature in one language, or one feature
  in several languages when the kit case and the runner change are shared.
- Order the units so that shared engine changes (`crates/eval`, `crates/conformance`, the kit
  under `conformance/cases/`) come first. Adapter-only units come after and do not depend on each
  other.
- A unit whose form the language's writer does not write is the kit's own convention. State that
  in the story, the crate doc and the commit; refuse every near miss with a test.

## 2. One integration tree, one branch

- One managed worktree, one branch `wave/<name>` from `main`. Every unit is built in this tree,
  one after the other. No worktree per unit: each one costs a cold build and a merge.
- Agents that work at the same time work in the same tree on disjoint directories
  (`adapters/<lang>/…` each) and never run `task docs-generate`. Shared files (`CHANGELOG.md`,
  kit cases) are edited by the coordinator, or by one agent at a time.
- At most one build runs in the tree at a time. Give each agent its own test command for its
  adapter (`cargo test -p <crate>`, the Java lane, the Go lane), not the full gate.

## 3. One commit per unit, no generated files in it

- A unit's commit holds source, tests, kit case sources, the crate doc and its CHANGELOG entry.
- It does not hold regenerated files: `website/data/conformance/*.json`, the reader corpora under
  `conformance/reader/`, `support-matrix.json`. These change with every unit and conflict with
  every other unit.
- Per unit, before the commit: failing test first, then the change, then the adapter's own tests
  green. Commit as the bot with a body that says what is claimed and what is refused.

## 4. Regenerate once

After the last unit commit: `task docs-generate`, then one commit "Regenerate the reports" with
the per-language summaries in its body. Then `task check` once.

## 5. One review, one fix pass

- One adversary pass over the whole branch diff (`git diff main...wave/<name>`), briefed per unit
  with what each claims. It writes red tests and findings, changes no implementation.
- One fix pass over all findings that the wave introduced, one commit per unit fixed. Then
  regenerate only if a report changes, and `task check` again.
- A pre-existing finding becomes a backlog draft. A second adversary pass only for a finding the
  fix pass could not close.
- Record per story: the review result (immutable), `review_outcome`, `test_result`, then move it
  to implemented. One store commit.

## 6. One PR

Sweep and `b10x-gates check` (AGENTS.md, Delivery), push the branch as the bot, one PR to `main`
with one commit per unit plus the regenerate, fix and store commits. Merge commit, no squash.
What could not be finished goes into one story in the same branch, listing each open item with
its measured state.

## 7. Adopting a new language

Same pass, with these units in this order:

1. The adapter program skeleton, its `--describe` output with an empty claim list, and its lane
   in `task adapters` and `task adapters-check`.
2. The reader corpus wiring (`conformance/reader/<lang>/`) and the report file for the language.
3. Features in groups by kind (`type.*`, `expr.*`, `stmt.*`, `decl.*`), one unit per group, each
   with kit case sources under `conformance/cases/<case>/<lang>/` and refusal tests for what the
   group leaves out.
4. The language's page and the support matrix entry, with the regenerate commit.

A feature that cannot be read without guessing stays unclaimed, with its reason in the adapter's
README.

## Never

- No disk probes (`df`, `du`, `find` over a tree) and no free-space rules: AGENTS.md.
- No benchmark or timing run on the workstation.
- No `task docs-generate` inside a unit.
- No worktree, PR or review round per unit.

## Why

Measured on 2026-11-10: six reader features (Rust +1, Java +3, C +2) took one worktree, one
adversary pass, one fix pass and one merge each. Every merge conflicted in nine generated report
files and needed a full regenerate, and every worktree built from cold.
