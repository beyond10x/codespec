---
format: aep.planning-md/3
id: review-result:adversary-series-rust-to-go-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:series-rust-to-go'
relations:
- reviews: story:series-rust-to-go
revision: 1
---
unit: story:series-rust-to-go, uncommitted working tree on `impl/series-rust-to-go` (base 034277e)
verdict: nothing found
cases: executed 20→20, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (`<session scratch>/wave-sbw/series-rust-to-go/adversary/`)
needs-coordinator: the checked-in `out/go-from-rust/series.go` reproduces only with `go-writer-len-conversion.patch` applied to `adapters/go`, which story java-example-ledger owns this wave

**1. Diff stat.** These are all the implementor's changes:
- `crates/docs/src/series.rs` +106
- `examples/series/harness.toml` +5
- `website/docs/examples/series.md` +358
- 4 untracked paths

**2. Cases added:** none. Every probe came back green.

**3. Mutation probe.** I made five mutants of `out/go-from-rust/series.go` and ran the harness on each, 200 seeds × 300 commands. It exited 1 on all five:
- an error text
- a 32-bit wrap
- `<` changed to `<=`
- a missing residue case
- a length off by one

**4. Findings:** none.

**5. Attacked and could not break**
- The harness detects a divergence.
- The driver covers the same 16 commands as the original driver.
- Re-reading the source gives the checked-in document byte for byte, and it validates as canonical.
- The page excerpts match the files.
- `docs generate --check` catches any change to the Go-from-Rust output.
- The int64 API: the overflow paths are reached.

```findings
[]
```
