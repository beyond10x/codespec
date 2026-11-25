---
format: aep.planning-md/3
id: story:rules-sanitize-and-fixtures
kind: story
status: draft
title: Each rule is sanitized, generalized or dropped, and has a from-scratch fixture with positive and near-miss rows
summary: The seven category imports; every rule moved, generalized or dropped with its reason; every kept rule with a made-up fixture holding a match and a near miss.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-import-mechanics
- depends_on: story:rules-runner
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: docs/rules.md
- confidence: inferred
  path: rules/concurrency
- confidence: inferred
  path: rules/correctness
- confidence: inferred
  path: rules/inspect
- confidence: inferred
  path: rules/performance
- confidence: inferred
  path: rules/resource
- confidence: inferred
  path: rules/robustness
- confidence: inferred
  path: rules/security
revision: 6
---
# Story: Each rule is sanitized, generalized or dropped, and has a from-scratch fixture with positive and near-miss rows

## Outcome

The seven category pull requests land, in the order of `story:rules-import-mechanics`, each
imported with its script from an export directory outside this repository. In each:

- Every rule is read in full (title, description, names defaults, `cql`) and gets one of three
  outcomes, listed in the pull request: moved (no change but `provenance`), generalized (a name,
  constant, string literal, package path or count that belongs to one code base replaced by a
  names list a review sets at run time, or by the general defect class), or dropped (the idea
  exists in one code base only), with its reason in `CHANGELOG.md`.
- A description that cites a run over private code (a row count "over the organization's
  repositories", the seed of a sweep) is rewritten to say what the rule matches and what it
  cannot see, without the run. A description that names a tool that no longer exists is rewritten.
  The inventory names three such rules; the reviewer of each pull request looks for more.
- Every kept rule has a fixture written from scratch with made-up names: at least one positive row
  (code the rule must match) and at least one near miss (code that looks alike and must give no
  row), the near miss marked `near miss` in the fixture source. The 37 rules that have no fixture
  today get one; the 32 existing fixtures are checked for a near miss and get one where it is
  missing. A `pending` rule keeps or gets its fixture; the check runs it when it becomes `active`.
- The two Rust rules widened in codesec's open pull request (string truncation inside a multi-byte
  character; a URL with user information printed) arrive with their widened fixtures.
- codesec's page of first pattern drafts is not copied; what of it is still true goes into
  `docs/rules.md` without its provenance sentence about an earlier hunt.
- `docs/rules.md` gains a table of the catalogue rules that overlap a built-in check of
  `crates/query/checks/`, and which one `codespec check` runs.

No change to `crates/rules` here: every refusal these pull requests must pass is
`story:rules-layout`'s, `story:rules-sanitization-gate`'s or `story:rules-third-party-attribution`'s.

## Why

The owner asked that the queries and pattern matches carry no project-specific content. A rule is
public only when its fixture is: a rule with no fixture cannot be checked by anybody reading the
repository, and a fixture without a near miss proves nothing about noise.

## Done When

Failing check first, per pull request: the import script over the category's export fails before
its fixtures are written (`fixture: null` or a missing near miss is refused) and passes after.

- `codespec rules check` exits 0 over all seven categories; the `test b10x-codespec-rules` part
  stays within 120 s, timed with the last category and the time written in its pull request;
- the Gates scan of `story:rules-sanitization-gate` passes on each pull request;
- `CHANGELOG.md` lists every dropped rule with its reason; the rules moved, generalized and dropped
  add up to the number exported (69 plus any added by codesec's open pull requests), and
  `codespec rules list` counts the moved and generalized ones and no other;
- for each of the seven category directories, `git log --format=%an -- rules/<category>/` on
  `main` prints only `b10x-bot[bot]`, and no commit of this repository has a codesec commit as an
  ancestor;
- the overlap table of `docs/rules.md` names at least one rule, and `codespec rules check` (which
  fails on an id in that table that is not a rule, `story:rules-layout`) passes;
- near miss: a fixture whose `near miss` code is changed to match makes the check fail, naming the
  rule and the new row (shown once in the first pull request's description).

## Scope

`rules/concurrency/`, `rules/correctness/`, `rules/inspect/`, `rules/performance/`,
`rules/resource/`, `rules/robustness/`, `rules/security/`, `docs/rules.md` (the overlap table),
`CHANGELOG.md`. Depends on `story:rules-import-mechanics` and on `story:rules-runner` (it shares
`CHANGELOG.md` and `docs/rules.md` with the runner and lands after it). Seven pull requests, one at
a time.
