---
format: aep.planning-md/3
id: story:rules-layout
kind: story
status: draft
title: Rule files live under rules/, read and checked by codespec rules
summary: rules/<category>/<id>.yaml with fixtures and committed fixture documents; crates/rules; codespec rules list and check in the gate; versioned with the release.
relations:
- decomposes: epic:security-rules-in-codespec
scope:
- confidence: inferred
  path: .github/workflows/release-build.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/rules
- confidence: inferred
  path: docs/rules.md
- confidence: inferred
  path: rules/categories.tsv
- confidence: inferred
  path: rules/subcategories.yaml
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/docs/reference/cli.md
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 3
---
# Story: Rule files live under rules/, read and checked by codespec rules

## Outcome

- Layout: `rules/categories.tsv` (the closed list `security`, `performance`, `robustness`,
  `correctness`, `concurrency`, `resource`, `inspect`, one line each with its meaning),
  `rules/subcategories.yaml`, one rule per file `rules/<category>/<id>.yaml`, its fixture in
  `rules/<category>/fixtures/<id>/` (source, `rows.txt` with the rows the rule must give, and the
  committed `cir/1` documents read from the source with a digest of the source and the reader).
- A rule file holds `id` (equal to the file name, kebab-case; Rust rules `rs-*`, Java rules
  `java-*`), `title`, `description`, `category` (equal to the directory), `subcategory`,
  `languages`, `severity`, `status` (`active` or `pending`), `needs` (the codespec story a pending
  rule waits for, `null` when active), `references` (CWE ids or URLs), `names` (the defaults of
  every names list the query declares), `fixture`, `provenance` (`story:rules-third-party-attribution`)
  and `cql`. The format is written in `docs/rules.md`.
- New crate `crates/rules` (`b10x-codespec-rules`): reads and checks the catalogue. `codespec rules
  list [--category c] [--language l] [--format text|json]` prints every rule with its status;
  `codespec rules check` fails, naming file and field, when a file does not parse, lacks a field,
  names a category outside the closed list or an unlisted subcategory, is not named by its `id`,
  declares other names lists in `cql` than `names` holds, names a fixture that does not exist, has
  `fixture: null` (every rule, active or pending, has a fixture), or has a fixture whose `rows.txt`
  holds no positive row or whose source holds no `near miss` marker; it also fails on a rule id in
  the overlap table of `docs/rules.md` (catalogue rules that overlap a built-in check) that is not a
  rule.
- `codespec rules check` and the fixture check run in the gate as one `test` part of
  `scripts/check-parts.txt` (`test b10x-codespec-rules`). The fixture check queries the committed
  fixture documents only; it reads no source and runs no reader, so its cost does not grow with the
  number of rules times a reader start. Regenerating fixture documents is `task rules-fixtures`,
  not in the gate; a digest check in the gate fails when a fixture's source changed and its
  documents were not regenerated.
- Versioning: the catalogue is versioned with codespec. A rule's id never changes meaning; a
  changed rule is a `CHANGELOG.md` line under the release that ships it; a removed rule is a
  CHANGELOG line too. The release archive carries `rules/` (`.github/workflows/release-build.yml`),
  and `codespec rules list --format json` prints the codespec version it belongs to.
- The seven built-in cq checks under `crates/query/checks/` stay where they are and keep their
  meaning; which catalogue rules overlap one of them is written by
  `story:rules-sanitize-and-fixtures`, when the rules exist.

## Why

The rules need one home and one format before any rule moves (`epic:security-rules-in-codespec`).
codesec's format is kept so 69 files move without a rewrite of their queries; only `provenance` is
new. The fixture check must stay within the gate's budget: codesec's `hunts-check` read every
fixture with a reader on every run, an expensive step per rule.

## Done When

Failing tests first, in `crates/rules/tests/main.rs` (one integration-test binary), over made-up
rule files under `crates/rules/tests/data/`; every test within 60 s, the part within 120 s:

- a well-formed rule is listed with its id, category and status; `--language rust` lists no Go rule;
- each refusal above names its file and field: a missing `provenance`, a category `secruity`, an
  id that differs from the file name, a `names` key the `cql` does not declare (near miss: the same
  key declared passes);
- a fixture whose committed rows differ from the rule's rows on its committed documents fails,
  naming the first differing row; the same fixture with its rows restored passes;
- a fixture whose source digest differs from the recorded one fails with "regenerate with task
  rules-fixtures";
- `sh scripts/check-part.sh matrix` lists the new part, and the `lint` part's `check-parts` fails
  before the part is added;
- a rule with `fixture: null`, and a fixture with no `near miss` marker, each fail naming the rule;
  an overlap table naming an unknown id fails, an empty one passes;
- `codespec rules list --format json` holds `codespec_version` equal to the workspace version (a
  test compares it with `CARGO_PKG_VERSION`);
- the release package step: the script the release workflow calls to pack an archive is run on the
  tree into `target/` and the archive's listing holds `rules/categories.tsv`; it fails before the
  step packs `rules/`;
- `codespec rules list` appears in `website/docs/reference/cli.md` after `task docs-generate`.

Not done by this story: importing any real rule (`story:rules-sanitize-and-fixtures`).

## Scope

The crate `crates/rules` and its tests; `crates/cli/src/lib.rs` (the subcommand); `rules/`
(the two lists only); `docs/rules.md`; `scripts/check-parts.txt`; `Taskfile.yml`; `Cargo.toml`,
`Cargo.lock`; `.github/workflows/release-build.yml`; `CHANGELOG.md`; `AGENTS.md` (the table of
paths and the gate table); the generated `website/docs/reference/cli.md` and `crates.md`.
Shares `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md`, `CHANGELOG.md` and the CLI with
most stories of the epic: those depend on this one.
