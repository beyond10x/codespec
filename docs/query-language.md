# cq: a query language over `cir/1` — design (2026-11-09)

Status: proposed. Inputs: a survey of 13 code query languages and an inventory of `cir/1`, its
derived graphs and 36 query kinds from a defect hunt over a third-party Go library (2026-11-08).
Both are summarised in § 10. Related plan items: `story:cir-patterns`, `story:defect-patterns`,
`story:taint-flows`, `story:unhandled-branches`, `story:builtin-defect-checks`.

## 1. The problem

A `codespec read` result is a set of `cir/1` documents with a type on every expression, plus the
graphs `codespec effects`, `reach` and `trace` derive from them. Today a question about that set
is either a built-in command or a throwaway script over the JSON. The 2026-11-08 hunt wrote 4 such
scripts, 1,344 lines of Node, to ask 36 kinds of question. Three things went wrong in them:

| problem | count | cause |
|---|---|---|
| "before", "after" and "between" decided by line number | 11 of 26 detecting kinds | `cir/1` has no control-flow edges, and expressions have no position |
| call resolution weaker than codespec's own | 9 kinds | the scripts rebuilt the call graph: an interface call reached every method of that name, with no CHA/RTA and no linknames |
| a missing body read as "no call here" | every negated kind | nothing in the JSON says which absences are unknown |

The language replaces the scripts. A query states the question once, runs on codespec's own graph,
and says when an answer rests on code it could not see.

## 2. Requirements

From the inventory (R-numbers are its ids):

- **R1, R18.** Input is a linked set of 200–240 documents, 40–96 MB. A whole-set query answers in
  seconds, the scale of `codespec effects` (3.6 s to 15.7 s on 234 documents).
- **R3.** Structural patterns over typed statements and expressions, with variables and type
  constraints.
- **R4.** Control flow inside a body: successor, dominance, "on every path", "between A and B".
- **R5, R6.** One call relation over read bodies and call lists, resolved by codespec's link, CHA or
  RTA dispatch, linknames and barrier. Transitive reach, with a witness path.
- **R7, R8, R9.** Type and signature predicates, method-set lookups, name lists as data.
- **R10.** Negation, aggregation, grouping.
- **R11.** Data flow within a body first; taint across the call graph later.
- **R12.** Effects, syscalls, `unknown` reasons, dispatch candidates and pass facts as relations.
- **R13.** `unknown` is an answer: a negation that crosses unread code says so.
- **R14, R15.** Deterministic output, `codespec-<name>/1` formats, Rust, minimal dependencies.
- **R16.** A match names a node that a later rewrite can replace.
- **R17.** Bodies and types come from every reader; call lists, externs and dispatch come from the Go
  reader only. Queries must still run on Rust, Java, Groovy and C documents.

## 3. Decision

**cq is a typed Datalog over a fact base derived from `cir/1`, written with named-field record
patterns, with two forms of sugar compiled into it: path patterns and typestate properties.**

| layer | what it is | taken from |
|---|---|---|
| fact base | relations computed from the documents and codespec's graphs: functions, nodes, calls, control flow, dominance, def-use, reach, effects, facts, refusals | CodeQL databases, Joern's code property graph |
| rules | Datalog: recursion, stratified negation, aggregates, parameterised modules | Soufflé, CodeQL |
| record patterns | `call{callee: "sync.Mutex.Lock", recv: R}`: fields by name, any order, omitted fields free | Glean's Angle |
| path patterns | `path(A -> B, avoid: P)` over control flow or the call graph, with a witness | GQL quantified paths, CPGQL steps |
| properties | small automata over the calls of a body or a call path, for "A, then B on every path" | Infer's Topl |

Rejected as the core:

- **Pattern-by-example** (Semgrep, ast-grep, GritQL, Comby). `cir/1` has no surface syntax of its
  own, so an example pattern would be Go or Rust. A source-language example may still be *lowered*
  into a record pattern later (§ 9).
- **jq, JSONPath, JMESPath.** No joins or recursion in JSONPath and JMESPath (RFC 9535). jq can do
  both by hand only. jq stays the escape hatch over raw documents.
