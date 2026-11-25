---
format: aep.planning-md/3
id: story:ecmascript-reader
kind: story
status: draft
title: An ECMAScript reader for a typed subset, on swell's parser
summary: codespec-read-ecmascript reads one strict script of numbers, typed arrays, fixed-shape objects and closures into cir/1 and refuses the rest with a reason and file:line.
relations:
- decomposes: epic:ecmascript-to-native
- depends_on: story:ecmascript-swell-surface
- depends_on: story:ecmascript-number-semantics
- informed_by: story:cir1-type-test
- informed_by: story:readers-bound-untrusted-input
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/ecmascript-read/src/body.rs
- confidence: inferred
  path: adapters/ecmascript-read/src/lib.rs
- confidence: inferred
  path: adapters/ecmascript-read/src/main.rs
- confidence: inferred
  path: adapters/ecmascript-read/src/types.rs
- confidence: inferred
  path: adapters/ecmascript-read/tests/data
- confidence: inferred
  path: adapters/ecmascript-read/tests/main.rs
- confidence: inferred
  path: adapters/ecmascript-read/tests/reader.rs
- confidence: inferred
  path: conformance/reader/ecmascript
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: crates/conformance
revision: 4
---
# Story: An ECMAScript reader for a typed subset, on swell's parser

## Outcome

`codespec-read-ecmascript`, a Rust crate `adapters/ecmascript-read` (library `codespec_read_ecmascript`),
reads one ES5-plus-`let`/`const`/arrow script into one `cir/1` module, with a type on every
expression, and refuses everything outside the subset below with a reason and `<file>:<line>`.
`codespec read --lang ecmascript <file.js>` prints it.

The subset, each line a rule a test holds (decisions D1 and D2 of the epic):

- The script starts with the directive `"use strict"`; a script without it is refused at line 1.
  Top-level statements are function declarations and `const` declarations with a literal
  initializer; any other top-level statement is refused (the order of module initialisation is not
  modelled).
- Types. `number` is `float` 64. A value is `int` 32 only where a form fixes it: `e | 0`, `e >> n`,
  `e << n`, `e & m`, `e ^ m`, `~e` (their operands read as `int` 32 through the conversion of
  `story:ecmascript-number-semantics`), `e >>> 0` (`int` 32 unsigned), `Math.imul(a, b)` and
  elements of `new Int32Array(n)`. `boolean` is `bool`. A string is `string`: literals of printable
  ASCII, `+` of two strings, `===`, `!==` and `.length`; a literal with a code point outside ASCII,
  a string index and every string method are refused (JavaScript counts UTF-16 units, `cir/1`
  bytes).
- Types of entry functions come from JSDoc tags on the comment directly before the declaration:
  `@param {number} x`, `{boolean}`, `{string}`, `{number[]}`, `{Float64Array}`, `{Int32Array}`,
  `{{x: number, y: number}}` (a fixed shape), `{function(number): number}`, and `@returns`. An
  entry function without a tag for each parameter is refused naming the parameter. Inside the
  file every local, and every function called only from the file, is inferred; a call site whose
  argument types disagree with another's is refused (the function is polymorphic).
- Arrays: `[1, 2, 3]` (a slice of one element type), `new Float64Array(n)` and `new Int32Array(n)`
  (zeroed slices, as JavaScript zeroes them), `.length`, `a.push(x)` as a statement. A read or write
  `a[i]` is read only where `i` is shown in range by one of two forms: the body of a `for` whose
  condition is `i < a.length` with `i` not assigned in the body, or a statement guarded by an
  `if (i >= 0 && i < a.length)` on the same name. Anywhere else it is refused: JavaScript gives
  `undefined` (or grows the array) and `cir/1` has no such value. Holes, `a.length = n`, `delete`,
  `for-in`, spread and array methods other than `push` are refused.
- Objects of a fixed shape: a literal `{x: 0, y: 1}` with identifier keys and values of the subset
  is a `struct` named by its keys, written as a pointer (`&{…}`), so JavaScript's reference
  semantics holds; reads and writes `o.x` by name only; the same key set and types is one struct.
  Computed keys, `in`, `Object.keys`, a property added after the literal, a method (`o.f()`) and
  `this` are refused.
- Functions and closures: function declarations, function expressions and arrows become
  functions and `lambda` with `captures`; a captured variable is shared with its scope as
  JavaScript shares it. `call_value` for a call through a variable. A closure that captures a
  `let` declared in a loop head is refused (a fresh binding per iteration is not modelled).
- Statements: `let`, `const`, `var` (only where no use precedes it textually), assignment and
  compound assignment, `++`/`--` as statements, `if`, `for`, `while`, `do`-`while`, `break`,
  `continue` (the `for` post statement runs, `sem.continue.post`), `switch` on a number with
  literal cases and `break` (a fall-through case is refused), `return`.
