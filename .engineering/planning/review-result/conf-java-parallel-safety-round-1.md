---
format: aep.planning-md/3
id: review-result:conf-java-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1 (adapter-conformance and java-reader)
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

- story:runners-and-manifests — its acceptance runs on every case of `conformance/cases` and compares `expected.txt` and `refusals.json`, which story:conformance-cases creates; the only `depends_on` edge is to story:feature-vocabulary, so `waves` puts both in wave 2; add `depends_on story:conformance-cases` or merge the two (cited) — .engineering/planning/story/runners-and-manifests.md:41, .engineering/planning/story/conformance-cases.md:41
- story:support-matrix — its acceptance reads `website/data/conformance/` reports (the `verified` and `failing` cells) and links each feature id to its `conformance/cases` case; those are written by story:codespec-conform and story:conformance-cases, and the only edge is to story:runners-and-manifests, so it shares wave 3 with codespec-conform; add `depends_on` edges to both, or drop the report and case reads from the acceptance (cited) — .engineering/planning/story/support-matrix.md:41, .engineering/planning/story/codespec-conform.md:37
- story:conformance-cases — its acceptance reads `examples/*/*.cir.json`, a glob that will include the `examples/ledger` documents story:java-example-ledger creates, with no edge between the two and both in wave 2; either add `depends_on story:java-example-ledger` or name the example set the acceptance reads (cited) — .engineering/planning/story/conformance-cases.md:30, .engineering/planning/story/java-example-ledger.md:46
- story:conformance-cases — both this and story:runners-and-manifests create a workspace crate (`crates/conformance`, `adapters/rust-run`), so both edit the `members` line of root `Cargo.toml` (3), both change `Cargo.lock`, and both change the generated `website/docs/reference/crates.md`; no body lists these files and both are in wave 2, so add an ordering edge recording `Cargo.toml` as the reason, or have the coordinator own the `members` edit as it owns `Taskfile.yml` (cited for the crates; the shared files are cited from the tree, not from either body) — .engineering/planning/story/conformance-cases.md:41, .engineering/planning/story/runners-and-manifests.md:48, crates/docs/src/crates.rs:1
- story:codespec-conform — its acceptance says the reports in `website/data/conformance/` are "drift-checked", but the only drift check is the `outputs()` list in `crates/docs/src/main.rs`, which is not in its scope and is in story:support-matrix's, in the same wave 3; the body should say where the drift check lives and who edits that list (inferred: that conform needs `main.rs` is my reading, not stated) — .engineering/planning/story/codespec-conform.md:37, crates/docs/src/main.rs:177

What I read: 11 artifacts (the 2 epics, the 8 stories of both sets, and story:rust-reader-loops and story:series-rust-to-go as context), through `aep plan artifact show`, `aep plan artifact waves --kind story --status active`, `aep plan artifact graph` and `aep plan artifact relations`, plus `Cargo.toml`, `crates/docs/src/{main,crates,cli,status}.rs`, `crates/cli/src/*.rs`, `Taskfile.yml` and the generated `website/docs/reference/cli.md` in the codespec-plan tree.

Surface counts: 8 of the 8 stories (set 1 and set 2) were placed by cited scope, 0 by inference and 0 not at all. 11 acceptance reads traced to a producer in the set; 7 have a `depends_on` edge and 4 do not (the first, second and third findings above, counting the two reads in the second).

The three stories running now are disjoint on files. story:rust-reader-loops is on `adapters/rust-read/`, story:java-reader-javac on `adapters/java/` and story:feature-vocabulary on `crates/model` and `crates/cli`, `schemas/` and the Taskfile patch. `codespec read --lang java` needs no `crates/cli` change, because the language dispatch is generic (`crates/cli/src/main.rs:130`).

Could not establish, and not set into the verdict:
- Collisions in the `waves` output (`codespec-conform` with `conformance-cases`, `codespec-conform` with `feature-vocabulary`, `feature-vocabulary` with `runners-and-manifests`) are already ordered by `depends_on` chains, so they are not findings.
- The `CHANGELOG.md` collision between story:conformance-kit-release and story:series-rust-to-go involves an item outside the set; the two are in different waves (4 and 2).
- Not in my lane: story:feature-vocabulary's scope omits the regenerated `website/docs/reference/cli.md`. Its new `features` subcommand makes `crates/docs`'s `every_generated_file_is_current_in_this_tree` fail otherwise (`crates/docs/src/main.rs:304`). The same goes for story:codespec-conform. That also belongs to the design and acceptance critics.
- story:support-matrix lists `website/sidebars.ts` in its body scope but not in its structured scope, so `waves` cannot see it.

```findings
- file: .engineering/planning/story/runners-and-manifests.md
  line: 41
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs on every case of conformance/cases and compares expected.txt and refusals.json, which story:conformance-cases creates (conformance-cases.md:41); the only depends_on edge is to story:feature-vocabulary, so waves schedules both in wave 2; add depends_on story:conformance-cases or merge the two"
- file: .engineering/planning/story/support-matrix.md
  line: 41
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads website/data/conformance/ reports (verified and failing cells) and links each feature id to its conformance/cases case; story:codespec-conform and story:conformance-cases write those and the only edge is to story:runners-and-manifests, so it shares wave 3 with codespec-conform; add depends_on edges to both or drop those reads from the acceptance"
- file: .engineering/planning/story/conformance-cases.md
  line: 30
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the glob examples/*/*.cir.json, which will include the examples/ledger documents story:java-example-ledger creates, with no edge and both in wave 2; add depends_on story:java-example-ledger or name the example set the acceptance reads"
- file: .engineering/planning/story/conformance-cases.md
  line: 41
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "both this and story:runners-and-manifests (runners-and-manifests.md:48) create a workspace crate, so both edit the members line of root Cargo.toml, Cargo.lock and the generated website/docs/reference/crates.md (crates/docs/src/crates.rs:1), none listed in either body, both in wave 2; add an ordering edge naming Cargo.toml as the reason or make the members edit the coordinator's"
- file: .engineering/planning/story/codespec-conform.md
  line: 37
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the reports are said to be drift-checked but the only drift check is the outputs() list in crates/docs/src/main.rs:177, which is not in this scope and is in story:support-matrix's scope, same wave 3; say where the drift check lives and who edits that list (inferred: that conform needs main.rs is not stated in the body)"
```
