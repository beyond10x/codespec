---
format: aep.planning-md/3
id: story:diff-unknown-at-the-incomplete-function
kind: story
status: implemented
title: An unknown a function inherits from its callee is reported once, at the callee
summary: codespec diff reports calls-X-which-is-incomplete at the incomplete function with its callers as a list; a function whose own facts did not change gets no delta
relations:
- decomposes: epic:release-fact-diff
- depends_on: story:diff-command
scope:
- confidence: inferred
  path: crates/diff
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: website/docs/reference/cli.md
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T04:43:16Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-26T04:43:16Z", actor: "human:operator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-26T09:14:08Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: An unknown a function inherits from its callee is reported once, at the callee

## Outcome

`codespec diff` reports a reason a function is incomplete in one place: the function whose own
body gives the reason. A reason that a function only inherits from a callee (`calls <func> which
is incomplete`, `crates/effects/src/set.rs`) makes no delta of its own; the delta at the incomplete
callee carries the functions that inherit from it as a list.

The rules, each one a test:

- Own reasons (every reason of the table in the crate doc of `crates/effects` except `calls <func>
  which is incomplete`) are deltas of the function that has them, as `story:diff-command` defines.
- An `unknown-added` or `unknown-removed` delta of a function gains the field `callers`: the sorted
  `unit` names of every function that is incomplete through it in the revision where the reason
  exists (the new revision for `unknown-added`, the old for `unknown-removed`), to a fixed point
  over the call graph: callers of callers count. Omitted when the list is empty.
- A function whose only difference between the revisions is an inherited reason gets no delta of
  kind `unknown-*`: its facts did not change, its callee's did.
- A function that starts calling an incomplete callee keeps its delta
  `unknown-added <unit> calls <func> which is incomplete`: its own call is new, so its own facts
  changed. The test is the call-site identity of `story:stable-site-identity`: the call site
  (`callee`, ordinal `n`) exists in one revision only.
- The text form prints `callers=<n>` after the value; the JSON form holds the list. Decision D1,
  default A: the list is in JSON only. B (not taken): the text form lists the names too, which
  prints the 198 lines again, one per name.

## Why

A release review of a Go service ran `codespec diff` on two tags: 293 deltas, 234 of kind
`unknown`. 198 of the 234 read `calls <func> which is incomplete`, and the three most frequent
callees account for 63, 61 and 41 of them. 132 of the 198 are functions on lines no commit
between the tags touched: the reviewer reads 132 lines about code that did not change, one per
caller, to learn about three functions. The same facts as three deltas, each with its callers,
are what the reviewer can act on.

## Relation to `story:diff-unknowns-summary`

That story (draft, branch `plan/go-field-gaps`, not on main when this was written) groups the
deltas that exist by reason class and counts them. This story decides which deltas exist: a
reason is attributed to the function that has it. They compose: after this story the class
`calls ... which is incomplete` has few members (new call edges only), and the summary counts the
rest. Neither needs the other; both edit `crates/diff`, and whichever starts second rebases on
the other. `story:diff-unknown-reasons-measured` (same branch) counts classes on a public corpus:
its counts of the `calls ... which is incomplete` class are those before this story; it measures
with the mode `list`.

## Reproducer

Written for this story, `example.com/probe`, read with `codespec-read-go` at two revisions.
Revision `a`:

```go
package probe

func Lib() int { return 1 }

func A() int { return Lib() }
func B() int { return Lib() + 1 }
func C() int { return Lib() + 2 }
```

Revision `b` changes only `Lib`, to call a function of `example.org/lib` that no summary names:

```go
package probe

import "example.org/lib"

func Lib() int { return lib.Do() }

func A() int { return Lib() }
func B() int { return Lib() + 1 }
func C() int { return Lib() + 2 }
```

`codespec diff a-docs b-docs`.

## Observed

Not run: `crates/diff` does not exist on main until `story:diff-command` is implemented. By its
design and by the reason strings of `crates/effects/src/set.rs`, `unknown-added` is one delta per
function and reason, so the report holds four: `Lib` with `extern without summary:
example.org/lib.Do`, and `A`, `B` and `C` each with `calls example.com/probe.Lib which is
incomplete`. Inferred from the code, not measured.

## Done When

- A failing test first in the integration-test binary of `crates/diff` (`tests/main.rs`), over
  documents of the reproducer read once: the report holds exactly one `unknown-added`, for
  `example.com/probe.Lib` with value `extern without summary: example.org/lib.Do`, and its JSON
  `callers` is `["example.com/probe.A", "example.com/probe.B", "example.com/probe.C"]`; the text
  form ends the line with `callers=3`. Exit 1.
- Near misses, one fixture pair each:
  - the same pair with `A` calling `Lib` twice: one delta, `A` once in `callers`;
  - a chain, `A` calls `B` calls `Lib`: one delta, `callers` holds `A` and `B`;
  - `Lib` incomplete in both revisions and nothing else changed: no delta, exit 0;
  - `Lib` incomplete in `a` and complete in `b` (the reverse pair): one `unknown-removed` at `Lib`
    with the three callers;
  - a fourth function `D` that starts calling `Lib` in `b`: `D` has its own
    `unknown-added ... calls example.com/probe.Lib which is incomplete`, and `callers` of the
    `Lib` delta still lists `A`, `B`, `C` and `D`;
  - `A` itself gains an own reason (it calls a second unsummarised extern): `A` has that delta and
    is still in the `callers` list of `Lib`;
  - a call through an interface or a function value whose candidates include `Lib`
    (`--dispatch rta`): the caller is listed, as the effects report already says it is incomplete;
  - the same inputs twice give the same bytes; text and JSON hold the same counts.
- `codespec diff --help`, the crate doc of `b10x-codespec-diff` (the report format) and
  `website/docs/reference/cli.md` state the rule and the field; `task docs-generate` run;
  `task check` green; each new test under 60 s and the test step within the 120 s budget.

## Scope

- Own: `crates/diff/` (attribution, the `callers` field, `tests/main.rs`, its fixtures),
  `crates/effects/src/lib.rs` and `set.rs` only if the report must expose which reason is
  inherited (a field of the unknown entry, not a new parse of its text), and its tests,
  `website/docs/reference/cli.md`. Inferred; recorded as typed scope. `CHANGELOG.md` gets one
  line, outside the typed scope.
- After `story:diff-command`, which creates `crates/diff`.
- Shares `crates/diff` with `story:diff-unknowns-summary` and `crates/effects/src/lib.rs` with
  `story:diff-unknowns-summary` and `story:reach-inits-per-program`; whichever starts second
  rebases on the other.

## Out of Scope

- Effects: an effect a callee gains is inherited by every caller the same way and gives one
  `effect-added` per caller. Not changed here; a story of its own if the same release shows it.
- Narrowing which callers a call through an interface reaches (`story:reach-registered-candidates`).
- Changing how `codespec effects` reports a function (the report keeps `calls <func> which is
  incomplete` in `unknown`): the attribution is the diff's.
