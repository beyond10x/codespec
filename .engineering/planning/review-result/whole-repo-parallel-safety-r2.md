---
format: aep.planning-md/3
id: review-result:whole-repo-parallel-safety-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic:whole-repo-to-syscalls
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

story:syscall-barrier — its acceptance needs the function-value dispatch that story:dispatch-over-set produces in `crates/effects/`, and no `depends_on` path runs from the reader to that producer (the missing edge is `story:syscall-barrier` → `story:dispatch-over-set`; the other fix is merging the two). The acceptance requires `os.ReadFile`'s `syscalls` to include `openat` and `read`. In Go 1.27.1 both syscalls sit behind function-value calls (cited, Go source). `os.ReadFile` passes `f.Read` as a value to `readFileContents` (`/usr/lib/go/src/os/file.go:878`). `poll.FD.Read` calls `ignoringEINTRIO(syscall.Read, …)`, which calls `fn(fd, p)` (`/usr/lib/go/src/internal/poll/fd_unix.go:166`, `:741`). The `os` open path wraps `open` in `ignoringEINTR` with a function literal (`/usr/lib/go/src/os/file_unix.go:46`, `:431`). Resolving those calls is the outcome of `story:dispatch-over-set` ("a call of a function value has the effects of every function, method value and lambda in the set…"). `story:syscall-barrier`'s edges reach `story:link-modules` and `story:foreign-calls` only, and neither leads to `story:dispatch-over-set`. `waves` places `story:dispatch-over-set` in wave 15 and `story:syscall-barrier` in wave 17, but only through the shared-scope tie-break on `crates/effects/`; the printed line is `collision: story:dispatch-over-set story:syscall-barrier crates/effects/`. Without an edge, the order is not guaranteed. If `story:syscall-barrier` lands first, its acceptance reads `unknown` where it expects `read`. — `.engineering/planning/story/syscall-barrier.md:64`, `.engineering/planning/story/dispatch-over-set.md:21`

What I read: the 16 stories that decompose `epic:whole-repo-to-syscalls` (the epic's own body I did not open), the `review-result:whole-repo-parallel-safety-r1` record, and the Go 1.27.1 standard library source. I used `aep plan artifact list --format json`, `aep plan artifact waves` (text and JSON), `cat -n` on every story body, `git grep` and `ls` for cited paths, and `grep` on `/usr/lib/go/src`.
- **Surfaces:** 16 cited, 0 inferred, 0 unplaceable.
- **Acceptance reads:** 33 traced to a producer in the set. 32 are recorded by a `depends_on` edge, direct or transitive, which is the count round 1 gave. The one unrecorded read is the finding above.
- **Store state:** the story files were last written at 02:53 to 02:56, before round 1's 02:58 review, so the set round 1 approved is the set I read. The finding is new, not a repeat.
- **Shared-scope collisions:** I did not file the pairwise collisions between the `story:cir1-*` stories or between `story:foreign-calls`, `story:link-modules` and `story:syscall-barrier`. They are recorded in each `scope`, and `waves` puts the pairs in different waves. That is the same call round 1 made. A person decides the order by tie-break or by adding edges.
- **Files not created yet:** `crates/cli/tests/reach.rs`, `crates/effects/tests/reach.rs`, `crates/effects/tests/trace.rs`, `crates/cli/tests/trace.rs`, `crates/cli/tests/barrier.rs` and the library example directory are absent from the tree. Each is created by one story. The exception is the library example directory, which `story:library-syscall-map` and `story:call-trace-view` both write, and an edge orders those two.

What I could not establish:
- Whether `story:cir1-module-vars` needs `story:cir1-extern-types` first. Its context says "`var ErrX = errors.New(...)` once externs carry it", but its acceptance only requires that no refusal names its own cause, so I filed nothing.
- Whether `story:syscall-barrier`'s acceptance also needs the reader features from `story:cir1-module-vars`, `story:cir1-embedded-fields`, `story:cir1-variadic`, `story:cir1-arrays-floats` and `story:cir1-generics`. `story:foreign-calls` opaque functions probably make it independent of them, and I cannot confirm without running the reader on `os`, `internal/poll` and `syscall`.
- Out of my lane, for the acceptance critic: how `story:call-trace-view` produces the library example's trace file when `story:library-syscall-map` does not check in the linked documents of the measured library.
- Out of my lane, for the design critic: eight `story:cir1-*` stories share an identical 20-file core, so they run strictly one after another.

```findings
- file: .engineering/planning/story/syscall-barrier.md
  line: 64
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires os.ReadFile's syscalls to include openat and read, which in Go 1.27.1 are reached only through function-value calls (os/file.go:878, internal/poll/fd_unix.go:166 and :741, os/file_unix.go:46) that story:dispatch-over-set resolves in the shared crates/effects/, and no depends_on path runs from story:syscall-barrier to story:dispatch-over-set, so waves orders them only by scope tie-break; add the edge or merge the two (cited)"
```