- **A graph database** (Neo4j, Cypher). It would add a server and a dependency, and all-paths
  matching grows exponentially (Cypher manual). The path sugar keeps the notation without the engine.
- **Embedding Soufflé.** It is a C++ toolchain, and the workspace rule is Rust with minimal
  dependencies. Ascent compiles rules into Rust at build time, so it cannot run a user's query. cq
  has its own interpreter (§ 7).

Why Datalog: every check from the hunt is a closure plus a negation. Examples are "no remove
reachable", "no Unlock on some path to exit" and "no range check dominating the conversion".
Stratified negation over recursion is the native form of that shape. Aggregates cover the counting
kinds (`repeated`, `timer-stop-coverage`). One self-reported evaluation had agents write correct
Datalog in 88 of 90 tasks (codeintel). Named fields remove the positional mix-ups that study
reports.

## 4. The fact base

`codespec query` builds the fact base once per input set and caches it next to the documents
(`<dir>/.cq/<digest>.facts`, keyed by the documents' SHA-256 and the codespec version). Every
relation has a fixed schema. `codespec query --schema` prints them, and the documentation site
gets a generated schema page, as it does for `cir/1` (`reference/cir-1.md`).

### 4.1 Identity

`cir/1` gives functions an id (`<path>.<key>`), but not call sites, lambdas, statements or
expressions (inventory § 1.10). The fact base gives every node an id:

```
<module path>.<function key>#<index path>       e.g. net/http.Client.Do#3.then.1.x
```

The index path is the position of the node in the function's tree: statement 3, its `then` branch,
statement 1, field `x`. It is stable under re-reading the same source, unique within a line, and
names the node a rewrite would replace (R16). Reports print it beside the `file:line` position the
node's statement has.

### 4.2 Relations

Relations from the documents (every reader):

| relation | fields |
|---|---|
| `module` | `path, name, lang, reader, version` |
| `func` | `id, module, name, recv, exported, kind` (`func`, `method`, `lambda`, `init`, `opaque`), `pos` |
| `param`, `result` | `func, index, name, t` |
| `stmt` | `id, func, kind, pos, parent` |
| `expr` | `id, func, kind, t, stmt` plus the kind's fields as columns (`op`, `name`, `value`, `x`, `y`, `args`) |
| `call` | `site, func, kind, callee, recv, args, pos` (one relation over `call`, `method`, `extern`, `extern_method`, `iface_call`, `call_value` and every `CallRef`) |
| `type` | `id, kind, name, path, bits, signed, elem, key, fields` (one type view over the `Type` tree and the `sig` strings of call refs) |
| `fact` | `kind, module, item` (`fallible`, `comma_ok`, `shared`, `unordered_iteration`, …) |
| `refusal` | `module, pos, what, reason` |
| `foreign` | `stmt, lang, reason, calls_known` |

Relations the engine derives per read body:

| relation | meaning |
|---|---|
| `cfg_next(a, b)` | control may pass from node `a` to node `b`. Derived from the structured statements: `if`, `for`, `for_range`, `switch`, `break`, `continue`, `return`. `cir/1` has no `goto`, so this is exact. |
| `entry(f, n)`, `exit(f, n)` | the first node of `f`, and every `return` or fall-off end |
| `dominates(a, b)`, `postdominates(a, b)` | on `cfg_next` |
| `guards(cond, n, branch)` | `n` runs only when `cond` is `true` (or `false`) |
| `def(v, n)`, `use(v, n)`, `flows(n1, n2)` | local variables: reaching definitions and def-use on `let`, `var`, `assign` |
| `value(e, v)` | the local or parameter an expression reads, so `recv: R` can mean the same value, not the same type |

Relations from codespec's graph (`crates/effects`, today internal to `set.rs`):

| relation | meaning |
|---|---|
| `calls(site, target)` | a resolved edge: link, linkname, or dispatch candidate under `--dispatch cha` or `rta` |
| `reaches(f, g)` | transitive closure, with `reaches_path` for a witness |
| `scc(f, c)` | strongly connected components |
| `effect(f, e, site)`, `syscall(f, name, site)` | the effects report as relations |
| `unknown(f, site, reason)` | every place the analysis could not see |
| `makes(f, t)` | RTA's type creation sites |
| `method_set(t, name, sig, target)`, `implements(t, iface)` | method sets and the CHA test |

### 4.3 Unknown

Each read reports what it could not see: a `foreign` statement with no `calls`, an opaque function,
a dispatch with no candidate, a reader that fills no call lists. The fact base records these in
`unknown` and `foreign`, and in `reader_has(lang, part)` (inventory § 5: only the Go reader fills
call lists today).

A negated atom `!p(x)` is **definite** when no `unknown` node can contribute a `p(x)` fact.
Otherwise it is **possible**. The engine tracks this per derived fact, as a second bit beside the
fact. A finding carries `certainty: definite | possible`, with the unknown sites that made it
possible. A query never turns an unknown into "absent". This is the evidence rule of
`docs/design.md`, applied to answers.

## 5. The language

### 5.1 Rules

```
// one rule: head :- body.
locks(F, Site, R) :- call{func: F, site: Site, callee: "sync.Mutex.Lock", recv: R}.

// record patterns: named fields, any order, omitted fields are free
narrow(E, From, To) :- expr{id: E, kind: "conv", t: To, x: X}, expr{id: X, t: From},
                       To.bits < From.bits.

// negation (stratified), regex on strings, aggregates
hot(F, C, N) :- call{func: F, callee: C}, N = count{ call{func: F, callee: C} }, N >= 3.

// a query: a rule with ?- prints its bindings
?- hot(F, C, N).
```

- Values are strings, integers, booleans, node ids and types. Types are records (`To.bits`,
  `T.kind == "int"`).
- `~"regex"` matches a string field: `callee: ~"^strconv\.Parse(Int|Uint)$"`.
- `in @names.receive` tests membership in a named list from a data file (R9). Lists ship with the
  checks and can be replaced with `--names <file>`.
- Aggregates: `count`, `min`, `max`, `sum`, `collect`, each over a body in braces.

### 5.2 Path patterns

```
path(A -> B, over: cfg, avoid: N where unlock(F, N, R))
path(F -> G, over: calls, max: 8)
```

A path pattern is true when a path exists from `A` to `B` over the named edge relation, with no node
on it matching `avoid`. `every_path(A -> B, through: P)` is its dual: every path passes a node
matching `P`. Both compile to recursive rules. A path in a finding is reported as its witness, one
shortest path chosen deterministically (lowest node ids first).

### 5.3 Properties

A property is an automaton over the calls of one body (`scope: body`) or along a call path
(`scope: calls`). It has one register per tracked value:

```
property lock_released(scope: body) {
  start -> held:  call{callee: ~"^sync\.(RW)?Mutex\.(R)?Lock$", recv: M} => m := M
  held  -> start: call{callee: ~"^sync\.(RW)?Mutex\.(R)?Unlock$", recv: m}
  held  -> start: defer call{callee: ~"^sync\.(RW)?Mutex\.(R)?Unlock$", recv: m}
  held  -> error: exit
  message "Lock at {start} has a path to return without Unlock"
}
```

It compiles to Datalog by a product of automaton states and `cfg_next`. Registers are limited to 3
(Topl reports cost exponential in registers, with 3 the most used in practice).

### 5.4 Modules and checks

A `.cq` file is a module: rules, properties and `check` declarations. A check is a query with a
name, a one-line description, a severity, and the names lists it uses:

```
check lock-without-unlock {
  about "a Lock with a path to return that passes no Unlock of the same value"
  severity medium
  find lock_released
}
```

`codespec check` (`story:builtin-defect-checks`) becomes the runner of the checks in
`checks/*.cq`, shipped in the release archive. A project adds its own with `--checks <dir>`.

## 6. Examples

Two queries in cq. Relation names are those of § 4.2. The pattern library itself is kept outside
this repository.

**Dropped error.** A fallible call whose error result is discarded.

```
dropped_error(Site) :- call{site: Site, callee: C}, fact{kind: "fallible", item: C},
                       stmt{id: S, kind: "let"}, let_name(S, -1, "_"), contains(S, Site).
dropped_error(Site) :- call{site: Site, callee: C}, fact{kind: "fallible", item: C},
                       stmt{kind: "expr", id: S}, contains(S, Site).
```

**Exploration.** "Who reaches `connect`, and how?"

```
?- reaches_path(F, G, P), syscall(G, "connect", _), func{id: F, exported: true}.
```

## 7. Engine

- **Evaluation.** Bottom-up, semi-naive, stratified. Rules are compiled into join plans with hash
  indexes on the bound fields. Aggregates are evaluated per stratum. Written in Rust, in a new crate
  `crates/query`. It adds no dependency beyond the workspace's `serde_json`.
- **Scale.** The largest measured set has 237 documents, about 57k statements, 145k expressions and
  58k call refs (inventory § 1.12). With control-flow and def-use edges, that is in the low millions
  of tuples, well within an in-memory engine. Reachability over 15k functions is the cost that
  `codespec effects` already pays.
- **Determinism.** Results are sorted by their tuple. A witness path is the lexicographically least
  shortest path. Output is `codespec-query-report/1` (JSON) or one line per finding. Exit 0 with no
  findings, 1 with findings, 2 on an error (as `story:builtin-defect-checks`).
- **Explanations.** `--explain` prints one derivation tree per finding: the rules and facts that
  produced it. Absence is explained by the `unknown` sites that make a finding `possible` (§ 4.3).

## 8. What `cir/1` needs

- **`defer` as a statement** (`{k: "defer", call}`), read by the Go reader. Today `defer` is
  `foreign` with its call list but without the receiver, so `lock_released` cannot match the
  Unlock to the Lock's value. This is additive (`cir/1` minor).
- **A receiver root on call refs** (`recv_var` on `CallRef.method` and `iface`), so value identity
  reaches into `foreign` statements. Additive.
- **Nothing else is required.** Node ids, control flow, dominance and def-use are derived by the
  engine from what `cir/1` holds (§ 4.1, § 4.2).

## 9. Plan and out of scope

Plan, as stories under `epic:codebase-transformations`:

1. `crates/query`: fact base (§ 4), interpreter (§ 7), `codespec query` with `--schema`,
   `--explain`, `-f <file>` and an inline query. Acceptance: the 8 built-in checks of
   `story:builtin-defect-checks` re-expressed in cq give the same findings on its fixtures and on
   google/capslock.
2. Path patterns and properties (§ 5.2, § 5.3).
3. The `defer` statement and the call-ref receiver root (§ 8), with conformance cases.
4. Taint (`story:taint-flows`) as a cq module over `flows` and `calls`.
5. Source-language patterns lowered into record patterns: a Go fragment such as `$m.Lock()` read by
   the Go reader into a record pattern. This gives Semgrep-style authoring without making it the
   core.
6. Rewrites (`story:cir-rewrite`): a match's node id plus a `cir/1` template.

Out of scope: an interactive server, incremental evaluation (FlowLog shows it can be added later),
inter-procedural path sensitivity, and points-to analysis beyond codespec's dispatch.

## 10. Sources

- Survey (2026-11-09) of CodeQL, Semgrep, ast-grep, GritQL, Joern, Soufflé, Glean, tree-sitter
  queries, jq/JSONPath/JMESPath, Comby, GQL/Cypher, Infer Topl, FlowLog, Ascent and weggli. Each
  claim was checked against the project's documentation. Findings used above:
  - CodeQL's lock query is about 120 lines and rated medium precision.
  - Semgrep's `...` matches statement sequences, not control-flow paths, and its data flow states
    "no path sensitivity".
  - Joern hides raw queries behind tools because LLMs "struggle to generate complex static analysis
    queries" (codebadger, 2026).
  - Cypher's manual warns of exponential all-paths matching.
  - Topl reports cost exponential in registers.
- Inventory (2026-11-09) of `crates/model`, `schemas/cir-1.schema.json`, `crates/effects`,
  `docs/design.md`, `docs/static-analysis.md`, the planned stories, and the hunt's query scripts
  (kept outside this repository): 26 query kinds that detected a confirmed finding, 10 that did not,
  each with the primitives it needs.
