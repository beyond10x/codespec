---
format: aep.planning-md/3
id: story:native-run-harness
kind: story
status: draft
title: A deterministic container run harness on the build node
summary: codespec-native-run builds and runs a command in a pinned, network-less container through slot and returns a canonical record; a microVM step is named, not built.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:ecmascript-number-semantics
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
  path: containers/base
- confidence: inferred
  path: crates/native-run
- confidence: inferred
  path: scripts/check-parts.txt
revision: 5
---
# Story: A deterministic container run harness on the build node

## Outcome

`codespec-native-run`, a Rust crate `crates/native-run`, builds and runs a command in a container
on the build node and returns its standard output, exit code, wall time and `inputs_sha256`, the
digest of everything that decided the result: the sha256 of the canonical JSON of the image digest,
the command, the limits and the sha256 of the mounted work directory's file list and contents. The WebAssembly runner, the x86-64 runner, the ECMAScript
oracle and the speed comparison call it; none of them starts a container itself.

```console
codespec-native-run --image <name> --mount <dir>:/work:ro --cpus 1 --memory 1g --timeout 60s -- <command>
```

## Why

Three stories need to run code that was just generated, in a place that cannot hurt the
workstation and gives the same answer twice. The build node has x86_64, 32 cores, `docker` and
`podman`, and no `/dev/kvm`: containers now, a microVM later. Generated assembly and Wasm are
untrusted input to an interpreter of a different kind than a source file: a run must not see the
network, the host's files or the host's clock, and must end.

## Done When

- A failing test first: `second_run_is_byte_equal` runs `sh -c 'echo hi'` twice in the base image and
  compares the two records without their `wall_ns` field (the record's one field that varies, below);
  it fails until the record omits everything else that varies (container id, timestamps, host
  paths). The node's CPU model is the same on both runs and stays in the compared part.
- Images are built from `containers/<name>/Containerfile` files, and the harness owns the
  mechanism and one image: `containers/base`, a base image pinned by `@sha256:` digest with a shell
  and nothing else. Every other image belongs to the story that needs its tools and pins each by
  version and the SHA-256 of its release archive: `containers/node` (`node`) to
  `story:ecmascript-run-oracle`, `containers/wasm` (`wasm-tools`, `wasmtime`, `node`) to
  `story:wasm-writer`, `containers/x86-64` (`binutils`, `cc`) to `story:x86-64-writer`. The
  mechanism refuses `latest`, an unpinned base and an `apt-get` without a pinned snapshot. The image
  tag is the digest of the Containerfile and its inputs; a cached image is reused and a changed one
  is rebuilt, never silently. Building runs on the node through `slot` and takes the network it
  needs; running never does.
- The harness is for correctness runs: the oracle against `node`, the writers against the evaluator.
  It does not measure speed for any comparison (swell's `bench/` does); its record holds a wall
  time only as a diagnostic.
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
  exit code, verdict, `stdout_sha256` (the digest of the stdout), the stdout, `inputs_sha256` (as in
  the Outcome) and `wall_ns` (wall time in nanoseconds) and the node's CPU model. `wall_ns` is the
  only field excluded when two records are compared, and it is in neither `stdout_sha256` nor
  `inputs_sha256`. A test changes one byte of a mounted file and expects a different
  `inputs_sha256`, and changes only the wall time and expects the same one.
- The microVM step is named, not built: the harness has one `Isolation` value, `container`, and a
  test refuses `microvm` with the message `microvm is a later step of epic:ecmascript-to-native; no
  /dev/kvm on this node`. No story for it exists yet, so none is named. Nothing else is stubbed.
- The tests run within budget: each under 60 s, the step under 120 s; the tests that need a
  container are marked and run on the node, in one shared fixture (one image set, built once per
  run, keyed by digest), never an image or a container per test. The ten slowest tests print.

## Scope

- Own: `crates/native-run` (new), `containers/base` (new), root `Cargo.toml` `members`,
  `Cargo.lock`, `scripts/check-parts.txt`, `Taskfile.yml`, `AGENTS.md`, `CHANGELOG.md`.
- Shares `AGENTS.md` and `CHANGELOG.md` with `story:ecmascript-number-semantics`, which comes
  first (this story depends on it for that reason only), and root `Cargo.toml`, `Cargo.lock`,
  `Taskfile.yml` and `scripts/check-parts.txt` with every later story of the epic; this story comes
  before `story:ecmascript-swell-surface` and the stories after it. The other `containers/`
  directories are those stories'.
