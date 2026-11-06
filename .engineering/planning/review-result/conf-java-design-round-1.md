---
format: aep.planning-md/3
id: review-result:conf-java-design-round-1
kind: review-result
status: active
title: Design critic, round 1 (adapter-conformance and java-reader)
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
needs-revision

story:runners-and-manifests — its acceptance runs the runners "on every case of `conformance/cases`" against `case.cir.json` and `expected.txt`, which story:conformance-cases creates, and the only edge goes to story:feature-vocabulary; add a `depends_on` edge to story:conformance-cases — docs/conformance.md:53-58 (the case layout), `aep plan artifact graph` (runners-and-manifests has one `depends_on`, to feature-vocabulary)

story:support-matrix — its acceptance reads the `website/data/conformance/` reports that story:codespec-conform generates, which are the only source of its `verified` and `failing` cells, and links each feature id to a case under `conformance/cases`. The only edge runs to story:runners-and-manifests, so those two states cannot be checked when the matrix lands beside the conform story; add a `depends_on` edge to story:codespec-conform, which carries cases and runners transitively — `aep plan artifact graph` (support-matrix has one `depends_on`, to runners-and-manifests), docs/conformance.md:171-176

**What you read:** 10 artifacts (the two epics and eight stories) with `aep plan artifact show`, plus `relations`, `graph` and `validate`, which printed `valid`. I read `docs/conformance.md` §§ 3-10 and the CLI adapter lookup in `crates/cli/src/lib.rs:35`. I walked all 27 declared edges across the whole graph, outside the set included, and found no cycle. The only edge leaving the set is `epic:groovy-grails-reader` → `epic:java-reader`, which points inward. I traced about 14 acceptance reads to a producer in the set. Eleven of them have an edge, and the two findings above are the ones without. The set is not a serial chain: the order is vocabulary, then cases and runners in parallel, then conform, then release, with the matrix branching off. Neither epic's stories are cut by layer, and no half of an abstraction sits in a different item. Set 2 (`java-reader-javac` → `java-example-ledger`) is consistent. `codespec read --lang java` finds `codespec-read-&lt;lang&gt;` by name, so it needs no CLI change.

**What you could not establish, out of my lane:**
- story:codespec-conform's reader acceptance needs in-repo Go and Rust reader sources (`conformance/reader/&lt;lang&gt;/&lt;case&gt;/`, docs/conformance.md:127-128, D3 option A), and no story's scope creates them. This belongs to the scope or acceptance critics.
- story:support-matrix expects a Java row, but `epic:java-reader` has no `--describe` manifest, so the cell has no source. That follows from its "once `epic:java-reader` lands" wording and is not a set defect.

```findings
- file: .engineering/planning/story/runners-and-manifests.md
  line: 24
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs the runners on every case of conformance/cases (case.cir.json, expected.txt), which story:conformance-cases creates, and no depends_on edge runs to story:conformance-cases; add one"
- file: .engineering/planning/story/support-matrix.md
  line: 24
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its verified and failing cells and its feature-to-case links read the website/data/conformance reports and conformance/cases that story:codespec-conform and story:conformance-cases produce, and the only depends_on edge runs to story:runners-and-manifests; add a depends_on edge to story:codespec-conform"
```
