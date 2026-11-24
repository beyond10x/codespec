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
  path: scripts/examples-edit.sh
- confidence: inferred
  path: website/data/status.json
- confidence: inferred
  path: website/docs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
---
# Story: Checked-in edit examples in the gate, and the site page

## Outcome

`examples/edit/` holds a Go module (two packages) and a Rust crate. For each of the two languages
it holds one `.cq` program per wave-1 operation (`rename`, `rename_field`, `retarget`, `delete`:
eight programs) and one per refusal class that applies to the language (a `lowered` span, a
reference in unread code, an exported target without `--closed-world`, an interface method, a name
collision, a `generated` file for Go, overlapping edits), each with its expected `.diff` or expected
refusal text, and a `<name>.args` file with its flags (`--closed-world` for every program whose
target is exported).

`scripts/examples-edit.sh` (POSIX sh) runs every program as a dry run and compares output and exit
code; for each refusal it compares the SHA-256 of every input file before and after (unchanged);
then it applies the operations to a copy under `target/` with `--build` and runs them again (no
change the second time). `--self-test` first copies one expected `.diff` with one byte changed and
requires the comparison to fail. `task examples-edit` runs the self-test, then the examples; it is a
part of the gate.

The site has a page on the edit language (notation, operations, refusals, verification), the CLI
reference is generated, and the capability list names `codespec edit` as shipped with its check.

## Done When

- `task examples-edit` passes in `task check` and in CI (`scripts/check-parts.txt`), in under
  120 s, with eight operation programs (four per language) and the refusal programs listed above.
- `task examples-edit` fails when its self-test does not detect the changed byte (the self-test is
  part of the task, not a manual step).
- `website/docs/` has the page; `crates/docs/src/status.rs` names the capability with
  `Taskfile.yml::examples-edit`; `task docs-generate` run, `docs-check` passes.
- `AGENTS.md` lists `crates/edit` in the table of what the repository owns, `task examples-edit` in
  the gate table.

## Critic findings answered

- Nothing requires one program per operation and refusal (acceptance, blocker): the counts above.
- A one-byte corruption checked by hand (acceptance): the automated self-test.
- Every operation on Go and Rust (scope, blocker): four programs per language.
- No file changed on a refusal (scope): the hash comparison.
- Programs targeting exported functions need `--closed-world` (design): `.args` files.

## Scope

- Own: `examples/edit/` (new), `scripts/examples-edit.sh` (new), `Taskfile.yml` (`examples-edit`),
  `scripts/check-parts.txt`, `website/docs/` (one new page and the sidebar entry),
  `crates/docs/src/status.rs`, `website/data/status.json` (generated), `AGENTS.md`, `README.md`,
  `CHANGELOG.md`.
- Runs after `story:edit-build-and-format`. `scripts/check-parts.txt` is also edited by
  `story:edit-spans-format`, which runs earlier.
