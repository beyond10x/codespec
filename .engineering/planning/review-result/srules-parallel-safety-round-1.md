---
format: aep.planning-md/3
id: review-result:srules-parallel-safety-round-1
kind: review-result
status: active
title: Plan critic parallel-safety, round 1, epic security-rules-in-codespec
relations:
- reviews: epic:security-rules-in-codespec
- reviews: story:rules-layout
- reviews: story:rules-sanitization-gate
- reviews: story:rules-third-party-attribution
- reviews: story:rules-runner
- reviews: story:rules-import-mechanics
- reviews: story:rules-sanitize-and-fixtures
- reviews: story:rules-climb-lane
- reviews: story:rules-catalogue-docs
- reviews: story:rules-planning-merge
- reviews: story:rules-ui-crate
- reviews: story:codesec-retirement
revision: 1
---
needs-revision
story:rules-runner — collides with story:rules-sanitize-and-fixtures on `crates/rules/src`, `crates/rules/tests/main.rs` and `CHANGELOG.md` (cited, both Scope sections), no `depends_on` edge runs between them (the runner depends only on layout, sanitize only on import), and neither body names the other; remedies, unchosen: an ordering edge recording the shared files as its reason, or splitting the surface (the fixture refusal in its own module and test file) — .engineering/planning/story/rules-runner.md:77-82, .engineering/planning/story/rules-sanitize-and-fixtures.md:79-85
story:rules-ui-crate — collides with story:rules-climb-lane on `Cargo.toml`, `Cargo.lock`, `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md` and `CHANGELOG.md` (cited, both Scope sections), both depend on the runner and sanitize but not on each other, the epic's order puts 7 and 10 in one group ("7, 8 and 10 after 5"), and neither body says they share these files; remedies, unchosen: an ordering edge with the shared files as its reason, or splitting the surface (a workspace-members and gate-table change landed once ahead of both) — .engineering/planning/story/rules-ui-crate.md:66-70, .engineering/planning/story/rules-climb-lane.md:77-82
story:rules-climb-lane — adds the crate `crates/climb` but its Scope omits the generated `website/docs/reference/crates.md`, which `codespec-docs` rebuilds from `cargo metadata` and `docs-check` fails on when stale; `story:rules-ui-crate` adds a crate too and `story:rules-layout` lists the page, so two unlisted writers share it and `aep plan artifact waves` cannot see it (surface cited by `crates/docs/src/crates.rs:12`, the writer set inferred from "adds a crate"); remedy, unchosen: name the page in both scopes with an ordering edge, or move the regeneration to one later change — .engineering/planning/story/rules-climb-lane.md:77-82, crates/docs/src/crates.rs:12
story:rules-catalogue-docs — shares `AGENTS.md` (generated-files table) with story:rules-climb-lane and story:rules-ui-crate (cited, all three Scope sections), no edge orders them, and no body names the overlap; remedies, unchosen: an ordering edge, or splitting the surface so each story's table line is a separate change — .engineering/planning/story/rules-catalogue-docs.md:72-77, .engineering/planning/story/rules-climb-lane.md:80-81
story:rules-sanitization-gate — its "Shares" sentence names `crates/rules` and the test file only, but it also edits `docs/rules.md`, which `story:rules-third-party-attribution` (§ provenance) and `story:rules-runner` (§ running) edit too, with no edge between them (cited, three Scope sections; the sections differ, so the claim is a same-file textual collision); remedy, unchosen: say so in the body and order the three, or give each section its own file — .engineering/planning/story/rules-sanitization-gate.md:82-84, .engineering/planning/story/rules-third-party-attribution.md, .engineering/planning/story/rules-runner.md:79-82
story:rules-import-mechanics — its Done When reads state `story:rules-sanitize-and-fixtures` writes ("after the last import, `codespec rules list` counts the rules the seven pull requests list"; "`git log ... -- rules/` on `main` shows only the bot"), because the seven category pull requests land in that story, while the edge runs sanitize to import, so the reader cannot be checked before its producer and an edge the other way would be a cycle; move those two lines to `story:rules-sanitize-and-fixtures` (the missing edge cannot be added) — .engineering/planning/story/rules-import-mechanics.md:58-61, .engineering/planning/story/rules-sanitize-and-fixtures.md:36-37
story:rules-planning-merge — its Scope claims the whole directories `.engineering/planning/epic/` and `.engineering/planning/story/`, and says existing artifacts are edited "to add a relation" without naming which, so the surface is unassessed against any item that edits store files (for example the status of a story of this epic landing); name the existing artifacts it edits or narrow the scope to the new files (inferred: the relation targets are not cited) — .engineering/planning/story/rules-planning-merge.md:67-70

