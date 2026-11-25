---
format: aep.planning-md/3
id: story:stable-site-identity
kind: story
status: draft
title: A call site is named by its function, its callee and its ordinal, never by line
summary: A call-site id {callee, n} per function, defined in the effects crate doc (order over walk_stmts and stmt_exprs of crates/model walk.rs, which is not changed) and carried in codespec effects reports beside the positions
relations:
- decomposes: epic:release-fact-diff
scope:
- confidence: inferred
  path: crates/cli/tests/effects.rs
- confidence: inferred
  path: crates/cli/tests/fixtures/moved-lines
- confidence: inferred
  path: crates/docs/src/netcheck.rs
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: crates/effects/tests/effects.rs
- confidence: inferred
  path: examples/envcheck/envcheck.effects.json
- confidence: inferred
  path: website/docs/examples
- confidence: inferred
  path: website/docs/guides/map-a-repository-to-syscalls.md
revision: 9
---
# Story: A call site is named by its function, its callee and its ordinal, never by line

## Outcome

`codespec effects` names every site it reports by a call-site id that is the same in two
revisions of the code when the function, the callee and the sites of that callee before it are
the same. Each `{name, at}` entry of `syscalls` and `effect_sites` gains `sites`: the ids of the
function's own sites that lead to that syscall or effect, sorted by `callee`, then `n`, each
once. `sites` is a list of its own, not parallel to `at`: two sites on one line are one position
in `at` and two ids in `sites`. `at` stays for people.

### The call-site id, a definition

A site is `{"callee": <callee>, "n": <n>}` inside the report entry of the function that holds it
(the entry names the function by `path` and `name`); in text, `<callee>@<n>`. No part holds a
file or a line.

The sites of a function are the nodes in the table below, in this order: the body's statements in
pre-order (`walk_stmts`), within a statement its expressions in `stmt_exprs` order and each
expression tree in pre-order (the outer call of `f(f(x))` before the inner), then the statements
of each lambda body the function holds, in the order found (a lambda's sites are its holder's).
`n` counts from 1 per callee within the function.

| node | `callee` |
|---|---|
| `Expr::Call` | `<module path>.<name>`, the holder's module |
| `Expr::Method` | `<module path>.<type>.<name>`, the module and name of the receiver's named type; on a receiver of type `error`, `iface:error.<name>` (the analysis dispatches it as a call through the interface `error`) |
| `Expr::Extern` | `<path>.<name>` |
| `Expr::ExternMethod` | `<path>.<type>.<name>` |
| `Expr::IfaceCall` | `iface:<interface key>.<name>`, the key `Interface::key` gives |
| `Expr::CallValue` | `value:<signature>`, the signature spelled as the effects analysis spells one (`func([]byte) (int, error)`) |
| `Expr::Intrinsic` `println` | `println` |
| a `CallRef` of a `foreign` statement's or an opaque function's `calls`, in the order listed | `func`: `<path>.<name>`, `path` the holder's module path when empty; `method`: `<path>.<recv>.<name>`, the same; `iface`: `iface:<iface>.<name>`; `value`: `value:<sig>`; `lambda`: `lambda:<sig>` (the literal's own calls are sites of the literal, which the holder calls); `ref`, `new` and `named` are no site |
| a function without a body whose syscalls the barrier fixes, or which a linkname links | `body`, `n` 1 |

The module path is the module's `path`, or its `name` when it has none (the rule of `func_id`
in `crates/eval/src/id.rs`). Every other node is no site: a function used as a value
(`Expr::FuncRef`, `CallRef::Ref`) or a function literal in a body (`Expr::Lambda`) is not
called there, and the calls inside such a literal are its holder's sites as above. A call the
analysis records as `unknown` instead of an edge or an effect (an extern without a summary, a
method on an undeclared type, a `named` call ref, a call of an undeclared function) is no site
either: the `unknown` entry keeps its position and reason.

The table and the order are written in the crate doc of `b10x-codespec-effects` (the section on
`codespec-effects-report/1`), which is the definition; a tool outside this repository that reads
`cir/1` follows it. The order uses `walk_stmts` and `stmt_exprs` of `crates/model/src/walk.rs` as
they are; this story does not change that file.

### How the analysis carries it

`crates/effects/src/set.rs` records the site at each `set()` and `edge()` call, beside the
position it records today, in new fields named apart from the existing `Node.sites` (bit and
`Pos`): `Node.bit_calls` for the bits a call sets and `Node.edge_calls` for the edges, by index.
The report gathers `sites` from them the way it gathers `at`. `unknown()` records no site.
Nothing joins sites to calls by position.

