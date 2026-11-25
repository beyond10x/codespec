---
format: aep.planning-md/3
id: story:rules-third-party-attribution
kind: story
status: draft
title: Third-party sources of the rules and their data are attributed under their licenses
summary: rules/NOTICE, a provenance field on every rule, references as CWE ids or URLs, no copied rule text.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-layout
- depends_on: story:rules-sanitization-gate
scope:
- confidence: inferred
  path: crates/rules/src
- confidence: inferred
  path: crates/rules/tests/main.rs
- confidence: inferred
  path: docs/rules.md
- confidence: inferred
  path: rules/NOTICE
revision: 3
---
# Story: Third-party sources of the rules and their data are attributed under their licenses

## Outcome

- `rules/NOTICE` lists every third-party source the catalogue and its measurement data draw on,
  with its license and what is taken: the Go vulnerability database (report ids and summary text
  in the measurement cases; data published under CC-BY 4.0, code under BSD-3, the notice checked
  against the database's own at import), the public repositories the measurement loop reads (by URL
  and commit only, with their licenses), and catalogues used only as sources of defect classes
  (CWE, OWASP Top 10, SpotBugs, FindSecBugs, Error Prone, PMD, Semgrep rules), from which no text is
  copied.
- Every rule's `provenance` field says one of: `written here` (the rule and its fixture were
  written in this repository's words), or `imported from codesec <digest>` (the rule file's
  SHA-256 before import, `story:rules-import-mechanics`). A rule whose class came from a public
  catalogue names it in `references`; a rule never copies a rule body, message or fixture from
  one. In particular no Semgrep rule text: its rules license restricts reuse.
- `codespec rules check` fails on a `provenance` that is neither form, and on a `references` entry
  that is neither a `CWE-<n>` id nor an `https` URL.
- The catalogue UI's pinned script URL (Datastar, MIT, loaded from a CDN with an SRI hash) is listed
  in `rules/NOTICE` when the UI moves (`story:rules-ui-crate`).

## Why

The measurement data quotes the Go vulnerability database's summaries; publishing it needs the
attribution its license asks for. The planned Java rules name six rule catalogues as sources of
classes; without a written line, a later rule could copy one.

## Done When

Failing tests first, in `crates/rules/tests/main.rs`, on made-up rule files; within 60 s each:

- a rule with `provenance: copied` fails naming the field; `written here` and
  `imported from codesec sha256:<64 hex>` pass; `imported from codesec sha256:abc` fails;
- a `references` entry `see semgrep` fails; `CWE-22` and `https://cwe.mitre.org/data/definitions/22.html` pass;
- `rules/NOTICE` exists and names the Go vulnerability database with CC-BY 4.0 before any data file
  of `story:rules-climb-lane` lands.

## Scope

`rules/NOTICE`, `crates/rules/src/` (the provenance and references checks),
`crates/rules/tests/main.rs`, `docs/rules.md` (§ provenance). Shares the crate, its test file and
`docs/rules.md` with `story:rules-sanitization-gate` (lands before, this story depends on it) and
`story:rules-runner` (lands after, depends on this one). The Datastar line of `rules/NOTICE` is
written and checked by `story:rules-ui-crate`, not here.
