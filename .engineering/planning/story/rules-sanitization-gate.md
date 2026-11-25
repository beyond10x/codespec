---
format: aep.planning-md/3
id: story:rules-sanitization-gate
kind: story
status: draft
title: A sanitization gate that keeps every private name out of the rules, in CI, forever
summary: Private names as Gates policy literals held outside the repository; structural checks on every rule and fixture in codespec rules check; fixtures written from scratch.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-layout
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: crates/rules/src
- confidence: inferred
  path: crates/rules/tests/main.rs
- confidence: inferred
  path: docs/rules.md
revision: 3
---
# Story: A sanitization gate that keeps every private name out of the rules, in CI, forever

## Outcome

Two checks, both run on every pull request and every push to `main`:

1. Denylist. The private names a rule must never carry (reviewed code bases, private repository
   names, private identifiers, hosts and people) are literals and patterns of the existing Gates
   policy (`forbidden_literals`, `forbidden_patterns`), which lives in a private repository and
   reaches CI as the secret it already uses; codespec is already enrolled and its shared-gates
   workflow already scans every change. This story adds the names of the private inventory taken
   on 2026-11-25 to that policy (a change in the private policy repository, made by the owner or on
   the owner's word) and proves the scan covers `rules/`. This repository commits no list, no hash
   and no count of those names.
2. Structure, in `codespec rules check` (`crates/rules`), with no private string in it: it fails
   when a rule's text or a fixture holds a home-directory path, an e-mail address outside the
   reserved domains (`example.com`, `example.org`, `example.net`, `*.example`, `*.invalid`,
   `*.test`), a URL host outside an allow list kept in `crates/rules` (`cwe.mitre.org`,
   `owasp.org`, `pkg.go.dev`, `github.com/golang`, `doc.rust-lang.org`, the vulnerability
   databases), an IPv4 outside loopback, the documentation ranges of RFC 5737 and the private
   ranges, a commit hash of 7 or more hexadecimal characters, or a `file:line` that is not inside
   the rule's own fixture; and when a fixture's Go module path is not under `example.com/` or its
   Rust crate is not named `fixture`. Over the measurement data under `rules/climb/`
   (`story:rules-climb-lane`), which names public repositories by commit and `file:line`, the
   check applies the home-path, e-mail, host and IP classes; every repository URL there must be in
   the allow list of public repositories that data reads.

Rule written into `AGENTS.md`: every fixture is written from scratch with generic names; it never
copies, shortens or renames code of a reviewed code base. A rule found from a private review is
written as the general defect class, in this repository's words, with its own fixture. The current
line of `AGENTS.md` § Delivery that says the pattern library lives in codesec and nothing of it is
copied here is replaced by this rule and the two checks.

Why the denylist is a secret list and not salted hashes in the repository: many private names are
short words; a hash of a short word, salted or not, is found by trying every word when the salt is
public, and a secret salt is a secret list by another name. The Gates policy is already that secret
list, already scans every change of this repository, and runs on pull requests from forks through
the workflow it uses today. A second mechanism would be a second list to keep current.

## Why

The owner asked that the rules carry no project- or company-specific content. A one-time cleanup
does not hold: every later rule written after a private review is a new chance to copy a name. The
gate must outlive the import.

## Done When

Failing tests first, in `crates/rules/tests/main.rs`, on made-up data; every test within 60 s:

- each structural refusal above fails a made-up rule or fixture, naming file, line and class;
  near misses pass: `alice@example.com`, `192.0.2.1`, `127.0.0.1`, `CWE-78`, a `file:line` into the
  rule's own fixture, a 6-character hexadecimal word;
- the Gates scan, run locally with `b10x-gates` against a test policy holding the made-up literal
  `quokka-ledger-7`, refuses a branch that adds that literal under `rules/` and accepts the same
  branch without it (a recorded run in the story's evidence; no real private literal in any file);
- the owner (or a session on the owner's word) confirms the policy holds the inventory's names: the
  evidence is the policy's commit id, not its content;
- `AGENTS.md` carries the from-scratch rule and no longer says nothing of the catalogue is copied
  here.

## Scope

`crates/rules/src/` (a module for the structural checks), `crates/rules/tests/main.rs`,
`AGENTS.md` (§ Delivery and the invariants list), `docs/rules.md` (a section on what a rule may
hold). Outside this repository: the private Gates policy (owner). Shares `crates/rules`,
`crates/rules/tests/main.rs` and `docs/rules.md` with `story:rules-layout` (depends on it),
`story:rules-third-party-attribution` and `story:rules-runner`; the three land in that order (the
two later ones depend on this one), one at a time.
