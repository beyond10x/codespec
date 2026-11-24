---
format: aep.planning-md/3
id: story:edit-examples-and-docs
kind: story
status: implemented
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:48Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-24T07:54:01Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Checked-in edit examples in the gate, and the site page

## Outcome

`examples/edit/` holds a Go module (two packages) and a Rust crate. For each of the two languages
it holds one `.cq` program per wave-1 operation (`rename`, `rename_field`, `retarget`, `delete`:
eight programs) and one per refusal class the language's source can show: Go six (a reference in
unread code: a call inside a `go` statement; an exported target without `--closed-world`; a method
of an interface; a name collision with a local; a `generated` file; overlapping edits), Rust five (a
reference in unread code: a call inside a macro the reader does not read; an exported target
without `--closed-world`; a trait method; a name collision with a local; overlapping edits). The
`lowered` refusal has no wave-1 source form in the examples (no wave-1 name kind is lowered there);
it is tested on hand-written spans in `crates/edit/tests/main.rs` (`story:edit-engine-rename`).
Each program has its expected `.diff` or expected refusal text, and a `<name>.args` file with its flags (`--closed-world` for every program whose
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
  120 s, with eight operation programs (four per language) and eleven refusal programs (six Go, five
  Rust).
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
- Round 2, the per-language refusal list is not stated: six Go and five Rust programs, named above,
  and why `lowered` is not among them.

## Scope

- Own: `examples/edit/` (new), `scripts/examples-edit.sh` (new), `Taskfile.yml` (`examples-edit`),
  `scripts/check-parts.txt`, `website/docs/` (one new page and the sidebar entry),
  `crates/docs/src/status.rs`, `website/data/status.json` (generated), `AGENTS.md`, `README.md`,
  `CHANGELOG.md`.
- Runs after `story:edit-build-and-format`. `scripts/check-parts.txt` is also edited by
  `story:edit-spans-format`, which runs earlier.
