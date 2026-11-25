---
format: aep.planning-md/3
id: story:rules-planning-merge
kind: story
status: draft
title: codesec's plans carry over rewritten, and its open issues map to codespec stories
summary: Which companion epics and stories move rewritten, which stay or drop; issues 13 and 14 mapped to codespec stories.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-layout
- depends_on: story:rules-sanitization-gate
scope:
- confidence: inferred
  path: .engineering/planning/epic/java-rules-catalogue.md
- confidence: inferred
  path: .engineering/planning/epic/rules-catalogue-follow-up.md
- confidence: inferred
  path: .engineering/planning/epic/rules-measurement-loop.md
- confidence: inferred
  path: .engineering/planning/story/cir1-format-characters-in-string-values.md
- confidence: inferred
  path: .engineering/planning/story/query-memory-bounded-by-document-count.md
revision: 5
---
# Story: codesec's plans carry over rewritten, and its open issues map to codespec stories

## Outcome

codesec's planning store holds 3 epics and 73 stories. This story moves what is still work, in
this repository's words, with no private name, as new artifacts in three new epics, and leaves the
rest:

| codesec artifacts | count | here |
|---|---|---|
| the pattern library epic's five open rule stories (defect patterns, rules not applicable, precision on a later codespec, taint flows, unhandled branches) and the catalogue UI's 3 stories | 8 | 8 stories under the new `epic:rules-catalogue-follow-up`; a measurement over a private code base becomes one over the public corpus |
| the measurement-loop epic and its 14 stories | 15 | the new `epic:rules-measurement-loop` and 14 stories |
| the Java rules epic and its 26 live stories | 27 | the new `epic:java-rules-catalogue` and 26 stories; every count taken over a private service becomes a count over the pinned public Java corpus; the defect classes keep their generic wording |
| 21 stories that ask codespec for a fact or a reader change | 21 | not moved: each names a codespec story that owns it; where one names none, a codespec story is drafted |
| the pattern library epic itself, 2 archived stories, the store's project file | 4 | not moved (`epic:security-rules-in-codespec` replaces the epic) |
| 2 stories about the pages and clones of private reviews | 2 | stay private, with the review outputs |

A codesec story that is dropped rather than rewritten (because codespec already did it, or it only
made sense for a private run) is listed with its reason in the new epic's body.

codesec's two open issues map as follows:

- Issue 13 (a performance rule grows to 33 GB over 5,087 Rust documents, no bound): the runner's
  per-rule memory bound, `story:rules-runner`; the query engine's growth with the document count, a
  new codespec story drafted by this story beside `story:query-lookup-skips-wildcard-tuples`.
- Issue 14 (gaps found over a public 5,087-file Rust workspace): async functions refused whole,
  `story:rust-reader-async`; macro and `assert!` refusals, `story:rust-reader-std-macros`; impls of
  undeclared types, `story:rust-reader-impls-on-undeclared-types`; `&mut` parameters,
  `story:rust-reader-borrow-kinds`; generic bounds with associated types,
  `story:rust-reader-generic-trait-impls`; items inside `macro_rules!`,
  `story:rust-reader-trait-and-macro-items`; one format character in a string literal failing a
  whole file, a new codespec story drafted by this story (strict reading keeps refusing it in a
  name; whether a string `value` may hold it is that story's design question); one file per module:
  already answered by the implemented `story:rust-reader-whole-crate` (the issue's scan read each
  file as its own crate), nothing to file; runner items (language filter, first failure ending a category,
  out-of-memory kills, test files), `story:rules-runner`; rule coverage of two Rust rules,
  `story:rules-sanitize-and-fixtures`.

## Why

Two stores for one product is the split the owner wants gone. Copying 73 stories as they are would
copy their private references; leaving them would lose planned work.

## Done When

- `aep plan artifact show` finds `epic:rules-catalogue-follow-up`, `epic:rules-measurement-loop`
  and `epic:java-rules-catalogue`; `aep plan artifact list --format json` counts 8, 14 and 26
  stories decomposing them, less the ones each epic body lists as dropped with a reason (the counts
  plus the dropped add up to 8, 14 and 26);
- the two new codespec stories exist: `story:query-memory-bounded-by-document-count` (issue 13) and
  `story:cir1-format-characters-in-string-values` (issue 14);
- every new artifact names the codesec artifact id it came from in its body;
- `aep plan artifact validate` is valid;
- the Gates scan of `story:rules-sanitization-gate` passes on the pull request (the private
  literals are checked there, never in a file here); before that story has landed, this story does
  not start;
- each issue item above names an artifact id that `aep plan artifact show` finds;
- near miss: a rewritten Java story naming a count directory of a private run is refused in review
  (`git grep -n -i private -- .engineering/planning` over the new files lists only the words "a
  private code base").

## Scope

New files only, created through `aep plan artifact new`; no existing artifact of this store is
edited. The three new epic files `.engineering/planning/epic/rules-catalogue-follow-up.md`,
`.engineering/planning/epic/rules-measurement-loop.md`,
`.engineering/planning/epic/java-rules-catalogue.md`, the two new stories
`.engineering/planning/story/query-memory-bounded-by-document-count.md` and
`.engineering/planning/story/cir1-format-characters-in-string-values.md`, and the 48 rewritten
stories as new files under `.engineering/planning/story/` (new names; none collides with an
existing file). Depends on `story:rules-layout` and `story:rules-sanitization-gate`.
