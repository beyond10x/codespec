---
format: aep.planning-md/3
id: story:use-cases-page
kind: story
status: implemented
title: 'A use-cases page: why use codespec'
relations:
- informed_by: story:builtin-defect-checks
scope:
- confidence: cited
  path: website/docs/index.md
- confidence: cited
  path: website/docs/use-cases.md
- confidence: cited
  path: website/product.json
- confidence: cited
  path: website/sidebars.ts
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-09T02:47:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-09T09:40:10Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
# Story: A use-cases page: why use codespec

## Outcome

The documentation site has a page `website/docs/use-cases.md`, "Use cases", in the Start here
group after the overview. It answers "why would I use this" with three uses, each a short section:
the problem, the commands with their real output from the installed binary, and what the user gets.

1. **Translate between languages with evidence.** Go → `cir/1` → Rust on `examples/orders`, with the
   harness byte-equal on 200 seeds × 300 commands and the refusal list for what was not translated.
2. **Know what a dependency can do.** `codespec effects` with the barrier on `examples/netcheck`:
   the syscalls each entry point may reach, and a CI step that fails when a new version of a
   dependency reaches a new effect (`proc`, `net`, `fs.write`). The page shows the exact `jq` or
   `diff` check.
3. **Find defects on one representation.** This story writes the section as planned: what a check
   reads (`cir/1` only, no source text), which checks `story:builtin-defect-checks` adds, that they
   are tested on google/capslock, and that they read the same `cir/1` every reader produces while
   being tested on Go only. It links the status page. No `codespec check` command or output appears,
   because 0.6.1 has none. Replacing it with real output is a follow-up after a release that holds
   the checks.

A section "Neighbours" names Google's capslock, which the operator named on 2026-11-09: what it
reports for Go packages and how use 2 differs.

## Acceptance

- Every command on the page is run with the installed 0.6.1 `codespec` (the Getting started install
  steps) in a clone at tag `0.6.1`, and its output is pasted from that run.
- Each claim about capslock links a page of capslock's own documentation and quotes the passage it
  rests on.
- No file under `website/` contains a name on the deny-list the coordinator holds outside the
  repository. The coordinator runs `grep -r -i -F -f <deny-list> website/docs website/src
  website/product.json` before the merge, and it must print nothing.
- The page is under 900 words, by `wc -w`.
- The overview (`website/docs/index.md`) links the page once, and the home page (`website/product.json`)
  has an action or card linking it.
- Site build green, `task ux-check` green, with the ratchet updated if a value improves.

## Out of Scope

- New features: the page describes what exists or says "planned".
- Neighbours other than capslock.

## Shared surface and merge order

The operator asked (2026-11-09) for `story:c-reader-clang`, `story:c-writer`,
`story:builtin-defect-checks` and `story:use-cases-page` to run in one wave. They share these
files, each by appending one entry:

- `Cargo.toml` `[workspace] members` and `Cargo.lock`: `adapters/c-read`, `adapters/c`,
  `adapters/c-run`, `crates/check`.
- `.github/workflows/check.yml`: one test-matrix line per new crate.
- `Taskfile.yml`: the `adapters` task and the manifest loops (C stories only).
- `crates/docs/src/main.rs` `REPORTS` (C writer only), `crates/docs/src/support.rs` `LANGUAGES` and the generated
  `website/data/support-matrix.json` (C stories only).
- `crates/cli/src/lib.rs` `Cmd` (`story:builtin-defect-checks` only adds `check`; the C stories add
  no command).

This story adds `website/docs/use-cases.md` and edits `website/sidebars.ts`, `website/docs/index.md`
and `website/product.json`, which no other story in the wave touches. The coordinator merges in the
order `story:c-writer`, `story:c-reader-clang`, `story:builtin-defect-checks`,
`story:use-cases-page`, resolves the append conflicts in the shared files by keeping both entries,
and runs `task check` after each merge.

Generated files (`website/docs/reference/crates.md`, `website/docs/reference/cli.md`,
`website/data/support-matrix.json`, `website/data/conformance/*.json`) are never merged by hand:
the coordinator takes either side and runs `task docs-generate` after each merge.
`crates/docs/src/support.rs` `LANGUAGES` gains `c` in `story:c-writer` only. `story:c-reader-clang`
does not edit `LANGUAGES` or `REPORTS`.
