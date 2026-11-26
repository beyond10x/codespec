---
format: aep.planning-md/3
id: story:test-budget-existing-suites
kind: story
status: implemented
title: Existing test suites keep the test budget
summary: Suites measured over the 60 s per-test and 120 s per-step budget on 2026-11-23 make expensive products once and kill slow tests
relations:
- informed_by: epic:maintainer-review-2026-11-23
scope:
- confidence: inferred
  path: .config/nextest.toml
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: adapters/c-read/src/lib.rs
- confidence: cited
  path: adapters/c-read/tests/adversary1.rs
- confidence: cited
  path: adapters/c-read/tests/adversary_bounds.rs
- confidence: cited
  path: adapters/c-read/tests/bounds.rs
- confidence: cited
  path: adapters/c-read/tests/cases.rs
- confidence: cited
  path: adapters/c-read/tests/runtime.rs
- confidence: cited
  path: adapters/c/tests/secreview2.rs
- confidence: cited
  path: adapters/c/tests/writer.rs
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/adversary2_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/adversary_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/bounds_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/calls_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/describe_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/entries_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/labeled_test.go
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go/main.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/spans.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/whole_package_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/adversary_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/java_forms_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/write_test.go
- confidence: cited
  path: adapters/rust-read/tests/bounds.rs
- confidence: cited
  path: adapters/rust-read/tests/cases.rs
- confidence: cited
  path: adapters/rust/tests/writer.rs
- confidence: cited
  path: crates/check/tests/checks.rs
- confidence: inferred
  path: crates/conformance/src
- confidence: cited
  path: crates/conformance/tests/conform.rs
- confidence: cited
  path: crates/effects/tests/trace.rs
- confidence: cited
  path: crates/eval/tests/adversary1.rs
- confidence: cited
  path: crates/eval/tests/adversary3.rs
- confidence: cited
  path: crates/eval/tests/diff.rs
- confidence: cited
  path: crates/eval/tests/runner.rs
- confidence: cited
  path: crates/query/tests/checks.rs
- confidence: cited
  path: scripts/check-part.sh
- confidence: cited
  path: scripts/check-parts.txt
revision: 43
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T04:43:17Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-26T04:43:17Z", actor: "human:operator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-26T09:14:09Z", actor: "human:operator", revision: 43, decided_on: {"recorded":{"test_result":1}}}
---
# Story: test-budget-existing-suites

## Outcome

Every existing test suite of codespec keeps the "Tests stay fast" rules of `~/beyond10x/AGENTS.md`:
an expensive product (a reader run, a synthesis, a build, a written crate) is made once per suite,
never per test; the runner kills a test that runs over 60 s and prints the ten slowest; a test step
ends within 120 s. A step over budget is a defect fixed before the next feature.

## Context

While the findings of the maintainer review of 0.10.0 were fixed (PRs #24-#35, under
`epic:maintainer-review-2026-11-23`), the suites below were measured over budget. Each time was
taken on the shared workstation on 2026-11-23 under host load 100-180 (1-minute load average, 20
CPUs), so it is not a clean measurement: it says which suites to work on first, not how slow each is
on an idle machine. Times are wall time of one `cargo test` or `go test` run through the slot pool.

## Over budget (measured 2026-11-23, host load 100-180)

- `crates/eval` tests: `runner` 75-163 s, `adversary1` 31-112 s (the test
  `adv_a_huge_array_zero_value_is_refused_not_an_aborted_allocation` alone took 173 s once),
  `adversary3` 87 s, `diff` 66 s; the whole evaluator suite about 497 s run serially.
- `crates/effects` test `trace`: 22-98 s; `the_readfile_fixture_reaches_an_openat_syscall_leaf`
  runs the Go reader.
- `crates/query` `tests/checks.rs`: 52-236 s for 4 tests; it runs the Go reader per fixture.
- `crates/check` integration test binary: 61-84 s.
- `adapters/rust` `tests/writer.rs`: 172 s.
- `adapters/rust-read`: `cases.rs` 325 s, `bounds.rs` 72 s.
- `adapters/c`: `secreview2_no_doc_text_leaves_its_comment_under_any_c_standard_or_trigraph_setting`
  328 s, `every_kit_case_the_writer_writes_builds_with_wall_werror` 165 s.
- `adapters/c-read`: `adversary_bounds` 130 s, `adversary1` 99 s, `bounds` 89 s, `cases` 82 s.
- `crates/conformance`: `a_stub_writer_that_writes_every_case_right_conforms_and_its_report_is_canonical`
  and `a_copy_of_the_rust_writer_whose_addition_saturates_fails_sem_int_wrap_at_its_first_line`,
  each over 60 s.
- Go: `go test` of `codespec-write-go` 128.8 s, of `codespec-read-go` 149.7 s.

## Fix

Apply the beyond10x `AGENTS.md` "Tests stay fast" rules to each suite above:

- Make each expensive product once per test binary or package (a shared fixture built on first
  use: one Go reader run over all fixtures, one written crate or C build reused by the tests that
  need it), never per test.
- Kill a test that runs over 60 s and print the ten slowest tests of the run.
- Keep each test step of `scripts/check-parts.txt` within 120 s; split or shrink a step that is not.

## Done When

- No test of the suites above runs over 60 s and each test step of `scripts/check-parts.txt` that
  runs them ends within 120 s, measured on an unloaded runner (CI), not on the shared workstation.
- The runner kills a test over 60 s and prints the ten slowest.
- No suite above makes an expensive product per test.

## Out of Scope

- Timing comparisons or benchmarks on the shared workstation (`AGENTS.md` § Never).
