# codespec — the musl release archive and its allocator (2026-11-26)

## The archive

Each release carries `codespec-<tag>-x86_64-unknown-linux-musl.tar.gz` beside the gnu archive, with
the same files. Consumers on x86-64 Linux pin it: a gnu binary needs the glibc of its build host or
newer, and a build node with glibc 2.34 warns or fails on it. In the musl archive every binary is
static: the Rust ones built with `--target x86_64-unknown-linux-musl`, the Go ones with
`CGO_ENABLED=0`, `codespec-read-odin` linked with `-extra-linker-flags:-static`.
`scripts/release-static-check.sh archive` fails the release job when one of them has an interpreter
or a `NEEDED` entry, or when its `--version` (or usage line) does not run in a plain `alpine`
container with no network; `codespec --version` must print `codespec <tag>` there. The script's
`test` mode, run by the lint part, builds one program for the musl target and one for the gnu
target and checks that the first passes and the second fails. The BEAM programs of the archive
are `sh` scripts that need Erlang on the host; the check covers the ELF files.

## The allocator

`codespec` built for musl allocates with mimalloc (`crates/cli/src/main.rs`); every other target
keeps the system allocator, and the other binaries of the archive keep musl's, as they allocate
little. The `cc` crate compiles mimalloc's C with `musl-gcc` (`x86_64-linux-musl-gcc`); the release
job installs `musl-tools` for it. musl's own allocator made the three commands that build large in-memory fact bases slower.

Input: 157 `cir/1` documents, 90 MB, from `scripts/read-repo.sh --deps adapters/go <out>`: the Go
reader's own packages and every package they import, the standard library included, read with the
musl `codespec` and a `codespec-read-go` built with go1.27.1 on the workstation. Summaries and
barrier: `summaries/go-std.effects.json` and `summaries/go-linux-amd64.barrier.json`
(`--no-default-summaries --summaries … --barrier …`). Both binaries: `cargo build --release
--target x86_64-unknown-linux-musl -p b10x-codespec-cli`, one without and one with mimalloc 0.1.52.
Runs interleaved, three per command and binary, on a load average of 0.9 to 1.5; the median wall
time, the CPU time (`perf stat -e task-clock`) of the same run, and the peak resident set (`VmHWM`)
of an earlier run of five interleaved rounds. Both binaries print the same bytes.

| command | musl malloc | mimalloc | change | peak RSS musl → mimalloc |
|---|---|---|---|---|
| `query -f crates/query/checks/unreached.cq` | 16.30 s | 9.13 s | −44 % | 3.91 GB → 4.20 GB |
| `effects` | 3.91 s | 2.88 s | −26 % | 0.70 GB → 0.83 GB |
| `diff --format json <set> <set>` | 7.62 s | 4.81 s | −37 % | 1.10 GB → 1.03 GB |

mimalloc is kept: it is faster on all three. It costs a larger peak resident set on `query` (+7 %)
and `effects` (+18 %).
