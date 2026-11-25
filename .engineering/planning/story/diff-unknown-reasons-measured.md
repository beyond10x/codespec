---
format: aep.planning-md/3
id: story:diff-unknown-reasons-measured
kind: story
status: draft
title: The unknown reasons of codespec diff, measured on public Go projects
summary: Counts by reason class over three Go projects at pinned commits; one draft story per class with 5 or more occurrences
relations:
- decomposes: epic:release-diff-go-field-gaps
- depends_on: story:diff-command
- informed_by: story:diff-unknowns-summary
scope:
- confidence: inferred
  path: scripts/diff-unknowns-corpus.sh
revision: 2
---
# Story: The unknown reasons of codespec diff, measured on public Go projects

## Outcome

The `unknown-added` and `unknown-removed` deltas of `codespec diff` are counted by reason class on
three public Go projects, each at two pinned commits, and the counts are written in this story's
body (section `## Measured`). Each reason class with 5 or more occurrences in total gets one draft
story under `epic:release-diff-go-field-gaps` naming the class, the construct behind it with a
source line of the project that shows it, the count, and the change that resolves it (a reader
fix, an analysis fix or a summary entry).

The corpus, each side one commit of one Go module, the older first:

| project | old | new |
|---|---|---|
| `github.com/gin-gonic/gin` | `v1.9.1`, `4ea0e648e38a63d6caff14100f5eab5c50912bcd` | `v1.10.0`, `75ccf94d605a05fe24817fc2f166f6f2959d5cea` |
| `github.com/spf13/cobra` | `v1.8.1`, `e94f6d0dd9a5e5738dca6bce03c4b1207ffbc0ec` | `v1.9.1`, `40b5bc1437a564fc795d388b23835e84f54cd1d1` |
| `github.com/caddyserver/caddy` | `v2.8.4`, `7088605cc11c52c2777ab613dfc5c2a9816006e4` | `v2.9.1`, `0e570e0cc717f02cf3800ae741df70cd074c7275` |

The reason class is the reason text of the `unknown-*` delta's `value` up to its first `: `
(`method on undeclared type`, `extern without summary`, `not in its module`, `call of undeclared
function`); a reason with no `: ` (`foreign statement`) is its own class. The strings are those
of `crates/effects/src/set.rs` (`unknown(..)`). An occurrence is one delta; the three pairs are
summed. The same rule is the one `story:diff-unknowns-summary` gives the command, so the counts
here are what that command prints for the corpus.

The measurement also splits the deltas of each class in two: those of a function in both revisions
(its set of reasons changed) and those that a unit added or removed carries. Inferred, not
measured: most of the 234 `unknown` deltas of the review are the second kind, carried by units
added or removed in a release; if the measurement shows that, a story folds them under their
`unit-added` or `unit-removed` delta and is drafted as one of the stories above, whatever its
count.

How each side is read: every package of the module and every package it imports, with
`codespec-read-go` at the module's pinned Go version, linked as `task examples-netcheck` does for
`examples/netcheck` (no network at read time: the sources are fetched once into a directory the
script names). A script `scripts/diff-unknowns-corpus.sh` (POSIX sh, run by hand, not in the gate,
not under `slot`, as it waits on `git`) fetches the six commits, reads, runs `codespec diff
--format json` per pair and prints the counts with `jq`.

## Why

234 of 293 deltas of a release review were of kind `unknown`, and a reviewer cannot read 234 lines
to find the 59 that matter. Which of them are noise a fix removes and which are real limits of
the analysis is not known from one service. A public corpus makes the answer reproducible by
anyone, and a story per class turns noise into work.

## Reproducer

```
sh scripts/diff-unknowns-corpus.sh <work dir>
```

prints, per project and in total, `<class> <added> <removed> <of units added or removed>`.

## Observed

Not run: `crates/diff` does not exist on main until `story:diff-command` is implemented.

## Done When

- `scripts/diff-unknowns-corpus.sh` exists, is shellcheck-clean, fails when a commit it fetches is
  not the pinned one, and runs outside the gate; two runs give the same counts.
- The `## Measured` section of this story holds the table of class counts per project and in
  total, the split above, and the tool versions (`codespec --version`, `go version`).
- One draft story per class with 5 or more occurrences, each decomposing
  `epic:release-diff-go-field-gaps` and `aep plan artifact validate` valid; a list of the classes
  under 5, with counts, so the cut is visible.
- `aep plan artifact validate` valid for the whole store.
- No test of the gate reads the network: the script's output is data, not a test.

## Scope

- Own: `scripts/diff-unknowns-corpus.sh`, this story's body, the draft stories it files. Inferred;
  recorded as typed scope.
- After `story:diff-command` (it needs the command) and informed by `story:diff-unknowns-summary`
  (the class rule); no file in common with either.
- Not a test and not in `scripts/check-parts.txt`: it needs the network and exceeds the 120 s step
  budget on purpose; run by hand.
