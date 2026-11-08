# Changelog

Every change a user of `codespec` or of `cir/1` would notice.

## Unreleased

### Added

- `codespec trace <documents…> --from <path>.<function>` prints `codespec-trace/1`, the call tree
  from one function over a set: each `if`, `switch` and loop a branch node with its condition,
  each call expanded once where it is called fewest calls from `--from` (a repeat is a `ref`
  node), a call through an interface or a function value a `dispatch` node listing its
  candidates, syscalls and summarised externs at the leaves, a `cut` node past `--depth`
  (default 8). Calls are resolved on the call graph `codespec effects` uses, with `--from` the
  only entry for rapid type analysis. The site's Call trees page draws the tree of
  `envcheck.Announce`.
- `cir/1` has variadic functions and spread arguments: a function's last parameter may be marked
  `variadic` (a slice inside the body), an extern may be declared `variadic`, and the expression
  `spread {x}` passes the slice `x` as the last argument of a call of one or of `append` (features
  `decl.variadic`, `expr.spread`; conformance cases `decl-variadic`, `expr-spread`). Additive;
  every earlier document reads unchanged. Strict reading refuses a spread anywhere else or to a
  callee whose last parameter is not variadic, a variadic parameter that is not the last or not a
  slice, and a variadic extern call with fewer arguments than its fixed parameters.
  `codespec-read-go` reads variadic functions and methods, variadic calls of other packages
  (`slog.Info`, `errors.Join`, a method of `*slog.Logger`) as extern calls, and `f(xs...)` as a
  spread; `fmt.Sprintf`, `fmt.Errorf` and `fmt.Println` stay intrinsics unless called with a
  spread. `codespec-write-go` writes them back; `codespec-write-rust` refuses a function holding
  them; the Rust reader and writer list both features under `not`.
- `cir/1` names the functions a module calls outside itself: `Module.externs` declares each one
  (import path, name, parameter and result types), and the expression `extern` (feature
  `expr.extern`, conformance case `expr-extern`) calls one. Both are additive; every earlier
  document reads unchanged. Strict reading refuses an extern call that is not declared or has the
  wrong number of arguments, and unsorted or repeated declarations.
- `codespec-read-go` reads a call of a function of another package, not variadic and with `cir/1`
  types only, as an extern call instead of a `foreign` statement; `codespec-write-go` writes it as a
  package call with its import. `codespec-write-rust` refuses a function holding one; the Rust,
  Java and Groovy readers do not claim the feature.
- `cir/1` has extern types and extern method calls: the type `extern {path, name}` (feature
  `type.extern`, conformance case `type-extern`) is an opaque type of another package, such as
  Go's `sync.Mutex`, and the expression `extern_method {path, type, name, recv, args}` (feature
  `expr.extern_method`, case `expr-extern-method`) calls a method of one. An extern declares a
  method with the new optional `recv`; `externs` sort by path, receiver type name (empty for a
  function), then name. Additive; every earlier document reads unchanged. Strict reading refuses
  an undeclared extern method call, a wrong argument count, and a receiver that is not an extern
  type of the declaration's path. `codespec-read-go` reads a type of another package as `extern`,
  so a struct with such a field is declared, and a method call on one as `extern_method`; a field
  of a struct of another package stays `foreign`. `codespec-write-go` writes both back;
  `codespec-write-rust` refuses a function holding them. `codespec effects` looks an extern method
  up in a summary as `<type>.<method>`.
- `codespec effects <module.cir.json> --summaries <file>…` prints `codespec-effects-report/1`: per
  function, the effects it may have (`env`, `fs.read`, `fs.write`, `net`, `proc`, `rand`, `stdin`,
  `stdout`, `time`) from `codespec-effects/1` summaries of the functions outside the module, and
  every extern without a summary, `foreign` statement or unresolved method listed as `unknown`
  with its position. `summaries/go-std.effects.json` summarises part of the Go standard library;
  `examples/envcheck` shows the report and `task examples-effects` holds it. Design:
  `docs/static-analysis.md`.
