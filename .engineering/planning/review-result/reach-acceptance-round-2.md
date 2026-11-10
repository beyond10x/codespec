---
format: aep.planning-md/3
id: review-result:reach-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic on the reach epic, round 2
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
revision: 1
---
needs-revision

All eleven round-1 findings are resolved. Revised bodies carry seven new findings, all warnings. The blocker from round 1 (`reach-entry-file` report shape) is gone.

Round-1 check:
- **`reach-entry-file`:** `source` is now "file and line (`--entry` for a flag)". The acceptance compares only `status`, `distance` and `nearest_entry` with the flags. The annotation entry form is dropped.
- **`reach-three-valued`:** `--format 1` is gone. The first line is `codespec-reach-report/2`, and there is a bullet for each unresolved-site kind, both spread rules and `value`.
- **`reader-indirect-call-sites`:** `cir/1` stays `cir/1`, and a `design.md` bullet records it. The four C kinds are enumerated. The conformance check is a command, `git diff --exit-code website/data/conformance/reader-*.json`.
- **`reach-at-positions`:** `not read` is in the Outcome and comes from the module's refusal list (`crates/model/src/lib.rs:1486`, `Refusal { pos, what, reason }`). Two definitions on a line and the closing-brace case now have bullets.
- **`reach-registered-candidates`:** the callback, trait-object and keep-everything cases have bullets, and the stored-nowhere function is added to `examples/reach-c`.
- **`reach-framework-entries`:** the GORM finder moved to the jvm story, and there is now a filter in the fixture and a `--list-entries` expected file.

New findings:

story:reader-indirect-call-sites — the acceptance names `task schema-check`, which does not exist: `Taskfile.yml` has only `schema` (regenerate, line 267), `git grep schema-check` finds only this body, and the acceptance does not say the task is added — .engineering/planning/story/reader-indirect-call-sites.md:84
story:reader-indirect-call-sites — the matching rules give Go no rule for `reflection` and Groovy no rule for `closure`, although `story:reader-indirect-sites-go` expects a function called only by `reflect` to be `unknown` — .engineering/planning/story/reader-indirect-call-sites.md:65
story:reader-indirect-call-sites — the only matching-rule bullet is the C one (`on documents marked as C`), so the Rust, Go and Java/Groovy rules in the Outcome can be unbuilt and the story still closes — .engineering/planning/story/reader-indirect-call-sites.md:62
story:reader-indirect-sites-jvm — the Outcome does not say which `kind` an `invokeMethod` site gets, and "one checked-in source file per kind … Groovy: 4" counts forms (`invokeMethod`, computed name, finder, closure) rather than kinds, so the expected `indirect` entries cannot be fixed — .engineering/planning/story/reader-indirect-sites-jvm.md:39
story:reader-indirect-sites-jvm — the ground-truth bullet does not say which of the two entry lists each expectation is checked under, so "`unreachable` (the example has no `reflection` site in reached code for this case: a second entry list is used)" cannot be run the same way twice — .engineering/planning/story/reader-indirect-sites-jvm.md:44
story:reach-at-positions — the Outcome says `not read` is "a list of one entry" but also "given beside the function's status when a function holds the line", and the acceptance's "in the same list, the holding function's status" needs two entries, so the list shape is contradictory — .engineering/planning/story/reach-at-positions.md:37
story:reach-at-positions — the Outcome's "a line before the first function of its file … is `no function`" has no acceptance bullet; the first bullet covers only a file no document names — .engineering/planning/story/reach-at-positions.md:47
story:reach-registered-candidates — the `trait-object` bullet needs a type made only in an `unreachable` function and one made only in an `unknown` function, which `examples/reach-rust` as `story:reader-indirect-sites-rust` specifies does not hold. This story does not extend it, unlike the C one ("extended here"), and does not list it in scope — .engineering/planning/story/reach-registered-candidates.md:62
story:reach-registered-candidates — `via` naming `inferred` and the narrowing for a `through` that is a module variable (a `through` form in `story:reader-indirect-call-sites`) have no bullet; stores are listed only for a struct member and a parameter — .engineering/planning/story/reach-registered-candidates.md:39
story:reach-framework-entries — "names those five and nothing else" does not fix the count, because an interceptor gives `before` and `after` and the URL-mapping target may or may not be one of the two actions. The bullet "a service method it calls has 1" has no service in the fixture list — .engineering/planning/story/reach-framework-entries.md:41
story:reach-framework-entries — `--list-entries` prints "one per line with its kind", a form `codespec-entries/1` in `story:reach-entry-file` does not have (name, pattern or `kind: <kind>`), so the printed file cannot be passed back with `--entries` as bullet 2 requires — .engineering/planning/story/reach-framework-entries.md:34
story:reach-entry-file — "runs each against its language's example set" names no example set and no expected result: the example sets for C, Rust and Java are created by later stories that depend on this one, and "runs" does not say what passes — .engineering/planning/story/reach-entry-file.md:52

