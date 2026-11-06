---
format: aep.planning-md/3
id: review-result:conf-java-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1 (adapter-conformance and java-reader)
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

epic:adapter-conformance — "kit.json" (format `codespec-conformance-kit/1`, the feature list the manifest is "expand[ed] against", `docs/conformance.md:52,101`) and manifest expansion (trailing `*`, `not` subtraction) are promised, and no story claims them. `story:conformance-cases` lists only the five per-case files, and `story:runners-and-manifests` only validates strictly. Natural owners: kit.json to `story:conformance-cases`, expansion to `story:codespec-conform`. — docs/conformance.md:52 and docs/conformance.md:98-101

epic:adapter-conformance — `codespec conform --check &lt;report&gt;` ("re-runs and compares") is part of the design's report promise, and no story's outcome or acceptance mentions it. Natural owner: `story:codespec-conform`. — docs/conformance.md:161

epic:adapter-conformance — The reader corpus is unclaimed. § 6 requires "per claimed feature, a source in their language and its `calls.json` (`conformance/reader/&lt;lang&gt;/&lt;case&gt;/`)", and the Go and Rust readers' reports must conform (Done When). `story:codespec-conform` says only "the Go and Rust readers' reports conform", and no story authors Rust (or Go) reader sources, `calls.json` files or the `--corpus` flag. Natural owner: `story:codespec-conform`, or a new story. — docs/conformance.md:127-128 and .engineering/planning/epic/adapter-conformance.md:49

story:codespec-conform — The Status page promise has a scope line (`crates/docs/src/status.rs`) and no acceptance. "shown on the Status page, one row per feature" and "The status capability list gains one row per adapter, its evidence the report" are claimed by nothing checkable. The Done When wording "`task check` runs `codespec conform` on the Go and Rust readers and writers" is also asserted for writers only: the acceptance generates and drift-checks writer reports and gives reader reports no gate. — docs/conformance.md:165-170 and .engineering/planning/story/codespec-conform.md:36-44

story:support-matrix — The epic's Scope is the closed list "`docs/conformance.md` § 10, in order", which does not include the support matrix. Its Out of Scope does not name it either, so the story's outcome has no sentence in the parent. The operator's 2026-11-06 request is quoted in the story only. The epic's Scope should list it, or this story should say it was added by a later decision. — .engineering/planning/epic/adapter-conformance.md:31-36

story:java-reader-javac — Part of the epic's reader scope is silently narrowed. The acceptance fixture has no `HashMap` and no `Optional` → comma-ok, so the iteration-order difference between `HashMap` and `TreeMap` ("The facts pass must see the difference") is untested. Only four of the seven "kept as `foreign`" categories have a fixture; inheritance beyond one error hierarchy, reflection, annotations with behaviour and streams have none. Add them to the acceptance, or record them as deferred. — .engineering/planning/epic/java-reader.md:39-45 and .engineering/planning/epic/java-reader.md:57

**What I read.** 8 artifacts (2 epics, 8 stories counted as 6 + 2) plus `docs/conformance.md` in full. Commands: `aep plan artifact show` for each id, `aep plan artifact graph`, a grep of the stories for `kit.json`, `--check`, `--corpus` and `status.rs`, and a grep of `crates/cli/src/lib.rs` for the adapter lookup.

**Promises counted.** Adapter-conformance: 14 extracted from the epic and design §§ 2–8 and 10; 10 traced to an item, with the 4 gaps above not traced (kit.json and expansion, `--check`, reader corpus, Status rows) and 1 item without a parent sentence (the matrix). Java-reader: 11 extracted; 9 traced, with Optional/HashMap and the unlisted `foreign` categories narrowed.

**What I could not establish.**
- The `Done When` clause "task check runs conform" depends on the coordinator's Taskfile patch, which I cannot see.
- Out of my lane:
  - `story:codespec-conform` bundles writer and reader conformance, so it is large (design or parallel-safety).
  - `story:conformance-cases` covers only the features the in-repo examples use, although § 2 says "no feature exists without a case". This matches § 10 step 2, so I did not count it.
  - The `codespec-read-java` launcher from `target/adapters` is unspecified (acceptance or design).

```findings
- file: docs/conformance.md
  line: 52
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "epic:adapter-conformance: kit.json (codespec-conformance-kit/1) and manifest expansion (trailing * and not subtraction against kit.json) are promised by the design and claimed by no story; natural owners are story:conformance-cases (kit.json) and story:codespec-conform (expansion)"
- file: docs/conformance.md
  line: 161
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "epic:adapter-conformance: 'codespec conform --check &lt;report&gt; re-runs and compares' is promised by the design and claimed by no story; natural owner is story:codespec-conform"
- file: docs/conformance.md
  line: 127
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "epic:adapter-conformance: the reader corpus (per-claimed-feature sources and calls.json under conformance/reader/&lt;lang&gt;/&lt;case&gt;/, and the --corpus flag) is required for the Go and Rust readers' conforming reports in the epic's Done When, and no story authors it; natural owner is story:codespec-conform or a new story"
- file: .engineering/planning/story/codespec-conform.md
  line: 36
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:codespec-conform lists crates/docs/src/status.rs in scope but no acceptance claims the Status page rows (one row per feature, one capability row per adapter with the report as evidence), and reader reports are given no task check gate although Done When says task check runs conform on readers and writers"
- file: .engineering/planning/epic/adapter-conformance.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:support-matrix is not traceable to the epic's Scope, which is the closed list of docs/conformance.md § 10 and omits the support matrix of § 8; the epic Scope should list it or the story should record it as a later decision"
- file: .engineering/planning/epic/java-reader.md
  line: 39
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:java-reader-javac silently narrows the reader scope: no HashMap or Optional-to-comma-ok in the acceptance fixture (so the HashMap/TreeMap iteration-order fact of the Risks is untested), and no foreign-with-refusal fixture for inheritance beyond one error hierarchy, reflection, annotations with behaviour or streams"
```
