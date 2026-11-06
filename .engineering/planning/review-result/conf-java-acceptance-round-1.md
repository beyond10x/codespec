---
format: aep.planning-md/3
id: review-result:conf-java-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1 (adapter-conformance and java-reader)
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
story:feature-vocabulary — the acceptance derives ids from "the generated JSON schema's `kind` enums", but `schemas/cir-1.schema.json` has no `kind` enums (each variant is a `"const"` on the `k` tag), so the check it names cannot be run as written — .engineering/planning/story/feature-vocabulary.md:37
story:conformance-cases — "Each `sem.*` id has a case that crosses its edge" is checked by nothing: the bullet above finds subjects only through `examples/*/*.cir.json`, where a `sem.*` id does not appear, and only `sem.int.wrap` has its edge defined — .engineering/planning/story/conformance-cases.md:32
story:runners-and-manifests — "they print `expected.txt` or the function is in `refusals.json`" passes when an adapter refuses every function, so the acceptance does not name that refusals are limited to features the manifest does not claim — .engineering/planning/story/runners-and-manifests.md:42
story:codespec-conform — "The boundary cases of § 6 run for every writer" names no pass or fail outcome and no test, so it reads true on a conform engine that runs them and ignores their result — .engineering/planning/story/codespec-conform.md:44
story:conformance-kit-release — the packing and smoke-test acceptance can only be observed when a release tag is pushed, and no pre-tag check is named that shows the archive and the `--kit` run work — .engineering/planning/story/conformance-kit-release.md:33
story:conformance-kit-release — "with a command run in this repository" names no command and no output, so two readers will not agree on whether the guide's section meets it — .engineering/planning/story/conformance-kit-release.md:37
story:support-matrix — the last bullet's Java clause ("Java, once `epic:java-reader` lands") cannot be observed when this story closes, since Java has no adapter yet, and no fixture or test shows a reader-only language rendering `none` for writing — .engineering/planning/story/support-matrix.md:50
story:java-example-ledger — the Outcome promises the harness byte-equal on 200 traces of 300 commands across the Java original, the Go output and the Rust output, but the acceptance says only "runs the harness", so neither the trace count nor the byte-equal result is checked — .engineering/planning/story/java-example-ledger.md:33
story:java-example-ledger — "reaches every error path" does not say what the error paths are, so the pinned seed-0 test has no list to check against — .engineering/planning/story/java-example-ledger.md:35

What I read: 10 of 10 ids (epic:adapter-conformance, six stories, epic:java-reader, two stories), each with `aep plan artifact show &lt;id&gt;`. I also read `docs/conformance.md` §§ 2–7, `schemas/cir-1.schema.json`, `crates/model/src/lib.rs`, `.github/workflows/release-build.yml`, `aep plan artifact kinds`, `aep plan artifact lifecycle story`, and `review-result:m1-acceptance-round-2` as precedent.

What I could not establish: I did not count the model's variants (16, 17, 12, 4, 7) against the story's figures. I did not run `codespec-docs` or the site build. Out of my lane: the epic's Outcome binds each report "to the adapter version and the kit by digest" and no story acceptance checks the digest binding (`codespec conform --check`, `docs/conformance.md` § 7), which is a scope-coverage question. Also out of my lane: the epic's Done When wants `task check` to run `codespec conform` on readers, while the reader bullet in `story:codespec-conform` names neither generation nor drift-checking of reader reports.

```findings
- file: .engineering/planning/story/feature-vocabulary.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance derives ids from the generated JSON schema's `kind` enums, but schemas/cir-1.schema.json has no `kind` enums (variants are `const` values on the `k` tag), so the named check cannot be run as written"
- file: .engineering/planning/story/conformance-cases.md
  line: 32
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'Each `sem.*` id has a case that crosses its edge' is checked by nothing: the bullet above finds subjects through examples/*/*.cir.json where sem.* ids do not appear, and only sem.int.wrap has its edge defined"
- file: .engineering/planning/story/runners-and-manifests.md
  line: 42
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'they print `expected.txt` or the function is in `refusals.json`' passes when an adapter refuses every function; the acceptance does not limit refusals to features the manifest does not claim"
- file: .engineering/planning/story/codespec-conform.md
  line: 44
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'The boundary cases of § 6 run for every writer' names no pass or fail outcome and no test, so it reads true on an engine that runs them and ignores the result"
- file: .engineering/planning/story/conformance-kit-release.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the packing and smoke-test acceptance is observable only when a release tag is pushed, and no pre-tag check is named that shows the archive and the --kit run work"
- file: .engineering/planning/story/conformance-kit-release.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'with a command run in this repository' names no command and no output, so two readers will not agree on whether the guide's section meets it"
- file: .engineering/planning/story/support-matrix.md
  line: 50
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Java clause ('Java, once `epic:java-reader` lands') cannot be observed when this story closes, since Java has no adapter yet, and no fixture or test shows a reader-only language rendering `none` for writing"
- file: .engineering/planning/story/java-example-ledger.md
  line: 33
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the harness byte-equal on 200 traces of 300 commands across the Java original, the Go output and the Rust output, but the acceptance says only 'runs the harness', so neither the trace count nor the byte-equal result is checked"
- file: .engineering/planning/story/java-example-ledger.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'reaches every error path' does not say what the error paths are, so the pinned seed-0 test has no list to check against"
```
