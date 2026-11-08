---
title: Map a repository to syscalls
sidebar_position: 3
description: Read a Go repository with every package it imports, run codespec effects over the set with the Linux syscall barrier, and read the effects, syscalls, dispatch sites and unknown reasons it reports.
lede: Two commands take a Go repository and its imports, the standard library included, to a report of the syscalls each function may reach and the line that leads there.
source: scripts/read-repo.sh, crates/cli/src/lib.rs, crates/effects/src/lib.rs, summaries/go-std.effects.json, summaries/go-linux-amd64.barrier.json, examples/envcheck, crates/cli/tests/fixtures/readfile, commit 827b232; every command run in this repository on 2026-11-08 with codespec 0.4.0 and Go 1.27.1 linux/amd64
---

# Map a repository to syscalls

This guide runs [static analysis](/docs/concepts/static-analysis) on Go code: first one package
with hand-written summaries, then a whole module with every package it imports, down to the
syscall barrier. Every command was run from the repository root after `task build`
([Getting started](/docs/getting-started)), with codespec 0.4.0 and Go 1.27.1 on linux/amd64.
Output is pasted from that run; `jq` lines shorten the JSON reports.

## What you need

| what | why |
|---|---|
| a codespec checkout after `task build` | `scripts/read-repo.sh` uses `target/debug/codespec` and `target/adapters` (or `$CODESPEC` and `$CODESPEC_ADAPTERS`) |
| Go on `PATH` | the Go reader type-checks with `go/types`; `go list -deps` finds the imports |
| a Go module: a directory with `go.mod` | `read-repo.sh` runs `go mod download` and `go list` in it |
| linux/amd64 | the only barrier is `summaries/go-linux-amd64.barrier.json` |

## 1. One package, summaries only

Read one package and report its effects from `summaries/go-std.effects.json`, 37 standard library
functions summarised by hand. `examples/envcheck` reads an environment variable, prints and writes
a file:

```shell-session
$ target/debug/codespec read --lang go examples/envcheck/source --out work/envcheck.cir.json --adapters target/adapters
$ target/debug/codespec effects work/envcheck.cir.json --summaries summaries/go-std.effects.json > work/envcheck.effects.json
$ jq -c '.funcs[] | {name, effects, unknown}' work/envcheck.effects.json
{"name":"Mode","effects":["env"],"unknown":[]}
{"name":"Label","effects":[],"unknown":[]}
{"name":"Banner","effects":["env"],"unknown":[]}
{"name":"Announce","effects":["env","stdout"],"unknown":[]}
{"name":"Save","effects":["env","fs.write"],"unknown":[]}
```

The report is byte-equal to the checked-in `examples/envcheck/envcheck.effects.json`
(`task examples-effects` checks it). `Label` has no effects and no `unknown`: pure as far as the
summaries say. `Save`'s statement `return os.WriteFile(…, 0o644)` is a refusal in the `cir/1`
(`fs.FileMode` has no `cir/1` type), but the reader lists the calls inside it, so `os.WriteFile`
still counts as `fs.write`.

This mode stops at the summaries. A call of a standard library function the 37 do not name is
`extern without summary`, reported as `unknown`.

## 2. Read the repository with every import

`scripts/read-repo.sh --deps <repo> <out>` reads every package `go list -deps ./...` names, the
standard library included, in parallel, one `cir/1` document per package, each with its import
path as `path`. Run on the fixture `crates/cli/tests/fixtures/readfile` (module
`example.com/readfile`, one function `Load` returning `os.ReadFile(path)`):

```shell-session
$ scripts/read-repo.sh --deps crates/cli/tests/fixtures/readfile work/readfile
cmp | ok | read 0 of 0 functions, n/a | refusals 5
errors | ok | read 2 of 8 functions, 25.0% | refusals 22
example.com/readfile | ok | read 1 of 1 functions, 100.0% | refusals 0
internal/abi | ok | read 10 of 22 functions, 45.5% | refusals 115
…
os | ok | read 38 of 96 functions, 39.6% | refusals 424
…
runtime | ok | read 186 of 722 functions, 25.8% | refusals 5490
…
syscall | ok | read 29 of 249 functions, 11.6% | refusals 675
time | ok | read 53 of 173 functions, 30.6% | refusals 415
unicode/utf8 | ok | read 3 of 19 functions, 15.8% | refusals 54
unsafe | ok | read 0 of 0 functions, n/a | refusals 9
```

| column | meaning |
|---|---|
| 1 | the import path; the document is `<out>/<import path with / as _>.cir.json` |
| 2 | `ok`, or `FAIL` and the reader's first error line; any `FAIL` makes the script exit 1 |
| 3 | `codespec measure`: functions read with no `foreign` statement, of all functions |
| 4 | refusals: constructs kept as `foreign` with a reason |

The fixture gives 54 documents, 13 MB, in 2.3 s on the machine this page was written on. A low
"read" share does not stop the analysis: a refused function stays in the document as an opaque
function with its calls (`Module.opaque`), and a `foreign` statement lists its calls.

For your repository, replace the fixture with its directory: `scripts/read-repo.sh --deps
~/src/myservice work/myservice`. The lines are also written to `<out>/read.txt`.

## 3. Run effects over the set, with the barrier

Pass every document, the summaries and the barrier. `--report` limits the output to one module;
every module is still analysed:

