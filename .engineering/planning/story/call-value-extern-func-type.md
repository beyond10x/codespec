---
format: aep.planning-md/3
id: story:call-value-extern-func-type
kind: story
status: draft
title: A call of a function value of a standard-library function type has an effect
summary: 'defer cancel() after context.WithTimeout gives no unknown: a summary entry can name a function type'
relations:
- decomposes: epic:release-diff-go-field-gaps
- informed_by: story:go-std-effect-summaries-common
revision: 1
---
# Story: A call of a function value of a standard-library function type has an effect

## Outcome

`ctx, cancel := context.WithTimeout(parent, d); defer cancel()` is the usual shape of a Go
service's request handling. `cancel` is a local of the extern function type
`context.CancelFunc`; the call `cancel()` is a `call_value` with no candidate function in the set,
so `codespec effects` reports `function value with no candidate in the set: context.CancelFunc`
(`crates/effects/src/set.rs`, `resolve_values`) for every such function, and the summary format has
no way to name it: an entry names a function or a method, not a function type.

The change: an effect summary entry can name a function type of a package
(`context.CancelFunc`, `context.CancelCauseFunc`, `http.HandlerFunc` called directly), and a
`call_value` whose value has that extern type, with no candidate in the set, takes the entry's
effects instead of the unknown. `summaries/go-std.effects.json` gets entries for the function
types of the packages it covers whose values the standard library itself produces
(`context.CancelFunc`, `context.CancelCauseFunc`: pure).

## Acceptance

- The reproducer of story:go-std-effect-summaries-common (`crates/effects/tests/data/probe`): `Run` has the effect `time` and an empty `unknown`.
- A `call_value` of a local of an extern function type with no entry still reports the unknown.
- A `call_value` whose value has candidates in the set keeps them; the type entry is used only with none.

## Found by

The adversary round of story:go-std-effect-summaries-common (2026-11-26): its acceptance clause
"an empty unknown" for `Run` cannot be met by summary entries.
