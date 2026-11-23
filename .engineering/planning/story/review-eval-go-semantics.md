---
format: aep.planning-md/3
id: story:review-eval-go-semantics
kind: story
status: implemented
title: Evaluator semantics match Go (review E1, E2, E3, E5, E6, E9)
summary: Array len, interface equality, nil-channel range, closure scopes, Println order and strict-writer refusals in closures follow Go
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: crates/eval/src/interp.rs
- confidence: cited
  path: crates/eval/src/runner.rs
- confidence: cited
  path: crates/eval/tests/adversary2.rs
- confidence: cited
  path: crates/eval/tests/fixtures/adversary2/globaldefer
- confidence: cited
  path: crates/eval/tests/fixtures/adversary2/gosem
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:16:55Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T04:16:56Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:17:04Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Evaluator semantics match Go (review E1, E2, E3, E5, E6, E9)

## Outcome

Someone running `codespec eval` or the evaluator's `cir` writer and runner gets Go's answer, or Go's
stop, for array lengths, interface equality, a range over a nil channel, a closure shadowing a
captured variable and `fmt.Println`, and the strict `cir` writer refuses a module whose
package-level function literal holds a `foreign` statement or a `raise`.

## Findings

- E1, `len` of an array or of a pointer to one is its type's length, nil pointer included, and its
  operand is evaluated when it holds a call or a receive:
  `crates/eval/tests/adversary2.rs::review_e1_len_of_an_array_is_its_type_length_and_a_call_in_it_runs`.
- E2, `==`, `!=` and a `switch` case on two interface values of one dynamic type that is not
  comparable panic; different dynamic types stay unequal:
  `crates/eval/tests/adversary2.rs::review_e2_interface_equality_of_an_uncomparable_dynamic_type_panics`.
- E3, a range over a nil channel stops as blocked; over a nil slice it runs no iteration:
  `crates/eval/tests/adversary2.rs::review_e3_a_range_over_a_nil_channel_blocks`.
- E5, the captured cells are a scope enclosing a function literal's body, so `x :=` declares and
  `x =` assigns the outer variable:
  `crates/eval/tests/adversary2.rs::review_e5_a_closure_declaration_does_not_overwrite_the_captured_variable`.
- E6, the `println` intrinsic evaluates every argument before formatting any:
  `crates/eval/tests/adversary2.rs::review_e6_println_evaluates_every_argument_before_formatting`.
- E9, `runner::refusals` walks each variable's initializer tree as well as function bodies:
  `crates/eval/tests/adversary2.rs::review_e9_the_strict_writer_refuses_a_foreign_statement_in_a_variable_initializer`.

The Go answers are those of `go run` (go1.27.1) of the fixture `tests/fixtures/adversary2/gosem`.

## Done When

- Each test above fails on `91086c62` and passes.
- `cargo test -p b10x-codespec-eval --locked`, `task eval-kit`, `task eval-diff` and
  `task examples-effects` are green.
