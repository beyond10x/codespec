---
format: aep.planning-md/3
id: story:edit-examples-and-docs
kind: story
status: proposed
title: Checked-in edit examples in the gate, and the site page
summary: examples/edit with one program per operation and refusal, task examples-edit, the site page
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-build-and-format
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/docs/src/status.rs
- confidence: inferred
  path: examples/edit
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/data/status.json
- confidence: inferred
  path: website/docs
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
---
# Story: Checked-in edit examples in the gate, and the site page

## Outcome

`examples/edit/` holds a Go module (two packages) and a Rust crate, and one `.cq` program per
wave-1 operation and per refusal case, each with its expected `.diff` or expected refusal text.
`task examples-edit` runs every program as a dry run and compares output and exit code, then applies
the operations to a copy under `target/` with `--build` and re-runs them (no change the second
time). It is a part of the gate. The site has a page on the edit language (notation, operations,
refusals, verification), the CLI reference is generated, and the capability list names
`codespec edit` as shipped with its check.

## Done When

- `task examples-edit` passes in `task check` and in CI (`scripts/check-parts.txt`), in under 120 s.
- Changing one expected `.diff` by one byte makes it fail (checked once by hand, stated in the PR).
- `website/docs/` has the page; `crates/docs/src/status.rs` names the capability with
  `Taskfile.yml::examples-edit`; `task docs-generate` run, `docs-check` passes.
- `AGENTS.md` lists `crates/edit` in the table of what the repository owns, `task examples-edit` in
  the gate table, and the new generated files.

## Scope

- Own: `examples/edit/` (new), `Taskfile.yml` (`examples-edit`), `scripts/check-parts.txt`,
  `website/docs/` (one new page and the sidebar entry), `crates/docs/src/status.rs`,
  `website/data/status.json` (generated), `AGENTS.md`, `README.md`, `CHANGELOG.md`.
- Runs after `story:edit-build-and-format`. `scripts/check-parts.txt` is also edited by
  `story:edit-engine-rename`, which runs earlier.