### Decision: derived by the tools, not a new `cir/1` field

- Every consumer of the id gets it the same way: the ids reach consumers in codespec's reports
  (`codespec effects` here, `codespec diff` in `story:diff-command`, which takes them from the
  same analysis). They are a function of the canonical document: callee names strict reading
  already resolves, and the order of nodes in the document, which is deterministic (`task
  examples-check`).
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
nothing else in the entry names the two calls. `at` is gathered as a set of positions per effect
or syscall (`crates/effects/src/set.rs`, `BTreeSet<Pos>`), so two calls on one line are one entry.

## Done When

- A failing test first, over the reproducer read with `codespec-read-go` (its sources checked in
  under `crates/cli/tests/fixtures/moved-lines/`): the `sites` of the `env` entry are
  `os.Getenv@1` and `os.Getenv@2` in both reports while `at` differs.
- The consumer's check: a fixture whose only change moves lines yields no delta: every `sites`
  list of the two reports is equal, and every `funcs` entry is equal once `pos` and `at` are
  dropped (a test). `story:diff-command` repeats this check through `codespec diff`.
- Tests over hand-built `cir/1` in `crates/effects/tests/effects.rs`, one per case:
  - `f(f(x))`, `f` an extern with an effect: sites `f@1` (the outer) and `f@2`, one position;
  - two calls of one callee in two statements on one line: two sites, one position;
  - a `foreign` statement with listed calls: each listed call is a site, spelled by the table,
    in the order listed;
  - a call inside a lambda counts as its holder's, after the holder's own statements; the
    lambda itself and a `FuncRef` give no site;
  - `err.Error()` on a value of type `error`: site `iface:error.Error@1`;
  - a call of an extern no summary names: no site, and its `unknown` entry is unchanged;
  - a function moved to another file of the same module: same sites;
  - two calls of different callees swapped: same sites;
  - two calls of the same callee swapped: `@1` and `@2` exchange what they reach (pinned, and
    written in the crate doc);
  - a call of the same callee inserted before an existing one: the existing call becomes `@2`
    (pinned, and written in the crate doc as the limit of the ordinal);
  - a body-less function the barrier fixes: site `body@1`.
- `examples/envcheck/envcheck.effects.json` regenerated with `sites`; `task examples-effects`
  exit 0.
- `crates/docs/src/netcheck.rs` (`reduce`) clears `sites` of every `syscalls` and
  `effect_sites` entry, so the reduced netcheck reports hold no `sites` and the checked-in
  `examples/netcheck` files stay byte-equal (`git diff --exit-code examples/netcheck`);
  `cargo test -p codespec-docs` and `task docs-check` exit 0.
- `website/docs/guides/map-a-repository-to-syscalls.md`: the report-field table's `syscalls` row
  names `sites`, and the pasted report lines (`{"name":"openat","at":[…]}` and the others) are
  what the guide's commands print now.
- `task docs-generate` run; `task check` green; each new test under 60 s, the test step within
  the 120 s budget.

## Scope

- Own: `crates/effects/src/set.rs` (sites recorded at `set()` and `edge()`),
  `crates/effects/src/lib.rs` (`Site.sites`, the crate doc with the definition),
  `crates/effects/tests/effects.rs`, `crates/cli/tests/effects.rs` and
  `crates/cli/tests/fixtures/moved-lines/` (the Go fixture, `codespec-read-go` built once per
  test binary as `crates/cli/tests/barrier.rs` does), `crates/docs/src/netcheck.rs`,
  `examples/envcheck/envcheck.effects.json`,
  `website/docs/guides/map-a-repository-to-syscalls.md`, `website/docs/examples/` (what `task
  docs-generate` rewrites). Inferred; recorded as typed scope. `CHANGELOG.md` gets one line,
  outside the typed scope; conflicts there are resolved at merge.
- `website/docs/concepts/static-analysis.md` names only a function entry's top-level fields
  (line 75), not the fields of a `syscalls` entry: not changed.
- No change to `crates/model`, the schema or any reader: no `cir/1` change.
- Shares no file with `story:go-http-entries`; both can run at once.
- `story:reach-entry-file` (epic `sound-reach`, draft) also edits `crates/effects/src/lib.rs`
  (entry matching); whichever starts second rebases on the other. No edge: neither needs the
  other's outcome.
