---
format: aep.planning-md/3
id: review-result:reach-design-round-2
kind: review-result
status: active
title: Design critic on the reach epic, round 2
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

Round 1 findings: 7 of 9 are resolved and 2 are only partly resolved. Reaching the invariant still fails in five places. The graph has no cycle, and the stories' `depends_on` edges and the reads in their acceptance lines are consistent.

Round 1 check:
- **Fixed:** rta spread (`reach-three-valued` lines 47-51 and its acceptance), the `through` slot (`reader-indirect-call-sites` line 54), the annotation entry form (dropped, `reach-entry-file` line 22), the single source of framework entries (`reach-framework-entries`), the dynamic-finder double claim (now only in `reader-indirect-sites-jvm`), the `trait-object` rule in `reach-registered-candidates`, and the epic's three-versus-five answers.
- **Partly fixed:**
  - The matching rule and its ground-truth gate: both exist now, but F2, F3 and F7 below hold against them.
  - The keep-everything list: it grew, but F4, F5 and F6 below remain.

Findings:

- story:reach-three-valued — the definition of an unresolved site covers an undeclared extern only when a function value is passed to it. An in-set value passed to such an extern is not covered: an object whose methods the extern calls through an interface (`sort.Sort`, `http.Handle`, a Java `Comparator`) or a struct holding function pointers (a C `ops` table). The methods or functions stay `unreachable`. This breaks the invariant under `story:reach-registered-candidates` line 46's own "passed to an extern no document declares". The body should widen the site to any argument that carries a method set or function fields, or state which narrower scope the invariant has. — .engineering/planning/story/reach-three-valued.md:42-43
- story:reader-indirect-call-sites — the per-language rule is narrower than what the languages allow, so `unreachable` can be given for a callable function.
  - Rust and Go: a method used as a function value takes its receiver as the first argument (Rust `Type::method` as `fn(&T, X)`, Go method expression `T.M`), but `Func.recv` is separate from `params` and "same number of parameters" misses it.
  - Java and Groovy: "name and parameter count" misses varargs, Groovy default arguments, and a method reference whose name differs from the functional interface's.
  - Each rule should state its receiver, variadic and default-argument handling, and function values bound to a differently named interface method. — .engineering/planning/story/reader-indirect-call-sites.md:61-68; crates/model/src/lib.rs:1107-1111
- story:reader-indirect-call-sites — the body does not say what `indirect` entries on a `foreign` statement do to the "foreign statement without a call list fits every function" rule. If they are not exhaustive, the narrowed sites lose the unlisted calls of the same statement. If they replace the rule, nothing makes the reader promise completeness. The body should say that a statement with `indirect` and `calls` is unresolved only for its `indirect` entries, and that the reader asserts that by giving both. — .engineering/planning/story/reader-indirect-call-sites.md:47-49; .engineering/planning/story/reach-three-valued.md:42; crates/model/src/lib.rs:1246-1256
- story:reader-indirect-sites-go — the `interface` and `fn-pointer` sites it adds are calls the Go reader already lists in `foreign.calls` as `CallRef::Iface` and `CallRef::Value`, which dispatch resolves by signature. The body does not say whether such a call also gets an `indirect` entry, so one call can be both resolved and unresolved, owned by two mechanisms. It should say that `indirect` applies only to calls absent from `calls`. — .engineering/planning/story/reader-indirect-sites-go.md:28; crates/model/src/lib.rs:662-678; docs/static-analysis.md:208
- story:reach-registered-candidates — the keep-everything list does not cover a store whose value is not a direct function reference. A function assigned to a local variable, parameter or other member that is then stored (`fp = f; s.cb = fp`), or passed on through a wrapper's parameter, is found by no "stored there" rule and narrows to `unreachable`. The list needs a line: any store into the member or parameter whose value is not a function reference keeps every fitting function, with a case for it. — .engineering/planning/story/reach-registered-candidates.md:33-35,42-51
- story:reach-registered-candidates — the foreign-statement conditions have no producer, so the story cannot decide them. "Written inside a `foreign` statement", "address taken" and "passed to an extern" inside one are not readable from `Stmt::Foreign`, which carries `src`, `reason`, `pos` and `calls` only. The scope is `crates/effects` and `examples/reach-c`, with no reader or model change. The body should name the fact a reader records (a `writes` or `escapes` list), or keep every fitting function whenever a reached or `unknown` function holds a foreign statement without that fact. — .engineering/planning/story/reach-registered-candidates.md:44-47; crates/model/src/lib.rs:1246-1256
- story:reach-registered-candidates — `through` can also be a module variable or an interface or trait name, but narrowing is defined only for member, array initializer, parameter and the `trait-object` kind. A module variable, and a Go or Java `interface` site `through` an interface name, have no store rule and no keep line. Read literally they have no stores, so the candidate set is empty and everything is `unreachable`. The body should keep every fitting function for those forms, with a case each. — .engineering/planning/story/reach-registered-candidates.md:33-36; .engineering/planning/story/reader-indirect-call-sites.md:54-56
- story:reader-indirect-call-sites — it owns the Rust, Go and Java matching rules in `crates/effects`, but its only effects test is "on documents marked as C" (line 88). The Rust, Go and JVM gates sit in stories whose scope excludes `crates/effects`, so a rule error they expose can only be fixed outside their scope. The body should add one effects test per language here, or give the reader stories `crates/effects/src/set.rs`. — .engineering/planning/story/reader-indirect-call-sites.md:88-90; .engineering/planning/story/reader-indirect-sites-rust.md:34-36
- story:reach-at-positions — its edges to `reader-indirect-sites-rust`, `-go` and `-jvm` are justified as "to follow them in the chain on `crates/effects` and `crates/cli`", but those three stories edit neither crate (scopes are the adapters, `examples/` and `Taskfile.yml`). The edges serialise `--at` behind three reader stories for a shared surface that does not exist. The real chain is `reader-indirect-call-sites` to this story, so the three edges should go or name a real reason. — .engineering/planning/story/reach-at-positions.md:9-11,66-67
- epic:sound-reach — the open question says the Grails dynamic finders become `unknown` "until the second phase narrows them", but `story:reach-registered-candidates` narrows only a site with a `through`, and a `dynamic-name` site has none. The promised narrowing has no story. The epic should drop the sentence or name the story that owns it. — .engineering/planning/epic/sound-reach.md:93-94; .engineering/planning/story/reach-registered-candidates.md:51

