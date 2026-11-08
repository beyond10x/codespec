---
format: aep.planning-md/3
id: review-result:whole-repo-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, epic:whole-repo-to-syscalls
relations:
- reviews: epic:whole-repo-to-syscalls
- reviews: story:go-reader-whole-package
- reviews: story:cir1-extern-types
- reviews: story:cir1-interfaces
- reviews: story:cir1-func-values
- reviews: story:cir1-embedded-fields
- reviews: story:cir1-module-vars
- reviews: story:cir1-variadic
- reviews: story:cir1-arrays-floats
- reviews: story:cir1-generics
- reviews: story:foreign-calls
- reviews: story:link-modules
- reviews: story:dispatch-over-set
- reviews: story:syscall-barrier
- reviews: story:library-syscall-map
- reviews: story:reachability
- reviews: story:call-trace-view
revision: 1
---
needs-revision

story:call-trace-view — the acceptance reads the library example's trace file as an input but names no command, task or story that produces it, so nobody can repeat it. The file is in this story's scope, and `story:library-syscall-map` writes only the library's effects report. The sentence is "the generated page holds the tree for envcheck and for the library's client entry point from the library example's trace file" — .engineering/planning/story/call-trace-view.md:52

story:call-trace-view — the Outcome says "each `if`, `switch` and loop a branch node with its condition … syscalls at the leaves", but the checks name only an `if` branch with `os.Getenv` and `println` leaves. `switch`, loop, the condition text and a syscall leaf are not observed anywhere — .engineering/planning/story/call-trace-view.md:47

**Round 1 outcome**
- All ten round-1 findings are fixed, and I did not repeat any of them.
- `story:cir1-arrays-floats`: now has a `uintptr` test and the Outcome keeps `uintptr` foreign.
- `story:cir1-embedded-fields`: now has the promoted-access test, `s.N` reads as `s.Inner.N`.
- `story:dispatch-over-set`: now checks both a non-matching signature and an incomplete candidate.
- `story:foreign-calls`: now reads a refused function into `opaque` with its call.
- `story:go-reader-whole-package`: the Outcome is narrowed to the measured library's 6 packages, and `story:link-modules` covers the 234.
- `story:reachability`: now has a named fixture with an exact expected output.
- `story:syscall-barrier`: the barrier test names `scripts/read-repo.sh --deps` on a fixture, and `io` and linkname each have a check.
- `story:library-syscall-map`: now has syscall assertions for the client and server entry points.

**What I read:** 16 of 16 ids, listed by `aep plan artifact list --format json` filtered on `decomposes epic:whole-repo-to-syscalls`, then `aep plan artifact show` on each, plus `aep plan artifact kinds`. I also read `review-result:whole-repo-acceptance-r1` and the epic body. In the tree I checked `scripts/read-repo.sh` and `git grep` for `client-do.trace` and `trace.json`.

**What I could not establish:**
- The second `story:call-trace-view` finding is new in this round. The Outcome's `switch`, loop and leaf wording did not change from round 1. I missed it then.
- The fixtures and tests the acceptances name (`crates/cli/tests/fixtures/readfile/`, the `trace.rs` and `reach.rs` tests, the library example directory) are future work and do not exist yet.
- Out of my lane (parallel-safety, design): `scripts/read-repo.sh` has no `--deps` flag yet. It is added by `story:link-modules`, which `story:syscall-barrier` depends on, so the order holds.

```findings
- file: .engineering/planning/story/call-trace-view.md
  line: 52
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads the library example's trace file as an input but names no command, task or story that produces it, so nobody can repeat the check; `story:library-syscall-map` writes only the library's effects report"
- file: .engineering/planning/story/call-trace-view.md
  line: 47
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises `if`, `switch` and loop branch nodes with their condition and syscalls at the leaves, but the checks observe only an `if` node with `os.Getenv` and `println` leaves, so `switch`, loop, condition text and a syscall leaf can fail while the story reads as done"
```
