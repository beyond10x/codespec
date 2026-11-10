---
format: aep.planning-md/3
id: review-result:rd-scope-round-1
kind: review-result
status: active
title: Scope critic, reader coverage, round 1
relations:
- reviews: epic:reader-coverage
- reviews: story:reader-go-kit
- reviews: story:reader-rust-kit
- reviews: story:reader-java-kit
- reviews: story:reader-c-kit-conventions
- reviews: story:reader-coverage-floor
revision: 1
---
needs-revision

epic:reader-coverage — The epic counts readers against all 127 kit features, but the repository measures reader coverage against 120 applicable features. That changes the targets: Go 124, Rust 121 and Java 121 can only be reached if those readers claim `fact.*`, which this repository's rules forbid for a reader. Restate Done When as verified of the 120 applicable reader features, as `website/data/support-matrix.json` and the bars on `/docs/reference/support` count them, and lower the thresholds to fit — .engineering/planning/epic/reader-coverage.md:14,28; website/docs/reference/support.mdx:36,45 ("coverage counts applicable features only"); docs/conformance.md:104-111; website/src/components/SupportMatrix/index.tsx:120-121
story:reader-go-kit — The Outcome promises 10 features that this repository says the Go reader cannot claim: the 7 `fact.*`, `stmt.lookup_else` (the lookup-guard pass produces it) and `decl.union`/`expr.zero`/`intrinsic.sorted_keys`/`type.unit` (no Go form). Narrow it to `type.tuple` plus a check that the other 5 refusals are documented, and set the floor at 115 of 120 — .engineering/planning/story/reader-go-kit.md:16-19,24; docs/conformance.md:104-111; crates/conformance/src/lib.rs:35-38; adapters/go/cmd/codespec-read-go/describe.go:7-16
story:reader-rust-kit — "the 38 kit features with verdict `untested`" includes the 7 `fact.*` that no reader can claim. Name the 31 non-fact features and restate 121 of 127 against 120 — .engineering/planning/story/reader-rust-kit.md:16,24; website/data/support-matrix.json (Rust read: n/a 7)
story:reader-java-kit — "the 50 kit features with verdict `untested`" includes the 7 `fact.*`. Name the 43 non-fact features and restate 121 of 127 against 120 — .engineering/planning/story/reader-java-kit.md:16,24; website/data/support-matrix.json (Java read: n/a 7)
story:reader-c-kit-conventions — The Outcome claims "the seven `fact.*` features", which this repository says no reader can claim. Drop them and restate 115 of 127 against 120 — .engineering/planning/story/reader-c-kit-conventions.md:26,31; website/docs/reference/support.mdx:36
story:reader-coverage-floor — The bullet "The site's support page shows the count and percentage per reader" asks for something that already exists, and `reader-coverage.json` would repeat the read columns of `support-matrix.json`. Drop both and have the floor step read `support-matrix.json` — .engineering/planning/story/reader-coverage-floor.md:25-27,31; website/src/components/SupportMatrix/index.tsx:120-146; website/docs/reference/support.mdx:41-46
story:reader-coverage-floor — The epic promises "the number per reader is printed by one command", but the story only writes a file and prints a count when a reader is below its floor. Name the one command that prints every reader's count on success too — .engineering/planning/epic/reader-coverage.md:16; .engineering/planning/story/reader-coverage-floor.md:25-30
story:reader-coverage-floor — "from the five reader reports" with "one number per reader" also puts a floor on the Groovy reader, which the epic places out of scope. Limit the floor to Go, Rust, Java and C — .engineering/planning/story/reader-coverage-floor.md:25,28; .engineering/planning/epic/reader-coverage.md (Out of Scope: "The Groovy reader")

