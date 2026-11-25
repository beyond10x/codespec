---
format: aep.planning-md/3
id: story:rules-ui-crate
kind: story
status: draft
title: The catalogue UI moves as its own crate
summary: crates/rules-ui with its loopback, origin and clone guards and tests, running codespec rules run.
relations:
- decomposes: epic:security-rules-in-codespec
- depends_on: story:rules-runner
- depends_on: story:rules-sanitize-and-fixtures
- depends_on: story:rules-climb-lane
scope:
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
  path: crates/rules-ui
- confidence: inferred
  path: rules/NOTICE
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/docs/reference/crates.md
revision: 4
---
# Story: The catalogue UI moves as its own crate

## Outcome

codesec's local catalogue explorer and scan control plane (one Rust binary: browse the rules,
discover repositories at a path or a git URL, scan them one job at a time under time and memory
bounds, mark each row true or noise) moves as `crates/rules-ui` (binary `codespec-rules-ui`). It
reads `rules/` and runs `codespec rules run` (`story:rules-runner`) instead of its own scan loop
where the two do the same thing. Its security properties stay and stay tested: loopback only; every
request names a loopback `Host` with its port; every POST carries its own `Origin` and
`Datastar-Request: true`; git URLs https or ssh only, no credentials in a URL, cloned with no hooks
and no LFS; a long token-like word of a scanned source is shown redacted. Its work directory
(clones, documents, rows, marks) is outside the repository. Its view fixtures are made-up code
only; its placeholder text names no home directory.

## Why

The UI is how a person reads rule rows and marks noise; it holds nothing private. Kept apart, it
would pin the private repository alive.

## Done When

Failing tests first, the crate's tests moved as one integration-test binary
`crates/rules-ui/tests/main.rs` with modules (discovery, guard, hardening, limits, views); every
test within 60 s, the part within 120 s:

- a POST without `Origin` is refused, with it accepted (moved test, run red first by removing the
  check in a scratch copy);
- a request whose `Host` is not loopback with the listening port is refused; the server refuses to
  bind a non-loopback address;
- a git URL with a password is refused naming the reason; the clone command's environment disables
  hooks and LFS (a test reads the command the UI builds);
- a token-like word of a scanned source line is shown as `[redacted]`, a short word is not;
- a scan started from the UI runs `codespec rules run` (a stub codespec binary under
  `crates/rules-ui/tests/` records its arguments; the test fails while the UI runs its own loop);
- `rules/NOTICE` names Datastar with its version and license (a test reads the file and the pinned
  URL in the crate and compares the version);
- a view renders from `crates/rules-ui/fixtures/` and the structural check of
  `story:rules-sanitization-gate` passes over those fixtures;
- the crate is a `test` part of `scripts/check-parts.txt`;
- `cargo tree -p codespec-rules-ui` dependencies are listed in the pull request; no other crate of
  the workspace gains a dependency.

## Scope

`crates/rules-ui/`, `Cargo.toml`, `Cargo.lock`, `scripts/check-parts.txt`, `Taskfile.yml`,
`rules/NOTICE` (the pinned script), `AGENTS.md` (table of paths), `CHANGELOG.md`, the generated
`website/docs/reference/crates.md`. Depends on `story:rules-runner`,
`story:rules-sanitize-and-fixtures` and `story:rules-climb-lane` (it shares the workspace files,
`scripts/check-parts.txt`, `Taskfile.yml`, `AGENTS.md`, `CHANGELOG.md` and `crates.md` with that
story and lands after it); `story:rules-catalogue-docs` lands after this one.
