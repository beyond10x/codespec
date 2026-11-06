---
format: aep.planning-md/3
id: story:conformance-kit-release
kind: story
status: active
title: The kit in releases and the adapter guide
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:codespec-conform
scope:
- confidence: cited
  path: .github/workflows/release-build.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: website/docs/guides/add-an-adapter.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 4}
---
# Story: The kit in releases and the adapter guide

## Outcome

Each release ships `codespec-conformance-<version>.tar.gz` beside the binaries, and
`website/docs/guides/add-an-adapter.md` tells an outside author to run `codespec conform`.

## Acceptance

- `task conformance-kit` packs `conformance/` and `schemas/cir-1.features.json` into
  `target/release-assets/codespec-conformance-<version>.tar.gz`, unpacks it into a scratch
  directory and runs `codespec conform --lang go --role writer --kit <unpacked> --out <scratch>/go-writer.json` with exit 0; it is
  in `task check`, so it holds before any tag.
- `.github/workflows/release-build.yml` runs `task conformance-kit`, adds the archive to the
  artifact and to `SHA256SUMS`.
- The guide's "What the gate needs" section shows the command
  `codespec conform --lang go --role writer --out work/go-writer.json` and the `summary` line of the
  report it wrote, both taken from a run in this repository, and names `--describe` and
  `codespec-run-<lang>`.

## Scope

- `.github/workflows/release-build.yml`, `website/docs/guides/add-an-adapter.md`, `AGENTS.md`
  § Release, `CHANGELOG.md`.
- `Taskfile.yml`: the coordinator's.
