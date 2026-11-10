---
format: aep.planning-md/3
id: review-result:reach-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic on the reach epic, round 2
relations:
- reviews: epic:sound-reach
- reviews: story:reach-three-valued
- reviews: story:reach-entry-file
- reviews: story:reader-indirect-call-sites
- reviews: story:reader-indirect-sites-rust
- reviews: story:reader-indirect-sites-go
- reviews: story:reader-indirect-sites-jvm
- reviews: story:reach-at-positions
- reviews: story:reach-registered-candidates
- reviews: story:reach-framework-entries
revision: 1
---
needs-revision

All eight round-1 findings are resolved except that one new, smaller one has surfaced.

Round-1 check:
- **Typed scope:** all nine stories now carry a typed `scope` from their Own lines. `aep plan artifact waves` places every one of them in a wave and lists none as unassessed.
- **Edges:** the `depends_on` edges form a chain in epic order.
  - three-valued → entry-file → indirect-call-sites.
  - indirect-call-sites → rust, go and jvm, side by side.
  - rust, go and jvm → at-positions → registered-candidates → framework-entries.
  - The waves output puts them in the order 21, 24, 28, 30/31/32, 34, 35, 36, with no cycle.
- **Model and schema blocker:** indirect-call-sites now has edges to `cir1-raise-out-of-entry`, `cir1-fields-of-imported-types` and `cir1-defer-receiver-root`. Its Scope section names them as the reason.
- **function-spans collision:** indirect-call-sites now has an edge to `reader-function-spans`. The rust and go stories have the same edge.
- **kit stories:** each reader story has an edge to its adapter's kit story (`reader-c-kit-conventions`, `reader-rust-kit`, `reader-go-kit`).
- **jvm story:** it has edges to `reader-java-quarkus-refusals` and the two `cir1-*` stories that edit `adapters/java`. It has no edge to function-spans, and function-spans' Own line does not list `adapters/java` or `adapters/groovy`.
- **cli and effects collisions:** the pairs on `crates/cli` and `crates/effects` are all ordered by the chain.
- **mcp-server-read-tools:** it depends on `reach-three-valued` for `crates/cli`.
- **groovy adapter:** `reach-framework-entries` and `reader-indirect-sites-jvm` share `adapters/groovy` and are ordered by an edge.

Rust, go and jvm own disjoint adapters and examples. Every pair among them shares only `Taskfile.yml` and `CHANGELOG.md`, which the coordinator resolves, so I did not report them.

Finding:
- **entry-file:** it is a reverse-order read, and I rest it on an inferred surface. `story:reach-entry-file` has one acceptance line that reads state a later story creates. The body does not name which example sets it uses.

What I read: 10 artifacts (the epic, the 9 stories, and round 1's review-result) with `aep plan artifact show`, `aep plan artifact waves`, and `git grep` and `ls` on the tree. I also read the neighbours `reader-function-spans`, `mcp-server-read-tools`, `reader-java-quarkus-refusals`, the three `cir1-*` stories, `reader-go-kit`, `reader-rust-kit` and `reader-c-kit-conventions`.

Surfaces established, of the 9 stories:
- 9 cited: each body's Own line is also the store `scope`.
- 0 inferred.
- 0 unplaced.

Acceptance reads traced to a producer in the set: 15. A direct `depends_on` edge records 11 of them. Four run through the chain only, from `reach-registered-candidates` to `indirect-call-sites`, `reach-rust` and `reach-c`, and from `reach-framework-entries` to `--entries`. Those four are ordered by the chain and need no edge of their own. The `entry-file` read above is a 16th and is not recorded.

What I could not establish:
- Whether `waves` is right to keep rust, go and jvm in separate waves (30, 31, 32) when the epic says they can run side by side. The only thing keeping them apart is `Taskfile.yml` and `CHANGELOG.md`, which is the coordinator's.
- `aep plan artifact waves` also lists collisions of the reach stories with stories outside this set on `crates/cli/src/lib.rs`, `crates/effects/src/lib.rs`, `crates/model/src/lib.rs` and `schemas/cir-1.schema.json`. They are outside the set I was given, so I did not assess them.

```findings
- file: .engineering/planning/story/reach-entry-file.md
  line: 52
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs each starter file (Go, Rust, C, Java) against 'its language's example set' without naming one; the only per-language example sets this epic creates are examples/reach-c, reach-go, reach-rust and reach-jvm, built by stories later in the chain (reader-indirect-call-sites, reader-indirect-sites-*), and examples/ holds no C or Java set today, so if those are meant the reader runs before its producer with no depends_on edge; name existing sets, or limit the test to parsing the files and add the runs to the reader stories. Inferred: the surface rests on the unnamed 'example set', not a cited path"
```
