---
format: aep.planning-md/3
id: story:review-query-facts-and-cache
kind: story
status: implemented
title: 'Query facts: reviewer-controlled fact cache and one var per lexical binding'
summary: codespec query reads no fact cache the inputs ship, and dataflow facts do not cross a lexical shadow (review findings Q1, Q8)
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/tests/query.rs
- confidence: cited
  path: crates/cli/tests/query_adversary1.rs
- confidence: cited
  path: crates/cli/tests/query_adversary2.rs
- confidence: cited
  path: crates/query/src/facts.rs
- confidence: cited
  path: crates/query/src/lib.rs
- confidence: cited
  path: crates/query/tests/expected/schema.txt
- confidence: cited
  path: crates/query/tests/paths.rs
- confidence: cited
  path: crates/query/tests/query.rs
- confidence: cited
  path: docs/query-language.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:45:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T03:45:11Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T03:59:17Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: query facts and cache

## Outcome

A reviewer running `codespec query` over a repository gets answers derived from the validated
input documents: a fact cache the repository ships beside its documents cannot change them, and
dataflow facts give each lexical binding its own var, so a declaration in a nested block or a
function literal does not flow to a use of the binding it shadows.

## Findings

- Q1 (security): the fact cache was read from `<dir of the first document>/.cq/<digest>.facts`
  before the documents were validated. It now lives in the reviewer's cache directory
  (`$CODESPEC_CACHE_DIR`, else `$XDG_CACHE_HOME/codespec/facts`, else `~/.cache/codespec/facts`),
  a cache directory in or under an input's directory is not used, the documents are validated
  first, and a cache file is read only when it names the digest of the documents.
  Tests: `crates/query/tests/query.rs::a_cache_planted_beside_the_input_does_not_change_the_answer`,
  `crates/query/tests/query.rs::a_cache_file_built_from_other_documents_is_not_read`,
  `crates/query/tests/query.rs::the_documents_are_validated_before_the_cache_is_read`.
- Q8 (logic): every binding of a name in a body shared the var `<body>$<name>`, so `x := 9` in a
  branch flowed to `return x` of the parameter after it. Bindings now follow the evaluator's
  lexical blocks; a later binding of a name in the same body takes `<body>$<name>@<node>`.
  Test: `crates/query/tests/query.rs::dataflow_does_not_cross_a_lexical_shadow`.

## Done When

- A planted `.cq/<digest>.facts` with false facts beside the input does not change the answer,
  also when that directory is named as the cache.
- A cache file naming another digest, or none, is rebuilt, not read.
- An invalid document is refused even when the cache holds facts under its digest.
- `flows` has no row from a shadowing declaration to a use of the shadowed binding.
- `cargo test -p b10x-codespec-query` and the `codespec query` tests of `b10x-codespec-cli` pass.
