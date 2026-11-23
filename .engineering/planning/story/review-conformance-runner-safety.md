---
format: aep.planning-md/3
id: story:review-conformance-runner-safety
kind: story
status: implemented
title: 'Conformance runner safety: deadlines, crashes, tree reads, check baseline'
summary: codespec conform keeps its per-program deadline, counts a crashed adapter as a failure, reads only regular files in its trees, and refuses a report inside its work directory
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/tests/conform.rs
- confidence: cited
  path: crates/conformance/src/conform.rs
- confidence: cited
  path: crates/conformance/tests/conform.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:58:41Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T03:58:41Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T03:58:50Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: review-conformance-runner-safety

## Outcome

Someone running `codespec conform` gets a run that keeps its per-program time limit, never counts a
crashed adapter as a refusal, fingerprints only the files inside the kit, corpus and writer output,
and never calls a report current after erasing it.

## Findings

- G5, an adapter invocation could outlast its timeout: `--describe` used the 300 s default, and a
  descendant holding a pipe after the program exited blocked the output threads.
  `crates/conformance/tests/conform.rs::a_program_whose_descendant_holds_its_output_after_it_exits_times_out_at_the_limit`,
  `crates/conformance/tests/conform.rs::describe_runs_within_the_limit_conform_adapter_within_is_given`.
- G6, a reader or writer ended by a signal counted as a refusal.
  `crates/conformance/tests/conform.rs::a_writer_ended_by_a_signal_fails_an_unclaimed_feature_and_the_unknown_kind_boundary`,
  `crates/conformance/tests/conform.rs::a_reader_run_program_ended_by_a_signal_fails_the_unclaimed_case_with_run_failed`.
- G7, kit, corpus and writer-output fingerprints followed links and held whole files in memory.
  `crates/conformance/tests/conform.rs::a_tree_digest_refuses_links_and_special_files_and_keeps_its_format`,
  `crates/conformance/tests/conform.rs::a_writer_that_writes_a_symbolic_link_ends_the_run_with_an_error`.
- G8, `conform --check` could erase the report it then called current.
  `crates/cli/tests/conform.rs::conform_refuses_a_report_inside_the_work_directory_and_keeps_it`.

## Done When

- Each test above fails on 0.10.0 and passes.
- `cargo test -p b10x-codespec-conformance`, `cargo test -p b10x-codespec-cli --test conform`,
  clippy and `task conformance-kit` are green.