- Library: `Math.sqrt`, `Math.abs`, `Math.floor` as the externs of
  `story:ecmascript-number-semantics`; `Math.imul`; `%` on numbers as `math.Mod`. Any other global
  or member is refused naming it.
- Refused, each with its own reason text: `eval`, `with`, `Function`, assignment to `.prototype` or
  `__proto__`, `Object.setPrototypeOf`, getters and setters, `Proxy`, `Reflect`, `class`, `new`
  other than the two typed arrays, `this`, `arguments`, generators, `async`, `await`, `try`,
  `throw`, `typeof`, `instanceof`, `in`, `delete`, `==` and `!=`, `undefined`, `null`, a truthiness
  test on anything but a `boolean`, `+` of a number and a string, a template literal, a regular
  expression, `NaN` and `Infinity` as names (they are globals that can be shadowed), label
  statements.

## Why

Requested by the owner (epic `epic:ecmascript-to-native`). `cir/1` has no tagged value, no property
lookup and no coercion; a reader that accepted the rest of JavaScript would have to invent them.
The refusal list above is the closed set of what a runtime would have to provide: every item in it is
one a later story could take, none is guessed. The kit's rule applies (`AGENTS.md` § Rules for
`cir/1`): what a reader cannot express is a `foreign` statement with a refusal, never dropped.

## Done When

- A failing test first, in `adapters/ecmascript-read/tests/`: `refuses_eval_with_line` reads a
  three-line script with `eval(s)` on line 3 and expects the refusal `ecmascript.js:3` naming
  `eval`; and `reads_fib` expects the checked-in `fib.read.cir.json` for a six-line recursive `fib`
  whose parameter is tagged `{number}`.
- One test per refused construct above (a one-line script each), each asserting the reason text and
  the line; one per rule of the subset with a near miss: `a[i]` where `i` is assigned in the loop body
  (refused), `x | 0` over a product (refused: the product is not exact in a `float`, use
  `Math.imul`), `"é"` (refused), a `switch` case that falls through (refused), a closure over a
  loop `let` (refused), a function called with a `number` here and a `string` there (refused as
  polymorphic), a `"use strict"` after the first statement (refused), the same script read twice
  (byte-equal).
- The command is wired: `codespec read --lang ecmascript tests/data/fib.js` prints the module equal
  to the checked-in `fib.read.cir.json` and exits 0; on `tests/data/eval.js` (the `eval` call on
  line 3) it prints a module whose `refusals` holds an entry at `eval.js:3` naming `eval`, the
  function holding the call a `foreign` statement, and exits 0, as the other readers do; on a script
  without the `"use strict"` directive, or one swell's parser rejects, it exits 1 with a message
  naming the directive or swell's error and its line. A test in `crates/cli` runs the three.
- Corpus: `conformance/reader/ecmascript/<case>/{case.js, calls.json}` has one case for each kit
  case whose subject the subset can write, at least 25 of them, each read by the reader without an
  error, and every kit case not claimed is named in the crate doc with the reason. The report
  `website/data/conformance/reader-ecmascript.json` (`conforms: true`, `summary.pass` of at least
  25) needs the runner `codespec-run-ecmascript`, which `story:ecmascript-run-oracle` creates, so
  that story checks it and registers the reader's report in `crates/docs`; this story does not.
- The reader is deterministic (the same bytes on a second read), bounded against untrusted input as
  `story:readers-bound-untrusted-input` states (a nesting limit through swell's `ParseOptions`, a
  source size limit), and reads no file but the one named.
- `--describe` prints a manifest `task adapters` reads strictly, with the features it claims.
- The reader takes swell through `story:ecmascript-swell-surface` and uses no other third-party
  crate. `task check` exits 0; each test under 60 s, the test step under 120 s; the time of the new
  test binary is stated in the report.
- The doc names, as not done: modules, classes, `try`, generators, string indexing, `==`, and a
  second type for `number`.

## Scope

- Own: `adapters/ecmascript-read` (new: `src/lib.rs`, `src/main.rs`, `src/types.rs`, `src/body.rs`,
  `tests/main.rs`, `tests/reader.rs`, `tests/data`; the rest, `Cargo.toml`, `swell-surface.txt`,
  `src/swell.rs` and `tests/swell.rs`, is `story:ecmascript-swell-surface`'s and is used, not
  rewritten), `conformance/reader/ecmascript` (new), `crates/cli` (the `--lang` value),
  `crates/conformance` (`CORPUS_LANGS`, the corpus builder), `AGENTS.md`, `CHANGELOG.md`.
- Shares `crates/conformance` and `crates/cli` with `story:ecmascript-run-oracle` (after this
  story) and with `story:reader-asm` and `story:reader-llvm-ir` (draft): they run one after another.
- Inferred by the planner, not said by the owner: JSDoc tags as the source of entry types (D1), and
  `number` as `float` 64 (D2).