What I read: 10 artifacts (the epic, the nine stories) plus `review-result:reach-design-round-1`. Commands: `aep plan artifact show` on each, `relations`, `graph`, `validate` (valid). I also read `crates/model/src/lib.rs`, `crates/effects/src/{lib,set}.rs` and `docs/static-analysis.md`. I walked 148 `depends_on` and `blocks` edges across the whole store, outside the set as well, and found no cycle. The chain is 1, 2, 3, then 4 to 6, then 7, 8, 9, and each edge has a written reason (see F8 for the one whose reason is false). I traced about 12 acceptance reads to a producer in the set. All are recorded by an edge, directly or transitively, and none is hidden.

What I could not establish:
- **Parallel safety (out of my lane):** `CHANGELOG.md` is in the scope of all nine stories and `Taskfile.yml` in four, and stories 4, 5 and 6 are meant to run beside each other.
- **Acceptance or feasibility (out of my lane):** whether a Grails filter closure or a URL-mapping target can be an `Entry`, which must name a function of `funcs` (`crates/model/src/lib.rs:141`).
- **Unease only:** the C rule with a variadic callee or a callee that takes fewer parameters than the site passes, and whether "same parameter count" counts a C `void` list.
- **Unease only:** the property test's reference interpreter makes every type at an unresolved site, while the story-1 body lets only `unknown` functions make types. They agree only because a site fits and so makes its callees `unknown`, which holds for `reflection` and `dynamic-name` but is not stated.

```findings
- file: .engineering/planning/story/reach-three-valued.md
  line: 42
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the unresolved-site definition covers an undeclared extern only when a function value is passed to it, so an in-set object whose methods the extern calls through an interface (sort.Sort, http.Handle) or a struct holding function pointers stays unreachable, breaking the invariant; widen the site to any argument carrying a method set or function fields, or state the narrower scope of the invariant"
- file: .engineering/planning/story/reader-indirect-call-sites
  line: 61
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the per-language rule is narrower than the language allows: it ignores the receiver as first argument (Rust Type::method as fn pointer, Go method expression), Java and Groovy varargs and default arguments, and method references whose name differs from the interface method; each rule must state how it treats them"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 47
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body does not say what indirect entries on a foreign statement do to the rule that a foreign statement without a call list fits every function, so unlisted calls in the same statement can be dropped; state that a statement with indirect and calls is unresolved only for its indirect entries and that the reader asserts that by giving both"
- file: .engineering/planning/story/reader-indirect-sites-go.md
  line: 28
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the interface and fn-pointer sites it adds are the calls the Go reader already lists in foreign.calls as CallRef::Iface and CallRef::Value and dispatch resolves, and the body does not say whether such a call also gets an indirect entry; say indirect applies only to calls absent from calls"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 42
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the keep-everything list omits a store whose value is not a direct function reference (a variable, a parameter passed on through a wrapper, another member), so a function reaching the member through one narrows to unreachable; add that line and a case for it"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 44
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the foreign-statement conditions (member written, address taken, struct passed to an extern) cannot be read from Stmt::Foreign, which holds only src, reason, pos and calls, and no story in the set makes a reader record them; name the recorded fact or keep every fitting function when a reached or unknown function holds a foreign statement without it"
- file: .engineering/planning/story/reach-registered-candidates.md
  line: 33
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "narrowing is defined for member, array initializer, parameter and the trait-object kind only, so a through naming a module variable or an interface name (Go or Java interface site) has no store rule and no keep line and reads as an empty candidate set; keep every fitting function for those forms, with a case each"
- file: .engineering/planning/story/reader-indirect-call-sites.md
  line: 88
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it owns the Rust, Go and Java matching rules in crates/effects but tests only the C rule, while the Rust, Go and JVM gates sit in stories whose scope excludes crates/effects; add one effects test per language here or give those stories crates/effects/src/set.rs"
- file: .engineering/planning/story/reach-at-positions.md
  line: 66
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its depends_on edges to the three reader-indirect-sites stories are justified as following them in the chain on crates/effects and crates/cli, which those stories do not edit, so they serialise --at behind three adapters for no shared surface; drop the edges or name a real reason"
- file: .engineering/planning/epic/sound-reach.md
  line: 93
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the open question says the second phase narrows Grails dynamic finders, but story:reach-registered-candidates narrows only a site with a through and a dynamic-name site has none, so no story owns that narrowing; drop the sentence or name the owner"
```