- `cir/1` has fixed-size arrays and floating-point numbers: the types `array {len, elem}` and
  `float {bits: 32|64}` and the float literal `float` (features `type.array`, `type.float`,
  `expr.float`; conformance cases `type-array-float`, `type-float`, `expr-float`). A float
  literal's `value` is the shortest decimal that reads back to the same number, plain when its
  exponent is between -7 and 21, else `<digits>e<exp>`; strict reading refuses one that is not a
  finite number of its type's width or is spelled otherwise. Additive; every earlier document
  reads unchanged. `codespec-read-go` reads Go arrays, `float32`, `float64`, their literals,
  constants and arithmetic; `uintptr` and `unsafe.Pointer` stay outside `cir/1`.
  `codespec-write-go` writes them back; `codespec-write-rust` refuses a function holding one,
  naming the feature, and the Rust reader does not claim them.
- `cir/1` has module-level variables: `Module.vars` declares each one (name, type, an optional
  initializer `init`, or `foreign` source for an initializer outside `cir/1`), and the expression
  `global` (features `decl.var`, `expr.global`; conformance cases `decl-var`, `expr-global`) reads
  one or, on the left of an `assign`, sets it. Additive; every earlier document reads unchanged.
  Strict reading refuses a `global` naming no declared var and a var with both `init` and
  `foreign`. `codespec-read-go` reads package-level `var` declarations other than
  `errors.New(<literal>)` sentinels as vars, an initializer it cannot express kept as `foreign`
  with a refusal; a variable of a type outside `cir/1`, the blank variable and variables set from
  one multi-value expression stay refused. `codespec-write-go` writes them back;
  `codespec-write-rust` refuses each var and every function holding a `global`, naming the
  feature, and the Rust reader does not claim them.
- `cir/1` has function values: the type `func {params, results}` (feature `type.func`), a module
  function or a bound method used as a value (`expr.func_ref`), a function literal naming the
  outer locals it captures (`expr.lambda`) and a call of a function value (`expr.call_value`);
  conformance cases `type-func`, `expr-func-value`, `expr-lambda`, `expr-call-value`. A `newtype`
  may be over a func, map or slice type. All additive; every earlier document reads unchanged.
  Strict reading refuses a lambda capturing a name that is not in scope where it stands.
- `codespec-read-go` reads function types, function literals, method values, calls of function
  values, and named func, map and slice types as newtypes; an interface declaration is refused as
  one. `codespec-write-go` writes them back. `codespec-write-rust` refuses a function holding one;
  the Rust reader and writer list the four ids under `not`. `codespec effects` counts the calls in
  a lambda's body toward the function holding it and dispatches a call of a function value over
  every `func_ref` and `lambda` of its signature (an `unknown` when there is none); the `read_only` fact is not given to a method holding a function value.
