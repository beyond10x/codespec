---
title: Static analysis
sidebar_position: 4
description: How codespec effects answers what a function may do outside its own memory, from extern declarations, effect summaries and the syscall barrier, and what it reports as unknown.
lede: codespec effects reports per function the effects and syscalls it may reach, each with the line that leads there, and lists every call it cannot account for as unknown instead of taking it as pure.
source: docs/static-analysis.md, crates/effects/src/lib.rs, crates/effects/src/set.rs, crates/cli/src/lib.rs, summaries/go-std.effects.json, summaries/go-linux-amd64.barrier.json, examples/envcheck, examples/netcheck, crates/cli/tests/barrier.rs, commit 827b232
---

# Static analysis

## The question

`codespec effects` answers one question per function: **what may this function do outside its own
memory?** The answer lists whether it reads the environment, the file system, the network or the
clock, prints, or starts a process. Read down to the operating system, it also lists the syscalls
the function may reach.

The analysis runs on `cir/1`, so one implementation serves every language a reader exists for.
Today the Go reader produces everything it needs. The design is in `docs/static-analysis.md`.

## Layers

codespec does not guess what code outside the module does. It works in three layers:

| layer | what | where it comes from |
|---|---|---|
| extern declaration | the module lists every outside function it calls (`Module.externs`: import path, name, parameter and result types); a call of one is an `extern` expression | the reader, from its language's own type checker (`go/types` for Go) |
| effect summary | per outside function, the effects it may have, from a closed vocabulary | a hand-written `codespec-effects/1` file (`summaries/go-std.effects.json`), or the function itself when its package is read into the same set |
| unknown | a call with no summary, a statement the reader could not express, a call through an interface with no implementation in the set | the analysis lists it with a position, never takes it as pure |

The third row applies the repository's rule for readers to analysis: a construct that cannot be
expressed is kept and reported. It is not approximated.

### Example: `envcheck`

`examples/envcheck/source/envcheck.go` reads `APP_MODE`, builds a banner, prints it and writes it
to a file. Its `cir/1` declares five externs (`os.Getenv`, `strconv.Itoa`, `strings.ToLower`,
`strings.ToUpper`, `strings.TrimSpace`). The report with `summaries/go-std.effects.json` is
`examples/envcheck/envcheck.effects.json`, and `task examples-effects` checks it:

| function | effects | unknown |
|---|---|---|
| `Mode` | `env` (at `envcheck.go:13`) | none |
| `Label` | none: pure | none |
| `Banner` | `env` (at `envcheck.go:27`, the call of `Mode`) | none |
| `Announce` | `env`, `stdout` | none |
| `Save` | `env`, `fs.write` | none |

## The effect vocabulary

The vocabulary is closed (`EFFECTS` in `crates/effects/src/lib.rs`). A summary or barrier with any
other effect is refused.

| effect | meaning |
|---|---|
| `env` | reads or writes the process environment or arguments |
| `fs.read` | reads the file system |
| `fs.write` | writes the file system |
| `io` | reads or writes a file descriptor whose origin the analysis does not follow: a file, a socket or a standard stream |
| `net` | opens or uses a network connection |
| `proc` | starts, signals or ends a process, including its own exit |
| `rand` | reads a random source |
| `stdin` | reads standard input |
| `stdout` | writes standard output or standard error |
| `time` | reads a clock or sleeps |

A summary entry with `"effects": []` declares the function pure.

## Formats

| format | what | file |
|---|---|---|
| `codespec-effects/1` | a summary: `{path, name, effects}` per outside function | `summaries/go-std.effects.json`, 37 Go standard library functions |
| `codespec-barrier/1` | where the source ends: functions that enter the operating system and what each syscall does | `summaries/go-linux-amd64.barrier.json` |
| `codespec-effects-report/1` | what `codespec effects` prints: per function `effects`, `unknown`, and when present `syscalls`, `effect_sites`, `dispatch`, `unknown_roots` | stdout |

All three are read strictly: unknown fields, unsorted or repeated entries and other formats are
refused. The report is canonical JSON: the same modules, summaries and barrier give the same bytes.
The crate doc of `b10x-codespec-effects` defines every field ([crates](/docs/reference/crates)).

## Linking a set by import path

One `cir/1` document holds one package. `codespec effects` takes several and reads them as one
program:

