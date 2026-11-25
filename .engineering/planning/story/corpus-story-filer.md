---
format: aep.planning-md/3
id: story:corpus-story-filer
kind: story
status: draft
title: Draft one story per reason code above a threshold, with a reproducer written from scratch
summary: 'codespec-corpus file: draft stories through aep for codes with 5 or more occurrences in 2 or more repositories, deduplicated by tag, reproducer checked against upstream.'
relations:
- decomposes: epic:reader-self-improvement-loop
- depends_on: story:corpus-triage-report
scope:
- confidence: inferred
  path: corpus/reproducers
- confidence: inferred
  path: crates/corpus/src/file.rs
- confidence: inferred
  path: crates/corpus/src/lib.rs
- confidence: inferred
  path: crates/corpus/src/main.rs
- confidence: inferred
  path: crates/corpus/src/reproducer.rs
- confidence: inferred
  path: crates/corpus/tests
revision: 4
---
# Story: Draft one story per reason code above a threshold, with a reproducer written from scratch

## Outcome

`codespec-corpus file --triage <triage.json> --language <name> --work <dir> [--min-occurrences 5]
[--min-repos 2] [--dry-run]` turns the triage report into draft stories in the planning store, through `aep
plan artifact new`, `body` and `scope` only (never an edit of a store file by hand), and prints
one line per code: filed, deduplicated, below threshold, or waiting for a reproducer.

For each code with at least `--min-occurrences` occurrences in at least `--min-repos`
repositories (defaults 5 and 2, so one repository cannot set the work), and no existing story:

- a draft story `story:corpus-<language>-<slug>` where the slug is the code in lowercase kebab
  with an 8-hex digest of the exact code, so two codes never share an id; its tag is
  `corpus-reason-<language>-<digest>`; the title names the code; the story `decomposes` the epic
  named by `--epic` (default `epic:reader-self-improvement-loop`);
- a body with `Outcome` (the reader reads or refuses with a sharper named reason, stated from the
  code and its `whats`), `Why` (the counts: occurrences, repositories, functions blocked; the
  `corpus_kind` and the manifest digest; the sites as upstream `owner/repo@commit path:line`,
  names and positions only, no line of upstream source), a `Reproducer` section that names a
  directory and the command that shows the reason, `Done When` (failing test first, near
  misses) and `Scope` (the reader's source directory, typed with `aep plan artifact scope
  --inferred`).

The reproducer is written from scratch, with generic names, by the session that files (a person
or an agent): `corpus/reproducers/<language>/<slug>/` holds source and a `run` line. The filer
does not invent one. A code over the threshold with no reproducer directory is reported as
`waiting for a reproducer` and no story is filed. Before filing it checks the reproducer:

- `codespec read` of it gives at least one refusal with this code (the reproducer shows the gap);
- no line of it, trimmed and 20 bytes or longer, appears in any source file of a site's
  repository at its commit (the filer clones each site at its commit into `--work <dir>`, depth
  1, or uses a clone already there; the bounded-read job's clone is gone with the job), and no part
  of it names the upstream repository, its owner or its module path; either fails the filing with
  the line.

Deduplication is by tag, over every status: a code whose tagged story is draft, proposed or active
is `deduplicated`; one whose story is implemented or archived and that appears again is reported
as `reappeared` (a regression of an implemented fix) and files nothing; the summary says so, for a
person to open. The filer never moves a story, never edits an existing one and never writes a fix.

## Why

Step 4 of the loop, and the point where it stops: filing is mechanical, solving is the normal
process (critics, implement, adversary). An invented or copied reproducer is the failure to
prevent: a story with a reproducer lifted from an upstream repository puts that source into a
public repository.

## Done When

Failing tests first, over a triage fixture, a temporary planning store made for the test and a
recorded clone fixture; no network, no real reader beyond the stub of `story:corpus-bounded-read`.
None over 5 s; `aep` is called as a process and the test says so if it is absent.

- Over a fixture of five codes: A over the threshold with a valid reproducer and no story; B over
  the threshold and tagged on a draft story; C over the threshold and tagged on an implemented
  story; D over the threshold with no reproducer directory; E under the threshold. The output
  is exactly one line each: A `filed`, B `deduplicated`, C `reappeared`, D `waiting for a
  reproducer`, E `below threshold`; the store holds exactly one new story, A's, and `aep plan
  artifact validate` calls it valid.
- A second run over the same five inputs, with A's story now in the store, prints A
  `deduplicated`, B `deduplicated`, C `reappeared`, D `waiting for a reproducer`, E `below
  threshold`, and the store's files are byte-equal to after the first run: idempotent.
- Near misses, each filing nothing: a code with 5 occurrences in 1 repository; 4 occurrences in 2;
  a reproducer whose read gives no refusal; a reproducer holding one 25-byte line that is in the
  clone fixture's source; a reproducer naming the upstream owner.
- Two codes that differ and kebab to the same slug both file, with different ids (the digest
  differs); a code with characters outside `[a-z0-9 :_-]` in its slug input is stripped in the id
  and its digest keeps it distinct from a code without them.
- `--dry-run` writes nothing to the store and prints the same lines.
- `aep` refusing a write (an invalid relation, a locked store) fails the run with its exact line,
  after the codes already filed are named; nothing is filed twice on the next run.

## Scope

- Own: `crates/corpus/src/file.rs`, `crates/corpus/src/reproducer.rs`, `crates/corpus/tests/`
  (module `file` of `tests/main.rs`), `crates/corpus/src/main.rs` and `src/lib.rs` (registers
  `file`), `corpus/reproducers/` (new, empty with a `README.md` that says what goes in it and
  that nothing of an upstream repository does).
- After `story:corpus-triage-report` (its input). Writes the planning store only through `aep`,
  so no other story needs to wait on a store file.
- `story:corpus-kpi-ratchet` runs after this story: both add a subcommand to `src/main.rs` and
  `src/lib.rs` and a module to `tests/main.rs`, so one `depends_on` edge orders them.
- Narrowing of the epic's "one story per reason above a threshold": a story is filed only for a
  code that has a reproducer. This story does not write reproducers (`corpus/reproducers/` ships
  empty); the writer is the session that runs `file`, and the `waiting for a reproducer` list is
  that session's queue. The first such reproducer is a Done When of `story:corpus-loop-lane`.
- Public repository: a story it files may name a public upstream repository as an example of a
  site; it names no company, customer or person.