- `cir/1` keeps the call graph whole where a body is outside it, additively: a `foreign` statement
  lists the calls its source makes (`calls`), and `Module.opaque` holds every function the reader
  refused or that has no body, with its receiver, position, reason and calls. A call is a
  `CallRef`: a function, a method, a call through an interface (with the interface's method set),
  a call of a function value, a function used as a value, or a function literal (its calls are
  the holder's too); a constant integer argument is kept. `Module.path` is the import path;
  `Module.methods` the method set of each named type, `Module.linknames` every `//go:linkname` and
  `Module.inits` the calls of the variable initializers, written only for a module with an import
  path. A `foreign` statement without `calls` means
  its calls are unknown. Strict reading refuses an opaque function named twice or like a function.
- `codespec-read-go` writes all of it from `go/types`, and the import path from `GOROOT` or the
  `go.mod` above the package (`CODESPEC_READ_GO_IMPORT_PATH` overrides it).
- `codespec effects a.cir.json b.cir.json …` reads several documents as one program: an extern
  naming the `path` of a document of the set is a call of its function, a document wins over a
  summary of the same function (the report's `notes` say so), a call through an interface reaches
  every method of the set with the interface's method set and a call of a function value every
  function, method or literal of the set used as a value of its signature (`dispatch` counts the
  candidates). `--barrier summaries/go-linux-amd64.barrier.json` (format `codespec-barrier/1`)
  names the functions that take a syscall number, the body-less runtime functions and the
  assembly-only packages, and maps
  each of the 385 linux/amd64 syscalls to effects, `read` and `write` to the new effect `io`; a
  report entry then lists its `syscalls` and, for each syscall and effect, the positions of its own
  calls that lead there. A body-less function is resolved through `//go:linkname`. `--report`
  limits the report to some modules, `--roots` counts the `unknown` reasons each function reaches.
- In a set, a `cir/1` `extern_method` is a call of the method of that type in the document with its
  path (a summary names it `<type>.<method>`), and `func_ref`, `lambda` and `call_value` take part
  in the dispatch like the `ref`, `lambda` and `value` calls a `foreign` statement lists: a
  `cir/1` function type is spelled as the Go reader spells a signature, so both meet.
- `scripts/read-repo.sh --deps` reads every package a repository imports, the standard library
  included, in parallel.
- `cir/1` marks embedded struct fields: a `Field` may carry `embedded: true` (feature
  `decl.embedded`, conformance case `decl-embedded`), skipped when false, so every earlier document
  reads unchanged. An embedded field is named after its type, a named or extern type or a pointer
  to one; strict reading refuses one that is not, and a struct naming an embedded field twice.
  `codespec-read-go` reads a struct with an embedded field instead of refusing it, and writes every
  promoted field access, method call and method value as the explicit path through the embedded
  fields (`s.Lock()` on a struct embedding `sync.Mutex` reads as `s.Mutex.Lock()`), so no consumer
  needs promotion rules; a field promoted from a struct of another package stays `foreign`.
  `codespec-write-go` writes the field back as embedded. `codespec-write-rust` refuses a function
  reaching through an embedded field, naming the feature; the Rust reader and writer list
  `decl.embedded` under `not`.
- `codespec reach <documents…> [--entry <path>.<func>…] [--exported <module>…]` prints
  `codespec-reach-report/1`: per module, the functions the entries reach (by a call, a use as a
  value, or as a candidate of a dispatch) and those they do not. Without entries, `main.main`;
  every Go `init` and every module's variable initializers always count.
- Rapid type analysis: `codespec effects` and `codespec reach` take `--dispatch cha|rta`; with
  `rta`, a call through an interface reaches only the types that code reached from the entries
  makes, and a function value call only the values that code uses. `rta` is the default with
  `--entry` or `--exported`, `cha` without. The `cir/1` call list gains `new {path, name}` (a
  value of a named type is made here); additive. `codespec-read-go` lists it, for a module with an
  import path, at a composite literal, `new`, `make` of a slice, a conversion, a constant and a
  `var` zero value of a named type, and for each named type held by value in a field.
- `cir/1` has interfaces: `Module.interfaces` declares each interface type with its whole method
  set (feature `decl.interface`, conformance case `decl-interface`), the module's own in source
  order and those of other modules a call goes through after them, sorted by path and name; a
  value of one has the type `named` (the module's own) or `extern` (another module's). The
  expression `iface_call {path?, iface, name, recv, args}` (`expr.iface_call`, case
  `expr-iface-call`) calls a method through one, and `to_iface {x}` (`expr.to_iface`, case
  `expr-to-iface`) converts a value to one explicitly. Additive; every earlier document reads
  unchanged. Strict reading refuses a call through an undeclared interface, naming a method the
  interface does not have or with the wrong argument count, unsorted or repeated declarations,
  an interface that is also a type, and a `to_iface` to a type that is not an interface.
- `codespec-read-go` reads an interface declaration (an interface with a type set, a variadic
  method or a method of a type outside `cir/1` stays refused, with that reason), a method call on
  a value of a named interface type as `iface_call`, and makes the implicit conversion to a
  named interface at an assignment, argument, return, struct field and conversion explicit with
  `to_iface`. `codespec-write-go` writes them back. `codespec-write-rust` refuses a function
  holding an interface call or conversion and a module declaring an interface, naming the
  feature; the Rust reader and writer list the three ids under `not`. `codespec effects`
  dispatches an `iface_call` over every type of the set whose method set holds the declared
  interface's methods, as it does an `iface` call a `foreign` statement lists; the `read_only`
  fact is not given to a method calling through an interface on, or with, its receiver.
- `cir/1` has type parameters: a `struct`, `newtype`, function or extern declares
  `tparams [{name, constraint}]`, each constraint an interface (`any`, the module's own `named`
  interface or another module's `extern` one; feature `decl.type_params`, conformance case
  `decl-generic`); a method of a generic type binds the type's parameters through its receiver's
  type arguments. The type `param {name}` (`type.param`) names one. `named` and `extern` types
  carry `targs` for an instance of a generic type, and `call`, `extern` and `extern_method` carry
  `targs`, the type arguments of a generic callee written out (`expr.instantiate`). An
  `iface_call` may go through a value of a type parameter, by the interface constraining it.
  Additive; every earlier document reads unchanged. Strict reading refuses a type parameter out
  of scope, a constraint that is not an interface, a parameter declared twice, and a call or type
  use whose type arguments do not match its declaration's type parameters in number.
- `codespec-read-go` reads generic types, functions and methods, writes the inferred type
  arguments of every call out from `Info.Instances`, reads a method call on a value of a type
  parameter as an `iface_call` through its constraint, and declares generic externs and methods of
  another package's generic types (`atomic.Pointer[T].Load`). A constraint with a type set
  (`~int | ~string`, `comparable`), a generic interface and an instantiated generic function used
  as a value stay refused, with that reason; a type alias is refused as `type alias`.
  `codespec-write-go` writes them back. `codespec-write-rust` refuses a function holding a type
  parameter or type arguments and a module declaring a generic type, naming the feature; the Rust
  reader and writer list the three ids under `not`.

### Fixed

- Strict reading (`Module::from_json`, so `codespec read`, `validate` and every command reading a
  module) admits a document nested up to 512 levels deep, where serde_json stopped at 128 with
  "recursion limit exceeded" (a long `+` chain, as in `golang.org/x/net/idna`); a deeper one is
  refused by name before it is parsed, never a stack overflow.
- `codespec-read-go` refuses, as one declaration with its reason, a struct embedding a type
  through an alias or embedding `error`, instead of writing an embedded field `cir/1` refuses
  (`crypto/internal/rand`, `encoding/json/jsontext`, `net/http`); and gives a type declared inside
  a function no `cir/1` name, so a call through a local interface is `foreign` instead of an
  `iface_call` to an undeclared interface (`crypto/tls`). The five packages read.

## 0.4.0

### Added

- The semantic feature `sem.error.propagate`, a fallible call whose error the caller returns
  unchanged, with the conformance case `sem-error-propagate`. `codespec-write-rust` writes
  `v, err := f(…)` followed by `if err != nil { return <zero values>, err }` as `let v = f(…)?;`
  (`f(…)?;` for a callee whose only result is the error) instead of refusing the function; a call of
  a fallible function whose error is wrapped, dropped or inspected, and any call of a comma-ok
  function, stay refused, naming the callee. `codespec-read-go` reads `err != nil` and `err == nil`,
  the `nil` taking the type of the operand it is compared with. `codespec-read-rust` does not claim
  the feature.

## 0.3.0

### Added

- Each release ships the conformance kit `codespec-conformance-<version>.tar.gz` (`conformance/` and
  `schemas/cir-1.features.json`) beside the binaries, listed in `SHA256SUMS`. `task conformance-kit`
  packs it, unpacks it and runs `codespec conform` on the Go writer against the unpacked copy; it is
  part of `task check`. The binary archives now also hold the runners `codespec-run-go` and
  `codespec-run-rust`, so a release has every program `codespec conform` calls for Go and Rust. The
  guide "Add a reader or writer adapter" shows an outside author how to run `codespec conform`; its
  sample run is written by `codespec-docs generate` from the Go writer's report.
- `codespec conform --out` creates a missing parent directory.
- `codespec measure <module.cir.json>` prints, per artefact (source file), the functions read and
  refused and the refusals by category, then the totals and the share read; `--format json` prints
  `codespec-measure/1`. Works on every reader's output; a reason without one of the Groovy reader's
  eight categories counts as `uncategorised`. The lending fixture's report is checked in as
  `adapters/groovy/testdata/lending.measure.txt`.
- `codespec-run-java` and `codespec-run-groovy` (`adapters/jvm/src/codespec/run`): the JVM runners
  of the conformance kit; they compile a package with javac or Groovy 2.4 and make a `calls.json`
  against its static methods.
- Conformance reports for every adapter in `website/data/conformance/`: the Java and Groovy readers
  join the Go and Rust readers and writers. The reader corpora `conformance/reader/<lang>` are built
  from the kit's case sources (`case.go`, `rust/case.rs`, `java/Case.java`) and drift-checked.
- The support matrix `website/data/support-matrix.json` is generated from the feature list, the
  adapters' manifests and their reports, with a note on every failing cell, every claim without a
  case and every refusal a report explains.
- A writer's refusal listed in `refusals.json` is reported as `name (pos): reason`.
- `codespec features` prints the `cir/1` feature vocabulary (`codespec-features/1`, 105 ids), checked
  in as `schemas/cir-1.features.json`; `features_from_json` reads it strictly.
- `cir/1` intrinsic `sorted_keys`: the keys of a map in ascending order (integers numerically, strings
  by UTF-16 code unit). No writer renders it yet.
- `codespec-read-java` (`adapters/java`): reads a Java 17 package through javac into `cir/1`, refusing
  every form whose meaning the document would not keep. Needs a JDK (default path: JDK 21).
- `codespec-read-groovy` (`adapters/groovy`): reads a directory of Groovy 2.4 files into `cir/1`,
  typed by the Groovy static type checker whether a class is annotated or not. Dynamic code is kept
  as `foreign` with a refusal in one of eight categories (`adapters/groovy/README.md`). Needs a JDK
  and Groovy 2.4.10 (`GROOVY_HOME`, default: the sdkman `groovy/2.4.10` candidate).
- `codespec-read-rust` reads the Rust writer's loops, switches and multi-assigns back: the `series`
  output reads with 0 refusals.
- `examples/series` runs a fourth implementation, the Rust output read back and written as Go
  (`out/go-from-rust`): 4 implementations byte-equal under the harness.
- `examples/ledger`: a Java package read with `codespec-read-java`, written as Go and Rust, and run
  byte-equal with the original Java under the harness.
- The conformance kit's reference cases: `conformance/kit.json` and 100 cases, one subject feature
  each, checked by `codespec-conformance` (`task conformance-cases`).
- `codespec-write-go` writes `sorted_keys`, converts a `len` of a non-`int` type, upper-cases the
  first letter of an exported lower-case name, and renames a name that would clash (a field beside
  its method, a Go built-in, an import).
- `codespec-write-rust` writes `conv` to a built-in integer and `sorted_keys`, and refuses a
  function it cannot write as valid Rust instead of failing the whole module.
- `codespec-read-go` reads a labeled loop whose label only its own `break` and `continue` use as the
  plain loop.
- `codespec conform --lang <lang> --role reader|writer`: runs an adapter against the conformance
  kit and writes a per-feature report (pass, fail, refused, untested) with the boundary checks.
- The kit holds a case for every `cir/1` feature, with Java and Rust sources beside the Go one where
  the language can express the subject, and Groovy sources where Groovy needs its own (`intdiv`).
  A subject may have several cases; a feature passes only when every one passes.
- Every reader and writer conforms: the Go reader claims 92 features, the Rust reader 88, the Java
  reader 76, the Groovy reader 72, the Go and Rust writers 104 each. The support matrix on the
  documentation site shows each cell's verdict.
- `codespec-write-rust` writes shifts with Go's semantics, newtype arithmetic, channels, `any`,
  pointers and dereference; it refuses only `stmt.foreign`.
- `codespec-read-groovy` reads Grails services and domain classes, keeping GORM calls foreign, and
  reads `a.intdiv(b)` as integer division. `examples/fees` runs a Grails service as Groovy, Go and
  Rust byte-equal.
- A JVM reference field with no initializer reads as a nil pointer.

## 0.2.0

### Added

- `cir/1` statements `for` (`init`, `cond`, `post`, each optional), `break` and `continue`. They are
  additive: every earlier document reads unchanged; a document using them is refused by a 0.1.2
  `codespec`.
- `codespec-read-go` reads three-clause and condition-only `for` loops, unlabeled `break` and
  `continue`, and `switch` with or without a tag. A `break` that would leave a `switch`, a labeled
  branch, `goto` and `fallthrough` are kept as `foreign` with a refusal.
- `codespec-write-go` writes them back; a `break` inside a `switch` gets a label on its loop.
- `codespec-write-rust` writes `for` as `while` or `loop` in a block holding `init`, runs `post`
  before each `continue`, writes `a, b = x, y` as every value bound first and then each target
  assigned, and moves a local slice or map that no later statement reads instead of refusing the
  copy.
- `examples/series`: an algorithmic Go package (insertion sort, sliding window, gcd, Collatz steps,
  trial-division primes, `switch` classification, integer overflow). Read, passed, written as Go and
  as Rust with no refusal; the harness trace `series` runs the original, the Go output and the Rust
  output on 200 seeds of 300 commands byte-equal (`task examples-series`).
- The documentation site has a generated page for `series`, `/docs/examples/series`: the Go
  source, the `cir/1` of four functions drawn as a tree, and each Go construct beside the Rust
  written from it.

## 0.1.2

### Fixed

- The release archives now hold `codespec-read-rust`; the `0.1.1` archives left it out. The release
  smoke test reads `examples/inventory` with it and compares the document with the checked-in one.

## 0.1.1

The first published binaries. The `0.1.0` tag exists, but its release build failed and it has no
release.

### Fixed

- `codespec-read-go` built with `-trimpath`, as the release archives are, finds the standard
  library through `go env GOROOT` instead of failing with `could not import errors`.
- The release build runs its macOS archive on `macos-15`; GitHub no longer offers `macos-14`.

## 0.1.0

The first release: the model, the Go and Rust readers, the passes, the Go and Rust writers and the harness.

### Added

- `cir/1`: the model in `b10x-codespec-model`, strict reading (unknown fields, unknown kinds, an
  untyped expression and any other format are refused), canonical JSON, and
  `schemas/cir-1.schema.json` generated from it.
- `codespec validate [--canonical]`, `codespec schema`, and `codespec read` / `codespec write`,
  which run a language's adapter and validate what crosses to and from it.
- `codespec-read-go`: a Go reader on `go/types`. A construct `cir/1` cannot express is kept as a
  `foreign` statement and listed as a refusal. Its `reader.version` is 0.1.0.
- `codespec-read-rust`: a Rust reader on `syn` with its own name and type resolution for the
  `cir/1` subset (`codespec read --lang rust <crate dir>`). It maps Rust's idioms back to the
  Go-shaped forms the passes read: `Result<T, Error>` to `(T, error)`, `Option<T>` to `(T, bool)`,
  `&mut self` to pointer receivers, `enum Error` to sentinel errors, `Display` to `String()` and
  `Error()`. A construct it cannot type is kept as a `foreign` statement with a refusal at its line.
  The Rust writer's `orders` output reads back to the passed document but for three named
  exceptions.
- `codespec pass`: the `lookup-guard` rewrite (a comma-ok map lookup and its early-return guard
  become one `lookup_else`) and seven facts in `Module.facts` (`shared`, `messages`, `fallible`,
  `comma_ok`, `read_only`, `displays`, `unordered_iteration`). Running it on its own output gives
  the same bytes.
- `codespec-write-go`: a Go writer that builds `go/ast` and prints it with `go/printer`; the output
  is gofmt-clean and vetted, `foreign` Go is spliced back verbatim, and a construct Go cannot hold
  is refused with its position, nothing written.
- `codespec-write-rust`: a Rust writer on `syn` and `prettyplease` that writes a Cargo crate
  (`Cargo.toml`, `src/lib.rs`, `refusals.json`) from a module and its facts. A function it cannot
  write is left out and listed in `refusals.json`; a module without facts is refused whole. The
  crate builds without warnings.
- `codespec harness <example>`: builds every implementation an example's `harness.toml` lists, runs
  each on seeded traces (`--seeds`, `--commands`, `--build-dir`) and requires byte-equal output; a
  difference exits 1 naming the seed, the implementation and the first differing line.
- `examples/orders`: a Go order service, its `cir/1` documents, its Go and Rust outputs and the
  harness drivers. The original, the Go output and the Rust output agree on 200 traces of 300
  commands.
- `examples/greeter`: a Go greeting service written as Go and as Rust with no refusal.
- `examples/inventory`: a hand-written Rust crate read, passed and written as Go and as Rust; the
  original, the Go output and the Rust output agree on 200 traces of 300 commands.
- `harness.toml` `trace`: `orders` (the default) or `inventory` selects the trace generator.
- The documentation site, <https://beyond10x.github.io/codespec/>, with its generated CLI, crate,
  `cir/1` and status pages and the showcase, from `codespec-docs`.
