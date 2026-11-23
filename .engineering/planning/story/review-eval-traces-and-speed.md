---
format: aep.planning-md/3
id: story:review-eval-traces-and-speed
kind: story
status: implemented
title: 'Evaluator traces and speed: append overwrites, unused extern sites, map index'
summary: A fault trace records an append's in-place overwrite of older storage; extern site ids cost nothing without a stub and one traversal with one; map reads and writes go through a hash index.
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: crates/eval/src/count.rs
- confidence: cited
  path: crates/eval/src/fault.rs
- confidence: cited
  path: crates/eval/src/host.rs
- confidence: cited
  path: crates/eval/src/id.rs
- confidence: cited
  path: crates/eval/src/interp.rs
- confidence: cited
  path: crates/eval/src/lib.rs
- confidence: cited
  path: crates/eval/src/value.rs
- confidence: cited
  path: crates/eval/tests/faults.rs
- confidence: cited
  path: crates/eval/tests/fixtures/appendalias
- confidence: cited
  path: crates/eval/tests/fixtures/mapfill
- confidence: cited
  path: crates/eval/tests/stops.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:21:07Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T04:21:07Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:21:14Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A fault trace names every change a faulted call leaves in older memory, an `append` that writes
an older backing array in place included; evaluator startup does no node-id work for extern
sites no stub names and finds the ids it needs in one traversal; map reads and writes compare
only keys that hash alike.

## Findings

- E4: an in-place `append` into older backing storage was not recorded, so the trace reported
  zero updates while the next call saw the change. The in-place path now writes each element
  that already exists through `Interp::write`, the hook ordinary assignments use.
  Test: `crates/eval/tests/faults.rs::an_append_overwriting_an_older_array_in_place_is_an_update`.
- E7: `install` resolved every extern site's id with `node_id`, a walk from the function's start
  per site, even with no stub. Without stubs it now skips the discovery; `extern_sites` and the
  fault plan's send sites take their ids from `id::node_ids`/`id::var_node_ids`, one traversal per
  function or initializer. Test (counted with `codespec_eval::count`, not timed):
  `crates/eval/tests/stops.rs::extern_sites_cost_no_id_work_without_a_stub_and_one_traversal_with_one`.
- E8: map reads and writes searched the entry vector linearly. `MapData` is now a `MapStore`:
  the entries in insertion order plus an index from a key hash consistent with `interp::equal`
  to entry positions. Test (key-comparison counts):
  `crates/eval/tests/stops.rs::map_reads_and_writes_compare_a_bounded_number_of_keys`.

## Done When

- The three tests pass and failed on the unfixed code.
- `cargo test -p b10x-codespec-eval --locked`, `task eval-kit` and `task eval-diff` are green.
