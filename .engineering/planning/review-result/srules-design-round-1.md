---
format: aep.planning-md/3
id: review-result:srules-design-round-1
kind: review-result
status: active
title: Plan critic design, round 1, epic security-rules-in-codespec
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
story:rules-import-mechanics — its two acceptance bullets read rules/ as it stands after "the seven pull requests", and story:rules-sanitize-and-fixtures (which owns those PRs) depends on it, so these bullets cannot be checked before their producer lands; the edge back would be a cycle, so the fix is to move both bullets to story:rules-sanitize-and-fixtures and keep only the script and its test here — .engineering/planning/story/rules-import-mechanics.md:58-60, .engineering/planning/story/rules-sanitize-and-fixtures.md:38
story:rules-sanitize-and-fixtures — it adds a fixture refusal to `crates/rules` in the first category PR, while story:rules-import-mechanics says each PR carries only `rules/<category>/` and the import script already stops on any `codespec rules check` refusal, so a fixture-less rule fails the import before the hand work starts; the refusal belongs to the checker's list in story:rules-layout (covered by the existing `depends_on`), and this story should then only write the fixtures — .engineering/planning/story/rules-sanitize-and-fixtures.md:68, .engineering/planning/story/rules-import-mechanics.md:35,40

Both findings describe one split abstraction. The seven category PRs are half in each story: the script and the PR shape sit in import-mechanics, the PR contents in sanitize-and-fixtures.

What I read: 12 artifacts (the epic and its 11 stories) with `aep plan artifact show`, plus `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`. Validate reported "valid"; its other output was review-result housekeeping for older reviews and is not a finding. I checked that the `story:rust-reader-*` and `story:query-lookup-skips-wildcard-tuples` ids cited by story:rules-planning-merge exist. I walked all 25 edges among the set and no edge leaves it. I found no cycle and the set is not a chain: story:rules-layout is the single root, and the order above it fans out and rejoins. I traced about 14 acceptance reads to a producer in the set, and every one has a `depends_on` edge, direct or transitive, except the reversed read in finding 1.

What I could not establish:
- `epic:reader-self-improvement-loop`, which story:rules-climb-lane names, has no artifact in this store. I treat it as a dependency outside the set, not a defect.
- Out of my lane (acceptance): in story:rules-climb-lane, the line about adding the rule step to the reader corpus loop's lane has no checkable acceptance.
- Out of my lane (parallel safety): the stories that share the `crates/rules/tests/main.rs` file and the `AGENTS.md` and `CHANGELOG.md` files.
- Out of my lane (scope): story:rules-planning-merge maps issue 14's "one file per module" item to `story:rust-reader-whole-crate`, whose status is already `implemented`.

```findings
- file: .engineering/planning/story/rules-import-mechanics.md
  line: 58
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its two acceptance bullets read rules/ as it stands after the seven pull requests, which story:rules-sanitize-and-fixtures lands, and that story depends_on this one, so the reader cannot be checked before its producer; the bullets belong in story:rules-sanitize-and-fixtures and this story keeps only the script and its test"
- file: .engineering/planning/story/rules-sanitize-and-fixtures.md
  line: 68
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it adds the fixture refusal to crates/rules in the first category pull request, while story:rules-import-mechanics says each pull request carries only rules/<category>/ and its import script stops on any codespec rules check refusal; the refusal belongs to the checker list of story:rules-layout, and this story should only write the fixtures"
```
