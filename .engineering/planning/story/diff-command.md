---
format: aep.planning-md/3
id: story:diff-command
kind: story
status: active
title: codespec diff reports what changed in the code facts between two revisions
summary: 'codespec diff over two directories of cir/1 documents prints codespec-diff-report/1: units, signatures, effects, syscalls by call site, raises, reach and entries'
relations:
- decomposes: epic:release-fact-diff
- depends_on: story:stable-site-identity
- depends_on: story:go-http-entries
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/cli/Cargo.toml
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/diff
- confidence: inferred
  path: crates/docs/src/status.rs
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/data/status.json
- confidence: inferred
  path: website/docs/reference/cli.md
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T00:32:28Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-25T00:32:28Z", actor: "human:timo", revision: 10}
---
# Story: `codespec diff` reports what changed in the code facts between two revisions

## Outcome

`codespec diff <old-dir> <new-dir>` reads every `*.cir.json` of each directory as one program
(the same program at two revisions), computes each side's facts with the code `codespec effects`
and `codespec reach` use, and prints `codespec-diff-report/1`. Options apply to both sides:
`--summaries`, `--barrier`, `--entry`, `--exported`, `--dispatch` (default `rta`), `--report
<module>`, `--format text|json` (default `text`). The entries of each side are its `--entry` and
`--exported` functions and every function its documents record in `Module.entries` (resolving
`Entry.path` of `story:go-http-entries`), with `main.main` and every Go `init` as `reach` takes
them. Exit 0 with no delta, 1 with deltas, 2 on an input error (a directory with no document, a
document strict reading refuses, two documents of one side with the same module path).

`codespec-diff-report/1`, canonical JSON (two-space indentation, final newline), the same inputs
giving the same bytes:

| field | type | meaning |
|---|---|---|
| `format` | string | always `codespec-diff-report/1` |
| `dispatch` | string | `cha` or `rta`, as both sides were analysed |
| `deltas` | array | one object per delta, sorted by `unit`, then `kind`, then `site` (`callee`, then `n`), then `value` |

A delta is `{kind, unit, site, value, old, new}`; `site`, `value`, `old` and `new` are omitted
when they do not apply:

| `kind` | `unit` | other fields |
|---|---|---|
| `unit-added`, `unit-removed` | the function | none |
| `signature` | the function | `old`, `new`: its type as `codespec effects` spells a signature (`func(string) (int, error)`), receiver first; parameter names are not part of it |
| `effect-added`, `effect-removed` | the function | `value`: an effect of `EFFECTS` (db, env, fs.read, fs.write, io, net, proc, rand, stdin, stdout, time) the function has in one revision only |
| `syscall-added`, `syscall-removed` | the function | `site`: the call-site id of `story:stable-site-identity` (`{callee, n}`) of the function's own call that leads there; `value`: the syscall |
| `raise-added`, `raise-removed` | the function | `value`: an extern type `path.name` in its `raises` in one revision only |
| `reach-gained`, `reach-lost` | the function | none: reached from the entries in the new revision only, or in the old only; a unit added or removed gives no reach delta |
| `entry-added`, `entry-removed` | the entry's function | `value`: `<kind> <method> <on>` (`http GET /api/items`), the method empty outside `http` |
| `unknown-added`, `unknown-removed` | the function | `value`: an `unknown` reason of `codespec effects` (its text, without its position) present in one revision only |

