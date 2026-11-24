---
format: aep.planning-md/3
id: design:cir1-checked-integer-operators
kind: design
status: in_review
title: 'Checked integer operators in cir/1: the design note'
summary: How cir/1 states a Rust integer operator (+ - * neg / % << >>) whose overflow depends on the build, written and adversary-reviewed before story:rust-reader-integer-overflow is implemented.
relations:
- designs: story:rust-reader-integer-overflow
- decomposes: epic:async-network-service-translation
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-11-24T15:10:42Z", actor: "human:timo", revision: 2}
---
# Design: Checked integer operators in cir/1

## Outcome

A design note says how `cir/1` states a Rust integer operator whose result on overflow depends on
the build, additively, before `story:rust-reader-integer-overflow` is implemented. It works out
that story's decision D1 = A (a checked integer operator in `cir/1`). The note is the `## Note`
section of this artifact.

## Done When

- The note exists as the `## Note` section of
  `.engineering/planning/design/cir1-checked-integer-operators.md` and states:
  - the operator family: `+`, `-`, `*`, unary `-`, `/`, `%`, `<<` and `>>`, with their compound
    assignments;
  - the three behaviours and which node carries each: dependent on `overflow-checks` (`+ - *` and
    unary `-` wrap without it and panic with it; a shift by a count at or past the operand's
    width W masks the count without it and panics with it); a panic in every build (`/` and `%`
    of the type's minimum by -1); explicit (`wrapping_*`, `checked_*`, `saturating_*` and
    `overflowing_*` calls);
  - how the reader takes `overflow-checks` from the Cargo profile it is told (the manifest's
    `[profile.*]` setting, else the profile's default), or refuses when it cannot tell, and
    where the module records the configuration it was read for;
  - that the profile input and the module's recorded configuration are the ones
    `story:rust-reader-cfg-features` uses for `debug_assertions`;
  - that every evaluator and writer that does not know the operator refuses it, none ignores it;
  - the surfaces the implementation will touch, by path (at least `crates/model/src/lib.rs`,
    `schemas/cir-1.schema.json`, `schemas/cir-1.features.json`, `crates/eval`,
    `adapters/rust-read/src/`, the Go writer in `adapters/go`, `adapters/rust/src/write.rs` and
    the C writer in `adapters/c`).
- An adversary review of the note is recorded: a `review-result` that `reviews` this artifact,
  with a verdict (`approve` or `needs-revision`). The note moves to `approved` only on an
  `approve`.

## Scope

- `.engineering/planning/design/cir1-checked-integer-operators.md`. No code.

## Note

Read on `wave/rust-misreads-1` (`efd4d802`), which branches from main `f09c740f`. Decision D1 = A
of `story:rust-reader-integer-overflow`, with its conditions.

### 1. In one line

`cir/1` gets checked integer operators as new `op` strings on the existing `binop`, `unop` and
`assign` nodes (`+!`, `-!`, `*!`, `/!`, `%!`, `<<!`, `>>!`, unary `-!`, compound `+!=` … `>>!=`),
each panicking where Rust's operator panics. The module gets an optional `build` record of the
configuration it was read for. No existing node, field or `op` string changes meaning.

### 2. The operator family and the three behaviours

The family is Rust's integer `+`, `-`, `*`, unary `-`, `/`, `%`, `<<`, `>>` and the compound
assignments `+=`, `-=`, `*=`, `/=`, `%=`, `<<=`, `>>=`. Floats and `String + &str` are outside it
and keep today's reading.

Three behaviours:

1. **Depends on `overflow-checks`.** `+ - *` and unary `-`: with the checks on, a result outside
   the type's range panics; without them, the result wraps. `<<` and `>>` by a count `n` at or
   past the left operand's width W (or a negative `n`): with the checks on, a panic; without
   them, Rust shifts by `n & (W - 1)`.
2. **A panic in every build.** `/` and `%` of a signed type's minimum by `-1` (Rust: "attempt to
   divide with overflow", "attempt to calculate the remainder with overflow"), and `/` or `%` by
   zero. Go gives the minimum and 0 for the first and panics on the second, which is today's
   `binop` meaning (`sem.int.wrap`, `sem.rem.sign`).
3. **Explicit.** `wrapping_*`, `checked_*`, `saturating_*` and `overflowing_*` calls mean the same
   in every build.

Which node carries each, by what the reader knows of `overflow-checks` (§ 5):

| Rust source, integer operands | checks on | checks off | cannot tell |
|---|---|---|---|
| `a + b`, `a - b`, `a * b` | `binop` `+!` `-!` `*!` | `binop` `+` `-` `*` (today) | refused at its line |
| `-a` (signed; `-` of an integer literal stays today's `unop -`, a constant) | `unop` `-!` | `unop` `-` (today) | refused at its line |
| `a / b`, `a % b`, signed | `binop` `/!` `%!` | `/!` `%!` | `/!` `%!` |
| `a / b`, `a % b`, unsigned, or a divisor that is an integer literal of 1 or more | `/` `%` (today; only a zero divisor panics, as in Go) | same | same |
| `a << n`, `a >> n`, `n` proven below W by `story:rust-reader-shift-count`'s rules | `<<` `>>` (today) | same | same |
| `a << n`, `a >> n`, `n` not proven, W fixed (8, 16, 32, 64) | `binop` `<<!` `>>!` | `a << (n & (W - 1))`: `binop` `&` with a literal of `n`'s type, then `<<` or `>>` | refused at its line |
| the same, left operand `usize` or `isize` (`cir/1` width `platform`) | refused at its line | refused at its line | refused at its line |
| `a op= b` for each operator above | `assign` with `+!=` … `>>!=`, `/!=`, `%!=` | today's `+=` …; a shift as `a <<= (n & (W - 1))` | refused at its line, except `/!=` `%!=` |
| `wrapping_add/sub/mul/div/rem` and `wrapping_neg` | `+ - * / %`, `unop -` (today: Rust's wrapping result is Go's) | same | same |
| `wrapping_shl(n)`, `wrapping_shr(n)` (`n: u32`) | `a << (n & (W - 1))`, `>>` likewise (new) | same | same |
| `checked_*`, `saturating_*`, `overflowing_*`, `pow`, `abs` | refused at its line, as today (a result of `Option` or a tuple, or outside the std table); a later story | same | same |
| an operator in a `const` or `static` initializer, an array length or an enum discriminant | today's operator: overflow there is a compile error in every build | same | same |

"Cannot tell" means no `--profile` was given, or the profile's setting cannot be resolved (§ 5).
The refusal names the operator, the type and why: "`+` on `i32` panics or wraps by the build's
`overflow-checks`, and the reader was told no profile (`--profile`)".

The platform row keeps wave 1's caution: W of `usize` is the target's (32 or 64), and `cir/1`'s
`platform` does not fix it for Rust. `+! -! *!` on `platform` are read with W = 64, as every
`platform` integer is today (Go's `int` on a 64-bit target).

### 3. `cir/1`: the checked operators

Meaning, for operands of one integer type T of width W (for a shift, `x` of T and a count `n` of
any integer type):

| `op` | feature id | result | panics when |
|---|---|---|---|
| `+!` `-!` `*!` | `op.add_checked` `op.sub_checked` `op.mul_checked` | the exact result | it lies outside T's range |
| `/!` `%!` | `op.div_checked` `op.rem_checked` | `/` `%` truncated, as `/` `%` | the divisor is 0; T signed, `x` T's minimum and the divisor `-1` |
| `<<!` `>>!` | `op.shl_checked` `op.shr_checked` | `<<` `>>` (`>>` arithmetic on signed) | `n < 0` or `n >= W` |
| unary `-!` | `unop.neg_checked` | `0 - x` | T signed and `x` T's minimum |
| `+!=` `-!=` `*!=` `/!=` `%!=` `<<!=` `>>!=` | `assign.add_checked` … `assign.shr_checked` | the operator's, assigned | as the operator |

A panic message is Rust's ("attempt to add with overflow", "attempt to shift left with overflow",
"attempt to divide by zero"), so an output that prints it prints the original's.

How it is stated, additively:

- `crates/model/src/lib.rs`: the doc of `Expr::Binop`, `Expr::Unop` and `Stmt::Assign` lists the
  new `op` strings; `op` stays a string, so no variant and no field changes. Strict reading
  (`Module::from_json`) gains one check: a checked operator has integer operands (or a named type
  over an integer) of one type equal to `t` (a shift: `x`'s type is `t`, `n` any integer), and
  `-!` a signed operand; anything else is refused.
- `crates/model/src/features.rs`: `BINOPS` gains the seven, `COMPOUND` their names, `UNOPS`
  `neg_checked`; 15 new ids, family `operator`. `features_used` (`crates/conformance`) does not
  add `sem.int.wrap` or `sem.rem.sign` for them: they do not wrap.
- `schemas/cir-1.schema.json`: regenerated. `op` is an unconstrained string, so the operator
  change is in the description text only; the `build` record (§ 4) adds one optional property to
  the module. `schemas/cir-1.features.json` and `conformance/kit.json`: regenerated with the 15
  ids.
- Old documents byte-equal: nothing a document holds today is renamed, re-encoded or given
  another meaning; `build` is skipped when absent. Every checked-in `*.cir.json` reads and
  re-serializes to the same bytes, and the Go, Java, Groovy, C, Elixir, Gleam and Odin readers
  print the same bytes as before.

Why an `op` string and not a field or a node kind:

- A field (`"overflow": "panic"`) is ignored by a consumer that does not know it. The Go writer
  decodes with `encoding/json` without `DisallowUnknownFields`
  (`adapters/go/cmd/codespec-write-go/main.go`), and the Elixir and Gleam writers match maps by
  the keys they name (`adapters/elixir/lib/scalar_writer.ex`): both would write a checked `+` as
  Go's wrapping `+`, silently.
- A node kind (`"k": "checked"`) is refused by every consumer, but every walk over expressions
  (`crates/model/src/walk.rs`, `crates/query/src/facts.rs`, `crates/eval/src/id.rs`,
  `crates/check`, `crates/passes`, `crates/edit`, the span and call-graph code) would have to
  learn its children, and a walk that missed it would drop the calls inside it from the call
  graph.
- An `op` string keeps the node's children where every walk already looks, and every consumer
  that dispatches on `op` refuses one it does not know (§ 7). `cir/1` already has an operator Go
  spells and Rust does not (`&^`).

### 4. The recorded configuration: `build`

`Module.build`, optional, skipped when absent, strict (unknown fields refused):

| field | type | meaning |
|---|---|---|
| `profile` | string, optional | the profile the reader was told (`dev`, `release`, `test`, `bench` or a custom one) |
| `overflow_checks` | bool, optional | the resolved setting; absent when the reader could not tell, and then every node of row "cannot tell" was refused |
| `debug_assertions` | bool, optional | the resolved setting, by the same rules; `story:rust-reader-cfg-features` reads `cfg(debug_assertions)` from it |
| `features` | array of string, optional, sorted, unique | the enabled Cargo features; written by `story:rust-reader-cfg-features` (`[]` means none, absent means not resolved) |

The record states what the reader read; no consumer needs it to give a node its meaning, which
the nodes carry. A writer that ignores it loses nothing. The Rust reader writes `build` whenever
it is told a profile; `story:rust-reader-integer-overflow` writes `profile`, `overflow_checks`
and `debug_assertions`, `story:rust-reader-cfg-features` adds `features`. Other readers write no
`build`.

### 5. How the Rust reader takes `overflow-checks`

The profile is an input, never assumed: `codespec-read-rust --profile <name> <crate dir>`, and
`codespec read --lang rust --profile <name>` passes it on (`--profile` for a reader that does not
take it fails at the reader, as `--spans` does). Without `--profile` the reader cannot tell
(§ 2). It then resolves the setting as Cargo does for that profile:

1. **The workspace root.** Cargo reads `[profile.*]` only from the workspace root's manifest and
   ignores it, with a warning, in a member's; the reader does the same. The root is the crate's
   own `Cargo.toml` when it has `[workspace]`; else the manifest that `package.workspace` names;
   else the nearest ancestor directory's `Cargo.toml` with `[workspace]`. An ancestor's
   workspace whose `members` (paths and globs) do not hold the crate and whose `exclude` does not
   name it is an error, as in Cargo; one that excludes it leaves the crate its own root. Ancestor
   manifests are read as regular files of at most `MAX_SOURCE_BYTES`, and only `[workspace]`,
   `[profile]` and `package.workspace` are used. Today the reader reads only inside the crate
   directory: this is the one widening.
2. **The chain.** `dev` and `release` are built in; `test` inherits `dev` and `bench` inherits
   `release`; a custom profile names its parent with `inherits`, which must end at a built-in.
   A custom profile not defined, without `inherits`, or in a cycle is an error, as in Cargo.
3. **The value.** The first of: `[profile.<p>.package.<crate>]` for each profile `p` of the chain,
   child first; `[profile.<p>]` for each `p` of the chain, child first; the built-in default
   (`dev`: `overflow-checks = true`, `debug-assertions = true`; `release`: both false).
   `[profile.<p>.package."*"]` and `build-override` do not apply to a workspace member and are
   ignored. `debug-assertions` resolves the same way and is independent: Cargo passes
   `-C overflow-checks` on its own.
4. **Configuration files in the source tree.** `.cargo/config.toml` (and `.cargo/config`) in the
   crate's directory and in each directory up to and including the workspace root: a
   `[profile.<p>]` setting there wins over the manifest, nearest directory first, as Cargo's
   configuration does. A `build.rustflags` or `target.*.rustflags` naming `overflow-checks` (or
   `debug-assertions`) there means the reader cannot tell.
5. **Not read.** `$CARGO_HOME/config.toml`, configuration files above the workspace root, and the
   environment (`CARGO_PROFILE_<P>_OVERFLOW_CHECKS`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`) belong
   to the build host, not to the source. The crate doc says so, and `build` names the profile and
   value read, so a build that sets the value there is visible as a mismatch.

The crate doc of `adapters/rust-read/src/lib.rs` states the profile rule and replaces the
operator row's "a trace that overflows is not equal by construction".

### 6. Shared with `story:rust-reader-cfg-features`

The `--profile` input, the resolution of § 5 and the `build` record are one: `cfg(debug_assertions)`
is read from `build.debug_assertions`, resolved by the same chain, and the module records the
features beside it. Without `--profile` both stories cannot tell, and refuse.

### 7. Every evaluator and writer refuses an operator it does not know

Today, for an `op` string it does not know (read off the code at `efd4d802`):

| adapter | binop | unop | compound assign | claim of the new ids | after the change |
|---|---|---|---|---|---|
| evaluator, `crates/eval/src/interp.rs` | `arith`: stop `unsupported`, "integer operator +!" | stop `unsupported`, "unary operator -!" | `=` stripped, then `arith`: the same stop | `op.*`, `unop.*`, `assign.*` (`runner.rs`): claimed | implements them, a stop `panic` with Rust's message |
| `codespec-write-cir` (`crates/eval/src/runner.rs`) | writes the module unchanged; the run stops `unsupported` when it reaches the node (E4), never past it | same | same | the evaluator's | as the evaluator |
| Go writer, `adapters/go/cmd/codespec-write-go/write.go` | not in `tokens`: refused, "binary operator "+!" has no Go form" | refused, "unary operator … has no Go form" | refused, "assignment operator … has no Go form" | `op.*` … (`describe.go`): claimed | writes them (§ 8) |
| Rust writer, `adapters/rust/src/write.rs` | `int_op` `_`: function refused, "binary +! on …" | `_`: refused | `=` stripped, then `binop`: refused | `op.*` … (`main.rs`): claimed | `x.checked_add(y).expect("attempt to add with overflow")`, `checked_sub/mul/div/rem/neg`; a shift `x.checked_shl(u32::try_from(n).expect(…)).expect(…)` |
| C writer, `adapters/c/src/write.rs` | `arith` `_`: refused, "the operator +!" | `_`: refused | `=` trimmed, then `arith`: refused | `op.*` … (`main.rs`): claimed | helpers in `adapters/c/src/codespec_rt.h` (`__builtin_*_overflow`, `cs_panic`), the kit's copies regenerated |
| Elixir and Gleam writers, `adapters/elixir/lib/scalar_writer.ex` (shared) | `binary`: refused, "unsupported operator" | refused, "unsupported unary" | not in `@compound`: refused | explicit lists: unclaimed | unchanged: refuse |
| Odin writer, `adapters/odin/src/write.rs` | `binary` `_`: refused | `_`: refused | explicit list `_`: refused | explicit list: unclaimed | unchanged: refuse |

None ignores an unknown `op`. One finding: the evaluator's writer does not refuse at write time;
the refusal comes when a run reaches the node, which `docs/conformance.md` § 6 scores `refused`.
Since it claims `op.*`, it must implement the operators, which the change does.

Claims: a wildcard claim (`op.*`, `unop.*`, `assign.*`) takes the 15 new ids with it. The Go
reader (`adapters/go/cmd/codespec-read-go/describe.go`) and the C reader
(`adapters/c-read/src/main.rs`) never print them, so each adds the 15 to `not`; the Java and
Groovy readers list ids explicitly and are unchanged. The Rust reader claims them.

`crates/query` and `crates/check` carry `op` strings through without interpreting arithmetic; a
cq rule that names `"+"` does not match `"+!"`, which is a different operator.

### 8. Go output of a checked operator

The Go writer writes a call of an unexported helper, one per operator and integer type the module
uses, written after the module's declarations, its name taken through the writer's name
allocation (`names.go`) so no name of the module or an import collides:

```go
func codespecAddInt32(x, y int32) int32 {
	r := x + y
	if (r > x) != (y > 0) {
		panic("attempt to add with overflow")
	}
	return r
}
```

`-! *!` check the same way (`*!` by `r/x != y` with `x != 0`, and `x == -1 && y == MinInt32`),
`/! %!` check the zero divisor, then the minimum by `-1`, `<<! >>!` check `n < 0 || n >= W`, then
shift. A named integer type converts to its underlying type and back around the call. Arguments
are evaluated left to right as Go evaluates a binary expression's, so no order changes. A
compound `+!=` is written `x = codespecAddInt32(x, y)` where the place `x` holds no call or
receive; one that does (`v[f()] +!= 1`) is refused, since it would be evaluated twice. The output panics where the Rust original panics and is otherwise the same as
today's `+`.

### 9. Surfaces the implementation touches

- `crates/model/src/lib.rs` (op docs, strict check, `Build`), `crates/model/src/features.rs`;
  `schemas/cir-1.schema.json`, `schemas/cir-1.features.json` (generated).
- `crates/eval/src/interp.rs` (`arith`, `Unop`), `docs/evaluator.md` § 5.1.
- `adapters/rust-read/src/{lib.rs,body.rs,main.rs}` (`binary`, the `Neg` arm, `compound`, the
  `wrapping` table, `--profile`), the profile resolution beside the manifest reading in
  `lib.rs` (`read_crate`), `adapters/rust-read/tests/reader.rs`, `tests/cases.rs` (`--profile
  dev`).
- `adapters/go/cmd/codespec-write-go/{write.go,names.go}` and a test file; `describe.go`
  unchanged.
- `adapters/rust/src/write.rs` (`binop`, `int_op`, `Unop`).
- `adapters/c/src/write.rs`, `adapters/c/src/codespec_rt.h`, `conformance/c/codespec_rt.h` and its
  copies (generated).
- `adapters/go/cmd/codespec-read-go/describe.go`, `adapters/c-read/src/main.rs` (`not`).
- `crates/cli/src/lib.rs` (`read --profile`), `crates/conformance/src/conform.rs` (the Rust
  reader corpus is read with `--profile dev`, the profile `codespec-run-rust` builds the source
  with at step 1), `crates/conformance/src/lib.rs` (`features_used`).
- `conformance/cases/`: 15 hand-written cases (`case.cir.json` with `reader.name` hand-written, a
  `case.go` reference, `rust/case.rs` with the plain operator, calls that do not overflow, since
  the calls encoding has no panic line); and the `rust/case.rs` of `op-add`, `op-sub`, `op-mul`,
  `op-div`, `op-rem`, `op-shl`, `op-shr`, `unop-neg` and their `assign-*` change to the
  `wrapping_*` form (or a proven count), which is what Go's operator means, so each still uses
  its subject when read with `--profile dev`.
- `Taskfile.yml` (`examples-inventory` reads with `--profile dev`, the profile its harness builds
  the original with), `examples/inventory/*` regenerated; `crates/docs` generated pages; the
  crate doc of `adapters/rust-read/src/lib.rs`; `docs/design.md` § 3 (a row); `CHANGELOG.md`.
- Unchanged and refusing: `adapters/elixir/lib/scalar_writer.ex`, `adapters/odin/src/write.rs`.

### 10. Test plan, red first

Each test under 60 s; each touched package's test step within the 120 s budget; no build from
a tree under `~/.cache`. Red first on main `f09c740f`:

1. `adapters/rust-read/tests/reader.rs`: the story's crate; `div(i32::MIN, -1)` and
   `rem(i32::MIN, -1)` read without `--profile` and evaluated stop `panic`. On main they print
   -2147483648 and 0: red.
2. The same crate with `[profile.release] overflow-checks = true`, read with `--profile release`:
   `add` holds `+!`, `add(i32::MAX, 1)` panics; with no `[profile]` and `--profile release`:
   `+`, -2147483648; `--profile dev`: `+!`; no `--profile`: `add` and `neg` refused at their
   lines, `div` and `rem` read. On main: red (`+` read in every case).
3. Shifts (`story:rust-reader-shift-count`'s crate): `--profile dev` gives `<<!`, `shl(1, 32)`,
   `mask(64)`, `shl_signed(1, 64)` panic; `--profile release` gives the masked form, 1, 0 and 1,
   as Rust; red on main (0, 18446744073709551615, 0).
4. Resolution, one crate each: a custom profile `inherits = "release"`; a member whose root has
   `[profile.release] overflow-checks = true` and whose own `[profile]` is ignored;
   `[profile.release.package.<crate>]`; `.cargo/config.toml` at the root overriding the manifest;
   `build.rustflags` with `-C overflow-checks` (cannot tell); a profile not defined (error); the
   `build` record of each.
5. Model: strict reading accepts each checked operator on integers and refuses it on a float, a
   string, mixed types and `-!` on unsigned; `build` with an unknown field refused;
   `examples-check` shows every checked-in document byte-equal.
6. Evaluator: each operator at its bounds (`MAX +! 1`, `MIN -! 1`, `MIN *! -1`, `-! MIN`,
   `MIN /! -1`, `MIN %! -1`, `x /! 0`, `x <<! W`, `x <<! (W - 1)`, `x <<! -1`) for widths 8 to
   64, signed and unsigned: a panic or the value.
7. Writers: Go (gofmt-clean, vetted, the helper panics at each bound), Rust (builds without
   warnings, same), C (through `codespec-run-c`); Elixir, Gleam and Odin refuse a module with
   `+!`, `-!` and `+!=`.
8. Kit and claims: `conformance-cases`, `eval-kit`, `conformance-kit`, the writers' conform runs
   and `adapters/rust-read/tests/cases.rs` green with the 15 cases; the Go and C readers' runs do
   not claim them.
9. Examples: `examples-inventory` (read with `--profile dev`, Go and Rust outputs regenerated,
   harness 200 × 300 byte-equal) and `eval-diff` over `inventory`.

### 11. Open questions

- Q1. Reading ancestor manifests and `.cargo/config.toml` widens what the reader reads beyond the
  crate directory. Alternative: refuse whenever the crate's own manifest has no `[workspace]`.
- Q2. The environment and `$CARGO_HOME` stay unread. An explicit input
  (`--overflow-checks on|off`) recorded in `build` would let a build that sets it there be read
  right; not proposed here.
- Q3. `usize` and `isize` shifts stay refused because `platform` does not fix W for Rust, while
  `+! -! *!` on them assume W = 64. Fixing the target width for Rust's `platform` would settle
  both.
- Q4. `codespec edit` runs the Rust reader with `--spans`; it should pass the profile through
  (`codespec edit --profile`), or a Rust edit refuses at each dependent operator.
- Q5. The conformance engine passes `--profile dev` to the Rust reader only: one per-language
  reader argument in a generic engine.
- Q6. Panics are not in the calls encoding, so the kit cases cover the non-overflowing path only;
  the panics are held by the tests of § 10. `docs/evaluator.md` § 9's `sem.panic.*` would let
  the kit hold them.
