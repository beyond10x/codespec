---
format: aep.planning-md/3
id: epic:release-fact-diff
kind: epic
status: draft
title: Diff the code facts of two revisions of a service
summary: codespec diff over two linked sets of cir/1 documents, with call sites named by function, callee and ordinal, and Go net/http and echo routes recorded as http entries
revision: 2
---
# Epic: Diff the code facts of two revisions of a service

## Outcome

`codespec diff <old> <new>` compares two linked sets of `cir/1` documents, the same program read
at two revisions, and prints `codespec-diff-report/1`: the units added and removed, each changed
signature, each changed effect, each syscall gained or lost at a call site, each changed raise,
each function that became reached or unreached from the entries, and each entry added or
removed. Functions and call sites are named so that they are the same across revisions when the
code is the same: a function by module path and function key, a call site by its function, its
callee and its ordinal among that function's calls of the callee, never by line. The Go reader
records the routes a `net/http` or echo service registers as `http` entries, so the diff of a Go
service starts from what its clients can call.

## Why

A release review compares the code facts of two tags of a service. Reach over-approximates: a
call through an interface or a function value reaches every candidate of the set, so the facts
of one revision list far more than the code will do. Only the differences between two revisions
carry signal: a syscall a handler did not reach before, an effect a function did not have, a
route that appeared. On main `5f82fff9` (release 0.12.0) no part of that comparison exists:

- `codespec effects` names each site by `<file>:<line>` only (`examples/envcheck/envcheck.effects.json`:
  `"at": ["envcheck.go:13"]`), so a line moved above a call changes every site below it;
- `crates/cli/src/lib.rs` has no `diff` command and there is no diff report format;
- the Go reader records no `Module.entries` (`adapters/go/cmd/codespec-read-go/ir.go` has no
  entries field); only the Java reader records `http` entries.

The asks come from a consumer that reviews releases of Go services; its checks are the Done When
lines below.

## Order

1. `story:go-http-entries` (C4) and `story:stable-site-identity` (C2) at once: they share no file
   but `CHANGELOG.md`, whose conflicts are resolved at merge. C4 lands in the Go reader and in
   `Entry` of `crates/model/src/lib.rs`; C2 in `crates/model/src/walk.rs` and `crates/effects`.
2. `story:diff-command` (C3) after both: it names sites the way C2 defines them, and diffs the
   entries C4 records.
3. The release that ships the three, then its installation on the workstation.

## Done When

- A Go fixture whose only change between two revisions moves lines yields no delta: `codespec
  diff` exits 0 and prints nothing (`story:stable-site-identity`, `story:diff-command`).
- A Go fixture where one call changes from one syscall to another (same callee, another constant
  syscall number) reports exactly two deltas, under one call-site id: removed `{syscall_a,
  site}`, added `{syscall_b, site}` (`story:diff-command`).
- An echo fixture yields one `http` entry per route, with its method and path
  (`story:go-http-entries`).
- `codespec-diff-report/1` is defined in the crate doc of `b10x-codespec-diff`, and `codespec diff`
  is on the site's CLI reference page.
- The codespec release that ships `story:stable-site-identity`, `story:go-http-entries` and
  `story:diff-command` is installed on the workstation; `codespec --version` prints that release.

## Out of scope

- Grails entries and their diff (C5 of the consumer; `story:reach-framework-entries` covers the
  Grails wiring).
- TypeScript, Vue and Kamailio sources: codespec has no reader for them.
- A map from entries to product features: which route belongs to which feature is the consumer's.
- Entry discovery for other Go frameworks (gin, chi, gorilla/mux) and for Rust
  (`story:rust-reader-http-entries`).
