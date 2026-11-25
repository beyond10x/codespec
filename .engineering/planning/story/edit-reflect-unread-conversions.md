---
format: aep.planning-md/3
id: story:edit-reflect-unread-conversions
kind: story
status: draft
title: edit refuses a Go rename whose type is converted to an interface in unread code
summary: the reflect rule also sees any(t) inside foreign or refused statements
relations:
- decomposes: epic:code-edit-language
revision: 2
---
# Story: edit refuses a Go rename whose type is converted to an interface in unread code

## Outcome

`codespec edit` refuses (rule `reflect`) a Go `rename_field` or exported method rename when a value
or pointer of the type is converted to an interface type inside code the reader kept `foreign` or
refused (`any(t)`, `fmt.Stringer(t)`, an `interface{}` argument in a foreign statement). Today the
`reflect` check reads only the read set, so such a conversion is not seen and the rename is
planned; printing or an implicit interface can then change behaviour.

Found by the fix pass of the edit wave's adversary findings (2026-11-24).

## Done When

- A CLI test in `crates/cli/tests/edit/mod.rs`: a Go package where the only conversion of the
  struct to `any` sits in a statement the reader keeps foreign; renaming its field exits 1 with rule
  `reflect` and writes nothing. Red before the change.
- The same for an exported method of the type.
