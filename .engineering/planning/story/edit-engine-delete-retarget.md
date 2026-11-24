---
format: aep.planning-md/3
id: story:edit-engine-delete-retarget
kind: story
status: implemented
title: 'crates/edit: retarget and delete'
summary: retarget a call to a function of the same module and signature; delete an unreferenced function
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-engine-rename
scope:
- confidence: inferred
  path: crates/edit
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:42:49Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T01:16:46Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-24T03:35:33Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: crates/edit: retarget and delete

## Outcome

`crates/edit` gains the two other wave-1 operations (`docs/code-edit-language.md` § 4.1, § 5):

- `retarget(Site, G)`: the call at `Site` names `G`; its callee name token is replaced by `G`'s
  source spelling (the `text` of `G`'s `name` span). Refused when `Site` is a listed call or has no
  `name` span, `G` is in another module than the current callee, `G`'s signature (parameters,
  results, receiver type) differs, or a local or parameter of the calling body has `G`'s source
  spelling.
- `delete(F)`: the `decl` span of `F` is removed as whole lines, with one blank line when both its
  neighbours are blank; `S'` drops `F` from `funcs` or `opaque`, `layout` and its type's method set,
  drops an `externs` entry no remaining call names, and moves later positions of that file up.
  Refused when anything in the set references `F` (a read call or function value, a listed call, an
  `extern` of another module, a Rust `use` path, an entry, a linkname, `transactional`), `F` is
  `main` or `init`, it implements an interface method, it is exported without `closed_world`, or its
  file is `generated`.

## Done When

`crates/edit/tests/main.rs`:

- retarget of two call sites (one in another module through `extern`) gives the expected `S'` and
  edits; one test per retarget refusal;
- delete of a function between two others, of the last function of a file, and of a method with a
  doc comment (its `MethodEntry` gone): expected `S'` with shifted positions and the expected file
  bytes; delete of a function whose only call is the last use of an extern drops that `externs`
  entry; one test per delete refusal;
- `delete` and `rename` of the same function in one change set overlap and refuse.

No test over 5 s.

## Scope

- Own: `crates/edit/src/retarget.rs`, `crates/edit/src/delete.rs` (new), `crates/edit/src/lib.rs`,
  `crates/edit/tests/main.rs`, `crates/edit/tests/fixtures/`.
- Runs after `story:edit-engine-rename` (same crate).
