---
format: aep.planning-md/3
id: story:native-run-harness
kind: story
status: draft
title: A deterministic container run harness on the build node
summary: codespec-native-run builds and runs a command in a pinned, network-less container through slot and returns a canonical record; a microVM step is named, not built.
relations:
- decomposes: epic:ecmascript-to-native
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
  path: containers
- confidence: inferred
  path: crates/native-run
- confidence: inferred
  path: scripts/check-parts.txt
revision: 2
---
# Story: A deterministic container run harness on the build node

## Outcome

`codespec-native-run`, a Rust crate `crates/native-run`, builds and runs a command in a container
on the build node and returns its standard output, exit code, wall time and the digest of
everything that decided the result. The WebAssembly runner, the x86-64 runner, the ECMAScript
oracle and the speed comparison call it; none of them starts a container itself.

```console
codespec-native-run --image <name> --mount <dir>:/work:ro --cpus 1 --memory 1g --timeout 60s -- <command>
```

## Why

Four stories need to run code that was just generated, in a place that cannot hurt the
workstation and gives the same answer twice. The build node has x86_64, 32 cores, `docker` and
`podman`, and no `/dev/kvm`: containers now, a microVM later. Generated assembly and Wasm are
untrusted input to an interpreter of a different kind than a source file: a run must not see the
network, the host's files or the host's clock, and must end.

## Done When

- A failing test first: `second_run_is_byte_equal` runs `sh -c 'echo hi'` twice in a stub image and
  compares the two records; it fails until the record omits everything that varies (container id,
  timestamps, host paths).
- The images are built once, from `containers/<name>/Containerfile` files in the repository: a base
  image pinned by `@sha256:` digest, each tool (`wasm-tools`, `wasmtime`, `node`, `binutils`,
  `cc`) pinned by version and the SHA-256 of its release archive, no `latest`, no `apt-get`
  without a pinned snapshot. The image tag is the digest of the Containerfile and its inputs; a
  cached image is reused and a changed one is rebuilt, never silently. Building runs on the node
  through `slot` and takes the network it needs; running never does.
- Every run has: `--network none`, a read-only root, a tmpfs for `/tmp`, the work directory
  mounted read-only except one output directory, no capabilities, a pids limit, `--cpus`,
  `--memory` and `--timeout` from the call. A run that reaches the limit ends with a named
  verdict (`timeout`, `oom`), not a hang; a test holds each with a one-second limit.
- podman and docker are both supported through one function (`rootless podman` first when it
  exists); a test per engine runs on the node, the others skip with a printed reason and are not
  counted as passed.
- Every call goes through `slot` on the node (`slot run --timeout <T> …`); the harness never calls
  `docker` or `podman` outside a slot job, and its own process, which waits, starts without the
  `slot` prefix (`~/beyond10x/AGENTS.md`, "Scripts that do not build stay off slot"; a build of an
  image takes the prefix). A job longer than 10 minutes names its time (at most 30).
- The record (`codespec-native-run/1`, JSON, canonical) holds: image digest, command, limits,
  exit code, verdict, stdout digest, the stdout, wall time in nanoseconds and the node's CPU model.
  The wall time is in the record, not in the digest of the output.
- The microVM step is named, not built: the harness has one `Isolation` value, `container`, and a
  test refuses `microvm` with `no /dev/kvm on this node` and the story that would add it. Nothing
  else is stubbed.
- The tests run within budget: each under 60 s, the step under 120 s; the tests that need a
  container are marked and run on the node, in one shared fixture (one image set, built once per
  run, keyed by digest), never an image or a container per test. The ten slowest tests print.

## Scope

- Own: `crates/native-run` (new), `containers/` (new), root `Cargo.toml` `members`, `Cargo.lock`,
  `scripts/check-parts.txt`, `Taskfile.yml`, `AGENTS.md`, `CHANGELOG.md`.
- Shares `Taskfile.yml` and `scripts/check-parts.txt` with every other story of the epic; this
  story first.
