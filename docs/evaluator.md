# An evaluator over `cir/1`: design (2026-11-09)

Status: proposed. Inputs: the operator's question of 2026-11-09 ("could we really evaluate it?"),
an inventory of `crates/model`, the conformance kit, `crates/effects` and the examples, and a survey
of interpreters, symbolic executors and fault injectors (§ 12). It reuses the node ids and
certainty marks of the query language design `cq` (`docs/query-language.md`, proposed).

## 1. The problem

`cir/1` says what a function computes. Today it is read, analysed and written out, never run. Two
questions need it run:

- **Simulation.** What happens when this I/O call fails? Which statement handles the error, which
  drops it, and what state is left behind?
- **Input search.** Which inputs reach this branch?

Native runs (a writer plus `codespec-run-<lang>`) answer neither: no hook at a call site, and
outputs only. An evaluator sees every node, and every effect passes through a handler it owns.

## 2. Requirements

- **E1.** Faithful: on every kit case, the evaluator prints the case's `expected.txt`.
- **E2.** Deterministic: the same module, inputs, seed and fault plan give the same bytes.
- **E3.** Effects at a named boundary, supplied by the host, replaceable per call site.
- **E4.** Never past the unknown: a `foreign` statement or an extern with no handler stops the
  run with a reason, or is replaced by a stub the user names. The result says which.
- **E5.** Sites named by the `cq` node id `<module>.<funckey>#<index path>`.
- **E6.** Rust, in this workspace, minimal dependencies. A solver is an optional external program.

## 3. Can `cir/1` be evaluated faithfully today?

Yes, for the part the kit pins. No, for concurrency, `defer`, or a body the reader kept `foreign`.

**What the IR carries.** A type on every expression (`Module::from_json` refuses one without),
integer `width` and `signed`, structured control flow, typed externs, interfaces, function values
and module variables. The `sem.` features (`crates/model/src/features.rs`) pin what languages
disagree on: `sem.int.wrap`, `sem.rem.sign`, `sem.continue.post`, `sem.multi_assign.order`,
`sem.map.order`, `sem.switch.break`, `sem.error.propagate`. The kit holds 135 cases, each a
module, a `codespec-calls/1` file and an `expected.txt` produced by Go.

**What blocks evaluation.**

| gap | evidence | effect |
|---|---|---|
| goroutines, `defer`, `select`, type switches, named results | the Go reader (`front.go`) keeps them `foreign` | a path through them stops (E4) |
| run-time panics | no node and no `sem.` id for index out of range, nil dereference, division by zero. The C writer's `codespec_rt.h` (in progress) exits 3 | Go's behaviour, unpinned until § 9 |
| string length unit | `intrinsic.len` is "length of a string, slice or map". Go counts bytes, Java UTF-16 units | a Java-read `len` may differ. Unpinned |
| literal spelling | `Lit.value` is "in the source language's spelling". Example strings are all double-quoted. Escapes across readers: not verified | Go quoting parsed, anything else refused |
| `sprintf` verbs | kit and examples use `%d`, `%s`, `%v` only | those three, others refused |
| slice aliasing after `append` | Go's growth policy is unspecified, no case pins it | risk where a program depends on it |
| expressions outside `cir/1` | `envcheck.Save`, the examples' only error-returning I/O call, is `foreign` because of `[]byte(...)` | no example has a real I/O error site yet |
| standard library from source | `examples/netcheck/read.txt`: 1,288 of 3,968 functions read in 75 packages (32.5%), `runtime` 264 of 932, `syscall` 31 of 294 | no evaluation down to syscalls. The boundary is the library API (§ 6) |

**Runs now:** every kit case. `greeter`, `series`, `inventory`, `ledger` and `fees` (no `foreign`),
`orders` but `Audit`, `envcheck` but `Save` (with handlers for its five externs).

**Does not run:** anything reaching a goroutine, `defer` or `recover`, or library code below the
API boundary.

