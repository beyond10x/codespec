---
format: aep.planning-md/3
id: story:corpus-bounded-read
kind: story
status: draft
title: Read a corpus under bounds, off the workstation, and write the records
summary: 'codespec-corpus read: one slot job per repository on the build node, memory cap, per-unit timeout, repository deadline, Go, Rust and Java drivers.'
relations:
- decomposes: epic:reader-self-improvement-loop
- depends_on: story:corpus-read-records
- informed_by: story:readers-bound-untrusted-input
- informed_by: story:jvm-readers-bound-run-time
- informed_by: story:java-read-dependencies
- informed_by: story:go-reader-whole-package
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/corpus/src/driver
- confidence: inferred
  path: crates/corpus/src/lib.rs
- confidence: inferred
  path: crates/corpus/src/main.rs
- confidence: inferred
  path: crates/corpus/src/read.rs
- confidence: inferred
  path: crates/corpus/tests
- confidence: inferred
  path: scripts/corpus-read-one.sh
revision: 5
---
# Story: Read a corpus under bounds, off the workstation, and write the records

## Outcome

`codespec-corpus read --manifest <manifest.json> --out <dir>` reads every repository of a manifest
with the reader of the manifest's language and writes one `codespec-corpus-records/1` document
per repository into the output directory, plus `run.json` (the manifest digest, the `codespec`
version, the tool versions the reader needs, the bounds in force).

`codespec-corpus read` is the driver: it waits and measures, so it starts without the `slot`
prefix. Per repository it starts one read job through `slot`: `slot run --timeout 20m sh
scripts/corpus-read-one.sh <name> <url> <commit> <out>`. The job runs on the remote build node (slot's default for a
command it routes; no `--local`), clones the repository at the pinned commit, checks the commit,
enumerates the units of the language, reads each, and prints the repository's records document on
standard output (one line, the log on standard error). The job holds the clone; nothing of it
returns except the records. On a forge runner, where `slot` is not on `PATH`, the same script runs
under `timeout` with the same bounds and no `slot` prefix; the prefix is chosen by whether `slot`
exists, never by a flag that turns the pool off. Bounds, all stated in `run.json`:

- a memory cap per reader process (`ulimit -v`, default 4 GiB): a reader killed by it is a
  `unit-failed:memory` record;
- a per-unit timeout (default 120 s): `unit-failed:timeout`;
- a per-repository deadline below the job's: 18 minutes inside the 20 the job names, so the
  records written so far survive and each unit not reached is `unit-skipped:repo-deadline`; the
  job's own limit is a backstop, never the bound (the pool kills a job over 10 minutes unless it
  names its time, at most 30);
- clone depth 1 at the commit; a clone that fails or lands on another commit is
  `unit-skipped:clone-failed`, the whole repository.

The drivers, in the crate (Rust), one per reader: Go (every package `go list` gives, as
`scripts/read-repo.sh` enumerates them), Rust (the crates of `cargo metadata --no-deps`), Java
(every `pom.xml` or `build.gradle*` module). A repository with none is `unit-skipped:no-units`.
Java dependencies that cannot be resolved are `unit-skipped:unresolved-dependencies`, not a
silent partial read; `story:java-read-dependencies` (draft) makes more of them resolvable and
this story depends on nothing of it.

The one rule (`~/beyond10x/AGENTS.md` § "Scripts that do not build stay off slot"): the driver
(`codespec-corpus read`, and `scripts/corpus-run.sh` of `story:corpus-loop-lane`) starts without
the `slot` prefix, and each read job it starts, `scripts/corpus-read-one.sh`, goes through `slot
run --timeout`. The corpus run is never a test and is not in `scripts/check-parts.txt`.

## Why

Real repositories hang a reader, exhaust its memory and hold a build slot; the readers' own bounds
(`story:readers-bound-untrusted-input`, implemented; `story:jvm-readers-bound-run-time`, draft)
protect the reader, not the host. A read error that a bound cut off is still a gap and must reach
the triage as a record, so a repository that killed its reader is counted, never dropped.

## Done When

Failing tests first. No test clones or runs a real reader on a real repository: they run the
driver against a recorded fixture clone (two tiny generic Go packages, one crate and one Maven
module, under `crates/corpus/tests/fixtures/`) and a stub reader written in POSIX sh that exits as
told. Every test ends within 60 s, the test step within 120 s; the times of any over 5 s are
reported.

- Over the fixture clone the driver writes the checked-in records document, byte-equal on two
  runs; a second repository whose stub reader sleeps past the unit timeout gives one
  `unit-failed:timeout` and the other units still read; a stub that allocates past the cap gives
  `unit-failed:memory`; a stub that prints invalid `cir/1` gives `unit-failed:invalid-output`.
- Near misses: a clone whose `HEAD` is not the manifest's commit (refused, `clone-failed`, nothing
  read); a repository with no unit; a deadline reached after two units (the rest
  `repo-deadline`, the two read kept); a manifest whose digest line does not match its content
  (refused before any job); the output directory not empty (refused: no mixing of two runs).
- The job line is checked as text by a test: with a stub `slot` on `PATH` it starts with `slot run
  --timeout`, names at most 30 minutes, and has no `--local`; without one it starts with
  `timeout`; the driver's own command line has no `slot` prefix; `scripts/corpus-read-one.sh` is
  `shellcheck`-clean and neither it nor the crate has Python.
- On the build node, one run of each committed manifest (Go, Rust, Java) completes and its output
  directory is the input of the first triage in `story:corpus-triage-report`, which commits the
  pinned triage of each language; the pull request states each run's wall time, the slowest
  repository and any repository that hit a bound (a measurement, not a test).
- A run's `run.json` lists every bound; a bound named on the command line is recorded, so two
  runs with different bounds are told apart.

## Scope

- Own: `crates/corpus/src/read.rs`, `crates/corpus/src/driver/` (`go.rs`, `rust.rs`, `java.rs`),
  `scripts/corpus-read-one.sh`, `crates/corpus/tests/` (module `read` of `tests/main.rs`),
  `crates/corpus/src/main.rs` and `src/lib.rs` (registers `read`), `Taskfile.yml` (task
  `corpus-read`, not in `task check`).
- After `story:corpus-read-records` (it writes the documents this story produces) and
  `story:corpus-manifest`.
- Overlap, `scripts/read-repo.sh`: the Go enumeration is copied in logic, not called; that script
  stays for `examples-netcheck` and is not edited.
