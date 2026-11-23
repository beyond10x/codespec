---
format: aep.planning-md/3
id: story:review-c-writer-assign-and-map
kind: story
status: implemented
title: 'C writer: multiple-assignment targets and indexed maps'
summary: Fix A2 (multiple assignment stores into the element named before the first store) and A7 (generated C map insertion is no longer quadratic)
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: adapters/c/src/codespec_rt.h
- confidence: cited
  path: adapters/c/src/write.rs
- confidence: cited
  path: adapters/c/tests/writer.rs
- confidence: cited
  path: conformance/c/codespec_rt.h
- confidence: cited
  path: conformance/reader/c
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:43:48Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T04:43:48Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:43:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: C writer: multiple-assignment targets and indexed maps

## Outcome

C written by `codespec-write-c` stores a multiple assignment into the targets Go would store into,
and its maps insert and look up keys in a bounded number of key comparisons, so a module with many
distinct map keys no longer runs in quadratic time.

## Findings

- A2 (maintainer review 2026-11-23): `i, values[i] = 1, 7` stored into `values[1]`. The writer
  now fixes each target after the first that is not a plain variable as a pointer to its slot
  before the first store, for values and for the results of one call
  (`write.rs` `pin_targets`). Test:
  `adapters/c/tests/writer.rs::a_multiple_assignment_stores_into_the_element_its_index_named_before_the_first_store`.
- A7 (maintainer review 2026-11-23): `cs_map_put` and `cs_map_find` scanned every key. The runtime
  now keeps an open-addressing index (entry position plus one, linear probing, at most half full)
  beside the insertion-ordered entry arrays; keys and values keep their own allocations, so value
  addresses and iteration order are unchanged. `CS_MAP_PROBE()` counts key comparisons for the
  test. Test:
  `adapters/c/tests/writer.rs::map_insertion_and_lookup_compare_keys_a_bounded_number_of_times_per_operation`.

## Done When

- Both tests fail on 014c8f7a and pass on this branch.
- `conformance/c/codespec_rt.h` equals the writer's header and the reader corpus is regenerated.
- `cargo test` of `codespec-write-c`, `codespec-run-c` and `codespec-read-c` is green.