```shell-session
$ codespec effects a.cir.json b.cir.json … --summaries go-std.effects.json --barrier go-linux-amd64.barrier.json
```

| rule | effect |
|---|---|
| a document's `path` is its import path | set by the Go reader from `GOROOT` or `go.mod`; `scripts/read-repo.sh --deps` sets it for every package |
| an extern whose `path` is the import path of a document in the set | is an edge to that document's function |
| a function both a summary and a document define | the document wins; the report lists it under `notes` |
| `--summaries` given more than once | where two name one function, the first counts |
| `--report <import path>` | prints only that module's functions; the others are still analysed |

Statements `cir/1` cannot express still count. The Go reader lists the calls inside a `foreign`
statement (`foreign.calls`). It keeps a refused or body-less function as an opaque function with
its calls (`Module.opaque`), so the call graph has no hole where a body is not expressible. It also
records each named type's method set (`Module.methods`), every `//go:linkname` (`Module.linknames`)
and the calls made by package variable initialisers (`Module.inits`).

Effects and syscalls are carried to a fixed point over one call graph of every function of the set.
The graph is walked by strongly connected components (iterative Tarjan), callees first, in time
linear in nodes plus edges (`crates/effects/src/set.rs`).

## Dispatch

A call through an interface or a function value has no single target. codespec resolves it in
one of two ways, chosen with `--dispatch`:

| `--dispatch` | a method call through an interface reaches | a call of a function value reaches | default |
|---|---|---|---|
| `cha`: class-hierarchy analysis | the method of every type in the set whose method set holds all the interface's methods with identical signatures | every function, method or function literal in the set that is used as a value and has the same signature | without entries |
| `rta`: rapid type analysis | the same, but only types that code reached from the entries makes | the same, but only values that reached code uses | with `--entry` or `--exported` |

The report's `dispatch` entry gives, per such call, its position and how many candidates it may
reach. A call with no candidate in the set is `unknown`.

## Entries and dead code: `codespec reach`

An entry is a function the program starts from: `--entry <path>.<name>` (or
`<path>.<type>.<method>`), every exported function of `--exported <module>`, or `main.main`
when neither is given. Go `init` functions and variable initialisers are always entries.
`codespec reach` lists, per module, the functions the entries reach and those they do not.
`rta` narrows dispatch, so it reaches fewer functions than `cha`.

