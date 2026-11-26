---
format: aep.planning-md/3
id: story:release-ships-go-summaries
kind: story
status: draft
title: The release archive ships the Go summaries and the syscall barrier
summary: codespec diff from an installed release alone resolves time.Now to its effect
relations:
- decomposes: epic:release-diff-go-field-gaps
- informed_by: story:go-std-effect-summaries-common
revision: 1
---
# Story: The release archive ships the Go summaries and the syscall barrier

## Outcome

A consumer who installs only a release archive gets the Go standard-library effect summaries and
the linux/amd64 syscall barrier at the release's version. Today
`summaries/go-std.effects.json` and `summaries/go-linux-amd64.barrier.json` are in neither the
x86_64 archive nor the conformance archive (`.github/workflows/release-build.yml:76` copies
binaries only), and the `codespec` binary has none built in. `codespec diff` and
`codespec effects` take them only as `--summaries` and `--barrier` files, so a release consumer
either runs without them or builds them from an unpinned checkout.

The change:

- `release-build.yml` puts `summaries/*.json` into every platform archive under `share/codespec/summaries/`, and the archive test checks they are there.
- `codespec diff`, `effects` and `reach` look for the summaries and the barrier beside the binary (`<prefix>/share/codespec/summaries/`) when no `--summaries` / `--barrier` is given, and print which files they read on `--verbose`. An explicit flag replaces the default; `--no-default-summaries` turns it off.
- The release notes and `website/docs` say where the files are and that they carry the release's version.

## Acceptance

- From an unpacked release archive alone, with no flag, `codespec diff` over two revisions of a synthetic Go module that adds a call to `time.Now` reports the call's `time` effect, not `extern without summary`.
- `codespec diff --no-default-summaries` over the same pair reports `extern without summary` for it.
- The release archive test fails when either file is missing from an archive.

## Reported by

A consumer moving its code facts onto the 0.14.0 release (gap G5, relayed by the host operator,
2026-11-26).