That is twelve finding lines, not seven: I counted the new findings by artifact (six artifacts across five stories) and wrote them out one per defect.

What I read: 10 artifacts (`epic:sound-reach`, nine stories, plus `review-result:reach-acceptance-round-1`), using `aep plan artifact show` on each. I checked `Taskfile.yml`, `crates/model/src/lib.rs`, `website/data/conformance/` and `examples/` in the tree.

What I could not establish:
- Whether "a stated distance from the entry to a named function" in the Rust, Go and JVM ground-truth bullets is asserted anywhere. The C story gives "distance of 3". I treated the others as checkable because the example is checked in with its entry list, but "stated" names no place where the number is written.
- Whether "a `params` type the module does not declare" (`reader-indirect-call-sites.md:84`) fits C primitives such as `int`.
- Out of my lane: the Rust, Go and Java/Groovy readers' matching rules sit in the C reader story, and the `reach-registered-candidates` Rust example is outside its typed scope. Both are scope or design matters, not acceptance.

```findings
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 84
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names `task schema-check`, which does not exist (Taskfile.yml has only `schema`), and does not say the task is added"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 65
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the matching rules give Go no rule for `reflection` and Groovy none for `closure`, although story:reader-indirect-sites-go expects a function called only by `reflect` to be `unknown`"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the only matching-rule bullet is the C one, so the Rust, Go and Java/Groovy rules in the Outcome have no acceptance and can be unbuilt while the story closes"
- file: .engineering/planning/story/reader-indirect-sites-jvm.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome does not say which kind an `invokeMethod` site gets, and 'one source file per kind ... Groovy: 4' counts forms rather than kinds, so the expected `indirect` entries cannot be fixed"
- file: .engineering/planning/story/reader-indirect-sites-jvm.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the ground-truth bullet does not say which of the two entry lists each expectation is checked under, so the `unreachable` case with 'a second entry list is used' cannot be run the same way twice"
- file: .engineering/planning/story/reach-at-positions.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome gives `not read` as a list of one entry but also beside the function's status, and the acceptance expects both in the same list, so the list shape is contradictory"
- file: .engineering/planning/story/reach-at-positions.md
  line: 47
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome's answer `no function` for a line before the first function of its file has no acceptance bullet; the first bullet covers only a file no document names"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the trait-object bullet needs a type made only in an unreachable function and one made only in an unknown function, which examples/reach-rust as story:reader-indirect-sites-rust specifies does not hold, and this story neither extends it nor lists it in scope"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`via` naming `inferred` and the narrowing for a `through` that is a module variable have no acceptance bullet, and stores are listed only for a struct member and a parameter"
- file: .engineering/planning/story/reach-framework-entries.md
  line: 41
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'names those five and nothing else' does not fix the count (an interceptor gives before and after, and the mapping target may be one of the actions), and the bullet needing a service method has no service in the fixture list"
- file: .engineering/planning/story/reach-framework-entries.md
  line: 34
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`--list-entries` prints one function per line with its kind, a form codespec-entries/1 in story:reach-entry-file does not have, so the printed file cannot be passed back with --entries as the second bullet requires"
- file: .engineering/planning/story/reach-entry-file.md
  line: 52
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'runs each against its language's example set' names no example set and no expected result, and the C, Rust and Java example sets come from later stories that depend on this one"
```
