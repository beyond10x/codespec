---
format: aep.planning-md/3
id: review-result:conf-java-design-round-2
kind: review-result
status: active
title: Design critic, round 2 (adapter-conformance and java-reader)
relations:
- reviews: story:feature-vocabulary
- reviews: story:conformance-cases
- reviews: story:runners-and-manifests
- reviews: story:codespec-conform
- reviews: story:conformance-kit-release
- reviews: story:support-matrix
- reviews: story:java-reader-javac
- reviews: story:java-example-ledger
revision: 1
---
approve

**What you read:** 11 artifacts (both epics, the eight stories and `review-result:conf-java-design-round-1`), shown with `aep plan artifact show`. I also ran `aep plan artifact relations`, `graph` (all declared edges, including outside the set) and `validate`, which printed `valid`. I read `docs/conformance.md` at the lines that matter (104, 128).

I walked every declared edge in the graph and found no cycle. The only edge leaving the set is `epic:groovy-grails-reader` → `epic:java-reader`, and it points inward.

The two round-1 findings are fixed in the edges:
- `story:runners-and-manifests` now has `depends_on story:conformance-cases`.
- `story:support-matrix` now has `depends_on story:codespec-conform`.

The shape is feature-vocabulary, then cases, then runners, then conform, then release and matrix. That is a mostly linear order, but every edge records a read of produced state, so it is not an unexplained serial chain. Each story can be demonstrated end to end on its own.

I traced 17 acceptance reads to a producer in the set, and an edge records every one (some of them transitively):
- The kit, cases and features file are read by `story:runners-and-manifests`, `story:codespec-conform` and `story:conformance-kit-release`.
- The manifests are read by `story:codespec-conform` and `story:support-matrix`.
- The reports are read by `story:support-matrix` and `story:conformance-kit-release`.
- The reader is read by `story:java-example-ledger`.

`story:codespec-conform` now creates `conformance/reader/` itself, so the reader-corpus gap I noted in round 1 is closed. The files shared between stories are `crates/model`, `crates/conformance`, `crates/cli` and `crates/docs/src/main.rs`. Each shared file sits between stories that already have an edge, so it is an ordering trade-off the edges already settle, not a split abstraction.

Set 2 (`java-reader-javac` → `java-example-ledger`) has a single edge that records the one read of produced state.

**What you could not establish:**
- Out of my lane (acceptance critic): the `story:codespec-conform` synopsis omits `--out`, which `docs/conformance.md:104` requires and `story:conformance-kit-release` uses.
- Out of my lane (scope critic): `epic:java-reader` has no `--describe` manifest, so the matrix has no Java cell source until one exists. The matrix body only counts Go and Rust, so it is not a set defect.

```findings
[]
```
