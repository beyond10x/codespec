# Changelog

Every change a user of `codespec` or of `cir/1` would notice.

## Unreleased

### Added

- `codespec-read-java` claims `decl.union`, 103 features, up from 102: a sealed interface whose
  permitted classes are records without members is read as a `union` in a package read, where
  it was refused at its line. The Go writer refuses a union, and the reader check's evaluator step
  judges the kit case `decl-union` instead. A sealed interface one of whose records declares a
  member is still refused at its line, its records with it.

### Fixed

- `codespec-run-cir` calls a function a `calls.json` names as Go spells it (`Echo`) by the name a
  reader of another language gives it (`echo`), as the Go writer exports it. The reader check's
  evaluator step refused such a call as one that does not fit, so no reader but Go's could pass a
  case the Go writer refuses.

## 0.7.0

### Added

- `cir/1` has `raise`, a statement that leaves a function and every caller up to the framework
  with a value of an extern type, and `raises` on a function, the extern types it can raise, its
  own and its callees', outside its results. The entry kind `exception-mapper` names the
  function the framework hands them to, `on` the extern type it takes. Additive: a document
  without them reads and writes byte for byte as before. Strict reading admits `raise` only in a
  function an entry names or one only callers listing what it raises reach (never from a
  variable's initializer or `init`, which run when the module loads), and refuses a `raises`
  that differs from what the bodies raise. `codespec effects` lists a function's `raises` with
  its callees', `codespec check` (`lock-without-unlock`) finds a lock held at a `raise` or at a
  call of a function that raises, `codespec query` has the relation `raises` and counts a
  `raise` and a call of a function that raises as exits, and `codespec eval` stops at a `raise`
  by name with its node id. `codespec-write-go` and `codespec-write-c` refuse a function holding
  `raise` by name with exit 1, `codespec-write-rust` lists it in `refusals.json`, and
  `codespec-write-cir` refuses it as it does a `foreign` statement; no reader or writer claims
  `stmt.raise`. The reader check counts an evaluator stop at a `raise` as `refused`, and
  `codespec-eval-diff` skips a function that raises, naming why. The kit case `stmt-raise` is
  written by hand.
- `codespec-read-java` reads, in a project, a `throw new X(…)` of a jar's unchecked exception in a
  method an entry names as `raise`, a `@Provider` `ExceptionMapper` as an `exception-mapper` entry,
  and prints a path parameter only the framework passes. The Quarkus quickstart reads with 8
  refusals, down from 13 (`adapters/java/README.md`).
- Fault plans for `codespec eval --session`: `--fault 'SITE:NTH=DO'` and `--faults FILE`
  (`codespec-faults/1`) make the effect at a node id, an extern call or a send, fail at a call
  count (`*` for every call) before a stub or handler answers: `error` (the call's last result an
  error, the others zero), `value`, `closed` (a send panics as on a closed channel), `block` (the
  run stops `blocked`) or `panic`. The trace gains `fault`; `update`, written when the faulted
  call ends, for each place of memory older than the call that the call left changed, as it is
  then (the partial update; a write undone is none); `tested`, `propagated`, `wrapped` and
  `dropped` for the fault's error, followed through errors, structs, slices, maps and pointers
  that hold it and into later calls; and one `summary` per fault: `panic, not handled in module`,
  `blocked`, `answered`, `propagated to <entry>`, `kept at <node>`, `handled at <node>`, `dropped
  at <node>`, `live at stop`, or `not reached`. `--each --sites FILE --max-nth N --out DIR` runs
  the session once per site and call count, a send `closed`, a call returning an error given
  one, another call `panic`, and writes one trace per run. A fault naming no effect site, a
  second for one site, a kind its site cannot take, a stub that a fault of every call leaves
  without a call, two `--each` sites whose trace files differ only in letter case, or a plan on a
  `--calls` run is refused (exit 2). A session or plan file whose object gives one key twice is
  refused. `docs/evaluator.md`
  § 8.1 is reproduced as `examples/orders/faults/pay-closed.eval-trace.jsonl`, and every send of
  `orders` faulted at its first call as `examples/orders/faults/each/` (`task eval-traces`). In
  the library, `Options` has `faults`, and `fault` has the plan, `read_sites` and `run_each`.

- `codespec eval --session` runs a `codespec-session/1` file, whose calls bind results that later
  calls take as receivers and arguments (`$name`), and prints the run's trace,
  `codespec-eval-trace/1`: one JSON object per call, return, effect (an extern call with the
  handler that answered, a send on a channel the session passed, a `println`), panic and stop. A
  panic ends its call, not the session. A bound value is a variable: a pointer-receiver method
  changes it. Writing the trace runs none of the module's code: an error is written by what it
  holds (`error(ErrWrongState{Status: "paid"})`), not through its `Error` method. `examples/orders/orders.session.json` gives the checked-in
  `orders.eval-trace.jsonl` (`task eval-traces` writes it, a test checks it). `--env KEY=VALUE`
  gives the run its environment, which `os.Getenv` and `os.LookupEnv` now read (empty unless
  given, never the host's); `strings.ToUpper`, `ToLower` and `TrimSpace` have built-in handlers
  on ASCII, so `envcheck.Banner("svc", 80)` with `--env APP_MODE=prod` prints `"PROD svc:80"`.
  `--stub SITE=VALUES` answers for an extern at every call (`SITE` its key) or at one call
  (`SITE` the call's node id), the values JSON, one per result (a site given twice, or a stub that
  would answer no call, is refused); an extern without a handler still
  stops the run unless a stub answers, and standard error names every site a stub answered at
  and how often. In the library, `Options` has `env` and `stubs`, and `Run` has `stubbed`.

- `codespec query` runs path patterns and properties (`docs/query-language.md` § 5.2 and § 5.3).
  `path(A -> B, over: cfg|calls, avoid: N where body, max: K, witness: W)` holds when a path
  passing no avoided node joins `A` to `B` over control flow or the linked calls, `W` its
  witness, the least of the shortest paths by node id; `!path(…)` and `every_path(A -> B,
  through: P)` are its negation. `property name(scope: body|calls) { … }` declares a small
  automaton over the calls of a body along its control flow, with `exit` at each return, or
  over the calls of a call path from each entry of the call graph (a function no call from
  outside its own strongly connected component reaches, so recursion hides no entry);
  transitions match record patterns of `call`, at most three registers carry a value from one
  to the next, and the property is the relation `name(Start, At, Message)` of the runs that
  reach `error`. The relation `reaches_path(from, to, path)` gives `reaches` its witness. Code
  nothing read makes a path, a negated path, a rule over a path or a property possible, naming
  the site; so does a register compared with the receiver of a listed call, which nothing read.
  `defer call{…}` is refused until `cir/1` has the `defer` statement (§ 8);
  `lock-without-unlock` waits for that alone now.
- The reader check's step 4 falls back to the evaluator when the Go writer refuses the passed
  module: `codespec-write-cir` writes it and `codespec-run-cir` runs the calls, both looked up in
  `--adapters` when a case first needs them, so a reader can be judged on `decl.union`, for which
  Go has no form. Output equal to `expected.txt` passes, different output fails; a stop is never a
  pass: a `foreign`, `no handler`, `no body` or `unsupported` stop is `refused` with the stop as
  the detail, a `panic`, `blocked` or `step budget` stop is `run_failed`. Such a feature entry
  carries the new optional field `runner`, the evaluator's program that judged it
  (`codespec-run-cir`, or `codespec-write-cir` when it refused the module). In the library,
  `conform_reader` and `conform_reader_within` take the programs as one `ReaderPrograms`.
- The evaluator is a conformance adapter: `codespec-write-cir` copies a module and refuses one
  holding a `foreign` statement, `codespec-run-cir` runs the calls with the evaluator, so
  `codespec conform --lang cir --role writer` judges it like any writer. It claims every kit
  feature but `stmt.foreign`, which it refuses, and `intrinsic.append`, whose case passes but
  whose sharing of a backing array may differ from Go's where Go's compiler puts a slice on the
  stack; its report is `website/data/conformance/writer-cir.json` and the support matrix has an
  Evaluator column, whose notes give the reason for each unclaimed feature. Both programs ship in
  the release archives.
- `task eval-diff` (in `task check`), per-function differential runs: every exported function of
  `examples/series` and `examples/inventory` gets 200 seeded calls, a method through a generated
  probe function that builds its receiver from the constructor and up to four drawn calls of the
  type's mutators (one that would return an error is skipped), and the evaluator,
  `codespec-run-go` and `codespec-run-rust` must print the same bytes. A method returning nothing
  prints its receiver's state after the call. Each example's line counts the calls that reached
  their function and those printing a result; a function reached by fewer than half its seeds
  fails. `Inventory.Find` is not run as Rust: the Rust writer refuses every call of a comma-ok
  function. Nothing is timed.
- `codespec-read-rust` claims 111 of the 120 features a reader can claim, up from 89. It reads
  generics: a function or struct with type parameters, each unbounded (`any`) or bound by one
  trait of the crate (its interface), a `where T: Trait` clause, an impl of a generic struct, and
  an instantiation written with a turbofish or inferred from the arguments, written out. A bound
  `cir/1` has no constraint for (`Clone`, `PartialEq`, two bounds), another `where` clause, a
  lifetime or const parameter, an associated type, `impl Trait` as a parameter type and a method
  with its own type parameters are refused at their line. It also reads
  `f64` and `f32` with their literals, `[T; N]` with `[0; N]` and indexing, `static` and
  `static mut` items as module-level variables (a `static mut` inside `unsafe` around it only),
  `fn(A) -> R` with a function's name as a value and a call through a local, parameter or field,
  a closure whose parameter and result types are written out as a function literal with its
  captures (a `move` closure is refused), a trait of the crate whose methods take `&self` as an
  interface with `&dyn Trait` and `Box<dyn Trait>` its type, an `impl Trait for T` as `T`'s
  methods and the coercion to the trait object as `to_iface`, `Rc<dyn Any>` as `any`, the Rust
  writer's channel with both ends (`Rc<(SyncSender<T>, Receiver<T>)>`, `try_send`, `try_recv`)
  as `chan T`, `T::default()` of a `#[derive(Default)]` whose every field defaults to its zero as
  `zero`, and map keys collected and sorted (`sort()` for integers, by `encode_utf16` for
  strings) as `sorted_keys`. Kit cases gain the Rust sources that hold the 22 new claims. A raw
  identifier (`r#type`, `struct r#match`, a raw trait or path segment) is named without its `r#`
  in every position.
- `codespec-read-c` reads C in runtime form: a directory holding the kit's runtime header,
  `conformance/c/codespec_rt.h` (the one the C writer writes), whose functions carry
  `/* codespec: func … */` lines. It reads `cs_string`, `cs_slice`, `cs_map`, `cs_chan`, `cs_any`
  and `cs_error` in the forms its crate doc lists (literals, wrapping arithmetic through
  `uint64_t`, append, index, map put and get, send and receive, boxing, sentinel errors, string
  builders as `sprintf`, `errorf` and `println`, result structs as tuples, multiple assignment,
  lookup with a miss, switch as an if chain, loops over a slice or map as `range`), and refuses a
  near miss of each by name. It claims 103 kit features, up from 76: `decl.var`, `expr.errvar`,
  `expr.index`, `expr.intrinsic`, `expr.make`, `intrinsic.*`, `op.eq`, `sem.error.propagate`,
  `sem.int.wrap`, `sem.map.order`, `sem.multi_assign.order`, `sem.switch.break`,
  `stmt.for_range`, `stmt.lookup_else`, `stmt.send`, `type.any`, `type.chan`, `type.error`,
  `type.map`, `type.slice`, `type.tuple`, `unop.recv`. A directory name that is no identifier
  (`zlib-1.3.1`) reads to one (`zlib_1_3_1`), and a parameter without a name reads as `_`.
- `codespec-read-c` reads what the C writer writes: its `<m>_` prefix, its static helpers, its
  range blocks, scoped `for`, `(void)` calls, shifts, zero literals and `int64_t` newtypes
  (`task c-round-trip` re-reads the writer's output for every kit case). It refuses an operator
  or call with two operands that make calls (C leaves their order unspecified), a multiple
  assignment whose later target reads an earlier one, an assignment through `cs_map_get` or
  `cs_chan_recv`, an argument or result of another type than the `codespec: func` line's, a
  receive or send against a channel's direction, and a loop over a map whose result depends on
  the order of its entries. `int64_t` and `uint64_t` take their width from the line.
- `codespec-read-c` refuses more orders C leaves open: two operands of which one is an `append`
  or an operation that can panic (an integer `/` or `%`, an index, a shift by a signed count)
  beside a call; an assignment to a place reading a global, memory or a local whose address is
  taken, from a call; a multiple assignment or a loop over a map that reads or writes through a
  pointer where a name may be aliased. A call of a function without a `codespec: func` line
  checks each argument against its C parameter type. A name of the C writer's that would lose
  its prefix onto the name of another declaration, a parameter or a local keeps it, with a
  refusal. A `codespec: func` line spells a receive-only channel `<-chan T`.
- `codespec-run-c` compiles each `.c` file as its own translation unit, as the reader reads it.
- `codespec-run-c` runs C written by hand in runtime form, typed by its `codespec: func` lines;
  a call of a function without one is refused. The C row of the reader corpus holds
  `codespec_rt.h` beside each `case.c`.
- `codespec-read-c` reads floats, pointers, arrays, function pointers and newtypes: `float` and
  `double`, their literals, `T *` with `&`, `*`, `->` and a null pointer as `nil`, `T[N]` with
  `a[i]`, `R (*)(P)` with a function's name as a value and a call through the pointer, and a
  `typedef` of an integer, `_Bool` or floating-point type as a newtype with `(T)x` its
  conversion, and a struct literal of zeros, `(struct T){0}`, as `zero`. It claims the 17 kit
  features whose cases now have a C source: `decl.newtype`, `expr.call_value`, `expr.conv`,
  `expr.float`, `expr.func_ref`, `expr.nil`, `expr.zero`, `stmt.continue`, `stmt.expr`,
  `type.array`, `type.float`, `type.func`, `type.named`, `type.ptr`, `type.unit`, `unop.deref`,
  `unop.ref`. `decl.variadic` and `expr.index` stay unclaimed; the crate doc says why. It refuses
  by name what the Go written from it would run otherwise: `-` of a floating-point literal and an
  operator on floating-point constants only, `&` of a variable a `for` declares, a second
  `typedef` of a name to another type and a `typedef` named as a struct or enum of the module.
- Three kit cases are variants of a feature whose first case needs a slice, an error or a string:
  `expr-nil-ptr`, `expr-zero-struct` (its `case.cir.json` written by hand, as the Go reader never
  writes `zero`) and `type-array-int`; `type-float-compare` has a C source.
- `codespec-run-c` runs C written by hand that declares `typedef long Name;` and passes or returns
  `Name` as an integer, and reads a function pointer or array parameter as one parameter; a call
  of a function taking a float, a pointer, an array or a function pointer is refused naming its
  type.
- `codespec-read-java` claims 102 of the 120 features a reader can claim, up from 77. A package
  read now maps shifts with the count masked as Java masks it, labeled `break` and `continue` of
  the innermost loop, `double` and `float`, mutable static fields as module-level variables,
  `Object` as `any`, interfaces with several implementations, unbounded static generic methods,
  the primitive functional interfaces with lambdas and method references, variable arity methods
  and arrays as slices, formatted exception messages as `errorf` and `System.out.println`. Each
  has a refused edge (`>>>`, an outer loop's label, `%` on a float, arithmetic on float
  constants, a shifted literal whose count is not constant, among others) at its line. A project
  read is unchanged. The kit holds the Java
  sources of these cases and six new variant cases: `decl-var-int`, `type-float-compare`,
  `type-tuple-error`, `intrinsic-println-int`, `decl-variadic-local` and `expr-spread-local`. The
  case `decl-union` has a Java source, a sealed interface of records, which the reader refuses at
  its line and does not claim, since the Go writer that checks a read refuses a union. `adapters/java/README.md` gives the reason for each feature the reader
  leaves unclaimed.
- `codespec-read-java` reads an integer constant expression that wraps (`2147483647 + 1`) as the
  literal of the value Java gives it, where the Go written from it computed the exact value. It
  refuses a compound assignment to a field whose right operand calls a method of the package, since
  Java reads the field before the call and the written Go after it.
- `codespec-run-java` and `codespec-run-groovy` print a result of type `Object` as the value it
  holds.
- `codespec query` answers a question over a set of `cir/1` documents written in cq, a typed
  Datalog with named-field record patterns (`call{func: F, callee: C}`), recursion, stratified
  negation and aggregates (crate `b10x-codespec-query`). `--schema` prints the fact base's
  relations: functions, statements, expressions and calls, each node with the id
  `<module path>.<function key>#<index path>`, types, facts, control flow, dominance, def-use, the
  call graph with its strongly connected components, the types each function makes, the CHA
  `implements` test, and `codespec effects`' effects and syscalls (from `--summaries` and
  `--barrier`); `reaches_path` is named as not built yet. The fact base is cached as
  `.cq/<digest>.facts` next to the first document, keyed by the documents and by the summaries
  in their order. A row whose negation crosses an opaque function, a foreign statement without a
  call list or an unlinked call (in the bodies, the call graph, effects, syscalls and the types
  made), or negates a fact that only possibly holds, is marked `possible`, with those sites; an
  aggregate over facts that only possibly hold takes each value they allow. `--explain` prints
  the rules by stratum. Path patterns and properties are refused by name; nesting, body length,
  the bodies a program compiles into and regular expressions have stated limits, past which a
  query is refused. `--names` reads the file `codespec check --names` reads, each list as
  `codespec check` reads it, and refuses a list neither knows. Seven of the eight checks of the
  first plan item are written in cq in `crates/query/checks` and give the findings `codespec
  check` gives on 21 of the 22 fixtures of those checks in `crates/check/tests/fixtures`; the
  22nd, `unreached/library`, needs `--exported`, which the cq check does not take.
  `lock-without-unlock` waits for properties and the `defer` statement.
- `codespec eval MODULE --calls CALLS` runs a `cir/1` module on a `codespec-calls/1` file with a
  new evaluator, the crate `b10x-codespec-eval`, and prints what the conformance kit's native
  runners print. A run that executes a `foreign` statement, calls an extern with no built-in
  handler or would panic stops with exit 1, naming the node by its id
  (`<module path>.<function key>#<index path>`), its position and the reason. A module that calls
  an undeclared function, passes a wrong number of arguments, names an undeclared type or a type
  holding itself, or needs a zero value over 4,194,304 cells, and an argument outside its
  parameter's width, are refused with exit 2. The gate step `eval-kit` runs every kit case twice
  and requires both runs equal to its `expected.txt`, or, for a case listed in
  `crates/eval/tests/kit-stops.txt`, the stop listed there after a prefix of its `expected.txt`.
- `codespec-read-go` claims `type.tuple`, which its case already passed, and its
  `adapters/go/cmd/codespec-read-go/README.md` names the five features it leaves unclaimed, each
  with its reason: `decl.union`, `expr.zero`, `intrinsic.sorted_keys`, `type.unit` and
  `stmt.lookup_else`.
- `codespec-read-go` reads `slices.Sorted(maps.Keys(m))` over integer keys as `sorted_keys`. Over
  string keys it stays as before: Go orders strings by byte, `sorted_keys` by UTF-16 code unit.
- A project read of `codespec-read-java` reads a second constructor (`New<Class>2`), a static
  nested class, a `static final` field set by a literal or by a jar's static method with constant
  arguments (`Logger.getLogger(C.class.getName())`) as a module-level variable, the `id` an entity
  inherits from `PanacheEntity`, and a static Panache method called on an entity
  (`Fruit.deleteAll()`) as an extern call with the entity as its type argument. It refuses at its
  line a constructor calling `this(…)`, an inner class, a `static final` field set otherwise, a
  field of a class of another package (an opaque extern there), a call of another package's
  method or constructor that package does not read or that returns an error or an Optional there,
  a Panache static method javac calls on the base itself, an access of a public field of an
  `@Entity`, `@MappedSuperclass`, `@Embeddable` or Panache class, outside its constructors,
  whose getter or setter (Quarkus's name and descriptor: `getxCoord()` for `xCoord`, `isLit()`
  for a `boolean`) the class declares, and a class implementing `ExceptionMapper`. A call of
  another package's function is an extern with the signature that package's module declares. It
  records `@Entity`, `@Cacheable`, `@Id`, `@GeneratedValue` and `@Column` on a struct or its
  instance field in `persistence`, each argument on one line without comments; on a class read as
  no struct they are refused. A refusal of a construct
  read more than once, such as a field initializer each constructor starts from, is listed once.
  A package read is unchanged. The pinned `hibernate-orm-panache-quickstart` reads with 13 refusals, down
  from 25: 9 throws of a jar exception and 2 exception mappers, neither of which `cir/1` has a
  form for, and 2 reads of a field of an entity of the other package.
- `cir/1` modules have an optional field `persistence`: the persistence annotations of the module,
  each with the type or field (`Type.field`) it marks, the annotation's simple name and its
  arguments as written. Strict reading refuses a mark on a type or field the module lacks, a mark
  that is no identifier and an empty argument. A module without it reads and writes as before.

### Changed

- The Rust, C and Go writers refuse a document holding a name that is not an identifier of their
  language: exit 1, a message naming it, nothing written. Before, a module name holding a quote
  and a newline added keys to the `Cargo.toml` the Rust writer wrote (a `build` script cargo
  runs), a name could add C or Go code, and the Go writer took a module name like `../x` as a path.
  The Rust writer no longer panics on such a name. The Go writer also refuses a literal that is
  not exactly one Go literal, and a document missing a part, where it crashed.
- Strict reading of `cir/1` refuses a module, declaration, field, parameter or local name that is
  empty or holds a character no Java or Rust identifier holds after its first, a control or
  format character, or an invisible default-ignorable character (a Hangul filler, a variation
  selector), after an optional `r#` (a Rust raw identifier, which the Rust writer writes back as
  one). Letters and digits of any script, `_`, currency signs such as `$` and `€`, combining
  marks and connector punctuation are names. A module `path` is segments of letters, digits, `_`,
  `-`, `.` and `~` joined by `/`, none empty, `.` or `..`. Any text of the document holding a
  control or format character is refused: names, positions, extern paths, names and types,
  reasons, a refusal's `what`, the reader's name and version, facts; a `doc`, foreign `src`, a
  literal's `value` and an error's `message` may hold line breaks and tabs. An assignment with no
  target or no value is refused. A message quoting the document writes its control and format
  characters as escapes, in strict reading and in all three writers, and so do the Rust and C
  writers' `refusals.json`.
- The C writer writes every `\` of a doc comment as `/` and breaks every `*/`, `/*` and `??`, so a
  doc ending a line in `*\` or `*??/` no longer ends its comment and turns the next line into C
  code, and no doc line reads as a `/* codespec: … */` header line. It refuses a module named, in
  any case, like a C11, C23 or POSIX header (`stdint`, `string`, `unistd` …) or glibc's
  `features`, whose `<name>.h` would stand in for the system's. `codespec-run-c` adds the written
  directory with `-iquote` instead of `-I`, so a header there is found by `#include "…"` only.
- The Go writer writes a doc line the go command would read as a build constraint (`+build …`) in
  parentheses, and refuses a module name whose file the go command skips or builds only on some
  platforms (`x_test`, `x_windows`, `x_arm64`, `_x`) and the module name `documentation`, a
  package the go command ignores. A module named `main` is still written as `package main`. A
  position from the document is named in the message of a construct it cannot write.
- The Rust, C and Go writers refuse a module named like a Windows device (`con`, `prn`, `aux`,
  `nul`, `com0` to `com9`, `lpt0` to `lpt9`, in any case), which they would write as a file name.
- The documentation site: the cir/1 reference lists its definitions before the first table, so
  every entry is one click from the first screen. The sidebar's group names are 14 px in sentence
  case. The home page, the overview and Readers, passes and writers name every reader and writer
  (Go, Rust, Java, Groovy and C readers; Go, Rust and C writers), and Status lists the conformance
  of the C reader and the C writer. Use cases links Getting started for the orders run instead of
  repeating it, and the overview links pages by their sidebar names.
- The documentation site: field names, types and commands in tables, and table headers, are 14 px,
  headers in sentence case. The pipeline diagram is on Readers, passes and writers only, the orders
  harness run on Getting started only, and pages whose header names the run no longer say it again
  in their first paragraph.
- The documentation site: Status says it describes this tree and points to Unreleased here for
  what the tree adds to the last release, the C reader and writer among them. The home page's lede
  names the C writer. Code in tables breaks inside long identifiers, so no Markdown table scrolls
  sideways at 1280 px and wider. Feature ids and descriptions on Language support, and the headers
  and status chips on Status, are 14 px. Add a reader or writer adapter links Getting started for
  what a release archive holds.
- The Rust writer names blank parameters `_cs_p<index>`, distinct from each other and from every
  other parameter and local; two Go parameters `_`, or `_` beside `__`, no longer give Rust that
  does not compile.
- The Groovy reader refuses a method whose quoted name is no identifier (`long "add two"()`),
  `unsupported`, where it read it as a function strict reading then refused.

### Fixed

- `codespec-read-go` keeps a range with `=` that assigns a variable declared before the loop
  (`for i = range xs`), labelled or not, foreign with a refusal at its line that says so. It was
  read as a `for_range` that declares its own `i`, so the outer `i` was never assigned; a key
  that is not a name (`for s.f = range xs`) stopped the reader with a panic.
- `codespec-read-go` keeps `f(g())` with a multi-valued `g` foreign with a refusal at its line. It
  was read as a call with one argument of tuple type, which `cir/1` does not define and the
  writers refused.
- Readers bound what an untrusted source can cost: past a limit is a refusal, exit 1 and a
  message, not a crash. `codespec-read-rust` refuses a `Cargo.toml` or `src/lib.rs` over 4 MiB and
  a source nested past 256 levels, counted on its tokens before parsing, and parses and lifts on a
  256 MiB stack; nesting crashed it with a stack overflow. `codespec-read-c` reads clang's JSON AST
  as a stream and refuses it past 64 MiB per file, 256 MiB for the directory's files together or
  4096 levels, and refuses `.c` and `.h` files over 16 MiB together; clang indents a level two
  spaces, so the JSON of a deep expression grows with the square of the source, and the reader
  held all of it, every file's tree until the module was assembled. A file name clang printed once is
  held once, not once per location. `codespec-read-java` and `codespec-read-groovy` run on a 64 MiB
  stack and refuse a source nested deeper than it holds; they crashed with a stack overflow.
  `codespec-read-go`'s libraries bound depth already. The limits are in each reader's crate doc or
  README and in `docs/conformance.md`.

## 0.6.1

### Changed

- The documentation site is shorter and easier to scan. The home page shows "Get started" on the
  first screen and names no release. The sidebar is grouped Start here, Guides, Concepts,
  Examples, Reference and Project, and only the open group is expanded. The navbar is
  Documentation, Getting started, Reference, Status and GitHub. Below 997 px wide, the search box
  no longer covers the menu button.
- Every page has one name for its title, sidebar label and heading: the CLI page is
  "codespec CLI", the docs index "Overview". Code blocks over 60 lines on the generated example
  pages are folded with their line count. Content repeated on several pages is a link to the page
  that holds it.
- The prose of eleven hand-written pages is tighter: 11,363 → 11,282 words, semicolon chains split
  into sentences.

## 0.6.0

### Added

- `codespec-read-java` claims `sem.error.propagate`: the conformance case `sem-error-propagate` has
  a Java source, a call of a throwing method left uncaught, which the reader reads as the
  propagation form. `codespec-read-groovy` refuses that source (a thrown exception is not read as
  an error) and does not claim the feature. `examples/ledger` throws `InsufficientFunds` from
  `Account.post` again, and `Ledger.post` calls it without catching; the Go and Rust outputs pass
  the error on, byte-equal with the original Java under the harness.
- `codespec-read-rust` reads `let v = f(…)?;`, `let (a, b) = f(…)?;` and `f(…)?;`, `f` a fallible
  function or method of the same crate, in a function whose result is a `Result`, as
  `v, err := f(…)` followed by `if err != nil { return <zero values>, err }`, and claims
  `sem.error.propagate`: the Rust writer's `?` output reads back. A `?` on an `Option`, on a call
  outside the crate, inside an expression or in a function whose result is not a `Result` stays
  `foreign` with a refusal at its line. The case `sem-error-propagate` has a Rust source.
- The site has search: a box in the navbar on every page, over an index built with the site
  (`@easyops-cn/docusaurus-search-local`, no external service), and a results page `/search`. Links
  in running text are underlined, the docs sidebar stays in view down to the footer, and the
  support matrix legend can be scrolled by keyboard.

### Changed

- `codespec-read-go` reads `if <names> := <call>; <cond> { … }` as `<names> := <call>` followed by
  the `if`, instead of keeping it as `foreign`, when the init and the condition read inside `cir/1`,
  the `if` has no `else`, is not an `else if`, the function has no `goto`, and no name is a parameter
  or occurs in the function body outside the `if` except in another such `if` declaring it with the
  same type, where no function literal captures it and its address is never taken; the first of
  those reads as the declaration, each later one as an assignment. So
  `if err := f(); err != nil { return 0, err }` reaches the Rust writer's `?`. Inside a function
  literal the same holds with the literal's body in place of the function's, so the names stay
  scoped to the literal. An init whose call or condition holds a type outside `cir/1` is not
  hoisted. Every other `if` with an init statement stays `foreign` with its refusal.

## 0.5.0

### Added

- `examples/netcheck`, a worked example of the static analysis: a Go module with a TCP server, a
  client, an interface with two implementations, a generic function and a variadic call, read
  with the 72 standard library packages it imports. `task examples-netcheck` runs `codespec
  effects` (`rta` from both `main` functions, and `cha`), `codespec reach` and `codespec trace`
  over the set twice, byte-equal, and writes the checked-in reports and the call paths to
  `connect` and `accept4` (`codespec-docs netcheck`). `cargo test -p codespec-docs` checks facts
  on them: only the server reaches `accept4`, the client reaches `connect`, `Answer` has no effect
  and no syscall, `Prober.Probe` has 1 candidate with `rta` and 2 with `cha`. The site gains the
  generated page "netcheck: down to syscalls", the netcheck trees on the Call trees page, and the
  example in the static analysis concept page and the syscalls guide.
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
