---
format: aep.planning-md/3
id: review-result:eca-panel-round-1
kind: review-result
status: active
title: Plan-critic panel, round 1 (errors-cross-every-adapter)
relations:
- reviews: epic:errors-cross-every-adapter
- reviews: story:rust-reader-error-propagation
- reviews: story:jvm-readers-error-propagation
revision: 1
---
Round 1 of the plan-critic panel on `epic:errors-cross-every-adapter`,
`story:rust-reader-error-propagation` and `story:jvm-readers-error-propagation`.

- Scope: approve. Five promises (three Outcome, two Done When) traced to an item, three exclusions
  respected; no overlap with older artifacts.
- Acceptance: needs-revision. (1) The Rust reader story promised `f()?;` as a statement with no
  acceptance line observing it. (2) Its read-back test named no form while the writer story allowed
  `?` or `match`. (3) The epic's Done When observed only three reader reports, not the Go reader or
  either writer.
- Design: needs-revision. (1) The Groovy reader refuses `throw` and claims no `type.error`, so the
  Groovy half of the JVM story could not pass by a claim string; the epic's Why Now and Done When
  inherited that. (2) The read-back form (as acceptance 2). (3) The ledger's `Account.post` is
  `void`, which the Java reader lifts to the error-only form, while the writer story tested only
  the value-returning form. The Java reader already lifts a throwing call into the form
  (`Reader.java`, `throwingCallee`, `propagate`).
- Parallel-safety: needs-revision. The two stories share `website/data/support-matrix.json` and the
  `kit.sha256` line of all six conformance reports (a file added under `conformance/cases/` changes
  it), unrecorded; the Rust reader story named only `reader-rust.json`.

Revisions: the epic drops Groovy (Out of Scope, its own epic later) and observes five reports; the
Rust reader story names `?`, adds a statement-form test, `tests/cases.rs` and `CHANGELOG.md` to
scope, and names all six reports; the JVM story is Java-only and names the error-only form; both
stories record that the shared generated files are regenerated on the merged head. The writer
story (revision 16) fixes `?` and adds an error-only callee test.

```findings
- file: .engineering/planning/story/rust-reader-error-propagation.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "f()?; as a statement promised, not observed"
- file: .engineering/planning/story/rust-reader-error-propagation.md
  line: 52
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "read-back names no form while the writer allowed ? or match"
- file: .engineering/planning/epic/errors-cross-every-adapter.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Done When observes only three reader reports"
- file: adapters/groovy/src/codespec/read/groovy/Reader.java
  line: 1640
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Groovy reader refuses throw; a claim string cannot make the Groovy half pass"
- file: .engineering/planning/story/jvm-readers-error-propagation.md
  line: 58
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the ledger's void callee is the error-only form the writer story did not test"
- file: .engineering/planning/story/jvm-readers-error-propagation.md
  line: 134
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shared generated files (support matrix, kit.sha256 in six reports) unrecorded"
- file: .engineering/planning/story/rust-reader-error-propagation.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope names only reader-rust.json; all six reports change"
```