Read: 8 artifacts (`aep plan artifact show` on the epic and its 7 stories; also `epic:java-quarkus-reader` and `story:go-reader-full-coverage`), plus `aep plan artifact graph`. I compared the per-story feature lists against `website/data/conformance/reader-{go,rust,java,c}.json` with a Python one-liner. I also read support-matrix.json, status.json, support.mdx, the SupportMatrix component, docs/conformance.md § 5, crates/conformance and crates/docs/src/support.rs, the Go describe.go and adapters/java/README.md § A quickstart.
Promises: 9 taken from the epic (Go, Rust, Java, C at their targets; fail 0 / conforms; the Quarkus quickstart at 2 refusals or fewer; unclaimed features documented; one command; floor). 8 are fully traced to a story. "Printed by one command" is only partly covered.
Feature lists: all unclaimed features of each report are covered. Go's 13 match the report. Rust and Java say "all untested", 38 and 50. The two C stories add up to exactly the 68 untested features (30 + 31 + 7 facts), with no overlap and nothing extra.
Groovy: the Groovy bullet in `story:reader-java-kit` is not out of scope. The Groovy corpus is built from the kit's `java/Case.java` files (conform.rs:87; AGENTS.md:132), so new Java cases change the Groovy report, and a `groovy/Case.groovy` is a case source, not reader code.
Quarkus: `story:reader-java-quarkus-refusals` covers every reason in the refusal table of adapters/java/README.md:125-129. Nothing in `epic:java-quarkus-reader` claims those refusals.
Prior meaning: `story:go-reader-full-coverage` (implemented, operator request 2026-11-06) defined full reader coverage as "the vocabulary minus `fact.*`". That is the strongest evidence for what "100%" means here.

What I could not establish:
- Whether the operator meant the support-page percentage or kit `summary.pass`. The precedent and the existing page both point to the 120 measure.
- The floor is not in the operator's sentence. It traces to the epic and is a cheap guard, so I did not count it as a finding; only the duplicate file and page are.
- No `codespec measure` output measures reader completeness (Taskfile.yml:180 runs it only on the Groovy lending app), so it is not another measure.
- support.mdx:46 says "all 98" where the data gives 120 applicable features. The page is stale, and that is outside these artifacts.
- Out of my lane (design): `story:reader-go-kit` owns `case.go` edits while other stories rely on `case.go` not changing. Out of my lane (acceptance): Rust/Java "case.rs/Case.java for at least 129 of the 135 cases".

```findings
- file: .engineering/planning/epic/reader-coverage.md
  line: 28
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic measures pass of 127 kit features while the repository's own reader-coverage measure (support-matrix.json, /docs/reference/support bars) counts 120 applicable features with fact.* n/a, so Go 124, Rust 121 and Java 121 are reachable only by readers claiming fact.*, which docs/conformance.md 5 forbids; restate Done When as verified of 120 applicable reader features with thresholds that fit"
- file: .engineering/planning/story/reader-go-kit.md
  line: 16
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises 10 features the repository documents as unclaimable by the Go reader (7 fact.*, pass-only stmt.lookup_else, Go-unreachable decl.union/expr.zero/intrinsic.sorted_keys/type.unit); narrow it to type.tuple plus a check that the other 5 refusals are documented, floor 115 of 120"
- file: .engineering/planning/story/reader-rust-kit.md
  line: 16
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the 38 kit features with verdict untested\" includes the 7 fact.* features no reader may claim; name the 31 non-fact features and restate 121 of 127 against 120"
- file: .engineering/planning/story/reader-java-kit.md
  line: 16
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the 50 kit features with verdict untested\" includes the 7 fact.* features no reader may claim; name the 43 non-fact features and restate 121 of 127 against 120"
- file: .engineering/planning/story/reader-c-kit-conventions.md
  line: 26
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome claims \"the seven fact.* features\", which the repository says no reader can claim; drop them and restate 115 of 127 against 120"
- file: .engineering/planning/story/reader-coverage-floor.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the per-reader count and percentage on the support page already exist (SupportMatrix index.tsx:120-146) and reader-coverage.json duplicates support-matrix.json's read columns; drop both and have the floor step read support-matrix.json"
- file: .engineering/planning/epic/reader-coverage.md
  line: 16
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic promises \"the number per reader is printed by one command\" and story:reader-coverage-floor only writes a file and prints a count when a reader is below its floor; name the one command that prints every reader's count on success too"
- file: .engineering/planning/story/reader-coverage-floor.md
  line: 25
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"from the five reader reports\" with one floor number per reader also floors the Groovy reader, which the epic places out of scope; limit the floor to Go, Rust, Java and C"
```
