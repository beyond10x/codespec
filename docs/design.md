# codespec — design (2026-11-06)

## Why

An ESS specification describes a system from the outside: types, entities, lifecycles, commands,
events, views. What a function body computes is an obligation there
(`ess/docs/design/ess-structural-synthesis-obligations-realizations-design-v0.1.md`, "Arbitrary
business algorithm | Obligation"). Translating code between languages, or rewriting it within one,
needs a representation of the inside. `cir/1` is that representation.

A prior experiment (Go only, `cir/0`) lifted a 136-line Go order service, emitted Go and Rust from
it, and ran the original, the Go output and the Rust output on 200 seeded traces of 300 commands:
82,703 output lines, byte-equal across all three; a bug planted in the Rust output was caught at the
first seed. `cir/1` is `cir/0` with the changes in § 3.

## 1. Concepts

| concept | what |
|---|---|
| `cir/1` | the format: one canonical JSON document per module |
| reader | source language → `cir/1` |
| pass | `cir/1` → `cir/1`; deterministic, total, named |
| facts | whole-module analysis results written by passes and read by writers, so writers decide nothing |
| writer | `cir/1` → source language |
| refusal | a construct a reader or writer does not handle, with position and reason |
| harness | runs N implementations of one program on seeded traces and compares outputs byte for byte |

## 2. Languages

- Repository tooling (model, CLI, passes, harness, checks) is Rust.
- Readers and writers are **adapters**, each in the language whose tooling handles its target best.
  A Go writer builds `go/ast` and prints it with `go/printer`; a Rust writer can build `syn` trees and
  print them with `prettyplease`; the Go reader uses `go/types`.
- The Rust reader parses with `syn`, which the Rust writer already depends on, and does its own name
  and type resolution for the subset of Rust that `cir/1` covers.
- An adapter is a program. A reader prints one module on stdout; a writer reads one on stdin.
  `codespec read` and `codespec write` validate each document against the model on the way through.

Two questions decide each adapter, and the harness answers both:

1. Does the reader capture enough detail to produce `cir/1`?
2. Does the writer produce code that behaves like the original?

## 3. `cir/1`

Defined by `crates/model/src/lib.rs`; `schemas/cir-1.schema.json` is generated from it.

| level | nodes |
|---|---|
| module | name, reader `{language, name, version}`, doc, types, consts, sentinel errors, vars `{name, t, init \| foreign}`, funcs, externs `{path, name, recv?, params, results}`, interfaces `{path?, name, exported, methods [{name, params, results}], doc?, pos?}`, layout, facts, refusals; for a whole-program analysis: import `path`, `opaque` functions, method sets `methods`, `linknames`, `inits` |
| types | `int {width: platform\|8\|16\|32\|64, signed}`, `float {bits: 32\|64}`, `bool`, `string`, `error`, `any`, `unit`, `named {name, targs?}`, `ptr`, `slice`, `array {len, elem}`, `map`, `chan {dir}`, `tuple`, `extern {path, name, targs?}`, `func {params, results}`, `param {name}` (a type parameter, feature `type.param`); `targs` instantiate a generic type (feature `expr.instantiate`) |
| declarations | `struct`, `newtype`, `enum`, `union` (a `newtype` may be over a func, map or slice type); module-level `var` (feature `decl.var`); a struct field marked `embedded` (feature `decl.embedded`); `interfaces`, the module's own and those of other modules a call goes through (feature `decl.interface`); `tparams [{name, constraint}]` on a `struct`, `newtype`, function or extern, each constraint an interface (`any`, `named`, `extern`), a method of a generic type binding them through its receiver's `targs` (feature `decl.type_params`) |
| statements | `let`, `var`, `assign`, `incr`, `decr`, `if`, `for_range`, `for`, `break`, `continue`, `switch`, `return`, `raise`, `expr`, `send`, `lookup_else`, `foreign` |
| expressions | `lit`, `float`, `nil`, `var`, `const`, `errvar`, `global`, `field`, `index`, `call`, `method`, `extern`, `extern_method`, `binop`, `unop`, `struct`, `conv`, `make`, `zero`, `intrinsic`, `to_error`, `func_ref`, `lambda`, `call_value`, `spread`, `iface_call`, `to_iface`; `call`, `extern` and `extern_method` carry `targs`, a generic callee's type arguments written out (feature `expr.instantiate`) |
| calls outside `cir/1` | `func`, `method`, `iface`, `value`, `ref`, `lambda`, `new` (a value of a named type is made here), and `named` (a method call on a value of a type the reader does not know, or a macro invocation, by its name as written): listed by a `foreign` statement (`calls`), an `opaque` function and `inits`, named from the source language's own types |

Changes from `cir/0`:

| change | why |
|---|---|
| Rust model is the definition, schema generated | one source of truth |
| `reader` provenance | a document says what produced it |
| `int {width, signed}` | `cir/0`'s `bits: 0` meant "Go `int`", a Go-only meaning |
| `pos` on every statement and declaration | refusals and diffs point at source |
| `union`, `switch` | Rust enums with data, Go `switch`, Rust `match` |
| every expression has a type, required | writers never infer |
| tagged nodes with strict fields | a document with an unknown field or kind is refused |
| `for`, `break`, `continue` (added with M3) | loops other than a range, and leaving one early; additive, so every earlier document reads unchanged |
| `extern` type, `extern_method`, `recv` on an extern | a type of another package (`sync.Mutex`) is opaque, and a method called on one is declared with its receiver; additive |
| `vars`, `global` | module-level variables, read and assigned by functions; an initializer outside `cir/1` kept as `foreign` source with a refusal; additive |
| `embedded` on a struct field | Go embedding: the field is named after its type, a named or extern type or a pointer to one, and the reader writes every promoted field access and method call as the explicit path through it, so no consumer needs promotion rules; skipped when false, additive. Strict reading refuses an embedded field not named after its type and a struct naming an embedded field twice |
| `func` type, `func_ref`, `lambda {captures}`, `call_value`; `newtype` over a func, map or slice type | function values: a handler registered as a value is reached only through one; additive. Strict reading refuses a lambda capturing a name not in scope where it stands |
| `variadic` on a function's last parameter and on an extern, `spread` | a variadic function (Go `...T`, a slice inside the body) called with any number of arguments, or with one slice passed `spread` as its last argument (also to `append`); additive. Strict reading refuses a spread anywhere else or to a callee whose last parameter is not variadic, a variadic parameter that is not the last or not a slice, and counts a variadic extern's last parameter as zero or more arguments |
| `interfaces`, `iface_call {path?, iface, name, recv, args}`, `to_iface` | a value of an interface type (the module's own `named`, another module's `extern`) and a call through one, the interface declared with its whole method set so analysis dispatches over every type that has it; a conversion to an interface is explicit. Additive. Strict reading refuses a call naming an undeclared interface, a method the interface does not have or a wrong argument count, and a `to_iface` to a type that is not an interface |
| `path`, `opaque`, `methods`, `linknames`, `foreign.calls` | the call graph of a set of modules has no hole where a body is outside `cir/1`: what such a body calls is listed, and a refused function stays in the document; additive, a `foreign` statement without `calls` means its calls are unknown |
| `named` call ref (`form` `method` or `macro`) | a call a reader names as written but does not resolve: a Rust method call on a value whose type the reader does not know, or a macro invocation; it keeps the name a query asks for and links to no function. Additive. Strict reading refuses another form, an empty name and a `method` with a path |
| `raise`, `raises`, entry kind `exception-mapper` | a framework entry leaving by an exception (Java `throw new WebApplicationException(…)` in a resource method) that the framework hands to its mapper: `raise` leaves the function and every caller up to the framework with a value of an extern type, `raises` lists the extern types a function can raise, its own and its callees', outside its results, so the signature the framework calls is unchanged; an `exception-mapper` entry names the function the framework hands them to. Additive. Strict reading admits `raise` only in a function an entry names or one only such callers reach (unexported, called directly, never a value, never from a lambda, an opaque function or a variable's initializer, and not `init`, which runs when the module loads), and refuses a `raises` that differs from what the bodies raise |

