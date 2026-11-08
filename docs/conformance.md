# Adapter conformance

Status: proposed, 2026-11-06. Not implemented.

## 1. The question

A reader or writer built outside this repository claims to handle part of `cir/1`. Today nothing
lets its author, or a user, check that claim. What exists:

| exists | what it checks | why it is not enough |
|---|---|---|
| strict reading at the boundary (`codespec read` / `codespec write`) | an adapter's output, or its input, is a valid `cir/1` module | a valid module can mean the wrong thing |
| `website/docs/guides/add-an-adapter.md` § What the gate needs | in-repo adapters: determinism, checked-in outputs, target builds | runs only inside this repository's `task check`, against its own examples |
| `codespec harness <example>` | whole programs byte-equal on seeded traces | three examples, each needing hand-written drivers per language; says nothing per feature |
| `refusals` / `foreign` | an adapter says what it did not handle | nothing compares the refusals with what the adapter claims |

The design below turns those pieces into one command an outside author runs against their adapter,
and one report a user can read: which `cir/1` features the adapter claims, and for each, whether it
passed, was refused, or failed.

## 2. Features: a closed vocabulary generated from the model

A **feature** is one thing an adapter can support, named by a stable id. The list is generated from
`crates/model/src/lib.rs` by `codespec features`, written to `schemas/cir-1.features.json` and
drift-checked by `task check` the way `schemas/cir-1.schema.json` is.

| family | ids (examples) | source in the model |
|---|---|---|
| statement | `stmt.for`, `stmt.break`, `stmt.switch`, `stmt.lookup_else` | `Stmt` variants (16) |
| expression | `expr.binop`, `expr.make`, `expr.intrinsic` | `Expr` variants (18) |
| operator | `op.add`, `op.rem`, `op.shl`, `unop.neg`, `assign.add` (`+=`), `assign.and_not` (`&^=`) | the `op` strings `Binop`, `Unop` and `Assign` document; each operator id carries its operator string in an `op` field |
| intrinsic | `intrinsic.sprintf`, `intrinsic.append`, `intrinsic.len` | the names `Intrinsic` documents |
| type | `type.int`, `type.map`, `type.chan`, `decl.union` (one id per variant; widths are not separate features) | `Type` (12), `TypeDecl` (4) |
| fact | `fact.fallible`, `fact.unordered_iteration` | `Facts` fields (7); a writer feature only |
| semantic | `sem.int.wrap`, `sem.rem.sign`, `sem.map.order`, `sem.multi_assign.order`, `sem.switch.break`, `sem.error.propagate` | behaviour two languages disagree on; one id per difference the harness has already met |

