---
format: aep.planning-md/3
id: story:read-spans-drops-unplaceable-spans
kind: story
status: draft
title: codespec read --spans drops a span that names no node and keeps the rest
summary: A span naming no node of the cir/1 document is dropped with a stderr line; every other check stays strict
relations:
- decomposes: epic:release-diff-go-field-gaps
- depends_on: story:go-spans-same-key-functions
- informed_by: story:edit-spans-format
scope:
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/edit/src/run.rs
- confidence: inferred
  path: crates/edit/src/spans.rs
- confidence: inferred
  path: crates/edit/tests/main.rs
- confidence: inferred
  path: docs/code-edit-language.md
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 2
---
# Story: codespec read --spans drops a span that names no node and keeps the rest

## Outcome

`codespec read --spans` (and `codespec edit`, which reads through the same `read_spanned` in `crates/edit`) never loses
the whole spans document to one span the `cir/1` document has no node for. Before strict reading
(`crates/edit/src/spans.rs`, `check`), the spans a reader printed are filtered: a span or an
`unspanned` entry whose `at` names no node of the module (`Ids::kind` is `None`) is dropped, and
the command prints one line per dropped entry on standard error:
`dropped span <at> <role>: names no node of the cir/1 document <label>`. The rest of the document
is read strictly as today, so a span of a wrong kind, an overlap, a bad byte range or a missing
required span still refuses the document (those are a reader's defects that must stay loud).
Exit 0 when only drops happened.

Why stderr and not a field: `codespec-spans/1` is a closed format read strictly (unknown fields
refused). A `dropped` field is a format change, and the consumer needs the reason once, in its
log. Decision D1, default A: stderr lines. B: a new optional `dropped` list in `codespec-spans/1`;
it makes the drops part of the document and costs a format change in `crates/edit`, the reader
adapters and `docs/code-edit-language.md` § 6.

A dropped span leaves the node it named without a span: the completeness rule (`required`) asks
a span or an `unspanned` entry only for nodes the document has, so nothing else changes.

## Why

A release review of a Go service lost the spans of whole packages because of one span that named no
node ("names no node of the cir/1 document"), and read without spans from then on. Strict reading
is right to refuse a document that contradicts itself, but the reader and the document can
disagree on a construct (`story:go-spans-same-key-functions` is the likely cause of the two
failures seen), and a location for 99 nodes should not be lost to the 100th.

## Reproducer

Any reader output with one span whose `at` names no node: a spans document for `examples/series`
with one extra span `{"at": "funcs.999.body.0", "role": "name", …}` added by hand. Run strict
reading through `codespec_edit::spans::check` (the test) and `codespec read --spans` with a
reader program that prints that document (a script in the test's directory, POSIX sh).

## Observed

Not run. By the code: `allowed` in `crates/edit/src/spans.rs` returns the refusal "names no node
of the cir/1 document …" and `check` returns the first refusal, so the whole document is refused.

## Done When

- A failing test first in the integration-test binary of `crates/edit` (`tests/main.rs`): the
  document above is refused on `main`'s code; after the change `read_spanned` returns every other
  span and an entry for the drop.
- Near misses stay refused: a span at a node of the wrong kind (a `decl` span on an expression), a
  span with a bad byte range, two overlapping `name` spans, a missing required span, a ref whose
  text differs from the file's bytes.
- Three drops give three stderr lines, in the order of the document; none gives no line.
- `docs/code-edit-language.md` § 6.3 (the adapter protocol) and `codespec
  read --help` state the drop and the line; `task docs-generate` run.
- `task check` green; each new test under 60 s and the test step within the 120 s budget.

## Scope

- Own: `crates/edit/src/spans.rs` (the filter before `check`), `crates/edit/src/run.rs`
  (`read_spanned` reports the drops), `crates/edit/tests/main.rs`, `crates/cli/src/main.rs`
  (prints the lines), `crates/cli/src/lib.rs` (the help text of `--spans`),
  `docs/code-edit-language.md`, `website/docs/reference/cli.md`. Inferred; recorded as typed
  scope. `CHANGELOG.md` gets one line, outside the typed scope.
- Runs after `story:go-spans-same-key-functions`, which edits `crates/edit/src/ids.rs` and
  `tests/main.rs` too; the order avoids a conflict in `tests/main.rs` and nothing else.
- `codespec edit` keeps its own refusals for a node an edit needs and that has no span (§ 7.1);
  a dropped span is such a node.
