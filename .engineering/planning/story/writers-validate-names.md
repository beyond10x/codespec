---
format: aep.planning-md/3
id: story:writers-validate-names
kind: story
status: implemented
title: Writers validate every name they emit
summary: A cir/1 name that is not an identifier of the target language is refused by the Rust, C and Go writers; no writer panics on input.
relations:
- decomposes: epic:adapters-untrusted-input
scope:
- confidence: inferred
  path: adapters/c
- confidence: inferred
  path: adapters/go/cmd/codespec-write-go
- confidence: inferred
  path: adapters/rust
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T03:37:01Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T03:37:01Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-10T17:52:04Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: Writers validate every name they emit

## Outcome

No writer puts a string from a `cir/1` document into generated output without either validating
it as an identifier of the target language or escaping it for the syntax it lands in.

## Acceptance

- `codespec-write-rust` (`adapters/rust/src/write.rs`, the `Cargo.toml` it formats from
  `m.name`): a module name that is not a valid package name is refused with exit 1 and a message
  naming it; nothing is written. A test feeds a name holding a quote and a newline followed by a
  `build = ` line and asserts the refusal and that no `Cargo.toml` exists.
- `codespec-write-c` (`adapters/c/src/write.rs`, `c_name` and every other place a `cir/1` name
  reaches the output): a name that is not a C identifier is refused the same way. A test feeds a
  name holding `;` and a call and asserts the refusal.
- `codespec-write-go`: the same check for Go identifiers and the package clause, with a test.
- Every writer: no input document makes it panic. The reproduced panic of the Rust writer has a
  test that expects exit 1 and a message.
- One shared check where the rule is the same for all writers: strict reading in `crates/model`
  refuses a module, declaration, field, parameter or local name that is empty or holds a character
  outside letters, digits and `_` (and `.`/`/` in a module path), if no checked-in `cir/1`
  document of the repository holds such a name; otherwise the check stays in each writer and the
  report lists the documents.
- `task check` steps `conformance`, `examples` and `adapters-check` exit 0: no checked-in document
  is refused.

## Scope

- Own: `adapters/rust` (writer), `adapters/c` (writer), `adapters/go/cmd/codespec-write-go`,
  `crates/model/src/lib.rs` (the name check only). Inferred.
- Not the readers: `adapters/rust-read` and `adapters/c-read` are being edited by other units.