[Map a repository to syscalls](/docs/guides/map-a-repository-to-syscalls#6-a-worked-example-entries-reach-and-trace)
runs `effects`, `reach` and `trace` on [`examples/netcheck`](/docs/examples/netcheck).

## The syscall barrier

The Go standard library is Go down to a few functions without a Go body. Those functions are the
barrier: below them is assembly, so their meaning is written by hand, once, in
`summaries/go-linux-amd64.barrier.json`.

The chain for a syscall in Go 1.27.1, linux/amd64, read in `$(go env GOROOT)/src`:

| step | where |
|---|---|
| `syscall.openat` calls `Syscall6(SYS_OPENAT, …)` with the syscall number as a constant | `syscall/zsyscall_linux_amd64.go:98` |
| `syscall.Syscall6` calls `RawSyscall6` | `syscall/syscall_linux.go:95` |
| `syscall.RawSyscall6` calls `linux.Syscall6` | `syscall/syscall_linux.go:63` |
| `internal/runtime/syscall/linux.Syscall6` is declared without a body | `internal/runtime/syscall/linux/syscall_linux.go:16` |
| its body is assembly | `internal/runtime/syscall/linux/asm_linux_amd64.s:27` |

The barrier file has four parts:

| part | count | what |
|---|---|---|
| `numbered` | 13 | functions whose argument 0 is the syscall number (`syscall.Syscall6`, `internal/runtime/syscall/linux.Syscall6`, `golang.org/x/sys/unix.Syscall`, …); a call with a constant there makes that syscall, and the body is not followed |
| `fixed` | 44 | body-less runtime functions with fixed syscalls (`runtime.getpid` → `getpid`) |
| `pure` | 29 | packages whose body-less functions are assembly that makes no syscall and calls nothing |
| `syscalls` | 385 | every linux/amd64 syscall by name (golang.org/x/sys v0.49.0 names) with its number and effects |

A body-less function with a `//go:linkname` resolves in either direction, for example
`internal/poll.runtime_pollWait` → `runtime.poll_runtime_pollWait`. With `--barrier`, each report
entry gains `syscalls`: each syscall with the positions of the function's own calls that lead
there.

The fixture `crates/cli/tests/fixtures/readfile` has one function, `Load`, which returns
`os.ReadFile(path)`. `crates/cli/tests/barrier.rs` reads it with every package it imports and finds
`openat` and `read` at `readfile.go:8`, the `os.ReadFile` line.

### What the barrier cannot say

`read`, `write` and `close` take a file descriptor. The syscall is the same for a file, a socket
and standard input. The analysis does not follow a descriptor back to the call that made it, so
these map to `io`, not to `fs.read` or `net`. `openat` maps to `fs.read` and `fs.write`, `connect`
and `socket` to `net`. 193 of the 385 syscalls map to no effect (for example `getpid`).

## What `unknown` means

An `unknown` entry says the function's `effects` and `syscalls` may be incomplete, and where. It is
not an error and not a claim that the function does IO. The reasons
(`crates/effects/src/lib.rs`):

| reason | when |
|---|---|
| `extern without summary: <path>.<name>` | no document of the set and no summary names the extern |
| `foreign statement` | the reader kept source it could not express and did not list its calls |
| `method on undeclared type: <type>.<name>` | a method call whose receiver type has no such method in the set |
| `calls <func> which is incomplete` | a function it calls, or one a dispatch site may reach, has an `unknown` entry itself |
| `call of undeclared function: <name>` | a call names no function of the module (a reader defect) |
| `not in its module: <path>.<name>` | a call of a module of the set that does not declare the function |
| `interface call with no implementation in the set: <iface>.<name>` | no type of the set has the interface's method set |
| `function value with no candidate in the set: <sig>` | no function of the set with that signature is used as a value |
| `function without a body: <path>.<name>` | a body-less function that neither the barrier nor a linkname resolves |
| `syscall number not constant: <path>.<name>` | a barrier function called with a number that is not a constant |
| `syscall number <n> has no name` | the barrier table does not name the number |

A function with no effects, no syscalls and no `unknown` entry is pure as far as the summaries and
the barrier say. `--roots` adds `unknown_roots`: every `unknown` reason of every function it
reaches, counted, most frequent first, which says what to summarise or read next.

## One call, with its branches: `codespec trace`

`codespec trace <documents…> --from <path>.<function>` prints `codespec-trace/1`, the call tree
from one function on the same call graph:

| in the source | in the tree |
|---|---|
| each `if`, `switch` and loop | a branch node with its condition |
| each call | expanded once; a repeat is a `ref` node |
| a call through an interface or a function value | a `dispatch` node listing its candidates |
| syscalls and summarised externs | the leaves |
| the tree past `--depth` (default 8) | a `cut` node |

[Call trees](/docs/examples/call-trace) draws it for `envcheck.Announce`.

## Measured: a third-party Go library

A third-party Go library of 6 packages and the 228 packages it imports, 234 documents in all,
were read in 27 s. `codespec effects` over the set took 3.6 s, reporting the library's packages.
Its client entry point reaches 74 syscalls, its server entry point 75 (the same plus `accept4`).
The library and its report are kept outside this repository.

## Current limits

| limit | consequence |
|---|---|
| dispatch is wide: `cha` takes the whole set, `rta` every type reached code makes | wide sets: with `cha`, a call through `io.Writer` or of a `func() error` value reaches every candidate in all 234 packages of the measured set, so most functions that do IO share one large strongly connected component and so one large set (`docs/static-analysis.md` § 5d) |
| file descriptors are not followed | `read`, `write`, `close` are `io`, never `fs.read` or `net` |
| generic interfaces and caller-implemented interfaces | `unknown`: on the measured library's paths, 66 interface calls with no implementation in the set |
| function values from outside the set or of generic signature | `unknown`: 59 on the measured library's paths |
| runtime functions that call back into Go (`systemstack`, `mcall`, `reflectcall`, `makeFuncStub`) | `unknown`: 27 body-less functions on the measured library's paths |
| one barrier | Go on linux/amd64 only; other readers produce no `foreign.calls`, so their `foreign` statements are `unknown` |
| the summaries are claims | not yet checked against a run under `strace` or seccomp (`docs/static-analysis.md` § 6) |

To run it on a repository of your own, see [Map a repository to syscalls](/docs/guides/map-a-repository-to-syscalls).
