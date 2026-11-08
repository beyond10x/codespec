---
format: aep.planning-md/3
id: review-result:whole-repo-parallel-safety-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic:whole-repo-to-syscalls
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

What I read: 16 stories that decompose `epic:whole-repo-to-syscalls`, plus the epic. I ran `aep plan artifact list --format json`, `aep plan artifact show` on each story, `aep plan artifact waves` (text and JSON) and `aep plan artifact relations`. I also ran `git grep` and `ls` in the tree to check which cited paths exist.

The store changed while I read it. `foreign-calls` and `reachability` went to revision 3, `call-trace-view` and `link-modules` to 4, and others also moved. Three problems I found on the first pass were fixed by those revisions, and the verdict covers the state after them (about 02:57 local):
- `story:foreign-calls` now depends on `story:cir1-interfaces` and `story:cir1-func-values`.
- `story:cir1-arrays-floats` now says `unsafe.Pointer` and `uintptr` stay foreign.
- `story:call-trace-view` now depends on `story:reachability` and `story:library-syscall-map`.

Placement and collisions:
- **Surfaces:** all 16 stories cite their files. None is inferred or unplaceable.
- **Shared files:** the 8 `story:cir1-*` stories share an identical 20-file core, and `foreign-calls`, `link-modules`, `syscall-barrier`, `dispatch-over-set`, `reachability` and `call-trace-view` overlap them on `crates/effects/`, `crates/model/src/lib.rs` and the Go reader directory. These collisions are recorded in each story's `scope`, and `waves` puts all 16 in separate waves (1, 3, 4, 6–10, 12–19), with no two sharing a wave. That is why I did not file them as unnamed collisions.
- **Order:** the order among the colliding stories comes from `waves`' tie-break, not from `depends_on` edges. That is for a person to decide.
- **Files that do not exist yet:** the library example directory, `crates/cli/tests/reach.rs`, `crates/effects/tests/reach.rs` and `crates/effects/tests/trace.rs` are not in the tree. Distinct stories create them, except the library example directory, where `story:library-syscall-map` and `story:call-trace-view` both put files and an edge orders them.

Acceptance reads traced: 32 reads trace to a producer in the set, and a `depends_on` edge records all 32. Four of the eight `read-repo.sh` reads reach `story:go-reader-whole-package` only through `story:cir1-extern-types`.

What I could not establish:
- Whether `story:syscall-barrier`'s acceptance (`os.ReadFile` reporting `openat` and `read`) needs reader features from `story:cir1-module-vars`, `story:cir1-embedded-fields`, `story:cir1-variadic`, `story:cir1-arrays-floats` or `story:cir1-generics`. It has no edge to them. I can't tell without running the reader on `os`, `internal/poll` and `syscall`. The foreign-statement call lists and opaque functions of `story:foreign-calls` look designed to make this independent. This is inferred, so I filed nothing.
- Out of my lane, for the acceptance critic: `story:call-trace-view` has its docs test read the library example's trace file. The acceptance does not say how the linked documents of the measured library, which `story:library-syscall-map` does not check in, are available to produce it.
- Out of my lane, for the design critic: 8 stories share one 20-file core and so run strictly one after another. Whether they should be fewer or ordered by explicit edges is a design question.
- `waves` also lists collisions with stories outside this set (`Taskfile.yml`, `AGENTS.md`, `README.md`, `scripts/read-repo.sh`). I did not assess those.

```findings
[]
```
