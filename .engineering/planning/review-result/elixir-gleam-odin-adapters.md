---
format: aep.planning-md/3
id: review-result:elixir-gleam-odin-adapters
kind: review-result
status: active
title: Native adapter round-trip review
relations:
- reviews: story:elixir-adapters
- reviews: story:gleam-adapters
- reviews: story:odin-adapters
revision: 1
---
The review traced names, control flow, signed integer bounds and quoted text through the native parsers, writers and runtimes. The regression cases live with the adapter they exercise.

```findings
[
{"file":"adapters/elixir/lib/reader.ex","line":395,"category":"correctness","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"A final signed bitstring binding returns the matched binary. Lifting it as a scalar return changes the result. Preserve the source with a refusal."},
{"file":"adapters/gleam/lib/reader.ex","line":284,"category":"correctness","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"The writer groups a comparison inside a one-expression block. Reading the generated three-branch function marks this comparison as foreign."},
{"file":"adapters/odin/src/write.rs","line":327,"category":"correctness","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"Interpreting the Go spelling 010 as a decimal integer changes its value. Check for canonical decimal input before emitting a numeric literal."}
]
```
