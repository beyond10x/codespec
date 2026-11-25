---
format: aep.planning-md/3
id: story:rules-climb-lane
kind: story
status: draft
title: The measurement loop over published Go fixes runs as a slow lane, in Rust, and gives the reader corpus loop the rule summary it reads
summary: crates/climb replaces the Go program and shell scripts; sanitized committed results with a ratchet and digest check in the gate; full measure in a slow lane; the runner summary is the interface the reader corpus loop reads.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-runner
- depends_on: story:rules-sanitize-and-fixtures
- depends_on: story:rules-third-party-attribution
scope:
- confidence: inferred
  path: .github/workflows/climb.yml
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
  path: crates/climb
- confidence: inferred
  path: rules/climb
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 6
---
# Story: The measurement loop over published Go fixes runs as a slow lane, in Rust, and gives the reader corpus loop the rule summary it reads

## Outcome

- codesec's measurement loop ("climb") moves: for each fix commit of a Go vulnerability report in
  public Go repositories, read the directories the fix changes before and after it, run every
  active rule outside `inspect`, and count a case as detected when a function of the fix has fewer
  rows after it. Its Go program and shell scripts are rewritten as the Rust crate `crates/climb`
  (binary `codespec-climb`: `cases`, `measure`, `noise`, `score`, `ratchet`, `baseline`,
  `check-digest`), calling `codespec rules run` (`story:rules-runner`) and reading its
  `summary.json`.
- Its data and prose move to `rules/climb/`, sanitized: the repository list (public Go
  repositories by URL), the cases, the results, the classes (which quote report summaries), the
  noise repositories and noise sample, the baseline, the missing-facts list and the scoreboard.
  Each file is checked against the inventory's list of those 13 files for any private name and run
  through the structural check of `story:rules-sanitization-gate` (home paths, e-mail, hosts, IPs);
  the classes and noise sample carry the attribution of `rules/NOTICE`. They are generated files
  with a digest of their inputs (the rule files, the case list, the codespec version) in
  `rules/climb/inputs.sha256`.
- The gate runs `codespec-climb ratchet` and `codespec-climb check-digest` over the committed files,
  as a part of `scripts/check-parts.txt` (no clone, no reader, no network): the ratchet fails when a
  case the baseline has as detected no longer is, or a rule has more noise rows than its baseline;
  the digest check fails when a rule file changed and the results were not regenerated. The full
  measure is the separate `slow` lane, `.github/workflows/climb.yml`, run by hand
  (`workflow_dispatch`), writing its results as an artifact; a pull request that changes the
  baseline or a rule carries the lane's output.
- The reader corpus loop (`epic:reader-self-improvement-loop`, planned on its own branch and not in
  this store yet) gets from this epic the interface it reads: `codespec rules run`'s `summary.json`
  per rule. Adding the rule step to that loop's lane is a story of that epic, not of this one: not
  covered here, and recorded as not covered in `epic:security-rules-in-codespec` § Out of scope.

## Why

The loop is the only measurement of the rules against real, published defects. It needs no private
input. It cannot be Go here: this repository admits Go for adapters only.

## Done When

Failing tests first, in `crates/climb/tests/main.rs`, over made-up committed data (no network, no
clone); within 60 s each, the part within 120 s:

- `ratchet` over a baseline and results where a detected case is lost exits 1 naming the case;
  with the case detected it exits 0; a rule with one more noise row than its baseline fails naming
  the rule;
- `check-digest` fails after a made-up rule file changes and passes after the digest is rewritten;
- `score` over a made-up results file writes the same scoreboard bytes twice;
- `cases` over a made-up vulnerability report directory and a made-up repository with one fix
  commit writes one case line with the fix, its parent and the changed directories;
- `sh scripts/check-part.sh matrix` lists the climb part; the `lint` part's `check-parts` fails
  before it is added;
- `.github/workflows/climb.yml` has `workflow_dispatch` and no `pull_request` trigger, and no part
  of `scripts/check-parts.txt` calls `measure` (a test greps both files);
- `codespec rules check` passes over `rules/climb/` with the data classes of the structural check;
- the first slow-lane run's detected count equals codesec's last scoreboard count, or the
  difference is listed per case in the pull request.

## Scope

`crates/climb/`, `rules/climb/`, `.github/workflows/climb.yml`, `Taskfile.yml`,
`scripts/check-parts.txt`, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `AGENTS.md` (the gate and
generated-files tables), the generated `website/docs/reference/crates.md`. Depends on
`story:rules-runner`, `story:rules-sanitize-and-fixtures` and
`story:rules-third-party-attribution`. `story:rules-ui-crate` and `story:rules-catalogue-docs`
share the workspace files, `AGENTS.md` and `crates.md` with this story and land after it.
