---
format: aep.planning-md/3
id: story:review-go-writer-init
kind: story
status: implemented
title: Go writer keeps every init function
summary: The Go writer writes each of a package's init functions, in source order (finding A1)
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/write.go
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/write_test.go
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:32:45Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T03:32:46Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:14:11Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A Go package with more than one `init` function, read into cir/1 and written back as Go, keeps every
`init` function in source order, so the written package initializes its globals as the original does.

## Findings

- A1 (Go output loses package initializers): the writer's declaration index held one builder per key,
  and every `init` function has the key `func:init`, so the last one replaced the others and only it
  was written. Each layout occurrence of a key now takes the next declaration of that key in model
  order. Test: `adapters/go/cmd/codespec-write-go/write_test.go::TestEveryInitFunctionIsWrittenInSourceOrder`
  (three `init` functions, `+1`, `*10`, `+2`: the written Go returns 12, and reading it back is a fixed
  point).

## Done When

- The test above passes and failed before the fix (`want 3 init functions, got 1`).
- `task adapters-check`, `task examples-go`, `task examples-greeter` and `task examples-series` pass.
