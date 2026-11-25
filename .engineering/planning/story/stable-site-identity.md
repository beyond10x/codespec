---
format: aep.planning-md/3
id: story:stable-site-identity
kind: story
status: draft
title: A call site is named by its function, its callee and its ordinal, never by line
summary: A derived call-site id {callee, n} per function, defined in crates/model walk.rs and carried in codespec effects reports beside the positions
relations:
- decomposes: epic:release-fact-diff
scope:
- confidence: inferred
  path: crates/cli/tests/effects.rs
- confidence: inferred
  path: crates/cli/tests/fixtures/moved-lines
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: crates/effects/tests/effects.rs
- confidence: inferred
  path: crates/model/src/walk.rs
- confidence: inferred
  path: examples/envcheck/envcheck.effects.json
- confidence: inferred
  path: website/docs/examples
revision: 4
---
# Story: A call site is named by its function, its callee and its ordinal, never by line

## Outcome

Every call a function makes has a call-site id that is the same in two revisions of the code
when the function, the callee and the calls of that callee before it are the same:

- the function: `<module path>.<function key>`, the module path being the module's `path`, or
  its `name` when it has none (the rule of `func_id` in `crates/eval/src/id.rs`);
- the callee, spelled as `codespec effects` resolves it: `<path>.<name>` for a function,
  `<path>.<type>.<name>` for a method of a type of a module or of an extern type, `iface:<interface
  key>.<name>` for a call through an interface, `value:<signature>` for a call of a function value,
  `println` for the built-in; a `foreign` statement's or an opaque function's listed calls spelled
  the same way from their `CallRef`;
- the ordinal `n`, from 1: this is the n-th call of that callee in the function, in the order
  `codespec effects` visits calls today: the body's statements in pre-order, each statement's
  expressions in `stmt_exprs` order, then the statements of each lambda body the function holds
  (a lambda's calls are its holder's).

In JSON a site is `{"callee": "<callee>", "n": <n>}`, with `"func": "<function>"` where the
function is not given by the enclosing object; in text, `<callee>@<n>`. No part holds a file or a
line. The id is derived: one function in `crates/model/src/walk.rs`,
`call_sites(module, func) -> Vec<CallSite>` with `callee`, `n` and the `pos` of the statement
making the call, defines the spelling and the order, and its doc comment is the definition.
`codespec effects` uses it: each `{name, at}` entry of `syscalls` and `effect_sites` gains
`sites`, the site of each call in `at`, in the same order. `at` stays for people.

### Decision: derived by the tools, not a new `cir/1` field

- Every consumer of the id gets it the same way: the ids reach consumers in codespec's reports
  (`codespec effects` here, `codespec diff` in `story:diff-command`), all computed by the one
  function in `crates/model`. It is a pure function of the canonical document: the callees are
  names strict reading already resolves, and the order of calls is the order of the document,
  which is deterministic (`task examples-check`). A tool outside this repository that reads
  `cir/1` itself follows the doc comment and gets the same ids.
- A field would be stored redundancy: strict reading would have to recompute and refuse a wrong
  one (as it does for `Func.raises`), every reader (Go, Rust, C, Java, Groovy, Elixir, Gleam,
  Odin) would write it, every checked-in example document and conformance case would be
  regenerated, and the writers' fixed-point tests would carry it. A field of identity is, by the
  rules for `cir/1` in `AGENTS.md`, a question for `cir/2`.
- The cq node id (`docs/query-language.md` § 4.1, `#3.then.1.x`) is no answer: it is stable when
  lines move but changes when a statement is inserted above the call. It stays as it is.

## Why

`codespec effects` names a site by `<file>:<line>` only. A line added at the top of a file moves
every site below it, so two revisions of unchanged code differ at every site and a diff of them
is noise. The consumer's check: a fixture whose only change moves lines yields no delta.

## Reproducer

A Go module `example.com/probe`, two revisions `a/` and `b/` of one package:

```go
// a/probe.go
package probe

import "os"

func Load() string {
	v := os.Getenv("MODE")
	if v == "" {
		v = os.Getenv("DEFAULT_MODE")
	}
	return v
}
```

`b/probe.go` is the same with three comment lines and a blank line added above `func Load`.
Read both with `codespec-read-go`, run `codespec effects` on each with
`summaries/go-std.effects.json`.

## Observed

Not run on main `5f82fff9`. By the report format: the `env` entry of `effect_sites` for `Load` is
`"at": ["probe.go:6", "probe.go:8"]` in `a` and `"at": ["probe.go:10", "probe.go:12"]` in `b`;
nothing else in the entry names the two calls.

## Done When

- A failing test first, over the reproducer read with `codespec-read-go` (its sources checked in
  under `crates/cli/tests/fixtures/moved-lines/`): the `sites` of every `effect_sites` and
  `syscalls` entry of the two `codespec effects` reports are equal (`os.Getenv@1`,
  `os.Getenv@2`) while `at` differs.
- The consumer's check: a fixture whose only change moves lines yields no delta: every `sites`
  list of the two reports is equal, and every `funcs` entry is equal once `pos` and `at` are
  dropped (a test). `story:diff-command` repeats this check through `codespec diff`.
- Near misses, each a unit test of `call_sites` over hand-built `cir/1` (in `walk.rs`):
  - a function moved to another file of the same module: same ids;
  - two calls of different callees swapped: same ids;
  - two calls of the same callee swapped: the ids keep their ordinals and follow the order, so
    `@1` and `@2` exchange what they reach (pinned, written in the doc comment);
  - a call of the same callee inserted before an existing one: the existing call becomes `@2`
    (pinned and written in the doc comment as the limit of the ordinal);
  - a call inside a lambda counts as its holder's, after the holder's own statements;
  - a `foreign` statement's listed calls get ids in the order listed.
- `examples/envcheck/envcheck.effects.json` regenerated with `sites` (`task examples-effects`
  equal); the netcheck files read with `sites` absent (the field defaults to empty) and are
  rewritten by `task examples-netcheck` where Go 1.27.1 is installed, else left and the report
  says so.
- The crate doc of `b10x-codespec-effects` (`codespec-effects-report/1`) names `sites`;
  `task docs-generate` run; `task check` green; each new test under 60 s, the test step within
  the 120 s budget.

## Scope

- Own: `crates/model/src/walk.rs` (`call_sites`, its doc, the near-miss unit tests),
  `crates/effects/src/set.rs` (sites carried with positions),
  `crates/effects/src/lib.rs` (`Site.sites`, crate doc), `crates/effects/tests/effects.rs`,
  `crates/cli/tests/effects.rs` and `crates/cli/tests/fixtures/moved-lines/` (the Go fixture,
  `codespec-read-go` built once per test binary as `crates/cli/tests/barrier.rs` does),
  `examples/envcheck/envcheck.effects.json`, `website/docs/examples/` (what `task docs-generate`
  rewrites). Inferred; recorded as typed scope. `CHANGELOG.md` gets one line, outside
  the typed scope; conflicts there are resolved at merge.
- No change to `crates/model/src/lib.rs`, the schema or any reader: no `cir/1` change.
- Shares no file with `story:go-http-entries`; both can run at once.
- `story:reach-entry-file` (epic `sound-reach`, draft) also edits `crates/effects/src/lib.rs`
  (entry matching); whichever starts second rebases on the other. No edge: neither needs the
  other's outcome.