What I read: 12 artifacts (the epic and the 11 stories) with `aep plan artifact show` on each, `aep plan artifact waves --kind story` for the collision list and wave placement, `aep plan artifact list`, and `crates/docs/src/crates.rs` and `AGENTS.md` in the tree.
Surfaces established: 11 cited (each body's Scope section), 0 inferred apart from the findings marked above, 0 unplaceable; note that the store's `scope` field marks every path `inferred`, which makes `waves` count every shared path as a collision. Acceptance reads traced to a producer in the set: 22; recorded by a `depends_on` edge (directly or through a chain): 21; the one not recorded is the import story's (finding above).
What I could not establish: whether `epic:reader-self-improvement-loop`, which `story:rules-climb-lane` reads and which is not in this set or the store listing, is ordered against it (outside the set). Out of my lane: the epic's order says "2, 3, 6 and 9 beside each other" while `waves` separates them on shared crate files, which the stories do name (design critic); whether planning-merge needs its edge to layout (design critic); whether "no file names a codesec path" in the retirement story fits the epic's own wording (acceptance critic).

```findings
- file: .engineering/planning/story/rules-runner.md
  line: 77
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "collides with story:rules-sanitize-and-fixtures on crates/rules/src, crates/rules/tests/main.rs and CHANGELOG.md (cited, both Scope sections); no depends_on edge between them and neither body names the other; remedies: an ordering edge recording the shared files, or splitting the surface"
- file: .engineering/planning/story/rules-ui-crate.md
  line: 66
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "collides with story:rules-climb-lane on Cargo.toml, Cargo.lock, Taskfile.yml, scripts/check-parts.txt, AGENTS.md and CHANGELOG.md (cited, both Scope sections); the epic order puts both in one parallel group, no edge orders them and no body names the overlap; remedies: an ordering edge with the shared files as its reason, or splitting the surface"
- file: .engineering/planning/story/rules-climb-lane.md
  line: 77
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adds crates/climb but its Scope omits the generated website/docs/reference/crates.md (rebuilt from cargo metadata, crates/docs/src/crates.rs:12) that story:rules-ui-crate and story:rules-layout also change; the page is cited, the writer set is inferred from 'adds a crate'; remedies: name the page in both scopes with an ordering edge, or move the regeneration to one later change"
- file: .engineering/planning/story/rules-catalogue-docs.md
  line: 72
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares AGENTS.md (generated-files table) with story:rules-climb-lane and story:rules-ui-crate (cited, three Scope sections); no edge orders them and no body names the overlap; remedies: an ordering edge, or splitting the surface"
- file: .engineering/planning/story/rules-sanitization-gate.md
  line: 82
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Shares sentence names crates/rules and the test file only, but docs/rules.md is also edited by story:rules-third-party-attribution and story:rules-runner with no edge between them (cited, three Scope sections); remedies: say so and order the three, or give each section its own file"
- file: .engineering/planning/story/rules-import-mechanics.md
  line: 58
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Done When lines read rules/ state and the git log that story:rules-sanitize-and-fixtures writes (the seven category pull requests land there) while the edge runs sanitize to import, so the reader cannot be checked before its producer; move those lines to story:rules-sanitize-and-fixtures (an edge the other way would be a cycle)"
- file: .engineering/planning/story/rules-planning-merge.md
  line: 69
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Scope claims the whole epic/ and story/ directories and says existing artifacts are edited to add a relation without naming which, so its surface is unassessed against other store writers; name the artifacts or narrow to the new files (inferred: relation targets are not cited)"
```
