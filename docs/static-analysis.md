# codespec as a static analysis tool — design (2026-11-08)

Status: proposed, 2026-11-08. Epic `epic:externs-and-effects` implements § 3 and § 4; § 5 and
§ 6 are later work.

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

## 5. Later: dependencies with source, and many modules

- **Summaries computed from source.** A dependency whose language has a reader is read like the
  module itself; `codespec effects --emit-summary` writes the `codespec-effects/1` of its exported
  functions, keyed by import path, to be passed as `--summaries` to its callers. Cached by the
  dependency's version from the lockfile (`go.sum`, `Cargo.lock`).
- **Several modules at once.** One module per document stays; a link step reads a set of documents
  and resolves an extern to a function in another document of the set.
- **Interfaces and dynamic dispatch.** `cir/1` has no interface types today; when it gains them, a
  call through one has the union of the summaries of every implementation in the analysed set,
  and `unknown` when the set is open.

## 6. Later: checking summaries against a run

A summary is a claim. The harness already runs every implementation of an example on seeded
traces; running the original under `strace -f` (Linux) or a seccomp filter in log mode and mapping
the observed syscalls to the effect vocabulary gives the observed effects of a run. An observed
effect that the report does not predict is a wrong or missing summary, found by a run rather than
by review — the evidence rule of `docs/design.md` applied to analysis.