`unit` is `<module path>.<function key>` (the module's `path`, or its `name` without one), for
functions and opaque functions alike. No field holds a file or a line. The text form prints one
line per delta in the same order, its fields separated by one space, a missing one skipped:
`syscall-removed example.com/probe.Load syscall.Syscall@1 getpid`, and `old -> new` for a
signature.

The format is defined in the crate doc of a new crate `crates/diff` (`b10x-codespec-diff`),
as `codespec-effects-report/1` is in `crates/effects`; the CLI holds only the command.

## Why

A release review compares the code facts of two tags of a service. Reach over-approximates, so
the facts of one revision say little; their difference is the signal. Today the reviewer would
run `codespec effects` and `reach` twice and compare reports whose sites are lines; there is no
diff command and no diff format.

## Reproducer

The module `example.com/probe`, revisions `a/` and `b/`, with a fixture barrier declaring
`syscall.Syscall` numbered by argument 0 and the two syscalls `getpid` and `getppid` with the
same effects:

```go
// a/probe.go
package probe

import "syscall"

func Who() uintptr {
	r, _, _ := syscall.Syscall(39, 0, 0, 0)
	return r
}
```

`b/probe.go` calls `syscall.Syscall(110, 0, 0, 0)`. Read both with `codespec-read-go`, then
`codespec diff a-docs b-docs --barrier <fixture barrier>`.

## Observed

Not run on main `5f82fff9`: `crates/cli/src/lib.rs` has no `diff` command.

## Done When

- A failing test first in the integration-test binary of `crates/diff` (`tests/main.rs`).
- The consumer's check: the reproducer reports exactly two deltas, under one call-site id:
  `syscall-removed example.com/probe.Who syscall.Syscall@1 getpid` and `syscall-added
  example.com/probe.Who syscall.Syscall@1 getppid`, exit 1; the JSON form holds the same two.
- The consumer's check of `story:stable-site-identity` through the command: the moved-lines
  fixture (`crates/cli/tests/fixtures/moved-lines/`) gives no delta, exit 0, empty output.
- One fixture pair per remaining kind, each giving exactly its delta: a function added and one
  removed; a parameter type changed; an effect gained through a new extern call; a raise gained;
  a function that becomes unreached when its only caller stops calling it; an `http` entry added
  (from the echo fixture of `story:go-http-entries`); an `unknown` reason gained.
- Near misses: a moved line, two calls of different callees swapped, and a function moved to
  another file give no delta; a parameter renamed gives no `signature` delta; the same inputs
  twice give the same bytes.
- Input errors exit 2 naming the directory or document: an empty directory, a document strict
  reading refuses, two documents of one side with one module path.
- `codespec diff --help` and the site's reference pages generated (`task docs-generate`); a
  capability `diff` shipped in `crates/docs/src/status.rs` naming its test; the new crate in
  the workspace members, in a `test` part of `scripts/check-parts.txt` (the `lint` part fails
  otherwise) and with a public `description` naming no private name; a row for `crates/diff` in
  `AGENTS.md`'s table of what the repository owns; `task check` green; each
  new test under 60 s and the crate's test step within the 120 s budget.

## Scope

- Own: `crates/diff/` (new crate: the report, the comparison, `tests/main.rs`, its fixtures),
  `Cargo.toml` (members), `Cargo.lock`, `crates/cli/Cargo.toml`, `crates/cli/src/lib.rs` and
  `crates/cli/src/main.rs` (the `Diff` command), `scripts/check-parts.txt`,
  `crates/docs/src/status.rs`, `website/data/status.json`, `website/docs/reference/cli.md` and
  `crates.md`, `AGENTS.md` (the row of `crates/diff` in the table of what the repository owns).
  Inferred; recorded as typed scope. `CHANGELOG.md` gets one line, outside the typed scope;
  conflicts there are resolved at merge.
- `story:entry-bindings` (draft) edits `crates/cli/src/lib.rs` too; it runs after this one:
  `story:entry-bindings` depends_on `story:diff-command`.
- After `story:stable-site-identity` (the site ids, `crates/effects`) and `story:go-http-entries`
  (`Entry.path` and the echo fixture): `depends_on` edges.
- `story:reach-entry-file` (epic `sound-reach`, draft) gives `reach` its default of recorded
  entries and `--entries <file>`, and edits `crates/cli/src/lib.rs` and `main.rs` too. `diff`
  takes recorded entries itself here; when `story:reach-entry-file` lands, `diff` takes its
  entry list from the same function. Whichever starts second rebases on the other; no edge.
