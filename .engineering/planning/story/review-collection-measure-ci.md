---
format: aep.planning-md/3
id: story:review-collection-measure-ci
kind: story
status: implemented
title: Repository collection, measure and CI cover what they claim
summary: read-repo.sh keeps every package and fails on any failure, measure counts opaque and closure-foreign functions, CI runs every test target from one list
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/cli/src/measure.rs
- confidence: cited
  path: crates/cli/tests/barrier.rs
- confidence: cited
  path: crates/cli/tests/fixtures
- confidence: cited
  path: crates/cli/tests/measure.rs
- confidence: cited
  path: scripts/check-part.sh
- confidence: cited
  path: scripts/check-parts.txt
- confidence: cited
  path: scripts/read-repo.sh
- confidence: cited
  path: website/docs/guides/map-a-repository-to-syscalls.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:41:52Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T04:41:52Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:42:01Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: review-collection-measure-ci

## Outcome

Someone who collects a repository with `scripts/read-repo.sh`, reads its coverage with `codespec measure`, or trusts a green CI run gets what each claims: one document per package and a nonzero exit on any failure, a share read that counts every function the reader did not hold, and a CI run of every test target and gate task the local gate runs.

## Findings

- G1 (a whole-repository scan silently loses packages): documents are named by a reversible encoding of the import path or directory (`%` as `%25`, `_` as `%5F`, then `/` as `_`), and two packages mapping to one name fail the collection. Test `crates/cli/tests/barrier.rs::import_paths_a_slash_b_and_a_underscore_b_give_two_documents`.
- G2 (the collector exits successfully after a reader failure): every package is still tried; a failed read, a failed `go list`, an import `go list` gives no directory for, or an enumeration with no package makes the script exit 1, with and without `--deps`. Tests `crates/cli/tests/barrier.rs::a_package_that_fails_to_read_fails_the_collection_after_every_package_is_tried`, `crates/cli/tests/barrier.rs::an_enumeration_that_fails_or_finds_no_package_fails_the_collection`.
- G3 (`codespec measure` reports full coverage for partially read code): an `opaque` function is a refused function, counted once when a declaration refusal `T.m(…)` already counts it, and a `foreign` statement inside a lambda's body in any expression makes its function refused. Tests `crates/cli/tests/measure.rs::an_opaque_function_and_a_foreign_statement_in_a_closure_are_refused`, `crates/cli/tests/measure.rs::a_refused_declaration_also_listed_as_opaque_counts_once`.
- G4 (the CI gate omits evaluator and effects tests): `scripts/check-parts.txt` is the one list of the gate's parts; `task check` runs it through `scripts/check-part.sh all` and CI's matrix is made from it (`scripts/check-part.sh matrix`, the `plan` job), every part run by the same `scripts/check-part.sh run`. The `lint` part runs `scripts/check-part.sh inventory` (also `task check-parts`), which fails when a cargo test target (lib, bin, integration test, doc test) is in no test part, or a part names a package or `--test=` target that does not exist. The list adds the effects and evaluator crates, the conformance crate's `adversary_evalstep` test and doc tests, and the tasks `examples-effects`, `eval-kit` and `eval-diff`.

## Done When

- The four tests above pass and fail on the unfixed code.
- `scripts/check-part.sh inventory` passes on the new list and fails on the list CI used before.
- `cargo test -p b10x-codespec-cli --locked`, clippy and fmt are green; the workflow YAML parses.
