---
format: aep.planning-md/3
id: review-result:conf-java-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2 (adapter-conformance and java-reader)
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

- story:runners-and-manifests — adds the `codespec-run-rust` crate (`adapters/rust-run`) but its Scope never says who edits the `Taskfile.yml` `adapters` task, whose `cargo build -p codespec-write-rust -p codespec-read-rust` line and `cp` line must name the new crate; story:java-reader-javac patches the same task and says the coordinator owns it, so the body should state "`Taskfile.yml` `adapters`: the coordinator's (the unit writes a patch)" (inferred, the Taskfile line is cited from the tree and the need follows from `codespec-run-&lt;lang&gt;` being run from the adapters directory) — .engineering/planning/story/runners-and-manifests.md:55, .engineering/planning/story/java-reader-javac.md:45, Taskfile.yml:54-60

**What I read:** 9 artifacts (the 8 stories of both sets plus story:rust-reader-loops and story:series-rust-to-go as context). Commands: `aep plan artifact show` on each, `aep plan artifact waves --kind story --status active`, `aep plan artifact graph`, and `aep plan artifact show review-result:conf-java-parallel-safety-round-1`. In the tree I read `Cargo.toml`, `Taskfile.yml`, `adapters/rust-read/src/*.rs`, `crates/docs/src/main.rs` and the `examples/` listing.

**Surfaces:** 8 of 8 stories are cited and 0 are inferred or unplaceable. The context story rust-reader-loops has 2 inferred files, which I did not count. I traced 11 acceptance reads to a producer in the set, and all 11 have a `depends_on` edge, directly or through a chain.

**Round 1 findings, all five landed:**
- The runners-and-manifests edge to conformance-cases is there.
- support-matrix now depends on both conform and runners.
- conformance-cases names its four examples instead of a glob.
- `Cargo.toml`, `Cargo.lock` and `crates.md` are listed in both bodies and ordered by the edge.
- codespec-conform now has `crates/docs/src/main.rs` and `status.rs` in scope and is ordered before support-matrix.

**The 10 collisions `waves` still prints** are each ordered by a `depends_on` chain. The exception is conformance-kit-release with series-rust-to-go on `CHANGELOG.md`, which involves an item outside the set, and the two sit in different waves (5 and 2).

**The three stories running now stay disjoint:**
- feature-vocabulary works in `crates/model`, `crates/cli`, `schemas/` and `cli.md`.
- java-reader-javac works in `adapters/java/`.
- rust-reader-loops works in `adapters/rust-read/src/{body,lib,names}.rs` and `tests/reader.rs`. The `--describe` flag from runners-and-manifests lands in `adapters/rust-read/src/main.rs`, a separate 31-line file.

**Could not establish, outside my lane or outside the set, so they do not set the verdict:**
- **Rust reader's manifest.** story:codespec-conform requires the Rust reader's report to say `conforms: true` for the features its manifest claims. What the reader can read for loops, `continue`, multi-value `assign` and `switch` is what story:rust-reader-loops produces. No edge or text orders the two. This is practically settled, since rust-reader-loops is being implemented now. Whoever writes the manifest in runners-and-manifests should claim only what has landed.
- **`series.rust-read.cir.json`.** conformance-cases reads "their `*.cir.json` documents" of `examples/series`. story:series-rust-to-go (outside the set, wave 2 beside conformance-cases) creates `examples/series/series.rust-read.cir.json`. Naming the documents the acceptance reads would remove the ambiguity.
- **Java adapter and the matrix.** support-matrix reads "each adapter's `--describe` manifest", and `codespec-read-java` from java-reader-javac has none. How the generator enumerates adapters is not stated, so I cannot tell whether Java would appear. That is for the acceptance critic.

```findings
- file: .engineering/planning/story/runners-and-manifests.md
  line: 55
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adds the codespec-run-rust crate but its Scope does not say who edits the Taskfile.yml adapters task (Taskfile.yml:54-60 names crates explicitly), which story:java-reader-javac also patches (java-reader-javac.md:45) as the coordinator's; state the Taskfile adapters line as the coordinator's patch (inferred: that the new crate must be added to the adapters task is my reading of the Taskfile, not stated in the body)"
```