A semantic feature is the important family. `sem.int.wrap` (Go's `int` wraps on overflow),
`sem.continue.post` (`continue` runs the `post` statement) and `sem.multi_assign.order` (every value
is read before any target is written) are where the 0.2.0 Rust writer was wrong before it was right.
A writer that writes `stmt.for` but not `sem.continue.post` is the case this design exists to catch.

Adding a variant to the model adds a feature id; the drift check fails until the vocabulary and the
cases (§ 3) are regenerated, so no feature exists without a case.

## 3. Cases: one small program per feature, with its expected output

The kit is a directory, `conformance/`, versioned with `cir/1` and shipped in every release archive
as `codespec-conformance-<version>.tar.gz` with a `SHA256SUMS` entry.

```
conformance/
  kit.json                       format codespec-conformance-kit/1, cir version, the feature list
  cases/<case>/
    case.go                      the reference program, Go (the reference language)
    case.cir.json                its cir/1 module, read and passed by this repository's tools
    calls.json                   the calls to make: function, arguments, in order
    expected.txt                 what the reference program prints for those calls
    features.json                the feature ids this case exercises; the first is its subject
```

- One case per feature at least; a semantic feature gets cases that cross its edge
  (`sem.int.wrap`: `MaxInt64 + 1`, `MinInt64 / -1`, `MinInt64 % -1`).
- `expected.txt` is produced by running `case.go` with the Go toolchain, and `task check` re-runs
  it, so an expected output cannot drift from the reference.
- `calls.json` uses the harness's value encoding: integers in decimal, strings quoted, results
  printed one per line as `ok <value>` or `err <message>`.
- Cases are written in Go because Go has a reader and a writer here and a toolchain that runs them;
  `case.cir.json` is what an outside writer reads, so its author never needs Go.

## 4. The runner: what an outside adapter provides beside itself

To run a written program the kit needs one more program from the adapter's author:

| program | arguments | does |
|---|---|---|
| `codespec-run-<lang>` | `<written dir> <calls.json>` | builds the written module with the target's toolchain, makes the calls in order, prints one line per result in the harness encoding |

The runner replaces the per-example drivers the harness uses today: one runner per language serves
every case. This repository ships `codespec-run-go` and `codespec-run-rust`, which also serve as the
worked examples, and `codespec-run-java` and `codespec-run-groovy` (`adapters/jvm/src/codespec/run`),
which compile a package with javac or groovyc and call its static methods by reflection; the JVM
runners serve step 1 of the Java and Groovy readers' runs. A runner cannot pass a case on its own: the module it runs comes from the writer
under test, and the output is compared with the reference's.

## 5. The claim: a manifest the adapter prints

```shell-session
$ codespec-write-zig --describe
{
  "format": "codespec-adapter/1",
  "role": "writer",
  "lang": "zig",
  "version": "0.3.1",
  "cir": "cir/1",
  "features": ["stmt.*", "expr.*", "type.int", "sem.int.wrap", "fact.fallible"],
  "not": ["type.chan", "stmt.send"]
}
```

`features` and `not` may end an entry in `.*` after an id prefix that names a family (`stmt.*`, `op.*`); a prefix no id has is an unknown family and refused. `not` subtracts. `codespec` validates the manifest
strictly (unknown ids are refused, so a claim cannot name a feature that does not exist) and
expands it against `kit.json`.

A reader's claim covers what the reader itself prints, not what `codespec pass` makes of it: the
module checked against the claim (§ 6, step 3) is the reader's output before any pass. A feature
only a pass produces is therefore not a reader feature of that reader. `stmt.lookup_else` is one:
the Go reader reads `v, ok := m[k]; if !ok { … }` to a `let` and an `if`, and the lookup-guard pass
rewrites them, so `codespec-read-go` does not claim it, while `codespec-read-rust`, which reads
`let Some(v) = m.get(&k) else { … }` to `lookup_else` itself, could. Counting read followed by pass
would credit every reader with the pass's work and say nothing about the reader. Facts (`fact.*`)
are the same case, and a writer feature only (§ 2). `stmt.foreign` is a reader feature: a reader
claims it when its source for a construct outside `cir/1` reads to a `foreign` statement holding
that source verbatim, with a refusal.

## 6. The verdicts

`codespec conform --lang <lang> --role reader|writer [--kit <dir>] [--corpus <dir>] --out <report.json>`

### A writer, per case

| the writer | the case's subject is claimed | not claimed |
|---|---|---|
| refuses the function (listed in `refusals`) | **fail**: claimed and refused | **pass**: refused, as it should be |
| writes it; the runner prints `expected.txt` | **pass** | **pass**, reported as unclaimed but working |
| writes it; output differs, or the build fails | **fail**: wrong, with the first differing line | **fail**: a writer must refuse what it cannot write, never approximate |

The last cell is the strictness rule the in-repo adapters already follow, made checkable for
everyone: an unclaimed feature may be refused, never written wrong.

Every writer also runs the boundary cases, which are not optional:

- a module with an unknown statement kind: exit non-zero, nothing written;
- a module without facts, where the writer reads facts: exit non-zero, naming `codespec pass`;
- each case written twice: the same bytes.

### A reader

A reader is checked by what it produces, run through a writer this repository trusts:

1. The author supplies, per claimed feature, a source in their language and its `calls.json`
   (`conformance/reader/<lang>/<case>/`). Their runner runs the source as written and must print
   the case's `expected.txt`: the source is checked against the reference behaviour before
   anything is read.
2. The reader reads it: exit 0, a valid module, the same bytes on a second read.
3. The module contains the case's subject feature at least once outside a `foreign`
   statement, so a reader cannot pass `stmt.for` by refusing every loop.
4. `codespec pass` and the Go writer write it; `codespec-run-go` prints `expected.txt`.

A claimed feature whose source the reader turns into `foreign` fails at step 3.

## 7. The report

`codespec-conformance-report/1`, JSON, canonical:

```json
{
  "format": "codespec-conformance-report/1",
  "adapter": {"role": "writer", "lang": "zig", "version": "0.3.1", "manifest_sha256": "…"},
  "kit": {"version": "0.3.0", "sha256": "…"},
  "codespec": "0.3.0",
  "features": [
    {"id": "stmt.for", "claimed": true, "verdict": "pass", "cases": ["for-three-clause", "for-cond-only"]},
    {"id": "sem.continue.post", "claimed": true, "verdict": "fail",
     "case": "continue-runs-post", "first_difference": {"line": 3, "expected": "ok 6", "got": "ok 9"}},
    {"id": "type.chan", "claimed": false, "verdict": "refused"}
  ],
  "summary": {"claimed": 61, "pass": 60, "fail": 1, "refused_unclaimed": 4},
  "conforms": false
}
```

`conforms` is true when every claimed feature passes and every boundary case passes. The report
binds the adapter manifest and the kit by digest, so a report is evidence for exactly that adapter
version against exactly that kit. `codespec conform --check <report>` re-runs and compares.

## 8. In this repository

- The in-repo adapters run `codespec conform` in `task check`, each with a manifest; their reports
  are generated into `website/data/conformance/` and shown on the Status page, one row per feature.
  That keeps the kit honest: the reference adapters must conform to it first.
- The existing harness examples stay: they test whole programs and the facts working together, which
  per-feature cases do not.
- The status capability list gains one row per adapter, its evidence the report.
- **The support matrix** (`/docs/reference/support`, `story:support-matrix`): one row per feature,
  grouped by family, one read column and one write column per language. A cell is `verified`
  (claimed, report passes), `claimed` (in the manifest, no report yet), `failing`, `refused` or
  `none` (no adapter for that role), shown with colour and a symbol, with a legend and per-column
  counts. Generated by `codespec-docs` from the feature list, the manifests and the reports, so it
  cannot claim more than the reports show.

## 9. Decisions this asks for

| | options | proposed |
|---|---|---|
| D1 reference language for cases | A Go (reader, writer and toolchain exist) · B cir/1 written by hand, outputs by an interpreter | A: an interpreter for `cir/1` would be a third implementation of the semantics to keep right |
| D2 ESS | A own report format · B an ESS conformance suite | A for now: an ESS suite checks a service's commands and views; this checks a translator's output per feature. Revisit with `docs/design.md` § Later (ESS as an equivalence oracle) |
| D3 where outside readers' sources live | A in the author's repository, passed with `--corpus` · B contributed here | A: the kit cannot hold sources in every language |

## 10. Order of work

1. `codespec features` and `schemas/cir-1.features.json`, drift-checked.
2. `conformance/` cases for the features the in-repo adapters use (every feature `orders`,
   `inventory`, `series`, `greeter` exercise), expected outputs checked in `task check`.
3. `codespec-run-go`, `codespec-run-rust`; adapter `--describe` manifests.
4. `codespec conform` for writers; reports for the Go and Rust writers in the gate.
5. Reader conformance; reports for the Go and Rust readers.
6. The kit in release archives; the guide `add-an-adapter.md` rewritten around `codespec conform`.
