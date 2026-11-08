# codespec as a static analysis tool — design (2026-11-08)

Status: proposed, 2026-11-08. Epic `epic:externs-and-effects` implements § 3 and § 4; epic
`epic:whole-repo-to-syscalls` implements § 5 to § 5c, § 5d records what is built; § 6 is later work.

## 1. The question

`cir/1` is one typed representation of what code does inside a function, read from Go, Rust, Java
and Groovy. A pass written once over it is an analysis for every language a reader exists for. The
passes that exist today (`crates/passes`) already are analyses: `fallible`, `read_only`,
`comma_ok`, `unordered_iteration`. Two things stop codespec from being a static analysis tool for
real programs:

| gap | where | effect |
|---|---|---|
| a module cannot name a function outside itself | `Expr::Call` is "a call of a module function" (`crates/model/src/lib.rs`) | every call into the standard library or a dependency is a `foreign` statement, so the analysis sees nothing at the point where a program does IO |
| nothing says what a library function does | there is no summary format | even a named call would carry no meaning an analysis could use |

The Go reader shows the first: only `fmt.Sprintf`, `fmt.Errorf` and `fmt.Println` become
`intrinsic`; any other selector call on a package returns "expression outside cir/1" and the
statement is kept `foreign` with a refusal (`adapters/go/cmd/codespec-read-go/front.go`, `call`).
The Groovy reader does the same for GORM and Spring calls (`adapters/groovy/README.md`).

## 2. The approach: name the boundary, summarise it, never guess past it

Static analysers for many languages solve dependencies in one of two ways: analyse the whole
program including every library (precise, expensive, needs every source), or replace library code
with **summaries** — a declared behaviour per library function (CodeQL's models-as-data, Infer's
compositional summaries). codespec takes the second, in three layers:

| layer | what | where its content comes from |
|---|---|---|
| extern declaration | a module lists every outside function it calls: import path, name, parameter and result types; a call names it | the reader. Each reader already resolves these types with its language's own tooling (`go/types`, `javac`), so the declaration costs nothing to produce and is typed like every other expression |
| effect summary | per extern: the effects it may have, from a closed vocabulary | the standard library: written once per language, checked in. A dependency with source: computed by reading it with the same reader (§ 5) |
| unknown | a call with no summary, a `foreign` statement, dynamic dispatch the module cannot resolve | the analysis reports it as `unknown` with its position, never as pure |

The third row is the existing rule of this repository applied to analysis: a construct a reader
cannot express becomes `foreign` plus a refusal and is never dropped or approximated. An analysis
that treated an unsummarised call as pure would be the approximation that rule forbids.

### Syscalls

codespec does not model syscalls. The standard library is the layer that wraps them, so a summary
of `os.ReadFile` as `fs.read` is a summary of the `openat`/`read`/`close` beneath it, at the level a
program's author can act on. A program that makes a syscall directly (`syscall.Syscall`,
`unsafe`, JNI, FFI) calls an extern whose summary is `unknown`, or has a `foreign` statement.

## 3. `cir/1` additions (additive)

Both are new optional parts; every earlier document reads unchanged, as with `for`, `break` and
`continue` (`docs/design.md` § 3). `Expr::Call` keeps its meaning.

| addition | fields | meaning |
|---|---|---|
| `Module.externs` | ordered list of `{path, name, params: [Type], results: [Type]}` | every function outside the module that the module calls, sorted by `path` then `name`, each once |
| `Expr::Extern` (`k: extern`) | `path`, `name`, `args`, `t` | a call of the extern `path`.`name`; it must be declared in `externs` with matching arity |

Feature id: `expr.extern`. Readers and writers that do not handle it list it in their manifest's
`not`. A writer that does not claim it refuses the function, as for any unclaimed feature.

What stays outside: a variadic extern, an extern whose parameter or result type has no `cir/1`
type, a method on a type from another package, a function value. Each stays `foreign` as today.

## 4. Effect summaries and the report

### `codespec-effects/1`: a summary document

```json
{
  "format": "codespec-effects/1",
  "language": "go",
  "externs": [
    {"path": "os", "name": "Getenv", "effects": ["env"]},
    {"path": "strings", "name": "ToUpper", "effects": []}
  ]
}
```

