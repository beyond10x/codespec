---
format: aep.planning-md/3
id: review-result:whole-repo-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, epic:whole-repo-to-syscalls
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

Findings:

epic:whole-repo-to-syscalls — "where does a side effect happen" (`the effect report with the position of each extern or syscall call`) is marked `epic` in § 5c, and the epic cites § 5c as its design, but no story claims reporting the position of each extern or syscall call. `story:syscall-barrier` or `story:library-syscall-map` would most naturally take it. — .engineering/planning/epic/whole-repo-to-syscalls.md:20 (promise quoted from docs/static-analysis.md:191)

story:cir1-arrays-floats — the outcome says `uintptr` and `unsafe.Pointer` "are extern types of the barrier (`story:syscall-barrier`)", but the story's model and acceptance do not cover them and `story:syscall-barrier` does not claim them, so nobody owns that outcome. — .engineering/planning/story/cir1-arrays-floats.md:59

**What you read**
- 17 artifacts: the epic and its 16 stories, all with whole bodies.
- Design doc: docs/static-analysis.md § 5 to § 5c.
- Commands: `aep plan artifact list --format json`, `aep plan artifact show` on the epic and each story, `aep plan artifact graph`.
- Other read-only commands: `b10x skill aep`, `sed`, `grep`.
- The epic file path was found with `find`.

**Promises**
- I extracted 8 promises from the epic body and § 5b/§ 5c; I traced 7 to a story and 1 to none.
- Every function of the measured library gets a report of syscalls and effects, with gaps listed as `unknown`: `story:library-syscall-map`, `story:syscall-barrier`, `story:link-modules`.
- The `cir/1` types of § 5a, all 8 rows: the `cir1-*` stories.
- Unreached functions: `story:reachability`.
- A call tree with branches: `story:call-trace-view`.
- The report is checked in: `story:library-syscall-map`.
- All stories implemented and `task check` passes: structural.
- The calls inside `foreign` statements and refused functions, and dispatch: `story:foreign-calls` and `story:dispatch-over-set`, covering § 5b.
- Side-effect position: no story. This is the finding above.

**Reach beyond the parent, duplicates, narrowing**
- Nothing in the set reaches beyond the epic's exclusions.
- No two stories claim the same outcome.
- No promise is silently narrowed. All eight § 5a rows are covered, including newtypes over map and slice types.

**What I could not establish**
- Out of my lane, for the design critic: `story:call-trace-view` draws "one entry point of the measured library" but does not depend on `story:library-syscall-map` or on the `cir1-*` stories that produce the documents of the measured library.
- Out of my lane, for the acceptance critic: `story:go-reader-whole-package` has an outcome of "every package it imports, standard library included", but its acceptance covers only the measured library's 6 packages.

```findings
- file: .engineering/planning/epic/whole-repo-to-syscalls.md
  line: 20
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"where does a side effect happen\" (docs/static-analysis.md:191: the effect report with the position of each extern or syscall call) is marked epic in § 5c, which the epic cites as its design, and no story claims reporting the position of each extern or syscall call; story:syscall-barrier or story:library-syscall-map would most naturally take it"
- file: .engineering/planning/story/cir1-arrays-floats.md
  line: 59
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome says uintptr and unsafe.Pointer are extern types of the barrier (story:syscall-barrier), but the story's model and acceptance do not cover them and story:syscall-barrier does not claim them, so nobody owns that outcome"
```
