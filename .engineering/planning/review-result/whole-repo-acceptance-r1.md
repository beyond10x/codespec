---
format: aep.planning-md/3
id: review-result:whole-repo-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, epic:whole-repo-to-syscalls
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
- supersedes: review-result:whole-repo-acceptance-round-1
revision: 1
---
needs-revision

Nine of the 16 stories have a defect in their acceptance. The other seven are clean: `story:cir1-extern-types`, `story:cir1-func-values`, `story:cir1-generics`, `story:cir1-interfaces`, `story:cir1-module-vars`, `story:cir1-variadic` and `story:link-modules`.

story:cir1-arrays-floats — the Outcome also makes `uintptr` and `unsafe.Pointer` extern types, but no acceptance bullet checks that. `type-array-float` and the model test only name `type.array`, `type.float` and `expr.float`, so that second outcome can fail while the story reads as done — .engineering/planning/story/cir1-arrays-floats.md:76

story:cir1-embedded-fields — the Outcome says the reader writes promoted accesses out as explicit paths, but the acceptance only checks that `decl.embedded` reads canonically and the case exists. Nothing observes a promoted access or method call becoming a field path — .engineering/planning/story/cir1-embedded-fields.md:76

story:dispatch-over-set — the Outcome's last sentence is "An `unknown` remains only when a candidate is itself incomplete". The test covers two complete implementations, so no check shows an incomplete candidate keeping `unknown`. It also doesn't show a non-matching signature being excluded — .engineering/planning/story/dispatch-over-set.md:39

story:foreign-calls — the Outcome says a refused function "stays in the document as an opaque function". The acceptance never reads a refused function through the Go reader to show it is now present with its calls. It only checks that `opaque` is canonical and that a name collision is refused — .engineering/planning/story/foreign-calls.md:48

story:go-reader-whole-package — the Outcome claims success on the measured library "and of every package it imports, standard library included". The acceptance's "`scripts/read-repo.sh` on the measured library exits 0 for all 6 packages" covers only the measured library's own packages. The 228 imports are unchecked, so the claim can fail while the story is closed — .engineering/planning/story/go-reader-whole-package.md:54

story:call-trace-view — the Outcome promises the `codespec trace --from` command writing `codespec-trace/1`, with dispatch nodes, repeat references, a depth cut, and a documentation page for an entry point of the measured library. The acceptance checks only envcheck `Announce` (an `if` branch with two leaves) and the envcheck page. The command and the library page are unchecked — .engineering/planning/story/call-trace-view.md:41

story:reachability — the only check on the command is "`crates/cli/tests/reach.rs`: deterministic output". It names no input and no expected content, so an empty `codespec-reach-report/1` passes. The Outcome's dispatch-candidate reach and the per-module list are also unchecked — .engineering/planning/story/reachability.md:41

story:syscall-barrier — the Outcome adds the `io` effect for `read`, `write` and `close`, and linkname resolution of body-less functions. The acceptance checks neither, only `openat` and `os.ReadFile` reporting `openat` and `read` — .engineering/planning/story/syscall-barrier.md:57

story:syscall-barrier — "Reading `os`, `syscall` and `internal/runtime/syscall/linux` from the toolchain's source: `os.ReadFile` reports `openat` and `read`" names no command or task that does the reading, so nobody can repeat it to get the same answer — .engineering/planning/story/syscall-barrier.md:59

story:library-syscall-map — "If they do not, the delivery notes say which `unknown` stops them" is satisfied whether or not the client and server entry points reach their syscalls. The bullet cannot fail, so it is a vote — .engineering/planning/story/library-syscall-map.md:51

**What I read:** 16 of 16 ids, via `aep plan artifact list --format json` filtered on `decomposes epic:whole-repo-to-syscalls`, `aep plan artifact show` on each, plus `kinds` and `lifecycle story`. In the tree I checked that `task docs-check` and `scripts/read-repo.sh` exist, and that 6 + 228 matches `docs/static-analysis.md:169`.

**What I could not establish:**
- Whether the conformance cases, `crates/effects/tests/trace.rs`, `tests/reach.rs` and the library example task exist. They are future work, and I did not look for them.
- Out of my lane (design critic):
  - `story:cir1-arrays-floats` names `story:syscall-barrier` in its Outcome but does not depend on it.
  - Several cir1 stories modify the same shared files.
  - `story:go-reader-whole-package` says the scripts are "moved from the session scratch directory", but `scripts/read-repo.sh` and `scripts/classify.mjs` are already in the worktree.
- I did not flag the multi-bullet acceptance lists as "more than one statement". They are the same shape in every story, each bullet is a separate check of one outcome, and flagging all 16 would not change what is drafted.

```findings
- file: .engineering/planning/story/cir1-arrays-floats.md
  line: 76
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome also makes `uintptr` and `unsafe.Pointer` extern types, but no acceptance bullet checks that, so the second outcome can fail while the story reads as done"
- file: .engineering/planning/story/cir1-embedded-fields.md
  line: 76
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says the reader writes promoted accesses out as explicit paths, but the acceptance only checks that `decl.embedded` reads canonically and the case exists, so no check observes promotion becoming a field path"
- file: .engineering/planning/story/dispatch-over-set.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says an `unknown` remains only when a candidate is itself incomplete, but the test uses two complete implementations, so neither an incomplete candidate keeping `unknown` nor a non-matching signature being excluded is checked"
- file: .engineering/planning/story/foreign-calls.md
  line: 48
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says a refused function stays in the document as an opaque function with its calls, but no acceptance reads a refused function through the Go reader to show it present with its calls; only `opaque` canonicality and a name collision are checked"
- file: .engineering/planning/story/go-reader-whole-package.md
  line: 54
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome claims success on every package the measured library imports, standard library included, but the acceptance 'exits 0 for all 6 packages' covers only the measured library's own packages and not the 228 imports"
- file: .engineering/planning/story/call-trace-view.md
  line: 41
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the `codespec trace --from` command writing `codespec-trace/1`, with dispatch nodes, repeat references, a depth cut and an entry-point page for the measured library, but the acceptance checks only envcheck `Announce` and the envcheck page"
- file: .engineering/planning/story/reachability.md
  line: 41
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the only check on the command is 'deterministic output', which names no input and no expected content, so an empty `codespec-reach-report/1` passes; dispatch-candidate reach and the per-module list are unchecked"
- file: .engineering/planning/story/syscall-barrier.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome adds the `io` effect for read, write and close and linkname resolution of body-less functions, but the acceptance checks neither, only `openat` and `os.ReadFile`'s syscalls"
- file: .engineering/planning/story/syscall-barrier.md
  line: 59
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'Reading `os`, `syscall` and `internal/runtime/syscall/linux` from the toolchain's source: `os.ReadFile` reports `openat` and `read`' names no command or task that does the reading, so nobody can repeat it to get the same answer"
- file: .engineering/planning/story/library-syscall-map.md
  line: 51
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'If they do not, the delivery notes say which `unknown` stops them' is satisfied whether or not the client and server entry points reach their syscalls, so the bullet cannot fail and is a vote"
```
