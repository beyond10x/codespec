---
format: aep.planning-md/3
id: story:groovy-integer-division
kind: story
status: implemented
title: The Groovy reader reads integer division
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:jvm-readers-conform
scope:
- confidence: cited
  path: adapters/groovy
- confidence: cited
  path: conformance/cases
- confidence: cited
  path: crates/conformance
- confidence: cited
  path: website/data
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T20:38:49Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-06T20:38:49Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Groovy reader reads integer division

## Outcome

The Groovy reader claims `op.div`, `assign.div` and `sem.int.wrap`. On wave head `728a7bf` it refuses all three: the kit's cases for them use Java's `/` on integers. Groovy evaluates `/` on two integers as a BigDecimal division, so the reader refuses it. Idiomatic Groovy writes integer division as `a.intdiv(b)`. Requested by the operator on 2026-11-06 ("100% everywhere as much as possible").

## Acceptance

- A case may carry a `groovy/` source directory. The Groovy reader's corpus then reads that directory instead of `java/`; every other reader is unchanged.
- The Groovy reader reads `a.intdiv(b)` on two integer operands of one type as `op.div`. Its `intdiv` assignment form reads as `assign.div` where Groovy has one; otherwise `x = x.intdiv(y)` reads as an assignment of `op.div`. It keeps Go's truncation toward zero, and division by zero panics as Go does, or the case says why not.
- `/` on integers stays refused.
- The cases `sem-int-wrap`, `op-div` and `assign-div` (exact names from the kit) gain `groovy/` sources. The Groovy conform report passes each one, and the Groovy claim adds the three features.
- The reports, the support matrix and `status.json` are regenerated; `task docs-check` and `task adapters-check` pass. The Go, Rust and Java reports are unchanged apart from the kit digest.

## Scope

- `adapters/groovy/src/codespec/read/groovy/` (Reader.java, Main.java claims), `adapters/groovy/test/`, `adapters/groovy/README.md`
- `crates/conformance` (`CASE_SOURCE_DIRS`, corpus generation for a `groovy/` dir), `crates/docs` if the corpus generator lives there
- `conformance/cases/*/groovy/` (new files only), `conformance/reader/groovy/`
- generated `website/data/`
- a Taskfile patch if needed