- `effects` is a sorted set from the closed vocabulary below; `[]` declares the function pure.
- Read strictly: unknown fields, unknown effects, duplicate `(path, name)` and any other format are
  refused.
- `summaries/go-std.effects.json` is the checked-in Go standard library summary.

| effect | meaning |
|---|---|
| `env` | reads or writes the process environment or arguments |
| `fs.read` | reads the file system |
| `fs.write` | writes the file system |
| `net` | opens or uses a network connection |
| `proc` | starts, signals or ends a process, including `os.Exit` |
| `rand` | reads a random source |
| `stdin` | reads standard input |
| `stdout` | writes standard output or standard error |
| `time` | reads a clock or sleeps |

### `codespec effects`: the report

`codespec effects <module.cir.json> --summaries <file>…` prints `codespec-effects-report/1`: one
entry per function (`Func::key`, in module order) with

- `effects`: the union of the effects of every extern it calls, every intrinsic with an effect
  (`println` → `stdout`) and every module function or method it calls, to a fixed point over the
  module's call graph;
- `unknown`: every reason the set may be incomplete, each with a position: an extern with no
  summary, a `foreign` statement, a method call whose receiver type the module does not declare,
  a call naming no function of the module, or a module function that itself has one.

The report is deterministic (canonical JSON, ordered collections only). A function with an empty
`effects` and an empty `unknown` is pure in the sense the summaries define.

## 5. Later: dependencies and the standard library from source, down to the syscall barrier

A standard library is mostly code in its own language: Go's `os.ReadFile` is Go down to
`syscall.Syscall6`, Rust's `std::fs::read` is Rust down to `libc`. The checked-in
`summaries/go-std.effects.json` is therefore a stopgap. The target is to read the standard library
and every dependency with the same reader, compute their summaries bottom-up, and write by hand
only what sits at the barrier, where no source in the language exists.

- **Summaries computed from source.** A dependency or standard library package is read like the
  module itself; `codespec effects --emit-summary` writes the `codespec-effects/1` of its exported
  functions, keyed by import path, to be passed as `--summaries` to its callers. Cached by the
  dependency's version from the lockfile (`go.sum`, `Cargo.lock`) or the toolchain version.
