---
format: aep.planning-md/3
id: story:writers-distinct-names
kind: story
status: draft
title: A writer never gives two names of a document one name in its output
summary: Name collisions and reserved names in the Rust, C and Go writers are renamed, not written as they come.
relations:
- decomposes: epic:adapters-untrusted-input
revision: 2
---
# Story: A writer never gives two names of a document one name in its output

## Outcome

Two different names of a `cir/1` document stay two different names in the written Rust, C and
Go, and a name the target language reserves is renamed instead of refused.

## Why

Found by `review-result:adversary-writers-validate-names-pass-1` and
`review-result:security-writers-validate-names-pass-1`; each existed before that unit.

- Rust writer: locals `fooBar` and `foo_bar` both become `foo_bar` (`adapters/rust/src/write.rs:773`);
  a body without a reassignment compiles and computes another value (F9).
- C writer: a local `int` is written `int_` and collides with a local `int_`
  (`adapters/c/src/write.rs:181`, F10); a module named `codespec_rt` or `cs` collides with the
  runtime's file and names (`adapters/c/src/write.rs:415`, F13, N4).
- Go writer: a Java parameter named `type` or `map` is refused instead of renamed
  (`adapters/go/cmd/codespec-write-go/write.go:623`, F7).
- Rust writer: rustc folds identifiers to NFC, so U+212B and U+00C5 become one identifier
  (`adapters/rust/src/write.rs:543`, N3).
- Go and C comments keep bidi control characters (N5).

- Go writer: a constant or module variable is written with a name a local can also take, so the
  local shadows it: `static mut TOTAL` with `let total` gives Rust 5,5 and Go 10,0
  (`adapters/go/cmd/codespec-write-go/write.go:1005`;
  `review-result:adversary-reader-rust-kit-pass-1`, F9).

## Acceptance

- The red cases `two_locals_differing_only_in_case_style_stay_distinct`,
  `a_local_named_like_a_c_keyword_beside_one_named_with_its_suffix_builds`,
  `a_module_named_like_the_runtime_header_keeps_its_own_header`,
  `a_module_prefix_never_gives_a_runtime_name` and `TestAParameterNamedLikeAGoKeywordIsWritten`
  pass.
- Each writer keeps one table of the names it has given in a scope and refuses or renames a
  second use; a doc comment holding a bidi control character is written with it escaped.

## Scope

- Own: `adapters/rust/src/write.rs`, `adapters/c/src/write.rs`,
  `adapters/go/cmd/codespec-write-go`. Inferred.