## 4. Passes (to port, M1)

| pass / fact | rule |
|---|---|
| `lookup-guard` | `v, ok := m[k]; if !ok { …return }` with `ok` unused elsewhere → `lookup_else` |
| `shared` | a reference to `T` stored in a container or field |
| `messages` | types sent on a channel of `any` |
| `fallible` | last result is an error and every error path returns zero values |
| `comma_ok` | `(T, bool)` always `(zero, false)` or `(x, true)` |
| `read_only` | a method writes nothing rooted at its receiver |
| `displays` | `Error()`/`String()` methods; string enums and newtypes |
| `unordered_iteration` | a loop over a map |

## 5. Milestones

| | content | done when |
|---|---|---|
| M0 | model, schema, CLI (`validate`, `schema`, `read`, `write`), Go reader, `orders` | `task check` green; `orders` read twice gives the same bytes |
| M1 | passes (Rust), Go writer (`go/ast`), Rust writer, harness | `orders`: original Go, Go output and Rust output byte-equal on 200 × 300 traces |
| M2 | dropped: a tree-sitter Go reader (§ 7) | — |
| M3 | a second, algorithmic example: loops, `switch`, slices, arithmetic (`series`) | M1 criteria on both examples |
| M4 | Rust reader on `syn`, Rust → `cir/1` → Go | harness green on a Rust example |

Release 0.1.0 is M0, M1 and M4: Go reader, Rust reader, Go writer, Rust writer, passes, harness.

## 6. Later

- ESS: a specification supplies a service's skeleton, `cir/1` bodies fill its obligations, and the
  ESS conformance suite is an equivalence oracle beside the harness.
- More target languages through the same adapter boundary.

## 7. Not used

Decided 2026-11-06.

- **tree-sitter.** Considered as the reader for languages without typed tooling, and not used. It
  gives syntax only, and `cir/1` needs a type on every expression. No tree-sitter-based tool infers
  types: Semgrep's generic AST covers 31 languages but stays close to the source text; GitHub's
  stack-graphs resolves names but not types, and was archived on 2025-09-09; tree-sitter tags
  queries only label definitions and references. A tree-sitter reader would have to bring its own
  type inference per language, and the project keeps its dependencies minimal. M2 is dropped.
- **rust-analyzer's `ra_ap_*` crates.** Not used for the Rust reader, for the same dependency
  reason; the reader resolves names and types itself for the `cir/1` subset, on `syn`.
