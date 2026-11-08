---
format: aep.planning-md/3
id: review-result:whole-repo-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, epic:whole-repo-to-syscalls
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
approve

I checked the two round-1 findings against the revised bodies, and both are fixed.
- **Position of each extern or syscall call (§ 5c, `docs/static-analysis.md:191`):** `story:syscall-barrier` now claims it in its outcome ("each effect and syscall carries the positions of the calls in that function that lead to it"). Its `crates/cli/tests/barrier.rs` acceptance checks that the `openat` entry carries the position of the `os.ReadFile` call.
- **`uintptr` and `unsafe.Pointer` ownership:** `story:cir1-arrays-floats` now says they stay outside `cir/1` and that a statement using them stays `foreign`, with its calls listed by `story:foreign-calls`. Its acceptance has a `uintptr` test. `story:foreign-calls` covers `unsafe.Pointer` in its acceptance, so the outcome has an owner.

**What you read**
- 17 artifacts: `epic:whole-repo-to-syscalls`, all 16 stories (whole bodies) and `review-result:whole-repo-scope-r1`.
- Commands: `aep plan artifact list --format json`, `aep plan artifact graph`, `aep plan artifact show` on each of them, and `b10x skill aep`.
- I also read `docs/static-analysis.md` § 5 to § 5c.
- I extracted 20 promises from the epic and § 5b/5c: 12 general ones plus the 8 § 5a rows counted separately. I traced all 20 to a story.
  - Every function of the measured library gets effects, syscalls and `unknown` gaps: `story:library-syscall-map`.
  - The 6 + 228 packages are read and linked: `story:link-modules`, whose acceptance reads all 234.
  - The 8 § 5a rows: `story:cir1-extern-types`, `story:cir1-interfaces`, `story:cir1-func-values` (including newtypes over func, map and slice), `story:cir1-embedded-fields`, `story:cir1-variadic`, `story:cir1-module-vars`, `story:cir1-generics` and `story:cir1-arrays-floats`.
  - Unreached functions: `story:reachability`.
  - The call tree with branches: `story:call-trace-view`.
  - The report is checked in: `story:library-syscall-map`.
  - Every story implemented and `task check` passing: structural.
  - Calls inside foreign statements and refused functions: `story:foreign-calls`.
  - Dispatch: `story:dispatch-over-set`.
  - The barrier, `go:linkname` and the syscall table: `story:syscall-barrier`.
  - Syscalls reached from an entry function: `story:syscall-barrier` and `story:library-syscall-map`.
  - Side-effect position: `story:syscall-barrier`.
- Reach, duplicates and narrowing:
  - No story reaches beyond the epic's exclusions. The Rust, Java and Groovy adapters only list the new features under `not` or refuse them, which the Out of Scope section allows.
  - No two stories claim one outcome.
  - No promise is silently narrowed.

**What you could not establish**
- Out of my lane, for the acceptance critic: the build-constraint file selection in § 5b ("select files by build constraints (`GOOS`, `GOARCH`)", `docs/static-analysis.md` § 5) is described in the context of `story:go-reader-whole-package`. Its acceptance has no case with an other-platform file, so no test would catch a wrong selection. This does not set my verdict.
- Out of my lane, for the design critic: round 1's note that `story:call-trace-view` did not depend on `story:library-syscall-map` is resolved, and `story:go-reader-whole-package` now states that the 228 imported packages are read by `story:link-modules`.

```findings
[]
```
