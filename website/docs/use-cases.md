---
title: Use cases
sidebar_position: 2
description: Why use codespec. Translate code with evidence, know what a dependency can do, find defects on one representation.
lede: Three problems codespec answers, the commands for each and what you get.
source: examples/orders, examples/netcheck; run with an installed codespec 0.6.1 at tag 0.6.1 on 2026-11-09
---

# Use cases

Every command on this page was run with an installed `codespec` 0.6.1
([Getting started](/docs/getting-started#install)) in a clone at tag `0.6.1`. Output is pasted
from that run.

## 1. Translate between languages with evidence

**The problem.** You want a Go service in Rust. A rewrite by hand or by a language model looks
right, but nothing shows that it behaves the same.

codespec reads the Go into `cir/1`, computes facts and writes Rust. The harness runs the original
and the outputs on the same generated inputs. [Getting started](/docs/getting-started) runs the
commands on `examples/orders`, with their output.

**What you get.** Rust for every function but `Service.Audit`, which starts a goroutine. The writer
lists it with its line and reason. The crate equals `examples/orders/out/rust`, which the harness
runs. The original Go, the Go output and the Rust output print the same bytes.

## 2. Know what a dependency can do

**The problem.** A new version of a dependency can start writing files, running programs or opening
connections. You want CI to stop when that happens.

`scripts/read-repo.sh --deps` reads your module with every package it imports. With the Linux
syscall barrier, `codespec effects` reports the effects and syscalls each function may reach.
`examples/netcheck` is a TCP server and client whose only dependency is the standard library. Here
the new version is a new Go. The 0.6.1 Go reader fails on the Go 1.27 standard library, so
`GOTOOLCHAIN` selects an earlier Go.

Write what each entry point reaches to `effects.lock`, one line per effect and syscall:

```shell-session
$ export CODESPEC=codespec CGO_ENABLED=0 GOTOOLCHAIN=go1.25.12
$ go -C examples/netcheck/source version
go version go1.25.12 linux/amd64
$ scripts/read-repo.sh --deps examples/netcheck/source work/netcheck > /dev/null
$ M=example.com/netcheck
$ codespec effects work/netcheck/*.cir.json \
    --barrier summaries/go-linux-amd64.barrier.json \
    --entry $M/cmd/server.main --entry $M/cmd/client.main \
    --report $M/cmd/server --report $M/cmd/client > work/effects.json
$ jq -r '.funcs[] | select(.name == "main") | .path as $p
    | (.effects[] | "\($p) \(.)"), (.syscalls[] | "\($p) \(.name)")' work/effects.json > effects.lock
$ grep -E ' (proc|connect|accept4)$' effects.lock
example.com/netcheck/cmd/client proc
example.com/netcheck/cmd/client connect
example.com/netcheck/cmd/server proc
example.com/netcheck/cmd/server accept4
example.com/netcheck/cmd/server connect
```

Only the server reaches `accept4`. Review the file and commit it. The CI step runs the same
commands with `jq` writing `work/effects.now`, then compares. Here it ran after
`export GOTOOLCHAIN=go1.26.10` and `rm -rf work`:

```shell-session
$ diff effects.lock work/effects.now
13a14,17
> example.com/netcheck/cmd/client epoll_create1
> example.com/netcheck/cmd/client epoll_ctl
> example.com/netcheck/cmd/client epoll_pwait
> example.com/netcheck/cmd/client eventfd2
28a33
> example.com/netcheck/cmd/client mprotect
35a41
> example.com/netcheck/cmd/client prctl
65a72,75
> example.com/netcheck/cmd/server epoll_create1
> example.com/netcheck/cmd/server epoll_ctl
> example.com/netcheck/cmd/server epoll_pwait
> example.com/netcheck/cmd/server eventfd2
80a91
> example.com/netcheck/cmd/server mprotect
87a99
> example.com/netcheck/cmd/server prctl
```

`diff` exits 1, so the step fails. With Go 1.26.10, each entry point reaches six more syscalls. A
new `proc`, `net` or `fs.write` line fails it the same way. To accept a change, copy
`work/effects.now` to `effects.lock`.

**What you get.** A reviewed list of what your program may reach, and a failing build when it
changes. Read the sets as "may reach":
[Map a repository to syscalls](/docs/guides/map-a-repository-to-syscalls#5-read-a-dispatch-site)
explains why they are wide.

## 3. Find defects on one representation

This use is planned. Release 0.6.1 has no command for it. [Status](/docs/status) lists what exists
today.

**The problem.** A static check is usually written for one language, on its source text or one
compiler's trees.

The planned checks read `cir/1` only: bodies, calls inside `foreign` statements, opaque functions,
method sets and facts. None reads source text. The first eight find unbounded growth on a receive
path, `math/rand` near token or key code, and integers parsed from text and narrowed unchecked. They
also find asymmetric case folding, clones that share a slice, dropped errors, a lock without its
unlock, and unreached functions.

They will be tested on [google/capslock](https://github.com/google/capslock), read with every
package it imports, with each finding triaged. Every reader produces the same `cir/1`, so the checks
will read Rust, Java and Groovy documents too. They will be tested on Go only.

**What you get.** One set of checks for every language codespec reads.

## Neighbours

[Capslock](https://github.com/google/capslock) from Google answers use 2 for Go packages. Its
[README](https://github.com/google/capslock#readme) calls it "a capability analysis CLI for Go
packages that informs users of which privileged operations a given package can access". It works
"by following transitive calls to privileged standard library operations". Its
[`-output` flag](https://github.com/google/capslock/blob/main/docs/README.md#-output) takes
`compare`, which "lets you identify which of the capabilities changed between package version".

How use 2 differs:

- Capslock stops at privileged standard library operations. codespec reads the standard library too
  and stops at syscalls. It names each syscall and the line of your function that leads there.
- codespec analyses `cir/1`, not Go. Today the only barrier is Go on linux/amd64.
