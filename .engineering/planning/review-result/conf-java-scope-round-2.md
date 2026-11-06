---
format: aep.planning-md/3
id: review-result:conf-java-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2 (adapter-conformance and java-reader)
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

**What I read.** 11 artifacts: the two epics and eight stories, all with `aep plan artifact show` run in the plan store. I also read `aep plan artifact graph`, `review-result:conf-java-scope-round-1`, and `docs/conformance.md` in full. I extracted 11 promises from `epic:adapter-conformance` and traced all 11. I extracted 14 from `epic:java-reader` and traced all 14. Round 1's six findings are each fixed in the revised bodies:
- `kit.json` is claimed in `story:conformance-cases`, and manifest expansion in `story:codespec-conform`.
- `--check` is claimed in `story:codespec-conform`.
- The reader corpus and `--corpus` are claimed in `story:codespec-conform`.
- The Status capability row now has an acceptance.
- The support matrix is listed in the epic's Scope.
- The Java fixture now has `HashMap`, `Optional`, and a refusal fixture for each foreign category except `threads`.

**What I could not establish.**
- The Taskfile patches that put `task check` behind each story belong to the coordinator and are not visible to me.
- The design (`docs/conformance.md:166`) says reports are "shown on the Status page, one row per feature". The story gives the Status page one capability row per adapter, and the per-feature view is the support matrix. I read that as the matrix replacing the per-feature Status rows, and it is not a finding. The story could say so in one clause.
- The epic's Decision says the reader reads "Java 8 included". No acceptance reads a source at Java 8 level. This is thin rather than missing, and it is the acceptance critic's lane.
- Threads and generics are each probed by one construct only: `synchronized` for threads, a generic method for generics. I did not count this as narrowing.
- Out of my lane:
  - `story:runners-and-manifests` has the Go and Rust writer outputs reproduce `expected.txt` for every case. `story:codespec-conform` then has the same writers report `conforms: true`. This overlap is for the design or parallel-safety critics.
  - The report's digest binding of manifest and kit is in the epic Outcome and `docs/conformance.md` § 7, but no acceptance line checks it. That belongs to the acceptance critic.

```findings
[]
```
