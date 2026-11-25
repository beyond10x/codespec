---
format: aep.planning-md/3
id: review-result:srules-parallel-safety-round-2
kind: review-result
status: active
title: Plan critic parallel-safety, round 2, epic security-rules-in-codespec
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
approve

What I read: 12 artifacts (the epic and the 11 stories) with `aep plan artifact show` on each, `aep plan artifact graph`, `aep plan artifact waves --kind story`, and the round-1 record `review-result:srules-parallel-safety-round-1`. I also checked in the tree that the stories named by `story:rules-planning-merge` exist under `.engineering/planning/story/`, and that `examples/netcheck` exists.

Round 1 findings, all seven fixed:
1. `story:rules-runner` vs `story:rules-sanitize-and-fixtures`. `sanitize` now has `depends_on` edges to `import-mechanics` and `runner`, and both bodies name the shared `CHANGELOG.md` and `docs/rules.md`. Fixed.
2. `story:rules-ui-crate` vs `story:rules-climb-lane`. The `ui-crate` story now has `depends_on story:rules-climb-lane`, and both bodies name the shared workspace files and `crates.md`. Fixed.
3. `crates.md`. It is now named in the Scope of `climb-lane`, `ui-crate` and `layout`, and those three are ordered by edges. Fixed.
4. `AGENTS.md` in `story:rules-catalogue-docs`. It has edges to `sanitize` and `ui-crate` (and so to `climb` through `ui-crate`), and its Scope names the overlap. Fixed.
5. `docs/rules.md` and the crate files in `story:rules-sanitization-gate`. Its Shares sentence now names `crates/rules`, the test file and `docs/rules.md` with `layout`, `attribution` and `runner`. The edges layout → gate → attribution → runner order them, and `attribution` and `runner` name them too. Fixed.
6. `story:rules-import-mechanics` reading state `sanitize` writes. The two Done When lines are gone, and the body says the seven category pull requests are `sanitize`'s. The edge runs `sanitize` to `import`, so the reader-beside-producer problem is gone. Fixed.
7. `story:rules-planning-merge` scope. It now says "New files only, no existing artifact of this store is edited" and lists the five paths. The other 48 rewritten stories are described as new files under `story/`. I checked that the existing stories it cites exist. Fixed.

New problems from the revision: none. Of the in-set collisions `aep plan artifact waves` printed, every pair of the 12 has an ordering path in the graph, directly or by a chain.
- Chain: layout → gate → attribution → runner → sanitize → climb → ui → docs → retirement.
- `import-mechanics` sits between `gate`/`attribution` and `sanitize`.
- `planning-merge` has edges to `layout` and `gate` and is in the same wave as `attribution` with no shared path.

Surfaces established, cited: 12 of 12 (each body's Scope section). Inferred: 0. Unplaceable: 0. The store's `scope` field marks every path `inferred`, which makes `waves` print collisions between the in-set pairs and with unrelated stories outside the set. Those are outside what I was given.
Acceptance reads traced to a producer in the set: 24. Recorded by a `depends_on` edge, directly or through a chain: 24.

What I could not establish: whether `epic:reader-self-improvement-loop` (outside the set and the store) is ordered against `story:rules-climb-lane`.

Out of my lane, and none of it sets the verdict:
- `story:rules-sanitize-and-fixtures` has an acceptance line that `git log --format=%an -- rules/` shows only the bot. `layout` and `attribution` also commit under `rules/`, so the line depends on who authors those commits (acceptance critic).
- `story:codesec-retirement` says "no file names a codesec path". `story:rules-planning-merge` writes codesec artifact ids into new artifacts, so the wording needs a decision on ids versus paths (acceptance critic).

```findings
[]
```