- **The barrier, Go on Linux (read in go1.27.1's source).** `syscall.Syscall6` calls
  `RawSyscall6`, which calls `internal/runtime/syscall/linux.Syscall6`, declared without a body
  and implemented in `asm_linux_amd64.s`. Every call site passes the syscall number as a
  constant: `openat` calls `Syscall6(SYS_OPENAT, …)` (`zsyscall_linux_amd64.go`), and
  `zsysnum_linux_amd64.go` names 303 numbers. A function without a body is the barrier; its
  summary is the one hand-written part: syscall name → effect.
- **What the barrier cannot say.** `read` and `write` take a file descriptor, and the syscall is
  the same for a file, a socket and standard input. The effect class comes from where the
  descriptor was made (`openat` → `fs`, `socket`/`connect` → `net`), so an effect at the barrier
  is `io` unless the analysis follows the descriptor back to the call that made it.
- **Calls inside `foreign` statements.** Most standard library code uses constructs `cir/1` does
  not express (`unsafe`, `uintptr`, closures, generics, `defer`). A reader that keeps such a
  statement `foreign` should still list the calls inside it, so the call graph reaches the barrier
  even where the body is not expressible.
- **Loading.** A Go package directory holds test packages and files for other platforms; the
  reader must select files by build constraints (`GOOS`, `GOARCH`) as the toolchain does.
- **Several modules at once.** One module per document stays; a link step reads a set of documents
  and resolves an extern to a function in another document of the set.
- **Interfaces and dynamic dispatch.** `cir/1` has no interface types today; when it gains them, a
  call through one has the union of the summaries of every implementation in the analysed set,
  and `unknown` when the set is open.

## 5a. A whole repository: what `cir/1` must hold first

Measured 2026-11-08 on a third-party Go library (6 packages, 756 `func`
declarations, 56 structs, 11 interfaces), read package by package with the Go reader after it was
changed to refuse a declaration it cannot express instead of failing the whole package: 36
functions read with no `foreign` statement, 971 refusals. The root causes, by declaration:

| construct | declarations | what `cir/1` needs |
|---|---|---|
| a type from another package: the standard library (`sync.Mutex`, `net.Conn`, `net.IP`, `slog.Logger`) or another package of the repository (a URI type declared in a sibling package) | 10 struct-field roots (stdlib), 17 (another package of the repository); 658 refusals name a package-qualified type, the cascade included | an opaque `extern` type `{path, name}` beside `extern` functions, extern methods (a receiver type), and a link step across the repository's packages (§ 5) |
| interface types | 11 declarations | an `interface` declaration and a method call that dispatches on it |
| named func, map and slice types; func-typed fields and parameters | 27 declarations, 4 fields | a `func` type, a function literal, a call of a function value; a newtype over any type |
| embedded fields | 16 structs | a field marked embedded; the reader resolves promotion to an explicit path |
| variadic functions | 20 | a variadic last parameter and a spread argument |
| package-level variables | 44 | module-level `var` declarations |
| generics | 12 | type parameters; last, being the largest change |
| arrays, `uintptr`, floats | 7 | an array type, a float type; `uintptr` and `unsafe` stay at the barrier |

A refusal cascades: a struct refused for one field takes every method on it, every function with
it in a signature and every statement using it. The first row therefore unblocks the most.

## 5b. The goal: a repository mapped down to syscalls

`epic:whole-repo-to-syscalls` implements § 5 and § 5a for one repository, the third-party Go library
of § 5a: its 6 packages and the 228 packages they import (217 of the standard library, 11 of
other modules; `go list -deps`, Go 1.27.1, linux/amd64) are read, linked into one set, and
`codespec effects` reports for every function of the library the syscalls it may reach and the effects
they map to. The pieces, in the order they depend on each other:

| piece | what it adds |
|---|---|
| reading a whole package | build-constraint file selection; refusal per declaration (done in the reader, § 5a) |
| the types of § 5a | extern types and methods, interfaces, function values, embedded fields, module `var`s, variadic, arrays and floats, generics |
| calls inside what stays outside | a `foreign` statement and a refused function still list the calls they make |
| a module's import path and a link step | `Module.path`; `codespec effects` over a set of documents resolves an extern to the function of another document |
| dispatch | a call through an interface or a function value reaches every method or function in the set that could stand there (class-hierarchy analysis over the set) |
| the barrier | a function with no Go body, `//go:linkname`, and a table from syscall number to effect; the report gains `syscalls` |

## 5c. Questions the linked set answers

Each question below is a pass over the linked call graph of § 5b. The first four are stories of
`epic:whole-repo-to-syscalls`; the rest need that graph and are later work.

| question | pass | status |
|---|---|---|
| which syscalls can this function, handler or endpoint reach | effects with the barrier, from an entry function | epic |
| where does a side effect happen | the effect report with the position of each extern or syscall call | epic |
| which functions are never called (tree shaking) | reachability from the entry functions: `main`, exported functions, registered handlers | epic |
| what does one call do, with its branches | a call tree from one function: each `if`, `switch` and loop as a branch node, each call expanded once, syscalls at the leaves, drawn on the documentation site | built: `codespec trace`, `codespec-trace/1` (`crates/effects/src/set/trace.rs`) |
| which branch is not handled | a `switch` over a type or constant set with no case for a member and no `default`; an error result that is dropped (`crates/passes` `fallible` per call) | later |
| where is it buggy or insecure | taint from a source (`net` read, `stdin`, `env`) to a sink (`proc`, `fs.write`, a query) along the linked graph | later |

## 5d. As built: calls, the linked set, dispatch and the barrier

Built 2026-11-08 without the `cir/1` types of § 5a (interfaces, function values, extern types):
everything the analysis needs from code `cir/1` cannot express is read from `go/types` directly.

| piece | what it is |
|---|---|
| `CallRef` | one call named outside `cir/1` expressions: `func` (with its integer constant arguments), `method`, `iface` (the interface's whole method set), `value` (a call of a function value, by signature), `ref` (a function or method used as a value), `lambda` (a function literal and its calls). A signature is spelled without parameter names, named types by full import path: `func([]byte) (int, error)` |
| `foreign.calls`, `Module.opaque` | a `foreign` statement lists its calls; a refused or body-less function stays as an opaque function with its calls. A `foreign` statement without `calls` (other readers) is `unknown` |
| `Module.path`, `methods`, `linknames`, `inits` | the import path; the method set of each named type with the method each entry runs (promotion resolved by `go/types`); every `//go:linkname`; every call and function value of the variable initializers. The last three only for a module with an import path |
| the link | `codespec effects a.cir.json b.cir.json …`: an extern or `CallRef` naming the `path` of a document of the set is an edge to its function; a document wins over a summary |
| dispatch | class-hierarchy analysis over the set: an `iface` call reaches the method of every type whose method set holds all the interface's methods with identical signatures; a `value` call reaches every `ref` and `lambda` of the set with its signature. A literal's calls also belong to the function holding it, so a literal passed to a function outside the set is still counted |
| rapid type analysis | `--dispatch rta`, the default with `--entry` or `--exported`: from the entries (with every `init` and every variable initializer), an `iface` candidate counts once reached code makes its type, a `value` candidate once the code using it as a value is reached, to a fixed point. A type is made by a `new` call ref (the Go reader lists composite literals, `new`, `make` of a slice, conversions, constants and `var` zero values, and the named types they hold by value in fields), a `cir/1` `struct`, `conv`, `zero`, `lit`, `const`, `to_error` or `var`, a struct of `cir/1` holding it in a field, or a method of it being reached. Values made where the set cannot see (reflection, `unsafe`, a function outside the set) are missed. Unknown reasons stay those of class-hierarchy analysis |
| reach | `codespec reach`: per module, the functions the entries reach by a call, a use as a value or a dispatch candidate, and those they do not (`codespec-reach-report/1`) |
| the barrier | `summaries/go-linux-amd64.barrier.json` (`codespec-barrier/1`): the 13 functions taking the syscall number as argument 0 (`syscall.Syscall6`, `internal/runtime/syscall/linux.Syscall6`, …), 44 body-less runtime functions with fixed syscalls, 29 assembly-only packages, and the 385 linux/amd64 syscalls with their effects. A body-less function with a linkname resolves in either direction (`internal/poll.runtime_pollWait` → `runtime.poll_runtime_pollWait`) |
| the fixed point | one call graph over every function, literal and dispatch site of the set; strongly connected components (iterative Tarjan), callees first, bitsets of effects and syscalls: linear in nodes plus edges |
| `cir/1` function values and extern methods | merged after § 5d was built: a `call_value` is a `value` call, a `func_ref` a `ref`, a `lambda` a `lambda` node whose statements also count for the function holding it; the `cir/1` function type is spelled as `go/types` spells the signature, named types by import path. An `extern_method` is the method `<type>.<method>` of the document with its path, or of a summary |

Measured on the third-party Go library of § 5a with its 228 imports (234 packages, 40 MB of
`cir/1`, read in 27 s in parallel): `codespec effects` over all 234 documents takes 3.6 s reporting
two of the library's packages and 15.7 s reporting all 14,958 functions. Its client's request
entry point reaches 74 syscalls, its server's listening entry point 75. Class-hierarchy analysis
over the standard library over-approximates widely: a call through `io.Writer` or of a
`func() error` value reaches every candidate in 234 packages, so most functions that do IO share
one large component. What remains `unknown` for the client entry point (155 functions it may
reach): 66 interface calls with no implementation in the set (generic interfaces such as
`ecdh.Point[P]`, interfaces only callers implement such as a tracing hook), 59 function values
with no candidate (generic signatures, values from outside the set), 27 body-less runtime and
reflect functions that call back into Go (`systemstack`, `mcall`, `reflectcall`, `makeFuncStub`),
3 methods on undeclared types.

The measured example and its report are kept outside this repository. With rapid type analysis
from every exported function of the library's 6 packages and its entry points, all 234 packages
read, 273 of its 756 functions have an empty `unknown`, and the client and server entry points
reach 56 and 57 syscalls. Every function of the library that reaches a syscall reaches at least
56: allocation alone (`runtime.mallocgc`) reaches the scheduler, the garbage collector and through
them the network poller and file IO by direct calls, so a parser of text shows every effect.

## 6. Later: checking summaries against a run

A summary is a claim. The harness already runs every implementation of an example on seeded
traces; running the original under `strace -f` (Linux) or a seccomp filter in log mode and mapping
the observed syscalls to the effect vocabulary gives the observed effects of a run. An observed
effect that the report does not predict is a wrong or missing summary, found by a run rather than
by review — the evidence rule of `docs/design.md` applied to analysis.