```shell-session
$ target/debug/codespec effects work/readfile/*.cir.json \
    --summaries summaries/go-std.effects.json \
    --barrier summaries/go-linux-amd64.barrier.json \
    --report example.com/readfile > work/readfile.report.json
$ jq -r .module work/readfile.report.json
54 modules
$ jq -c '.funcs[] | {name, effects, syscalls: (.syscalls | length)}' work/readfile.report.json
{"name":"Load","effects":["env","fs.read","fs.write","io","proc","time"],"syscalls":55}
$ jq -c '.funcs[0].syscalls[] | select(.name == "openat" or .name == "read")' work/readfile.report.json
{"name":"openat","at":["readfile.go:8"]}
{"name":"read","at":["readfile.go:8"]}
```

This ran in 0.5 s. `at` is the line of `Load` that leads to the syscall: `readfile.go:8`, the
`os.ReadFile` call. `crates/cli/tests/barrier.rs` checks exactly this.

| report field | what it says |
|---|---|
| `effects` | the effects of every syscall reached (per the barrier), every summarised extern and every function called |
| `syscalls` | each syscall reached, with the function's own call positions that lead there |
| `effect_sites` | the same per effect |
| `unknown` | why the sets may be incomplete, with a position; here `calls os.ReadFile which is incomplete` |
| `dispatch` | per call through an interface or a function value, how many functions of the set it may reach |
| `notes` | a function both a summary and a document define; the document is used. Here 12 (`os.ReadFile`, `os.Getenv`, …) |

The summaries still matter for externs whose package is not in the set. With every import read, the
documents replace them.

## 4. Read what is unknown

`--roots` adds `unknown_roots` to each reported function: every `unknown` reason of every function
it reaches, counted, most frequent first.

```shell-session
$ target/debug/codespec effects work/readfile/*.cir.json \
    --summaries summaries/go-std.effects.json \
    --barrier summaries/go-linux-amd64.barrier.json \
    --report example.com/readfile --roots > work/readfile.roots.json
$ jq '.funcs[0].unknown_roots | length' work/readfile.roots.json
33
$ jq -r '.funcs[0].unknown_roots[].reason | sub(":.*"; "")' work/readfile.roots.json | sort | uniq -c | sort -rn
     20 function without a body
      5 interface call with no implementation in the set
      5 function value with no candidate in the set
      3 method on undeclared type
```

| root | what to do |
|---|---|
| `function without a body: runtime.systemstack`, `runtime.mcall`, `runtime.reflectcall`, … | nothing yet: runtime functions in assembly that call back into Go; the barrier does not describe them |
| `interface call with no implementation in the set: io/fs.FileInfo.Size`, … | read the package that implements it into the set; a generic interface stays `unknown` |
| `function value with no candidate in the set: func(string) (string, error)`, … | the value comes from outside the set or has a generic signature |
| `method on undeclared type` | a method call whose receiver type has no such method in the set |
| `extern without summary: <path>.<name>` | read that package into the set, or add an entry to a `codespec-effects/1` file and pass it with `--summaries` |

## 5. Read a dispatch site

Report a standard library package to see dispatch. `os.SyscallError.Error` formats its wrapped
error with `err.Error()`, a call through the `error` interface:

```shell-session
$ target/debug/codespec effects work/readfile/*.cir.json \
    --summaries summaries/go-std.effects.json \
    --barrier summaries/go-linux-amd64.barrier.json \
    --report os > work/os.report.json
$ jq -c '.funcs[] | select(.name == "SyscallError.Error") | {name, dispatch, syscalls: (.syscalls | length)}' work/os.report.json
{"name":"SyscallError.Error","dispatch":[{"pos":"error.go:54","call":"error.Error","candidates":24}],"syscalls":32}
$ jq -c '.funcs[] | select(.name == "Getpid") | {name, effects, syscalls}' work/os.report.json
{"name":"Getpid","effects":[],"syscalls":[{"name":"getpid","at":["exec.go:240"]}]}
```

The `error.Error` call may reach 24 `Error` methods of the 54 packages, so formatting an error
message reaches 32 syscalls, `openat` among them. That is class-hierarchy analysis over the set: it
does not know which error `SyscallError` holds. `Getpid` makes one syscall that maps to no effect.

The same width is in step 3: `Load` reaches 55 syscalls, among them `wait4` and
`pidfd_send_signal`, which wait for and signal processes. Read a syscall list as "may reach, with
dispatch resolved over the whole set", not as what a run of the function does.

## 6. A real repository

A third-party Go library of 6 packages, measured with the 228 packages it imports; the library
and its report are kept outside this repository.

| measure | value |
|---|---|
| documents read with `read-repo.sh --deps` | 234, in 27 s |
| `codespec effects` over the set, reporting the library's packages | 3.6 s |
| syscalls its client entry point may reach | 74 |
| syscalls its server entry point may reach | 75: the same plus `accept4` |
| `unknown` reasons on those paths | 66 interface calls with no implementation in the set, 59 function values with no candidate, 27 functions without a body |

The sets are wide for the reason step 5 shows: an interface call reaches every type of the 234
packages with the methods, so most functions that do IO share one large set. This is the current
limit of dispatch, not a property of the library.

## Next

- [Static analysis](/docs/concepts/static-analysis): the formats, the effect vocabulary, the
  barrier and every `unknown` reason.
- [CLI reference](/docs/reference/cli): every flag of `codespec effects`.
