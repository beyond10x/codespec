---
format: aep.planning-md/3
id: review-result:reach-scope-round-2
kind: review-result
status: active
title: Scope critic on the reach epic, round 2
relations:
- reviews: epic:sound-reach
- reviews: story:reach-three-valued
- reviews: story:reach-entry-file
- reviews: story:reader-indirect-call-sites
- reviews: story:reader-indirect-sites-rust
- reviews: story:reader-indirect-sites-go
- reviews: story:reader-indirect-sites-jvm
- reviews: story:reach-at-positions
- reviews: story:reach-registered-candidates
- reviews: story:reach-framework-entries
revision: 2
---
needs-revision

- story:reader-indirect-call-sites — the story promises "a rule per source language that is at least as wide as what that language lets a program call there", yet gives none for the Go `reflection` kind that `story:reader-indirect-sites-go` emits, and none for the Groovy `closure` kind. The Go rule names only `interface` and `fn-pointer`. The Java and Groovy rule is "every method of that name and parameter count", but a closure held in a field has no method name to match. Add a Go `reflection` rule (every function) and a Groovy `closure` rule — .engineering/planning/story/reader-indirect-call-sites.md:63
- story:reach-registered-candidates — R5 asks for narrowing by "implement a given trait / are registered in a given table". The story narrows C stores and the Rust `trait-object`, and the epic's Done When names only "the C and Rust `unknown`". The Go and Java `interface` sites and the Go, Java and Groovy `closure` and `fn-pointer` sites are never mentioned, and nothing says they keep the phase-1 width. Either claim that narrowing or state the exclusion in this story and in the epic's Out of Scope — .engineering/planning/story/reach-registered-candidates.md:34

All four round-1 findings are resolved.
- The per-language example promise is now claimed by the Rust, Go and JVM stories, each with an `examples/reach-*` entry. Its phase-2 half survives only as the narrowing finding above.
- The GORM sentence is gone from `story:reach-framework-entries`. `story:reader-indirect-sites-jvm` alone claims the dynamic finder.
- C `callback` is now emitted by the C reader (reader-indirect-call-sites.md:104-107).
- `inferred_edges` is now stated for the report and for `--at` records (reach-registered-candidates.md:36-41).

What I read: the epic, nine stories and `review-result:reach-scope-round-1` in full through `aep plan artifact show` (the two review/decomposition outputs were also in the persisted tool-result file), plus `~/.cache/scratch/cdab1af5-5768-4ebb-b33d-b77497bd35c1/reach-requirements.md`. I ran `aep plan artifact graph`, but only its first 80 lines were printed, so I did not check for older artifacts that claim part of the epic. I extracted 26 promises from R1 to R7 and the phasing; 24 trace to a story or to the epic's "What exists", and 2 trace partly, as the findings above say. I found no reach beyond the parent. Everything drafted is traceable, including `--list-entries`, `--spans`, `approximate`, the `kind:` entry lines and the ground-truth run gates.

What I could not establish:
- Out of my lane, for the design and parallel-safety critics:
  - `story:reach-registered-candidates` expects the Rust method to flip from `unknown` to `reached` in `examples/reach-rust`, which that story's typed scope does not list.
  - `crates/cli/src/main.rs` and `lib.rs` are shared across several stories.
- Unease, not a finding: `story:reach-three-valued` withdraws `codespec-reach-report/1` ("no longer"). The requirements and the epic ask for a new status shape but never ask for the old format to go.
- I could not tell whether R6's `via_indirect:bool` needs its own field, since `status: unknown` plus `via_unknown` in the `--at` records seems to carry it.

```findings
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 63
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the per-language matching rule promised as 'at least as wide as what that language lets a program call there' gives no rule for the Go reflection kind that story:reader-indirect-sites-go emits, nor for the Groovy closure kind (the Java and Groovy rule matches by method name, which a closure in a field lacks), so a promise of the story is silently narrowed"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 34
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "R5's narrowing by implemented trait or registration is claimed only for C stores and the Rust trait-object, and the epic's Done When names only the C and Rust unknown; Go and Java interface sites and closure and fn-pointer sites are neither narrowed nor stated to keep the phase-1 width, so the R5 promise is narrowed with nothing recording the decision"
```
