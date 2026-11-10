---
format: aep.planning-md/3
id: review-result:reach-design-round-1
kind: review-result
status: active
title: Design critic on the reach epic, round 1
relations:
- reviews: epic:sound-reach
- reviews: story:reach-three-valued
- reviews: story:reader-indirect-call-sites
- reviews: story:reach-entry-file
- reviews: story:reach-at-positions
- reviews: story:reach-registered-candidates
- reviews: story:reach-framework-entries
revision: 1
---
needs-revision

The set has no cycles and no serialising chain. Three findings are blockers because the invariant is not secured by the split as drafted.

- **story 1 (`reach-three-valued`):** it says nothing about how `unknown` code interacts with the existing `--dispatch rta`.
- **story 2 (`reader-indirect-call-sites`):** its matching rule is not required to be conservative, and nothing tests it against ground truth.
- **story 5 (`reach-registered-candidates`):** its input is produced by nobody in the set.

Findings:

- story:reach-three-valued — the invariant is stated only for resolved edges: `reach` defaults to `rta` when entries are given, where an interface or value candidate counts only once reached code makes its type or uses the value, and `docs/static-analysis.md` already says values made where the set cannot see are missed. The body does not say that code in `unknown` functions counts as making types and using values, so a method reachable only through a type made in an `unknown` function can still come out `unreachable`. The property test models only call-graph edges, so it cannot catch this. — .engineering/planning/story/reach-three-valued.md:22-24,37-38; docs/static-analysis.md:209; crates/effects/src/set.rs:988
- story:reader-indirect-call-sites — it narrows a site to "functions of that parameter count and those types" with no rule that the match is conservative, and no gate at the story that introduces narrowing. A C function pointer cast to another type, a `void*` callback, Rust generics and Java erasure or subtyping all call functions whose declared types differ from the site's. The only acceptance case is an exact-type mismatch. The story-1 property test treats unresolved sites as calling every function, so it cannot check typed sites. The body should state the per-language compatibility the match must be at least as wide as, and a ground-truth interpreter over typed sites should be an acceptance line here. — .engineering/planning/story/reader-indirect-call-sites.md:24-26,36-40; .engineering/planning/story/reach-three-valued.md:37
- story:reach-registered-candidates — its input has no producer. It needs the struct member, array or parameter each site calls through, but `indirect` carries only `kind` and parameter and result types (story 2, line 19). The scope is `crates/effects` alone, so neither the model nor the readers gain that field and the story cannot be demonstrated. Either story 2's `indirect` gains a `through` slot, or this story owns the model and reader change. This is a split abstraction with the seam on the slot. — .engineering/planning/story/reach-registered-candidates.md:16,33; .engineering/planning/story/reader-indirect-call-sites.md:17-20
- story:reach-registered-candidates — the "cannot all be read" fallback covers only a member written inside a `foreign` statement. It leaves out a function stored by code outside the set, passed to an extern that retains it, written through an alias or `memcpy`, or copied from another struct. Those cases narrow to `unreachable` when they should stay `unknown`, and the acceptance tests only the `foreign` case. The body should list the escape routes that keep the full fitting set. — .engineering/planning/story/reach-registered-candidates.md:19-21,26-28
- story:reach-registered-candidates — for `trait-object` it takes "the types the reached code makes", so a type made only in an `unknown` function is dropped. The story-1 body needs the same rule as the first finding, and this body should point at it. — .engineering/planning/story/reach-registered-candidates.md:18
- story:reach-entry-file — "every function carrying a named annotation or attribute the reader recorded" reads state no reader records. `Func` has no annotation field (`crates/model/src/lib.rs:1101`), and `crates/model` is not in this story's scope. The annotation-entry acceptance (line 26) can pass only after a model and reader change that no item in the set owns. The existing `Module.entries` records framework-started functions, but only the kinds `http`, `scheduled` and `incoming`, and neither story mentions it. Drop the annotation form, or add the producer and an edge to it. — .engineering/planning/story/reach-entry-file.md:17-18,26,35; schemas/cir-1.schema.json:473-475
- story:reach-framework-entries — it defines a second source of framework entries, a `codespec-entries/1` file written by `--entries-from`, beside `Module.entries` (the entry list `cir/1` already has). It does not say which one wins. The scope is `adapters/groovy`, `crates/cli` and `examples/`, so a Grails controller action has no field in `cir/1` to carry it. The body should state the relation to `Module.entries` and name the model change. — .engineering/planning/story/reach-framework-entries.md:17-19,30; schemas/cir-1.schema.json:473-475
- story:reach-framework-entries — "A call to a GORM dynamic finder is an `indirect` site of kind `dynamic-name`" claims the same reader output as story 2's "call by a computed name" for the Groovy reader. Both items own the Groovy `dynamic-name` surface. Keep it in story 2 and have this story depend on it, or remove the sentence here. — .engineering/planning/story/reach-framework-entries.md:19-20; .engineering/planning/story/reader-indirect-call-sites.md:23
- epic:sound-reach — the outcome promises "for every `file:line` handed to it, one of three things", but story 4 answers `no function` and `not read` as well. The epic should say so, or story 4 should map those two answers onto the three statuses. — .engineering/planning/epic/sound-reach.md:16; .engineering/planning/story/reach-at-positions.md:19,26

