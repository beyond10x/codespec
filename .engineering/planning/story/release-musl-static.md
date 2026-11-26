---
format: aep.planning-md/3
id: story:release-musl-static
kind: story
status: active
title: A static musl Linux archive beside the gnu one
summary: codespec --version runs from the musl archive alone in a plain alpine container; mimalloc only where it is not slower
relations:
- decomposes: epic:release-diff-go-field-gaps
- depends_on: story:release-ships-go-summaries
scope:
- confidence: cited
  path: .github/workflows/release-build.yml
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/cli/Cargo.toml
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: cited
  path: website/docs/getting-started.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T15:11:24Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-26T15:11:24Z", actor: "human:operator", revision: 5}
---
# Story: A static musl Linux archive beside the gnu one

## Outcome

Each release carries an `x86_64-unknown-linux-musl` archive whose Rust binaries are statically
linked, beside the `x86_64-unknown-linux-gnu` archive. A host with an older glibc (2.34 on a
build node today) runs it without the warnings or failures a gnu binary built against a newer glibc
gives. Consumers pin the musl archive.

The change:

- `.github/workflows/release-build.yml` builds the Rust binaries of the release for `x86_64-unknown-linux-musl` and packs them like the gnu archive (same file list, the same `SHA256SUMS`); the Go binaries are built with `CGO_ENABLED=0`, so they are static already.
- A CI check fails when a binary of the musl archive is not static (`file` or `readelf -d` shows no `NEEDED` entry and no interpreter) or when `codespec --version` (and each other binary's `--version` or usage line) does not run in a plain `alpine` container.
- mimalloc as the global allocator in the binaries where allocation dominates (`codespec query`, `codespec effects`, `codespec diff` over a large set): measured against the musl default allocator on one recorded large input; kept only where it is not slower, and the numbers go into the commit message and `docs`.
- The install page (`website/docs/getting-started.md`) and `AGENTS.md` § Release name the musl archive and say consumers pin it.

## Acceptance

- The release-build workflow produces a musl archive; the static check passes on it and fails on the gnu archive (a test case for both sides).
- `codespec --version` prints the release version inside `alpine` from the musl archive alone.
- A recorded timing of the chosen allocator against the default on one large input, in the commit message.

## Reported by

The owner's approval of 2026-11-25 (static musl Linux releases for every Rust tool of the
organization, mimalloc where allocation-heavy), relayed by the host operator 2026-11-26.
