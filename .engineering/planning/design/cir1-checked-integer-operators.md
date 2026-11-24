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
revision: 5
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

Read on `wave/rust-misreads-1` (`65a79bc3`: shift-count, hand-written comparisons, platform-int
width and tuple-variant fields merged), which branches from main `f09c740f`. Decision D1 = A of
`story:rust-reader-integer-overflow`, with its conditions.

Revision 2 answers the adversary's round 1, `review-result:cir1-checked-integer-operators-r1`
(needs-revision, nine findings F1 to F9). Revision 3 answers round 2,
`review-result:cir1-checked-integer-operators-r2` (needs-revision, six findings R2-B1, R2-B2,
R2-W1, R2-W2, R2-N1, R2-N2). Each change is tagged with the finding it answers; § 12 maps them.

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

"Cannot tell" means the setting is unknown by § 5: without `--profile`, the root's profiles
disagree; with it, the pinned profile's setting cannot be resolved. (F8, F9) The refusal names the
operator, the type and why: "`+` on `i32` panics or wraps by the build's `overflow-checks`, and
the profiles of this root disagree (dev: on, release: off); pin one with `--profile`".

Types (F1). A checked operator is emitted only where the operand's `cir/1` type is the operand's
Rust type, width and signedness both; where the reader gives an operand another type than
rustc's, the operator is refused at its line. Since `story:rust-reader-platform-int-width`,
`v.len()` is `usize`, unsigned `platform`, so `v.len() - 1` under the checks reads as `-!` on an
unsigned `platform` operand and panics at 0, as Rust does. A reading that still differs in type
(the named index of `enumerate()`, already refused) must stay refused, never become a checked
operator over the wrong range.

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
  add `sem.int.wrap` for them, as they do not wrap; `%!` and `%!=` add `sem.rem.sign`, as `%`
  does, since the remainder still takes the dividend's sign (F2).
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
| `profile` | string, optional | the profile pinned with `--profile` (`dev`, `release`, `test`, `bench` or a custom one); absent when every profile of the root agreed and none was pinned (F8) |
| `overflow_checks` | bool, optional | the resolved setting; absent when the reader could not tell, and then every node of row "cannot tell" was refused |
| `debug_assertions` | bool, optional | the resolved setting, by the same rules; `story:rust-reader-cfg-features` reads `cfg(debug_assertions)` from it |
| `features` | array of string, optional, sorted, unique | the enabled Cargo features; written by `story:rust-reader-cfg-features` (`[]` means none, absent means not resolved) |

The record states what the reader read; no consumer needs it to give a node its meaning, which
the nodes carry. A writer that ignores it loses nothing, and no runner reads it (F6, § 5.6). The
Rust reader always writes `build`; `story:rust-reader-integer-overflow` writes `profile`,
`overflow_checks` and `debug_assertions`, `story:rust-reader-cfg-features` adds `features`.
Other readers write no `build`.

### 5. How the Rust reader takes `overflow-checks`

Inputs: `codespec-read-rust [--profile <name>] [--root <dir>] <crate dir>`; `codespec read --lang
rust` passes both on (refused for a reader that does not take them, as `--spans` is). No profile
is hard-coded.