What I read: 7 set artifacts, plus `story:reachability`, `story:dispatch-over-set`, `story:readers-bound-untrusted-input` and `story:reader-function-spans`. Commands: `aep plan artifact show` on each, `relations`, `graph` and `validate`, plus the model, schema, `crates/effects/src/set.rs` and `docs/static-analysis.md`. I walked about 30 declared edges, including those outside the set (to `epic:whole-repo-to-syscalls`, the `story:readers-bound-untrusted-input` dependencies, the `story:reachability` dependencies), and found no cycle. The only chain is 1 to 3 to 4, and its reason is written in the bodies as shared files, so by the rubric it is a trade-off and not a finding. I traced 10 acceptance reads to a producer. Seven are recorded by an edge. Two have no producer: the function annotation in story 3 and the call-through slot in story 5. One (story 4 reading `story:reader-function-spans`) is a conditional external dependency.

What I could not establish:
- **Parallel safety (out of my lane):** stories 2 and 3 both edit `crates/effects` and `CHANGELOG.md` with no edge between them. Story 2 is also said to run after the `epic:reader-coverage` reader stories, and no edge records that.
- **Out of my lane:** the "cir/1 optional field or cir/2" decision in story 2 is deferred to its first step. If it comes out as `cir/2`, stories 1, 3 and 5 are affected.
- **Unease only:** a GORM dynamic finder marked as a `dynamic-name` site that fits every function makes every non-reached function in a Grails application `unknown`. I did not check whether that defeats the epic's purpose.
- **Not judged:** whether the readers drop any call silently, outside `foreign` and `opaque`. The invariant is only as sound as the readers' completeness, and no story in the set audits it.

```findings
- file: .engineering/planning/story/reach-three-valued.md
  line: 22
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body does not say that code in unknown functions counts as making types and using values under the default rta dispatch, so a method reachable only through a type made in an unknown function can be unreachable, and the property test models only call-graph edges"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 24
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the signature match states no per-language compatibility rule (C casts, void* callbacks, generics, erasure, subtyping) that it must be at least as wide as, and no acceptance checks narrowing against a ground-truth interpreter over typed sites"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 16
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it needs the member, array or parameter each site calls through, which story:reader-indirect-call-sites does not put in indirect and this story's crates/effects-only scope cannot add; the slot field must go into story 2 or this story must own the model and reader change"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 19
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the keep-every-candidate fallback covers only a store inside a foreign statement and omits stores by code outside the set, functions retained by externs, aliased or memcpy writes and struct copies, which would narrow to unreachable"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 18
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "for trait-object it takes the types the reached code makes, which drops types made only in unknown functions and breaks the invariant; it should adopt the story-1 rule"
- file: .engineering/planning/story/reach-entry-file.md
  line: 17
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the annotation entry form reads a function annotation no reader records and cir/1 Func has no field for, crates/model is outside this story's scope and no item in the set adds it, so the annotation acceptance cannot pass"
- file: .engineering/planning/story/reach-framework-entries.md
  line: 19
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it defines a second framework-entry source (codespec-entries/1 written by --entries-from) beside cir/1 Module.entries without stating which wins, and its scope names no model change for a Grails controller action"
- file: .engineering/planning/story/reach-framework-entries.md
  line: 19
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the GORM dynamic finder as a dynamic-name site claims the Groovy reader output that story:reader-indirect-call-sites already owns for computed-name calls"
- file: .engineering/planning/epic/sound-reach.md
  line: 16
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome promises one of three statuses for every file:line, but story:reach-at-positions also answers no function and not read"
```