`docs/conformance.md` § 9 D1 rejected an interpreter as the source of `expected.txt` ("a third
implementation of the semantics to keep right"). Go stays the reference, and the evaluator is
held to the kit as a writer is.

## 4. Decision

**A tree-walking interpreter in Rust, `crates/eval` (`b10x-codespec-eval`), over the model types,
with effects as host handlers at the library API boundary, faults selected by node id, and input
search by coverage-guided search first and concolic search later.**

Rejected:

- **Native runs as the engine.** No hook per site, no statement coverage. They stay the
  differential oracle (§ 5.4).
- **A bytecode VM first** (wasmi translates before it runs). A tree walker keeps node ids for
  free. Revisit if search is too slow.
- **Syscall leaves as the boundary.** Two thirds of the standard library is not read (§ 3), and a
  fault there lands inside `os.WriteFile`, not in the user's code. Later, for fully read paths.
- **One semantics, tools derived** (K framework, Rosette). A second formalism beside the Rust
  model. The kit already is the executable definition.
- **Symbolic execution first** (KLEE, angr). It needs the concrete engine and a solver.

## 5. The engine

### 5.1 Values

```rust
enum Value {
    Int { bits: u64, w: IntWidth, signed: bool },   // platform = 64
    Float { v: f64, bits: FloatBits }, Bool(bool), Str(Rc<[u8]>), Unit,
    Tuple(Vec<Value>), Struct(Rc<[Value]>), Array(Rc<[Value]>),
    Ptr(Place), Slice { arr: HeapId, off: u32, len: u32, cap: u32 },
    Map(HeapId), Chan(HeapId), Func(Closure),
    Error(ErrorVal), Iface { t: TypeId, v: Box<Value> }, Nil(TypeId),
    Opaque { t: ExternType, handle: u64 },          // an extern type's value, owned by a handler
}
```

- Integers are `u64` masked to their width after each operation (`sem.int.wrap`). Division and
  remainder follow Go: truncated, `MinInt64 / -1` wraps, a zero divisor panics.
- Structs and arrays are values, copied on assignment.
- Strings are bytes. `len` counts bytes until § 9 pins it.
- `ErrorVal` is a sentinel (`errvar`), a message (`errorf`), a value with an `Error` method
  (`to_error`), or a host error. Each carries `origin: Option<FaultId>` (§ 6.4).

### 5.2 Memory

One arena per run, `Vec<Cell>` indexed by `HeapId`, dropped when the run ends, no collector. A `Place` is a cell plus a path of field and index steps, so `&o.Status` and `&xs[3]`
are pointers. A local whose address is taken is put in the arena on function entry. Maps keep
entries in insertion order with a `BTreeMap` index. Iteration order is a policy: `sorted`,
`insertion` or `seeded`. Several seeds give several orders, as Miri's `-Zmiri-many-seeds` does.

### 5.3 Control

`exec(stmt) -> Flow`, one of `Next`, `Break`, `Continue`, `Return`, `Panic` and `Stop`. `continue`
runs `post` (`sem.continue.post`). A multi-target `assign` reads every value first
(`sem.multi_assign.order`). `break` leaves the innermost loop (`sem.switch.break`). A step budget
ends a run that does not return (wasmi's fuel). `Stop` carries the node id and reason of the
`foreign` statement or unhandled extern that ended it.

### 5.4 How the semantics are pinned

- **The kit is the test suite.** `codespec eval <case.cir.json> --calls calls.json` prints the
  harness encoding, and every case must print `expected.txt`.
- **One more `codespec conform` runner.** An identity writer `codespec-write-cir` copies the
  module and `codespec-run-cir` evaluates it. `conform` looks programs up by language name, so
  `codespec conform --lang cir --role writer` needs no change.
- **Differential oracle.** Inputs generated per function from its `cir/1` signature (Flourine's
  per-function differential fuzzing) go as one `codespec-calls/1` file to the evaluator and to the
  Go, Rust, Java, Groovy and, once it lands, C runners. On a difference, the evaluator's trace
  names the first divergent function (c2rust's cross-checks). The Go original or the kit decides
  who is right, never the evaluator.

## 6. Effects, handlers and faults

### 6.1 The boundary

The evaluator leaves the module's memory only at these nodes:

| node | handler key |
|---|---|
| `extern`, `extern_method` | `<path>.<name>`, `<path>.<Type>.<name>` (the effects summary's keys) |
| `send`, `unop <-` on a host channel | the channel argument, as `codespec-calls/1`'s `chan` |
| `intrinsic println` | `stdout` |
| a call into another document of a linked set | that document's function, if its body is read |

Resolution order: the fault plan, then a read body in the linked set (only for an extern its
summary calls pure), then a built-in handler, then `Stop`. Effectful externs (`fs.*`, `net`,
`env`, `time`, `rand`, `proc`, `stdin`) always go to a handler. Built-ins are pure library
functions in Rust (`strings.ToUpper`, held to the kit's `expr-extern` case) and fakes with state
in the style of Go's `testing/fstest.MapFS` and `testing/iotest.ErrReader`: a file system in
memory, a fixed environment and clock, a seeded random source. Isolation is the default, as in
Miri.

### 6.2 Fault plans

A fault is a handler that answers instead of the default at a selected site and call count (LFI's
library-level injection, Jepsen's fault schedule):

```json
{"format": "codespec-faults/1", "faults": [
  {"at": "orders.Service.Pay#3", "nth": 1, "do": "closed"},
  {"at": "envcheck.Save#0.results.0", "nth": "*", "do": "error", "error": "io/fs.PathError", "message": "no space left on device"}
]}
```

`do` is `error` (last result that error, the others zero), `value`, `closed`, `block` or `panic`.
The second entry needs § 9's `[]byte` conversion. A module without an import path uses its name.
On the command line:

```console
codespec eval orders.cir.json --session s.json --fault 'orders.Service.Pay#3:1=closed'
codespec eval set/*.cir.json --session s.json --fault-sites sites.cq --each
```

`--fault-sites` takes a `cq` query whose first column is a site. `--each` runs the session once
per site and call count up to `--max-nth`. Every call of an extern that writes the file system:

```
?- call{site: S, callee: C}, effect(_, "fs.write", S).
```

### 6.3 Sessions

`codespec-calls/1` calls top-level functions only. `orders` needs a receiver from `New`, so
`codespec-session/1` adds named results that later calls reuse:

```json
{"format": "codespec-session/1", "calls": [
  {"bind": "s", "func": "New", "args": ["chan"]},
  {"bind": "id", "func": "Service.Place", "recv": "s", "args": ["\"ann\"", "30", "false"]},
  {"func": "Service.Pay", "recv": "s", "args": ["$id"]},
  {"func": "Service.Get", "recv": "s", "args": ["$id"]}
]}
```

A panic ends its call, not the session, which models a caller that recovers per request.

### 6.4 The trace

`codespec-eval-trace/1` is one JSON object per line: `call`, `return`, `effect`, `fault`, `panic`,
`stop`, and for an error value carrying a fault's `origin`:

| event | the error |
|---|---|
| `tested` | is compared with `nil` or matched in a `switch` |
| `propagated` | is returned |
| `wrapped` | is an argument of `errorf` or `sprintf` |
| `dropped` | is bound to `_`, or goes out of scope unread |

Each fault ends in one summary: `propagated to <entry>`, `handled at <node>` or `dropped at
<node>`, plus the heap writes its call made before the fault (partial updates).

## 7. Input search

### 7.1 Options

| | coverage-guided | concolic (SAGE, KLEE) |
|---|---|---|
| finds | common branch values, call sequences | narrow equalities, arithmetic edges |
| needs | seeded generator, node coverage | a symbolic shadow per value, a solver |
| weak at | `x == 0x5eed1234`, hashes | long sessions, `sprintf`, path explosion |

A mix hands the branches search cannot open to the solver. Coverage is free in an interpreter:
the evaluator records each node and branch outcome it runs.

### 7.2 Recommendation

**First:** coverage-guided search, `codespec explore --target <node>`. Inputs come from parameter
types, sessions from a state machine over the exported functions with earlier results reused as
arguments (proptest and Hypothesis state machines). Guidance is new coverage plus branch distance
to the target, `|a - b|` for an unmet `a == b`. The generator is the harness's `SplitMix64`. A
witness is shrunk (drop calls from the end, simplify arguments, drop from the front) and kept as
a regression seed.

**Later:** concolic search on the same engine (the mix). Run concretely, record the path
condition, negate one branch per generation (SAGE), solve, replay. The solver is a child process
speaking SMT-LIB 2 (z3 or cvc5, through `easy-smt`), so nothing links C++. Bitwuzla (bit-vectors,
arrays, floating point, no strings) is an option for integer-only paths.

### 7.3 Encoding path constraints

- **Integers** are bit-vectors of their width, so wrap-around is exact. `a + b < a` with `b > 0` is
  satisfiable under `sem.int.wrap`. An integer-theory encoding would say it is not.
  ```
  (declare-const a (_ BitVec 64))
  (declare-const b (_ BitVec 64))
  (assert (bvslt (bvadd a b) a))
  (assert (bvsgt b #x0000000000000000))
  ```
  `%` is `bvsrem` (`sem.rem.sign`), `/` is `bvsdiv`.
- **Strings** are byte sequences, `(Seq (_ BitVec 8))` in z3, so `len` counts bytes. Concatenation,
  equality and `len` stay symbolic. `sprintf` and extern results are concretised, and branches on
  them are marked incomplete.
- **Maps** in a session are concrete, built by earlier calls. A symbolic key's lookup is a
  disjunction over present keys plus "absent": `s.orders[id]` is `id = "o-1" ∨ id = "o-2" ∨
  id ∉ {"o-1", "o-2"}`. A symbolic map (an SMT array plus a key set) is later.

### 7.4 Unknown code

At a `foreign` statement or an extern with no handler, concrete search stops the path and reports
the node and reason. Concolic search havocs: results become fresh symbols of their types.

A reached target is **definite** when its witness replays concretely and crosses no unknown node,
else **possible** with the unknown sites listed, as `cq` marks findings (query design § 4.3). An
unreached target is "not reached within budget", never "unreachable".

## 8. Examples

Illustrative output: the evaluator does not exist yet.

### 8.1 A failing send in `orders`

`orders` has no call that returns an I/O error. Its effect is the event send `s.events <- …`. A
Go send returns no error, so the faults are what Go defines: a closed channel panics, a full one
with no reader blocks. Inject `closed` into the first `Pay`:

```console
$ codespec eval examples/orders/orders.cir.json --session pay.session.json \
    --fault 'orders.Service.Pay#3:1=closed'
call   New("chan")                     -> s
call   s.Place("ann", 30, false)       -> "o-1"
effect send OrderPlaced{ID: "o-1", Customer: "ann", Total: 30}   orders.Service.Place#4  orders.go:69
call   s.Pay("o-1")
fault  closed                          orders.Service.Pay#3  orders.go:82  nth 1
panic  send on closed channel          orders.Service.Pay#3  orders.go:82
  partial update: (*Order "o-1").Status = "paid"   orders.Service.Pay#2  orders.go:81
call   s.Get("o-1")                    -> {ID: "o-1", Status: "paid", …} true
summary  fault orders.Service.Pay#3: panic, not handled in module. 1 partial update, no event
```

The finding: a caller that recovers sees `o-1` paid with no `OrderPaid` event (status written at
line 81, send at line 82). With `block`, the evaluator reports `blocked: no receiver`. Once `envcheck.Save` reads (§ 9), an `error` fault at
`envcheck.Save#0.results.0` reports `propagated to envcheck.Save`.

### 8.2 Inputs reaching a branch

Target: the `return ErrWrongState` of `Cancel`, `orders.Service.Cancel#1.then.0`
(`orders.go:106`). It needs an order that is neither pending nor paid.

```console
$ codespec explore examples/orders/orders.cir.json --target 'orders.Service.Cancel#1.then.0' --seed 1
reached orders.Service.Cancel#1.then.0 (orders.go:106), definite
  s  = New(chan)
  id = s.Place("a", 1, false)   -> "o-1"
  s.Cancel(id)                  -> nil
  s.Cancel(id)                  -> err "order is cancelled"
shrunk from 11 calls to 4. Written to orders.Cancel-1-then-0.session.json
```

`orders.Service.Place#0.then.0` (`total <= 0`) is reached at once: as in the harness, a fifth of
generated integers are zero, negative or extreme.

## 9. What `cir/1` needs

Vocabulary and kit additions (`features.rs` and cases, no model change):

- `sem.panic.index`, `sem.panic.nil`, `sem.panic.div_zero`: the call ends in a panic. The calls
  output encoding gains a `panic <message>` line that every runner prints.
- `sem.string.bytes`: `len` and string indexing count bytes. A UTF-16 reader converts or refuses.

Model additions (additive, `cir/1` minor):

- `defer` as a statement (shared with the query design § 8), then `panic` and `recover`.
- A conversion between `string` and `[]byte`, so `envcheck.Save` and most file I/O sites read.

To check: whether every reader spells string literals as Go's `strconv.Quote` does (not
verified). If yes, the model doc says so. If not, that is `cir/2`.

Out: goroutines and `select`.

## 10. Runtime: one evaluator

Settled by the operator on 2026-11-09: there is exactly one runtime, this evaluator over `cir/1`,
written in Rust. Native runs stay only as the existing writer plus run-adapter path, for
conformance and differential comparison (§ 5.4).

## 11. Story plan

Under a new epic `epic:cir-evaluator`, in order:

1. **Core evaluator and the kit.** `crates/eval` (§ 5), `codespec eval --calls`. Acceptance: every
   kit case prints its `expected.txt`, twice byte-equal, in `task check`.
2. **Conformance runner and differential runs.** `codespec-write-cir`, `codespec-run-cir`,
   per-function differential runs. Acceptance: `codespec conform --lang cir --role writer` reports
   `conforms: true`, and 200 seeds per exported function of `series` and `inventory` are
   byte-equal with `codespec-run-go` and `codespec-run-rust`.
3. **Handlers, sessions and traces** (§ 6.1, § 6.3, § 6.4). Acceptance: a checked-in `orders`
   session gives a checked-in trace, and `envcheck.Banner("svc", 80)` with `--env APP_MODE=prod`
   gives `"PROD svc:80"`.
4. **Fault plans** (§ 6.2), `--fault`, `--each`. Acceptance: § 8.1 reproduced as a checked-in
   trace, and a fixture that discards an error reports `dropped at <node>`.
5. **Input search** (§ 7.2 first step). Acceptance: every `return` of the `orders` methods but
   `Audit` is reached with seed 1, each witness replays, the run twice byte-equal.
6. **`cir/1` additions** (§ 9), with kit cases, readers and writers. Acceptance: the new cases pass
   on the evaluator and the Go, Rust and C runners, and `envcheck.Save` reads without `foreign`.
7. **Sites from `cq`**, once `crates/query` ships. Acceptance: a query for every `send` in
   `orders` selects 4 sites, and `--each` gives 4 traces.
8. **Concolic search** (§ 7.2 later step), optional solver. Acceptance: a fixture branch on
   `a * b == 0x3fffffff00000001` that story 5 misses within its budget is reached, and the witness
   replays concretely.

## 12. Sources

Repository (origin/main d328913): the files cited above. Read on unmerged branches:
`docs/query-language.md` and the C writer's `codespec_rt.h`.

Prior art, each page checked on 2026-11-09:

- Miri <https://github.com/rust-lang/miri>
- KLEE <https://klee-se.org/> (its environment models: not verified on that page)
- Godefroid, Levin, Molnar, "Automated Whitebox Fuzz Testing" (SAGE), NDSS 2008
  <https://www.ndss-symposium.org/ndss2008/automated-whitebox-fuzz-testing/>
- angr SimProcedures <https://docs.angr.io/en/latest/extending-angr/simprocedures.html>
- Rosette <https://emina.github.io/rosette/>, the K framework <https://kframework.org/>
- wasmi <https://github.com/wasmi-labs/wasmi>
- proptest state machines <https://proptest-rs.github.io/proptest/proptest/state-machine.html>,
  Hypothesis stateful testing <https://hypothesis.readthedocs.io/en/latest/stateful.html>
- Marinescu, Candea, "LFI: A Practical and General Library-Level Fault Injector", DSN 2009
  <https://infoscience.epfl.ch/record/139395>
- Jepsen <https://github.com/jepsen-io/jepsen>
- Go `testing/iotest` <https://pkg.go.dev/testing/iotest>, `testing/fstest`
  <https://pkg.go.dev/testing/fstest>
- `easy-smt` <https://docs.rs/easy-smt>, the `z3` crate <https://docs.rs/z3>, Bitwuzla
  <https://bitwuzla.github.io/>. A pure-Rust solver with bit-vectors and strings: none found, not
  verified as absent.
- Static, for comparison: CodeQL <https://codeql.github.com/>, Infer <https://fbinfer.com/>
- Flourine <https://arxiv.org/abs/2405.11514>, c2rust cross-checks
  <https://github.com/immunant/c2rust/blob/master/docs/cross-check-tutorial.md>, both from the
  2026-11-09 landscape survey (kept outside this repository)