**5.1 The root (F6).** The settings that apply are the building workspace root's, not the
crate's own: Cargo reads `[profile]` only from the root manifest of the workspace it builds, and
ignores it in every other manifest, a path dependency's included. The reader reads the root it is
pointed at. `--root <dir>` names the directory whose `Cargo.toml` is the building root (a runner's
or a harness driver's workspace). Without `--root`, the root is the crate's own workspace by
Cargo's lookup: the crate's `Cargo.toml` when it has `[workspace]`; else the manifest that
`package.workspace` names; else the nearest ancestor `Cargo.toml` with `[workspace]` that holds
the crate as a member; else, with no ancestor workspace at all, the crate's own `Cargo.toml`.
An explicit `--root` is taken as the building root whether or not its manifest has
`[workspace]`.

A failed lookup never fails the read (R2-B1): an ancestor workspace that does not hold the crate
(Cargo would refuse to build it there), a `package.workspace` that names no readable manifest, or
membership the reader cannot decide makes the setting unknown. Only the profile-dependent
operators are then refused, and their refusal names both inputs: "no building root found for
this crate (the workspace at `../Cargo.toml` does not list it); pass `--root` and `--profile`".
The reader's own test crates without `[workspace]` under the repository's workspace
(`reader.rs` `spans-*`, `whole_crate.rs`, `bounds.rs`, `adversary_bounds.rs`, and edit's
`read_spanned` over them) read as before but for those operators (§ 5.7).

Membership follows Cargo (R2-B2): the root's package, if any; every directory `workspace.members`
names, globs expanded, minus `workspace.exclude`; and, transitively, every path dependency of a
member (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, their `target.*` forms,
and a `workspace = true` dependency through `[workspace.dependencies]`) whose directory lies under
the root. When the reader cannot decide (a glob form it does not expand, a member manifest it
cannot read, a dependency path it cannot follow), membership and so the setting are unknown.

Root, ancestor and member manifests are read as regular files of at most `MAX_SOURCE_BYTES`; only
`[workspace]`, `[profile]`, `package.name`, `package.version`, `package.workspace` and the
dependency tables' `path` keys are used: the one widening of what the reader reads, today only the
crate directory. Under a root that does not hold the crate as a member (a path dependency under
a runner's or driver's `--root`), the crate is a non-member, which decides the tiers of 5.3.

**5.2 The chain.** `dev` and `release` are built in; `test` inherits `dev`, `bench` inherits
`release`; a custom profile names its parent with `inherits`, which must end at a built-in. A
pinned profile that is not defined, a custom one without `inherits`, or a cycle is an error, as
in Cargo.

**5.3 Cargo's order (F4).** For profile `p` and the read crate, the first tier that sets the key
wins:

1. `[profile.<p>.package.<spec>]` whose spec matches the crate (5.4);
2. `[profile.<p>.package."*"]`, only when the crate is not a member of the root;
3. `[profile.<p>]`;
4. the default of the chain's built-in: `dev` `overflow-checks = true`, `debug-assertions =
   true`; `release` both false.

`[profile.<p>.build-override]` applies to build scripts, proc macros and their dependencies, not
to the crate's code, and is skipped. Within a tier, configuration comes before the manifest
(Cargo merges a profile's configuration table over its manifest table), and the child profile
before its parent along `inherits`. So a package override in the manifest beats a profile-level
setting in configuration, and at profile level configuration beats the manifest. The reader reads
the manifest's tables; a configuration layer it cannot place makes the value unknown (5.5).
`debug-assertions` resolves the same way. Cargo passes `-C overflow-checks` itself where it differs
from `debug-assertions`, so the two resolve independently, except through rustflags (5.5).

**5.4 Package specs (F3).** A key of `[profile.<p>.package]` is a package ID spec. The reader
resolves `name`, and `name@version` and `name:version` with a full or partial version (`1`, `1.2`,
`1.2.3`) compared component by component with the crate's `[package] version`. A
`version.workspace = true` resolves from the crate's own workspace (the lookup of 5.1 without
`--root`), whatever `--root` names, as Cargo resolves it where the package is declared (R2-N2). A
key naming another package is skipped. A key it cannot resolve makes the value unknown when it may
name the crate: a URL form (`registry+…#name@…`, `path+file://…#…`, any key with `#` or `://`), a
version with pre-release or build metadata, or a crate version the reader cannot read. Two keys
of one profile that both match the crate (`probe` and `probe@0.1`) make it unknown too: Cargo
refuses such a manifest (R2-N1).

**5.5 Unknown (F5).** Cargo reads configuration from the directory it runs in, that directory's
ancestors and `$CARGO_HOME`; the reader cannot tell where the build runs, and a build may run in
any directory under the root. So the reader reads every `.cargo/config.toml` and `.cargo/config`
under the root's directory (R2-W1), and those in the crate's directory and its ancestors up to
the root's directory, or, when the root is not an ancestor, up to their nearest common ancestor.
The walk does not follow symbolic links and skips the root's `target` and `.git` directories; a
walk past a fixed bound of directories makes the setting unknown. Any such file that sets a
`profile` key or any rustflags (`build.rustflags`, `target.<triple>.rustflags`,
`target.<cfg>.rustflags`) makes `overflow-checks` and `debug-assertions` unknown, with or without
`--profile`: no rule independent of the working directory places it. So do
`rustflags` in a `[profile.*]` table of the root manifest (the unstable `profile-rustflags`), and
`-C overflow-checks` or `-C debug-assertions` in any rustflags source the reader reads (rustc's
`overflow-checks` follows `debug-assertions` where it is not set itself). Unknown means the
profile-dependent operators are refused (§ 2) and `build.overflow_checks` is absent. Not read at
all, and named in the crate doc as outside the reading: `$CARGO_HOME/config.toml`, configuration
above the root, and the environment (`CARGO_PROFILE_*`, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`,
`CARGO_BUILD_RUSTFLAGS`).

**5.6 Default and pin (F8, F9).** Without `--profile`, the reader resolves the setting for every
profile the root can build: `dev`, `test`, `release`, `bench` and each custom profile the root
defines, each through its chain. All known and equal: that value, and `build` without `profile`.
Any unknown, or two that differ: unknown, and the refusal says which profiles gave what.
`--profile <name>` pins one. A root with no `[profile]` table disagrees (`dev` and `test` on,
`release` and `bench` off), so such a crate needs `--profile`; a root that sets
`[profile.release] overflow-checks = true`, as services often do, reads without one.

When any operator was refused because the setting is unknown, `codespec-read-rust` prints one
line on standard error beside the module (exit 0, the module unchanged): "3 operators refused:
the overflow-checks setting is unknown (profiles disagree: dev on, release off); pass --profile",
or, with no root found, "…unknown (no building root found); pass --root and --profile".
`codespec read` passes the line through. (Usability, adopted with round 2.)

**5.7 Who passes what (F6, F7).** No runner reads `build`; each caller pins the profile its build
uses:

- `codespec-run-rust` (`adapters/rust-run/src/lib.rs`, the generated `codespec-run` workspace)
  builds the subject as a path dependency with `cargo build`, profile `dev`, in a root without
  `[profile]`: checks on. The conformance engine reads the Rust corpus with `--profile dev`; the
  corpus crate's own root has no `[profile]` either, so the pin names what step 1 ran.
- The inventory harness builds `examples/inventory/source` as a path dependency of
  `drivers/rust-original` (its own `[workspace]`, no `[profile]`, `cargo build`). The source
  uses `+`, `-`, `*` and `+=`. Every read of it passes `--profile dev`, so the bytes compare:
  the `examples` task (`Taskfile.yml:336`, which regenerates the checked-in document), the
  `examples-inventory` task (`Taskfile.yml:687-688`, two reads compared with each other and with
  the checked-in document; R2-W2) and the release workflow
  (`.github/workflows/release-build.yml:88-89`, the same comparison).
- `codespec edit` runs the reader with `--spans` alone (`crates/edit/src/run.rs:1162`); it gains
  `--profile`, passed on for every read. Without it, a crate whose profiles agree reads as
  before; where they disagree, each profile-dependent operator is refused in every read, and edit
  refuses a change set that meets one, as it refuses any refusal. The Rust edit fixtures
  (`examples/edit/rust/source`, `crates/cli/tests/fixtures/edit/rust/*`) have no `[profile]` and
  use `+` and `*`, so their programs and tests pass `--profile dev`, the profile their `--build`
  commands build with, and `--root` the fixture crate where it has no `[workspace]` of its own.
- The reader's own tests (R2-W2). The library gains `read_crate_with(dir, &Inputs)` (`profile`,
  `root`) beside `read_crate`, and `read_source_with(name, file, src, Checks)` beside
  `read_source`: a source read without a manifest has no root, so the test states the setting
  (`overflow_checks`, `debug_assertions`, each on, off or unknown), recorded in `build` without
  `profile`. `read_crate` and `read_source` keep their signatures and read with no inputs
  (unknown for `read_source`). Each file of `adapters/rust-read/tests/`:

  | file | reads | passes |
  |---|---|---|
  | `reader.rs` `read()` | `read_source` of inline sources, documents compared node by node with Go reads | `Checks` off: today's nodes, so every existing assertion holds; the new overflow tests state theirs |
  | `reader.rs` `read_crate` calls and `spanned` (`spans-*`) | crates under `target/` without `[workspace]` | `--root <crate dir>` and `--profile dev`; without them only the dependent operators are refused (R2-B1) |
  | `evaluation.rs`, `adversary1.rs`, `adversary2.rs` | `read_source`, then `codespec pass` and `codespec eval`, held to what `codespec-run-rust` printed (a `dev` build) | `Checks` on, the setting the measured build had |
  | `whole_crate.rs` | `read_crate` of crates under `CARGO_TARGET_TMPDIR` without `[workspace]` | `--root <crate dir>`, `--profile dev` |
  | `bounds.rs`, `adversary_bounds.rs` | the binary on crates under `target/rust-read-tests` without `[workspace]`, testing limits | nothing; a test asserting no refusal over arithmetic passes `--root <crate dir> --profile dev` |
  | `cases.rs` | the binary on each kit case, a crate with its own `[workspace]` | `--profile dev` |
  | `listed_calls.rs`, `two_documents.rs` | `read_source`, call-graph queries over `n - 1`, `n + 1` | `Checks` off, so the calls inside stay read |
  | `union_case.rs` | `read_source` of a source without arithmetic | nothing |

The crate doc of `adapters/rust-read/src/lib.rs` states these rules and replaces the operator
row's "a trace that overflows is not equal by construction".

### 6. Shared with `story:rust-reader-cfg-features`

The `--profile` and `--root` inputs, the resolution of § 5 (order, specs, unknowns, the default
over all profiles) and the `build` record are one: `cfg(debug_assertions)` is read from
`build.debug_assertions`, resolved by the same rules, and the module records the features beside
it. When `debug-assertions` is unknown, or the profiles disagree on it without a pin, a
`cfg(debug_assertions)` item stays refused.

### 7. Every evaluator and writer refuses an operator it does not know

Today, for an `op` string it does not know (read off the code at `efd4d802`, unchanged at `65a79bc3`):

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
  `wrapping` table, `--profile`, `--root`), the profile resolution beside the manifest reading
  in `lib.rs` (`read_crate`), `adapters/rust-read/tests/reader.rs`, `tests/cases.rs` (`--profile
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
  its subject when read with `--profile dev`. `conformance/cases/sem-rem-sign/rust/case.rs`
  (`a % b` on `i64`) reads as `%!` and keeps its subject `sem.rem.sign` (F2); its corpus copy
  `conformance/reader/rust/sem-rem-sign` is regenerated unchanged.
- `Taskfile.yml:336` (`examples`) and `Taskfile.yml:687-688` (`examples-inventory`, R2-W2), each
  read passing `--profile dev`; `.github/workflows/release-build.yml:88-89` (the same flag);
  `examples/inventory/*` regenerated (F7).
- `adapters/rust-read/src/main.rs` (`--profile`, `--root`, the one-line summary on standard error)
  and `adapters/rust-read/src/lib.rs` (`read_crate_with`, `read_source_with`); every test file of
  `adapters/rust-read/tests/` as the table of § 5.7 says (R2-W2).
- `crates/edit/src/run.rs` (`read`, line 1162: `--profile` passed on), `crates/cli/src/lib.rs`
  (`edit --profile`), the programs of `examples/edit/rust` and `scripts/examples-edit.sh`, the
  tests over `crates/cli/tests/fixtures/edit/rust` (F7).
- `crates/docs` generated pages; the crate doc of `adapters/rust-read/src/lib.rs`;
  `docs/design.md` § 3 (a row); `CHANGELOG.md`.
- Unchanged and refusing: `adapters/elixir/lib/scalar_writer.ex`, `adapters/odin/src/write.rs`.

### 10. Test plan, red first

Each test under 60 s; each touched package's test step within the 120 s budget; no build from
a tree under `~/.cache`. Red first on main `f09c740f`:

1. `adapters/rust-read/tests/reader.rs`: the story's crate; `div(i32::MIN, -1)` and
   `rem(i32::MIN, -1)` read without `--profile` and evaluated stop `panic`. On main they print
   -2147483648 and 0: red.
2. The same crate, profiles (F8, F9): with `[profile.release] overflow-checks = true` and no
   `--profile`, every profile agrees: `add` holds `+!`, `add(i32::MAX, 1)` panics, `build` has
   no `profile`; with no `[profile]` and no `--profile`: `add` and `neg` refused at their lines
   (dev and test on, release and bench off), `div` and `rem` read; `--profile release`: `+`,
   -2147483648; `--profile dev`: `+!`. On main: red (`+` read in every case).
3. Shifts (`story:rust-reader-shift-count`'s crate): `--profile dev` gives `<<!`, `shl(1, 32)`,
   `mask(64)`, `shl_signed(1, 64)` panic; `--profile release` gives the masked form, 1, 0 and 1,
   as Rust; red on main (0, 18446744073709551615, 0).
4. Resolution, one crate each, with the `build` record of each: a custom profile `inherits =
   "release"`; a member whose root has `[profile.release] overflow-checks = true` and whose own
   `[profile]` is ignored; a manifest `[profile.release.package.<crate>]` beating `[profile.release]`
   (F4); package keys `<crate>`, `<crate>@0.1`, `<crate>:0.1.0` applied, `<crate>@0.2` skipped,
   `registry+https://…#<crate>@0.1.0` unknown (F3); `package."*"` applied only under a `--root`
   that does not hold the crate (F6); a `.cargo/config.toml` in the crate's or the root's
   directory, or in a sibling member's directory under the root, with a `profile` key or
   `build.rustflags`, and `rustflags` in the root's `[profile.release]`, each unknown (F5,
   R2-W1); a pinned profile not defined (error). Root and membership (R2-B1, R2-B2): a crate
   without `[workspace]` under a workspace that does not list it reads, with only the dependent
   operators refused, the refusal and the standard-error line naming `--root` and `--profile`; a
   crate reached only as a path dependency of a member under the root is a member (`"*"` does not
   apply to it); a `members` glob the reader does not expand is unknown. Specs (R2-N1, R2-N2):
   `probe` and `probe@0.1` both in `[profile.release.package]` is unknown; `version.workspace =
   true` resolves from the crate's own workspace under a `--root` elsewhere.
5. Types (F1): `pub fn last(v: Vec<i64>) -> usize { v.len() - 1 }` with `--profile dev` holds
   `-!` on unsigned `platform` and `last(vec![])` panics.
6. Model: strict reading accepts each checked operator on integers and refuses it on a float, a
   string, mixed types and `-!` on unsigned; `build` with an unknown field refused;
   `examples-check` shows every checked-in document byte-equal.
7. Evaluator: each operator at its bounds (`MAX +! 1`, `MIN -! 1`, `MIN *! -1`, `-! MIN`,
   `MIN /! -1`, `MIN %! -1`, `x /! 0`, `x <<! W`, `x <<! (W - 1)`, `x <<! -1`) for widths 8 to
   64, signed and unsigned: a panic or the value.
8. Writers: Go (gofmt-clean, vetted, the helper panics at each bound), Rust (builds without
   warnings, same), C (through `codespec-run-c`); Elixir, Gleam and Odin refuse a module with
   `+!`, `-!` and `+!=`.
9. Kit and claims: `conformance-cases`, `eval-kit`, `conformance-kit`, the writers' conform runs
   and `adapters/rust-read/tests/cases.rs` green with the 15 cases; the Go and C readers' runs do
   not claim them.
10. Examples and callers (F7): `examples-inventory` (read with `--profile dev`, Go and Rust
   outputs regenerated, harness 200 × 300 byte-equal), `eval-diff` over `inventory`, the release
   workflow's read of `examples/inventory/source` byte-equal, `task examples-edit` and the edit
   tests over `crates/cli/tests/fixtures/edit/rust` unchanged with `--profile dev`.

### 11. Open questions

- Q1. Reading the root's and ancestors' manifests and configuration files widens what the reader
  reads beyond the crate directory. Alternative: refuse whenever the crate's own manifest has no
  `[workspace]` and no `--root` is given.
- Q2. The environment and `$CARGO_HOME` stay unread. An explicit input
  (`--overflow-checks on|off`) recorded in `build` would let a build that sets it there be read
  right; not proposed here.
- Q3. `usize` and `isize` shifts stay refused because `platform` does not fix W for Rust, while
  `+! -! *!` on them assume W = 64. Fixing the target width for Rust's `platform` would settle
  both.
- Q4. Settled in revision 2 (F7, F8): `codespec edit --profile`, § 5.7.
- Q5. The conformance engine passes `--profile dev` to the Rust reader only: one per-language
  reader argument in a generic engine.
- Q6. Panics are not in the calls encoding, so the kit cases cover the non-overflowing path only;
  the panics are held by the tests of § 10. `docs/evaluator.md` § 9's `sem.panic.*` would let
  the kit hold them.

### 12. What each finding changed

- F1, `len()` typed signed: § 2 "Types" (checked only on the Rust type; `len()` is `usize`), test 5.
- F2, `%!` dropped `sem.rem.sign`: § 3 (`%!` keeps it), § 9 (the `sem-rem-sign` case).
- F3, package ID specs: § 5.4, test 4.
- F4, precedence: § 5.3 (Cargo's tiers; configuration before manifest within a tier), test 4.
- F5, configuration by working directory, profile rustflags, `-C` flags: § 5.5, test 4.
- F6, the building root decides: § 5.1 (`--root`), § 5.7 (runners; `build` unread), test 4.
- F7, missing callers: § 5.7, § 9 (release workflow, `examples-inventory`, edit), test 10.
- F8, F9, the default without `--profile`: § 5.6, § 2, § 4, test 2.

Revision 3, round 2 (`review-result:cir1-checked-integer-operators-r2`):

- R2-B1, a failed root lookup failed the read: § 5.1 (unknown, refusal names `--root` and
  `--profile`), § 5.7 (the reader's test crates), test 4.
- R2-B2, membership: § 5.1 (members, globs, `exclude`, path dependencies under the root;
  undecidable is unknown), test 4.
- R2-W1, configuration anywhere under the root: § 5.5 (every file under the root; any profile
  key or rustflags is unknown), test 4.
- R2-W2, missing callers: § 5.7 (`Taskfile.yml:687-688`, the table of reader test files), § 9.
- R2-N1, two keys for one crate: § 5.4, test 4.
- R2-N2, `version.workspace`: § 5.4, test 4.
- Usability, the summary line on standard error: § 5.6, § 9.
