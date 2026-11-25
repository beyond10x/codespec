# cir/1 semantics a lowering must preserve (asm_ir/1 input)

Paths below are relative to the repository root, at commit 8693a15b6. `interp.rs` = `crates/eval/src/interp.rs`; `rt.h` = `adapters/c/src/codespec_rt.h`; `cw` = `adapters/c/src/write.rs`.
Method: read the sources named; the kit classification (section 6) is a script over all 145 `case.cir.json`, `calls.json`, `expected.txt` (script output reproduced; data in `research/classes.json`, `research/table.md` beside this file).
Oracle status: `docs/evaluator.md:3` says "Status: proposed", but `crates/eval` is built and gated (`AGENTS.md` task `eval-kit`: every kit case prints `expected.txt` twice byte-equal, `crates/eval/tests/kit.rs:19`). Where the evaluator and Go differ, it says so in `crates/eval/src/lib.rs` (Limits) and I mark those below.

## 0. What the kit does and does not pin (read this first)

- Every one of the 128 cases that mention an integer type uses only `int` of width `platform`, signed (`jq` over `case.cir.json`: 128 files with `platform`+`s`, none with 8/16/32/64 or unsigned). The model and evaluator support all widths; the kit never exercises them. Command: `jq -r "[.. | objects | select(.k==\"int\") | (.width + (if .signed then \"s\" else \"u\" end))] | unique | join(\",\")" */case.cir.json | sort | uniq -c` gave `128 platforms`, 17 files with none.
- No kit case reaches a runtime failure (no `panic`, index out of range, nil dereference, divide by zero, negative shift), none calls a function with a `foreign`/`raise`/missing extern on the executed path (the only listed stop is `stmt-foreign`, `crates/eval/tests/kit-stops.txt:1`). Every `expected.txt` line begins `ok`, `err`, `sent`, or is a `println` line (grep, section 5).
- Shifts in the kit: only `Shl(3,4)`, `ShlBy(3,4)`, `Shr(48,4)`, `Shr(-48,4)`, `ShrBy(-48,4)`; no count >= 64, none negative. Division: `Div(7,2)`, `Div(-7,2)`, `Rem(7,3)`, the `sem-rem-sign` triple, and `MinInt64 / -1` and `% -1` (`conformance/cases/sem-int-wrap/calls.json`, `expected.txt`: `ok -9223372036854775808`, `ok 0`). No zero divisor.
- Float values are never printed directly (the Go driver prints only int, bool, string, slice: `conformance/driver/go/main.go` `show`); they reach output only through `sprintf("%v %v", float64, float32)` in `type-float` and `type-array-float` (section 2). No kit case converts int<->float (the only `conv` nodes are newtype conversions: Celsius, ID, Meters, Op, Tag, Tick over int/func/string).
- No case depends on slice capacity growth, slice aliasing after `append`, map iteration order, or string bytes outside printable ASCII (sections 4, 5).

## 1. Integer semantics

Model: `IntWidth` is `platform|8|16|32|64`, "`platform` is the source language machine-word integer (Go `int`)" (`crates/model/src/lib.rs:834-847`). `Type::Int{width,signed}` (`lib.rs:870-873`). The model does not state wrap or division rules; the vocabulary pins `sem.int.wrap` ("integer arithmetic wraps on overflow, as Go does") and `sem.rem.sign` ("the remainder takes the sign of the dividend") (`crates/model/src/features.rs:129-135`). The evaluator is the executable definition.

Representation in the oracle: an integer is an `i128` holding the mathematical value, re-canonicalised into its width after every operation (`interp.rs:128-146`: `int_bits` gives 64 for `platform` and `64`; `wrap(v,bits,signed)` masks to `bits` and sign-extends when signed). So a register must always hold the sign-extended (signed) or zero-extended (unsigned) value of its width after each op; comparisons are then plain signed or unsigned compares of the full value (`interp.rs:2507` `x.cmp(y)` on i128, correct for unsigned because values are stored non-negative).

| op (`arith`, `interp.rs:2525-2592`; result type `t` decides width/sign) | semantics | cite |
|---|---|---|
| `+ - *` | exact result wrapped to `t` (two complement); each width and signedness | `interp.rs:2539-2541`, wrap at `2570` |
| unary `-` | `0 - x` wrapped to `t` (so `-MinInt` = `MinInt`; unsigned `-x` = `2^w - x`) | `interp.rs:2199-2203` (float: sign flip, `2200`) |
| unary `^` | `!x` wrapped to `t` (bitwise complement within the width) | `interp.rs:2204-2210` |
| unary `+` | identity | `interp.rs:2198` |
| `& | ^` | bitwise on the canonical values (no wrap needed) | `interp.rs:2548-2550` |
| `&^` | `x & !y`, wrapped | `interp.rs:2551` |
| `/` | truncated toward zero (Rust `i128 /`), then wrapped: `MinInt / -1` = `MinInt` for every signed width | `interp.rs:2542-2546`, wrap `2570`; kit: `sem-int-wrap` `Div(-9223372036854775808,-1)` -> `ok -9223372036854775808` |
| `%` | Rust `i128 %`: sign of the dividend; `MinInt % -1` = 0; `-7 % 3 = -1`, `7 % -3 = 1`, `-7 % -3 = -1` | `interp.rs:2546`; kit `sem-rem-sign/expected.txt` (`ok -1`, `ok 1`, `ok -1`) |
| `/` or `%` by 0 | the run stops, kind `panic`, reason `integer divide by zero`, checked before the operation | `interp.rs:2543-2545` |
| `<<` | count `y` < 0 (a signed count) -> panic `negative shift amount`; `y >= bits` -> 0; else `x << y` wrapped to `t` | `interp.rs:2552-2561, 2570` |
| `>>` | `y < 0` -> same panic; `y >= bits` -> `-1` if `x < 0` else `0`; else arithmetic shift of the canonical value (logical for unsigned, since the value is non-negative) | `interp.rs:2552-2564` |
| compound `op=`, `++`, `--` | same `arith` with `l.t()` as the type; `++` is `+ 1` | `interp.rs:1325-1349` (compound `1326-1333`, Incr/Decr `1335-1349`) |
| literal | `wrap(parsed, bits, signed)`: an out-of-range literal is silently wrapped, not refused (the Rust writer refuses it, `adapters/rust/src/write.rs:1888`) | `interp.rs:2357-2359` |

Points a hardware lowering must handle (all follow from the table):
- `bits` in the shift test is the width of the left operand/result type, not 64: `int8 << 8` = 0 and `int8 >> 9` = -1 or 0. x86 `shl/sar/shr` mask the count to 5 or 6 bits, AArch64 register-count `lsl/asr/lsr` take it modulo 64, RISC-V `sll/sra/srl` use the low 6 bits (architecture manuals, not read from this repository); none give the Go result for `y >= bits`, so a compare plus select is required. A signed `>>` with `y >= bits` equals `>> (bits-1)`. Rust writer does exactly this: `checked_shl(n).unwrap_or(0)`, signed `>>` `wrapping_shr(n.min(width-1))`, unsigned `checked_shr(n).unwrap_or(0)` (`adapters/rust/src/write.rs:4165-4184`). C writer: `cs_shl/cs_shr` (`rt.h:69-74, 89-94`).
- A negative count check applies only when the count type is signed. The C writer applies `cs_shift_count` only for a signed count type and casts an unsigned count to `uint64_t` (`cw:1953-1958`); the evaluator treats a huge unsigned count as a plain count >= bits (result 0 or -1). Same behaviour.
- x86-64 `idiv`/`div` fault (SIGFPE) on a zero divisor and on `MinInt / -1`: test `y == 0` first (panic), then special-case `y == -1`: quotient `-x` wrapped, remainder 0. The C runtime does the same (`rt.h:51-68`: `y==-1` -> `(T)(0 - x)` and `0`). For i8/i16/i32, `MinInt / -1` also wraps back to `MinInt` (evaluator wrap at `2570`); widening to 64-bit before `idiv` avoids the trap for them.
- Narrow types: after `+ - * << unary- ^ / ` re-extend to the width (the oracle wraps after every op, so a value never carries extra high bits).
- `platform` is 64 bits on every target here (`interp.rs:130`, `cw:216`, Rust writer `i64` `adapters/rust/src/write.rs:1074`). No `usize`-like 32-bit variant exists in the oracle, so aarch64/riscv64 stay 64.
- Conversions (`conv`, `interp.rs:2406-2433`): int -> int of another width/sign: `wrap(value, target_bits, target_signed)` (truncate, then sign- or zero-extend; `int8(300)` = 44, `uint8(-1)` = 255, `int64(uint64 max)` = -1). Newtype/enum conversions are the identity on the value (`2432`). `conv` int -> string and slice -> string are `unsupported` (`2423-2425`); string -> string identity.
- Integer `lit` parses Go syntax: decimal, `0x`, `0o`, `0b`, leading-`0` octal, `_` separators, rune literals (`crates/eval/src/gofmt.rs:73-104`).
- Equality `==`/`!=` and ordering on ints: value compare after canonicalisation; ordering of unsigned is unsigned (`interp.rs:2507`).

## 2. Float semantics (evaluator)

Representation: `Value::Float(f64)`; a `float32` is a binary64 holding a binary32 value, re-rounded after every operation (`interp.rs:148-153`: `round` = `v as f32` back to `f64`, round-to-nearest-even). Because binary64 has more than `2*24+2` bits, `+ - * /` of two binary32 values computed in binary64 then rounded once to binary32 equals a native single-precision operation, so native `addss/subss/mulss/divss` (or `fadd.s`...) is exact for the oracle.

| item | oracle behaviour | cite |
|---|---|---|
| ops available | `+ - * /` only; `%`, `&`, shifts on floats are `unsupported`; unary `-` flips the sign (no rounding); `+x` identity | `interp.rs:2572-2582`, `2199-2201` |
| `/` by zero | no panic: IEEE result (inf, nan) | `interp.rs:2578` (`x / y` on f64) |
| `++`/`--` on float | adds/subtracts `1.0` | `interp.rs:1340-1343` |
| float literal (`Expr::Float`) | f32: decimal parsed directly as `f32` (single rounding); f64: parsed as `f64` | `interp.rs:1973-1982` |
| `Expr::Lit` with float type | integer literal -> `i as f64`, else `parse::<f64>`, then `round` to width (f32 literal through binary64: double rounding possible) | `interp.rs:2361-2366` |
| `==` | IEEE: `x == y` on f64; NaN != NaN, `0.0 == -0.0` | `interp.rs:3381` (`equal`), `2500-2502` |
| `!=` | `!(==)`: true when either is NaN | `interp.rs:2503-2505` |
| `< <= > >=` | `partial_cmp`: with a NaN operand every one is false | `interp.rs:2508-2523` (`None => false`) |
| as map key | `0.0` and `-0.0` hash alike, NaN equals nothing (each insert makes a new entry) | `crates/eval/src/value.rs:187-188` |
| int -> float | `i as f64` (i128 to binary64, RN) then `round` to the target width: for `float32` this is a double rounding (int64 -> f64 -> f32) which a single `cvtsi2ss` does not reproduce for |i| > 2^53 with specific low bits; unverified against Go, no kit case; do not copy | `interp.rs:2420` |
| float -> int | non-finite or `|f| >= 2^64` (1.8446744073709552e19): the run stops `unsupported`; else truncate toward zero and `wrap` to the target width, so a value in [2^63, 2^64) converted to int64 wraps (hardware `cvttsd2si` gives 0x8000...) | `interp.rs:2411-2418` |
| float -> float | `round` to target width (f64 -> f32 RN; f32 -> f64 exact) | `interp.rs:2421-2422` |
| zero value | `0.0` | `interp.rs:711` |
| constant folding | NOT exact: `0.1 + 0.2` is computed operator by operator in its type (prints 0.30000000000000004) where Go folds exactly; the Go reader is to fold | `crates/eval/src/lib.rs` "Limits", first bullet |

Printing floats (only through `%v`): `gofmt::float_v` = `strconv.FormatFloat(v,"g",-1,bits)`: shortest decimal that reads back to the same value of that width (32 or 64), exponent form when the decimal exponent is < -4 or >= 6, written `d.ddde+XX` (at least two exponent digits, `e+20`, `e-05`), `NaN`, `+Inf`, `-Inf` (`crates/eval/src/gofmt.rs:29-69`; examples as the test `gofmt.rs:172-196`: `1e6` -> `1e+06`, `123456789.0` -> `1.23456789e+08`, `100000` -> `100000`, `0.0001` -> `0.0001`, `0.00001` -> `1e-05`). The shortest round-trip algorithm (Ryu/Grisu class) is required for both widths; there is no fixed-precision shortcut.

Kit values that must come out exactly (`conformance/cases/type-float/expected.txt`, `type-array-float/expected.txt`):
- `Third(false)` = `"0.3333333333333333 0.33333334"`, `Third(true)` = `"3.333333333333333e+20 3.3333333e+20"`.
- `Weight(0..2)` = `"0.1 0.1"`, `"0.55 0.3"`, `"0.1375 0.3"` (float64 `w[1] = w[0]*3 + 2.5e-1`; float32 `h[1] = h[0]*3`, `h[2] = h[1] - 1e-8`).
- `type-float-compare`: `Exact(true)` false (`0.1+0.2 == 0.3` in f64), `Exact(false)` true (f32 sum rounds to `0.3f`), `Third(false)` true, `Third(true)` false (`1e21f/3*3 == 1e21f` fails): requires per-op f32 rounding, not f64 evaluation of f32 code. Expected: `ok false`, `ok true`, `ok true`, `ok false`.
- `expr-float` `Over`: `1.5e300/4 > 0.5`; `type-array-float` `Above`: `w[2] = 1e21 > 0.5`.
- Float is a field-less feature in the C writer: `float` has no C form there (`cw` doc line 27: "`float`, `array`, `func`, `extern`, type parameters and interfaces have no C form"); the C claim excludes `expr.float`, `type.float`, `type.array` (`adapters/c/src/main.rs:57-77`). Floats and arrays have therefore never been lowered by an existing low-level writer here.

## 3. Runtime failures: what the evaluator distinguishes and how it reports them

Mechanism: every failure is a `Halt` that ends the whole run with a `Stop{kind,node,pos,reason}` (`crates/eval/src/lib.rs:229-290`, `interp.rs:353-356`). `run_calls` stops at the first stop: the loop `for c in &calls.calls { if let Err(h) = it.call_entry(..) { stop = Some(..); break; } }` (`lib.rs:350-353`). Output already produced stays: every complete line of earlier calls plus anything `println` wrote during the failing call (`println` appends to `out` immediately, `interp.rs:2690-2693`); the failing call prints no `ok`/`err` line (the line is built after the call returns, `interp.rs:3159-3215`).

| failure | kind | reason text | where raised |
|---|---|---|---|
| index out of range (slice, array, string, `&xs[i]`, assignment target) | `panic` | `index out of range [i] with length n` (negative i: same text, Go prints `[-1]` without length; unpinned) | `interp.rs:1949-1951`; reads `2065-2090`, nil slice `2074` (len 0), array `2076`, string `2081`; places `1819-1860`; the place (and bounds check) is resolved BEFORE the right-hand side is evaluated (`interp.rs:1302-1311`) |
| nil pointer dereference (`*p`, `p.f`, `p[i]` on nil `*[N]T`, nil receiver of an extern method, call of nil func value, interface call on nil interface) | `panic` | `invalid memory address or nil pointer dereference` | `interp.rs:1797, 1846, 1860, 1911, 2141, 2305, 2333` |
| integer divide or remainder by zero | `panic` | `integer divide by zero` | `interp.rs:2542-2545` |
| negative shift count | `panic` | `negative shift amount` | `interp.rs:2553-2555` |
| write to nil map | `panic` | `assignment to entry in nil map` | `interp.rs:1924` |
| `==` of interface values holding one uncomparable dynamic type | `panic` | `comparing uncomparable type T` | `interp.rs:2438-2446` |
| send on closed channel | `panic` | `send on closed channel` (only a handler or the post-call drain closes; not reachable from the kit) | `interp.rs:1727` |
| `strings.Repeat` negative count | `panic` | `strings: negative Repeat count` | `crates/eval/src/builtins.rs:62-63` |
| send on full / receive from empty / nil channel, `range` over nil chan | `blocked` | e.g. `a send on a full channel blocks: no other goroutine receives` | `interp.rs:1588, 1715, 1733, 1743, 1760` |
| calls nest deeper than 1000, more than 10,000,000 steps (statement, call, loop iteration), nesting > 10,000, stack | `step budget` | `calls nest deeper than 1000` / `the run took more than N steps` | `interp.rs:22, 785-797, 800-813`; `lib.rs:161` |
| `foreign` statement executed; module variable with foreign initializer | `foreign` | `<reason> (<lang>: `<src>`)` | `interp.rs:1477-1481, 3105-3115` |
| `raise` executed | `raise` | `raise of <path.Name>: it leaves every caller up to the framework, which the evaluator does not run` | `interp.rs:1488-1495` |
| extern called with no built-in handler and no stub (missing stub) | `no handler` | `extern <path.Name> has no handler` | `interp.rs:1160-1168`; built-ins: `strconv.Itoa`, `strings.Repeat/ToUpper/ToLower/TrimSpace`, `path.Join`, `strings.Builder.WriteString/Len/String`, `os.Getenv/LookupEnv` (`builtins.rs:48-100`) |
| function declared without a body (`opaque`) called | `no body` | `<key> has no body in cir/1: <reason>` | `interp.rs:904-915` |
| construct the evaluator does not reproduce (sprintf verb other than `%d %s %v`, non-ASCII string in a result, float->int out of range, `string(int)`, map insertion during `range`, ...) | `unsupported` | varies | `crates/eval/src/lib.rs` crate doc "Stops" table |

Reporting (what the existing programs do):

| program | stdout | stderr | exit |
|---|---|---|---|
| `codespec eval --calls` (evaluator) | the lines of calls completed before the stop (plus that call println lines) | `codespec eval: stopped at <node-id> (<file:line>): <kind>: <reason>` | 1 (`crates/cli/src/eval.rs:5-9, 196-203`); 2 = refused before running (bad calls, unreadable module) |
| `codespec-run-cir` (evaluator as kit runner) | same | same | 1 run stopped or module unreadable; 2 refused (`crates/eval/src/runner.rs:21-26`) |
| `codespec-run-c` (C) | everything the C program printed; `cs_panic` does `fflush(stdout)` first (`rt.h:17-21`) | program writes `panic: <msg>` (`runtime error: index out of range`, `runtime error: integer divide by zero`, `runtime error: negative shift amount`, `assignment to entry in nil map`, `all goroutines are asleep - deadlock!`: `rt.h:17-21, 43-48, 51, 60, 136-143, 399-402, 559-562`) and the runner adds `codespec-run-c: the program: exit status: 3` | program exits 3 (`rt.h:20`), runner exit 1 (`adapters/c-run/src/lib.rs:707-740`); a program exit 2 is read as "a value it cannot print exactly" and the runner exits 2 |
| `codespec-run-rust` (Rust) | printed lines | Rust panic message (e.g. `attempt to divide by zero` from `wrapping_div`, `expect` messages for nil, slice index panic) | program exit 101, runner exit 1 (`adapters/rust-run/src/lib.rs:703-712`: any non-0/non-2 status is `Failed`; runner doc exit table lines 58-64) |
| Go reference driver | printed lines | Go runtime `panic: runtime error: ...` plus goroutine trace; `reference: a result of kind K cannot be printed` for an unprintable result | Go panic exit 2; unprintable `os.Exit(2)` (`conformance/driver/go/main.go` end of `show`) |

Facts about the encoding of failures:
- `docs/evaluator.md:50` records the gap: "no node and no `sem.` id for index out of range, nil dereference, division by zero ... Go behaviour, unpinned until section 9". Section 9 (`docs/evaluator.md:336-338`) only PROPOSES `sem.panic.index|nil|div_zero` and a `panic <message>` output line "that every runner prints". It is not implemented: `grep -rn "sem\.panic"` outside the design docs matches only `docs/evaluator.md:336` and `.engineering/planning/design/cir1-checked-integer-operators.md:515`; `SEMANTICS` (`crates/model/src/features.rs:127-158`) has no such id; no `expected.txt` holds a `panic` line. The conformance engine judges a stop of kind `panic`, `blocked`, `step budget` as `run_failed` (`docs/conformance.md:168-169`): a crashing runner is a failed run, not a pass.
- A session (not a calls file) lets a panic end only its call (`docs/evaluator.md:203`); `run_calls`, the kit path, ends the run.
- The kit: no case reaches any failure. `crates/eval/tests/kit-stops.txt` has the single line `stmt-foreign ... foreign: defer is outside cir/1`: Go prints `ok 3` (`conformance/cases/stmt-foreign/expected.txt`) and the evaluator stops, which the test requires (`crates/eval/tests/kit.rs:19`). `stmt-raise` never reaches its `raise` (`expected.txt`: `ok 7`, `ok 0`). All other 143 cases run to completion. So a backend may choose its failure encoding freely as far as the kit is concerned; matching the C runtime (stderr `panic: runtime error: <what>`, exit 3, stdout flushed first) is the closest existing convention.
- Pending model change (approved design, NOT in `crates/model` yet: `grep` for `"+!"` in `crates/model/src/lib.rs` and `interp.rs` finds nothing): `.engineering/planning/design/cir1-checked-integer-operators.md` adds operators `+! -! *! /! %! <<! >>!`, unary `-!` and compound forms that panic on overflow (Rust semantics), and states the existing `/` and `%` today panic only on a zero divisor, as in Go (design lines 55-58). A lowering should treat an unknown `op` string as a refusal, not as a wrapping op.

## 4. Value representations a lowering must choose

Oracle value kinds: `crates/eval/src/value.rs:79-125` (`Value`), design `docs/evaluator.md:93-104`.

| kind | oracle semantics | cite | what the kit exercises |
|---|---|---|---|
| string | immutable bytes (`Rc<[u8]>`); `len` = byte count; literal = Go literal syntax (interpreted with `\a \b \f \n \r \t \v \\ \" \xNN \NNN \uXXXX \UXXXXXXXX`, or raw backquote), decoded to bytes; `==`/`!=` byte equality; `< <= > >=` byte-lexicographic (unsigned bytes, shorter prefix is less); `+` concatenation; `s[i]` is the byte (0..255, an unsigned 8-bit int), out of range panics; `range s` yields (byte offset, decoded UTF-8 rune, invalid byte -> U+FFFD width 1); zero value `""`; `string(x)` only string->string, `string(int)` and `string([]byte)` are `unsupported` | `value.rs:87`; `interp.rs:2081-2086` (index), `1652-1668` (range), `2583-2587` (+), `2509` (ordering), `3384` (eq), `2623` (len), `gofmt.rs:106-165` (literal), `3493-3510` (`rune_at`) | 36 of 145 cases use the string type. Literals are plain printable ASCII with no escapes (`jq`: `"%v %v"`, `""`, `"x"`, `"w"`, `"v"`, `"two"`, `"Red"`, `"Green"`, `"one"`, `"not found: "`, `"negative"`, `"many"`, `"few"`, `"hello, "`, `"count"`, `"a" "b" "c"`, `">" "<" "!"`, `"%s=%d"`, `"negative: %d"`). String ops used: `+` (decl-var, expr-extern, expr-to-error, expr-to-iface, type-extern), `+=` (intrinsic-sorted-keys), `==` (expr-errvar, op-eq), `len` (expr-intrinsic, type-map); no string `<`, no `s[i]`, no `range s`, no non-ASCII. Entry string arguments are raw bytes between the quotes, no escape processing (`interp.rs:3178`, check `crates/conformance/src/lib.rs:78-96`) |
| slice | `{array, off, len, cap}` over a shared backing array. Zero value and `append(nil)` with nothing appended are nil (`Value::Nil`, len 0); `make` gives a NON-nil empty slice (len 0, cap 0); a variadic call with no variadic arguments passes nil, with arguments a fresh slice with len = cap = n. Nil and empty differ only through `== nil` (`equal(Slice, Nil)` is false, `3399`) and both print `[]`. No `s[a:b]` expression, no `cap`, no `copy`, no `make` with a size exist in `cir/1` (`Expr::Make` "An empty map or slice", `crates/model/src/lib.rs:1456-1458`; intrinsic list `sprintf errorf println append len sorted_keys`, `lib.rs:1462-1465`) | `interp.rs:405-413`, `716-722` (zero), `2251-2256` (make), `924-1003` (variadic), `2744-2749` | 13 cases (`slice` type or `append`) |
| `append` growth and aliasing | `need = len + k`. If `need <= cap` the elements are written into the SHARED array in place (visible through every other slice of it) and the result keeps `cap`. Else a new array is allocated holding a copy, and the new cap is Go 1.27 amd64 `runtime.growslice`: `nextslicecap` (double below 256, else `cap += (cap + 768) / 4` until it fits, or `need` if `need > 2*cap`), rounded up through Go malloc size classes by element size (ints: 1,2,4,8,16,32,64,128,256,512,848; 16-byte pointerful elements: 1,2,4,8,16,32,71; bytes: 8,16,32,64; `append(nil, 5 ints)` cap 6). The stack-buffer cap Go may choose for a non-escaping slice of at most 32 bytes is NOT modelled; `intrinsic.append` is therefore the evaluator one unclaimed kit feature | `interp.rs:2735-2800`; `crates/eval/src/growth.rs:100-143` (tests `growth.rs:148-165`); `crates/eval/src/runner.rs:34-40` (UNCLAIMED) | no case depends on it. Of the 11 case sources with `append`, ten are `xs = append(xs, ...)` self-assignments (so no aliasing is observable); the one other, `expr-spread` `ys := append(xs, xs...)` has len 2 = cap 2, reallocates, and `xs` is not used afterwards. A doubling rule (the C runtime: cap 0 -> 4 -> 2x, `rt.h:268-282`) passes the kit |
| `range` over slice | length fixed at loop start; each element read when its iteration is reached (so writes to later elements in the body are seen); over an array it ranges over a copy unless only the index is used | `interp.rs:1593-1612`, `1645-1651`, `1565-1575` | |
| struct | a value: `Struct(name, fields in declaration order)`, copied on every read, assignment, argument pass, return and value-receiver call; zero value = every field zero; a literal gives omitted fields their zero value and evaluates the given fields in LITERAL order, not declaration order; embedded fields are explicit fields (every promoted access is already a path) (`crates/model/src/lib.rs:949-957`); `==` field by field | `interp.rs:2220-2250` (literal), `2000-2003` (Var read clones), `1037-1050` (receivers), `3391-3394` (eq) | 22 `struct` literal nodes; value structs and `&Struct{}` |
| array | `[N]T` is a value, copied like a struct; zero = N zeros; index out of range panics; `len` is the constant N (the operand is evaluated only if it contains a call or receive); `==` element-wise | `interp.rs:2602-2635`, `708-775`; model `lib.rs:899-901` | `[3]int` (type-array-int), `[3]float64` and `[3]float32` (type-array-float), written with indexed assignment into a local |
| pointers, `&x` | a pointer is `Place{cell, path of Field/Index steps}`: `&x`, `&s.f`, `&a[i]`, `&xs[i]` all work and alias the original; EVERY local, parameter and receiver lives in its own arena cell, so an address taken never dangles after the function returns (heap-escape-all); `&T{..}` and `&<non-addressable>` allocate a fresh cell; `&m[k]` is not addressable; pointer equality = same cell and path; nil pointer = `Nil`; `p.f` auto-dereferences; the loop variables of a three-clause `for` get a fresh copy per iteration (Go 1.22) | `value.rs:21-45`; `interp.rs:1786-1868` (place), `1895-1903` (addr), `471-476, 822-826` (cells), `1506-1562` (per-iteration copies), `3387` (eq) | pointer uses: `&Struct{..}` x7, `&local` x6, pointer receivers, `*p` read/write; no function in any case takes or returns a pointer except through receivers (`jq`: no `results` or non-receiver `params` of `ptr` type), so no address outlives its frame in the kit. The C writer also gives an address-taken local heap storage (`cw` doc line 18) |
| nil slices / maps / errors / funcs / interfaces / chans / pointers | one `Nil`, interpreted by static type; `len(nil)` = 0, `range nil` = no iterations, `nil[i]` out of range panic with length 0, nil map read gives the zero value, nil map write panics | `interp.rs:716-722`, `2074`, `1924`, `2602-2635` | `expr-nil`, `expr-nil-error`, `expr-nil-ptr`, `expr-zero*` |
| map | entries kept in INSERTION order with a hash index; iteration (`range`, `sorted_keys` input) follows insertion order; keys compared by Go `==` (ints, strings, bools, pointers, structs, interface dynamic type+value); NaN keys never match; `make` gives an empty non-nil map; read of absent key gives zero (comma-ok form gives `(zero,false)`, `lookup_else`); `cir/1` has no delete; an insertion during a `range` over the same map is `unsupported`. The C runtime made the same choice (insertion order, open-addressing index: `rt.h:296-420`). Go order is random and no kit case depends on it: `sem-map-order` sums, `fact-unordered-iteration` counts, `intrinsic-sorted-keys` sorts | `value.rs:121-168`; `interp.rs:2040-2062`, `1672-1693` (range), `2698-2733` (sorted_keys: ints numeric, strings by UTF-16 code unit, which differs from byte order outside ASCII) | 7 cases; key/elem types int/bool, int/int, int/ptr, string/int; `range` over a map in 2 cases |
| error | nil = `Nil`; sentinel (`errvar`) equal by name; `errorf` value equal only to itself (identity); `to_error` value = (type, value) equal by both; `err.Error()` gives the message; nil error prints `<nil>` through `%v`; an error returned last turns the kit line into `err <msg>` | `value.rs:68-80`; `interp.rs:1120-1124, 3047-3075, 3373-3402` | 17 cases; messages `missing`, `negative`, `not found: <k>`, `negative: <n>` |
| tuple / several results | `Value::Tuple`; a call with several results can be spread into another call arguments, a multi-target assign, a `let`, or a `return`; map index, receive and `lookup_else` give comma-ok pairs; a function with no result gives `Unit` | `interp.rs:1072-1079, 924-1003` | 5 baseline cases (expr-method, fact-comma-ok, sem-multi-assign-order, stmt-expr-method, type-tuple) |
| `any` / interface | a concrete value entering an interface slot is boxed with its static type (`Iface(type, value)`); `==` compares dynamic type then value; a method call dispatches on the dynamic type, a nil interface panics; `Error()` on `error` handled specially | `interp.rs:565-575`, `1081-1131`, `2325-2340` | 9 cases (type-any, fact-messages, expr-to-iface, expr-iface-call, decl-interface, decl-generic, decl-union, expr-instantiate, type-param) |
| func value | `Func` / bound method (receiver copied when bound) / lambda capturing the enclosing cells BY REFERENCE; calling nil panics | `value.rs:48-66`; `interp.rs:2290-2310` | 5 cases (call_value, func_value, lambda, type-func, stmt-foreign) |
| chan | cell with a queue and capacity; no goroutines. Entry channel arguments have capacity 64 and are closed and drained after the call; `make(chan T)` has capacity 0 | `interp.rs:2253-2259`, `3185-3213`; `crates/conformance/src/lib.rs:1025` | 4 cases (type-chan, stmt-send, unop-recv, fact-messages) |
| globals | module `vars` initialised in Go dependency order (declaration order for the rest), then `init` functions sorted by file then line, before the first call | `interp.rs:3079-3157` | 3 cases with `vars` (decl-var, decl-var-int, expr-global) |
| generics | type parameters bound per call (`tbind`) and substituted; a lowering monomorphizes by `targs` | `interp.rs:493-533` | 3 cases (decl-generic, expr-instantiate, type-param) |

Evaluation order (all left to right; C refuses expressions with two calls rather than trust C order, `cw` doc "Order of evaluation"):
- `binop`: `x` then `y`; `&&` and `||` short-circuit (`interp.rs:2157-2160`). `index`: `x` then index (`2034-2038`). Call arguments: left to right (`924-1003`). Struct literal: literal field order (`2220-2250`).
- `assign` without `op`: every target (including the operands of its index/field/pointer expressions and the bounds check) first, then every right-hand side, then stores left to right: a multi-target assign reads all values before storing any (`sem.multi_assign.order`, `interp.rs:1302-1314`; `a, b = b, a+b` in `sem-multi-assign-order`). Compound `op=`: target, load current, evaluate right-hand side, `arith`, store (`1325-1333`).
- `for`: `init`, then `cond` before each iteration, `post` after each iteration including one ended by `continue` (`sem.continue.post`); `break` inside `switch` leaves the enclosing loop (`sem.switch.break`; `switch` returns its case body flow, `interp.rs:1428-1450`); `switch` evaluates the tag once, case values in order until one matches, no fallthrough, default when none (`1418-1450`).

## 5. Output encoding the runner must print

Source of truth: `crates/conformance/src/lib.rs:78-113` (calls and output), driver `conformance/driver/go/main.go`, evaluator copy `interp.rs:3159-3300`, C runner copy `adapters/c-run/src/lib.rs:1-60`.

Per call, in order (stdout only, `\n` line ends, no trailing spaces):
1. anything the function printed through `println` (written at call time).
2. the result line:
   - no results: `ok`
   - results, last not `error`: `ok <v> <v> ...` (one space between, no trailing space); an `ok` line with `()` unit prints `ok`.
   - last result is `error`: non-nil -> exactly `err <Error()>` (raw text, NOT quoted, other results dropped); nil -> `ok <v> ...` of the results before it (`ok` alone if none). `interp.rs:3236-3260`.
   - comma-ok result `(int,bool)`: `ok 0 false` (`fact-comma-ok/expected.txt`).
3. for each `chan` argument in argument order, after the call the channel is closed and each remaining value is printed `sent <v>` in send order (`crates/conformance/src/lib.rs:98-113`; `interp.rs:3185-3213`). Entry channel capacity is 64 (`lib.rs:1025`, `78-96`).

Value formats (`show`, `interp.rs:3262-3300`, Go driver `show`): signed integer in decimal (`-` for negative; evaluator also prints unsigned in decimal, the Go driver refuses unsigned kinds); `true`/`false`; string as Go `strconv.Quote`; slice as `[` elements joined by one space `]` (nil or empty `[]`; elements recursively formatted, a slice of `any` is refused); a newtype or enum prints as its underlying value; an `any` prints the value it holds. Other result types (floats, structs, pointers, maps) are refused when calls are generated (`crates/conformance/src/lib.rs:111-113`).

`strconv.Quote` as implemented and sufficient for the kit (`crates/eval/src/gofmt.rs:7-26`): `"` + per byte: `0x07` `\a`, `0x08` `\b`, `0x0c` `\f`, `0x0a` `\n`, `0x0d` `\r`, `0x09` `\t`, `0x0b` `\v`, `\` `\\`, `"` `\"`, `0x20..0x7e` as is, any other ASCII byte (0x00-0x06, 0x0e-0x1f, 0x7f) as `\x` and two lowercase hex digits; a byte >= 0x80 makes the evaluator stop `unsupported` (and the C and Rust runners exit 2) because Go prints printable runes raw and escapes others via a Unicode table the runners do not hold. No kit `expected.txt` contains a non-ASCII byte or a backslash (`grep -lP "[^\x00-\x7f]"` and `grep -h "\\"` over `expected.txt`: no output), and call string arguments are printable ASCII other than `"` and `\` (`crates/conformance/src/lib.rs:84-90`), so the kit never needs an escape; a general writer needs the table above for runtime-built strings.

Arguments in `calls.json` (`crates/conformance/src/lib.rs:78-96`): int = decimal within int64 and within the parameter width (`interp.rs:3166-3176`), `true`/`false`, string `"..."` (bytes between the quotes, `interp.rs:3178`), `chan`.

The four `println` cases and their exact output (`cat -A`): `intrinsic-println` `Show("a",1)` -> `a 1` then `ok`; `intrinsic-println-int` `Show(1)`, `Show(-20)` -> `count 1`, `ok`, `count -20`, `ok`; `stmt-expr` `Say("hello")`, `Say("a b")` -> `hello`, `ok`, `a b`, `ok`; `type-unit` `Greet("ada")`, `Greet("")` -> `ada`, `ok`, an empty line, `ok`. `println` = Go `fmt.Println`: operands formatted with `%v`, joined by one space, `\n` appended (`interp.rs:2669-2696`); stdout and the `ok` lines share one stream in program order.

Format strings in the kit (all `sprintf`, `errorf`, `println` nodes, `jq` over all 145 `case.cir.json`; the first argument is a literal in every `sprintf`/`errorf`):

| case | intrinsic | format | args (types) |
|---|---|---|---|
| intrinsic-sprintf | sprintf | `%s=%d` | string, int |
| intrinsic-errorf | errorf | `negative: %d` | int |
| expr-zero | errorf | `negative: %d` | int |
| type-float | sprintf | `%v %v` | float64, float32 |
| type-array-float | sprintf | `%v %v` | float64, float32 |
| intrinsic-println, intrinsic-println-int, stmt-expr, type-unit | println | n/a (operands only) | string, int / string |

Distinct verbs in the whole kit: `%d` (2 formats), `%s` (1), `%v` (1 format, used twice, both on floats). No `%w`, `%q`, `%x`, `%f`, `%t`, `%c`, width/precision/flags. The evaluator implements exactly `%d %s %v %%` (and `%w` in `errorf`) and stops `unsupported` on any other verb, a missing argument, an extra argument, a format ending in `%` (`interp.rs:2803-2900`). Verb semantics there: `%d` prints the integer in decimal (an `any` holding an int is unwrapped), non-integer is `unsupported`; `%s` and `%v` first call a value `Error()` or `String()` method when its type has one (not through unexported struct fields), else `%s` requires a string and `%v` prints ints, floats (shortest `g`, section 2), bools, strings raw, nil as `<nil>` (nil slice `[]`, nil map `map[]`), slices and arrays `[a b]`, structs `{a b}` (`interp.rs:2957-3045`).

## 6. Kit coverage by feature

Source: a script over all 145 `conformance/cases/*/case.cir.json`, `calls.json`, `expected.txt` (per-case flags: `research/classes.json`; the per-case table is below). Rules of the classification: a feature counts when a node or type of that kind occurs in the case module (`slice` = slice type or `append`/slice literal; `str-lit` = a string literal or string type but no runtime-built string; `str-rt` = a string BUILT at run time: `+`, `+=`, `sprintf`, `errorf`, `to_error`, extern result, a string param passed on); `base` columns (`struct ptr array tuple global method`) are inside the first cut. Tier = the highest rung of the ladder `slice < str-lit < str-rt < error < float < map` the case needs; T7 = needs any of func value, interface/any, chan, generics, extern, foreign, raise (all 23 T7 cases have at least one).

First cut = ints, bool, structs, pointers, fixed arrays, multi-result calls, methods, module globals, control flow (`if for switch break continue return`, `range` over nothing but ints, `assign` incl. compound and multi-assign): tier T0.

| step | adds (cases newly passing) | cumulative of 145 |
|---|---|---|
| first cut (T0) | 78 (of which 59 use no struct/ptr/array/tuple/global/method at all: ints and bool only; the other 19: struct 19, tuple 19, ptr 13, method 10, global 3, array 2, overlapping) | 78 |
| + slices (`[]int`, `append`, `len`, index, `range`, spread, variadic) | +7 | 85 |
| + string literals (string type, printed `"..."` or `err <msg>` from literal bytes, `==`, `len`, switch on string) | +11 | 96 |
| + runtime strings (`+`, `+=`, `sprintf`, `errorf`) | +2 | 98 |
| + errors (`error` values, `errvar`, `errorf`, `to_error`, `err` line) | +13 | 111 |
| + floats (f32/f64 ops, `%v` shortest printing) | +4 | 115 |
| + maps (`make`, index, `range`, `sorted_keys`, comma-ok) | +7 | 122 |
| + func value, interface/any, chan, generics, extern, foreign/raise | +23 | 145 |

The ladder order is mine; the increments depend on it. Order-independent view (a case is counted for feature F when it uses F at all; "alone" = F is its only extra beyond the first cut, so adding just F makes it pass):

| feature | cases using it | cases that need only F beyond the first cut |
|---|---|---|
| slice | 13 | 7 |
| string literal (incl. string type) | 23 | 8 |
| runtime string | 13 | 2 |
| error | 17 | 6 |
| float | 4 | 2 |
| map | 7 | 3 |
| func value | 5 | (all T7) |
| interface / any | 9 | (all T7) |
| extern | 7 | (all T7) |
| chan | 4 | (all T7) |
| generic | 3 | (all T7) |
| foreign / raise | 1 / 1 | (all T7) |

Observations a plan needs:
- Strings are needed for output, not only for string programs: the kit prints `err <msg>` for the `errvar` messages of 8 cases, and 6 of the 17 error cases contain no string expression at all (`expr-nil-error`, `expr-zero-error`, `fact-fallible`, `sem-error-propagate`, `type-error`, `type-tuple-error`): the message lives in the module `errors[].message` (e.g. `expr-nil-error/case.cir.json` path `errors.0.message` = `negative`) as static data, so a backend with only ints still needs a static string table and a way to print its bytes for those cases.
- Slice-only cases (7) never alias or grow observably (section 4); a fixed-capacity or doubling runtime passes.
- Float cases: 2 of 4 (`type-float`, `type-array-float`) are only observable through `%v` of float64 and float32 (shortest round-trip printing, section 2) which is therefore a runtime-string feature as well; `expr-float` and `type-float-compare` return bool. `type-array-float` also needs arrays of float.
- Only 4 cases print through `println` (`intrinsic-println`, `intrinsic-println-int`, `stmt-expr`, `type-unit`), all with a string operand (and an int in two): no case printing an int-only `println`.
- Map cases (7): the 4 with `range` or `sorted_keys` are order-independent by construction (section 4).
- Pointer-to-local escapes: no function returns a pointer other than a freshly allocated `&T{}`, so stack slots for locals are enough for the kit even where Go would heap-allocate.

Per-case table (`tier`; `base` = first-cut features present; `extras` = beyond; `io` = special output):

| case | tier | base | extras | io |
|---|---|---|---|---|
| assign-add | T0 | - | - | - |
| assign-and | T0 | - | - | - |
| assign-and-not | T0 | - | - | - |
| assign-div | T0 | - | - | - |
| assign-mul | T0 | - | - | - |
| assign-or | T0 | - | - | - |
| assign-rem | T0 | - | - | - |
| assign-shl | T0 | - | - | - |
| assign-shr | T0 | - | - | - |
| assign-sub | T0 | - | - | - |
| assign-xor | T0 | - | - | - |
| decl-embedded | T0 | struct, ptr, method | - | - |
| decl-enum | T0 | - | - | - |
| decl-enum-string | T2 | - | str-lit | - |
| decl-generic | T7 | struct, method | iface, generic | - |
| decl-interface | T7 | struct, method | iface | - |
| decl-newtype | T0 | - | - | - |
| decl-struct | T0 | struct | - | - |
| decl-union | T7 | - | str-lit, iface | - |
| decl-var | T3 | global | str-rt | - |
| decl-var-int | T0 | global | - | - |
| decl-variadic | T7 | - | slice, str-rt, extern | - |
| decl-variadic-local | T1 | - | slice | - |
| expr-binop | T0 | - | - | - |
| expr-call | T0 | - | - | - |
| expr-call-value | T7 | struct | func | - |
| expr-const | T0 | - | - | - |
| expr-conv | T0 | - | - | - |
| expr-errvar | T4 | - | str-lit, error | err-line |
| expr-extern | T7 | - | str-rt, extern | - |
| expr-extern-method | T7 | ptr, tuple | str-lit, error, extern | - |
| expr-field | T0 | struct | - | - |
| expr-float | T5 | - | float | - |
| expr-func-value | T7 | struct, method | func | - |
| expr-global | T0 | global | - | - |
| expr-iface-call | T7 | ptr, tuple | str-lit, error, iface, extern | - |
| expr-index | T1 | - | slice | - |
| expr-instantiate | T7 | struct | iface, generic | - |
| expr-intrinsic | T2 | - | str-lit | - |
| expr-intrinsic-slice | T2 | - | slice, str-lit | - |
| expr-lambda | T7 | - | func | - |
| expr-lit | T0 | - | - | - |
| expr-make | T6 | - | str-lit, map | - |
| expr-method | T0 | struct, ptr, tuple, method | - | - |
| expr-nil | T1 | - | slice | - |
| expr-nil-error | T4 | tuple | error | err-line |
| expr-nil-ptr | T0 | ptr | - | - |
| expr-spread | T7 | - | slice, str-rt, extern | - |
| expr-spread-local | T1 | - | slice | - |
| expr-struct | T0 | struct | - | - |
| expr-to-error | T4 | struct, method | str-rt, error | err-line |
| expr-to-iface | T7 | struct, method | str-rt, iface | - |
| expr-unop | T0 | - | - | - |
| expr-var | T0 | - | - | - |
| expr-zero | T4 | tuple | str-rt, error | err-line |
| expr-zero-error | T4 | tuple | error | err-line |
| expr-zero-struct | T0 | struct | - | - |
| fact-comma-ok | T0 | tuple | - | - |
| fact-displays | T2 | - | str-lit | - |
| fact-fallible | T4 | tuple | error | err-line |
| fact-messages | T7 | - | iface, chan | sent |
| fact-read-only | T0 | struct, ptr, method | - | - |
| fact-shared | T6 | struct, ptr | map | - |
| fact-unordered-iteration | T6 | - | map | - |
| intrinsic-append | T1 | - | slice | - |
| intrinsic-errorf | T4 | - | str-rt, error | err-line |
| intrinsic-len | T2 | - | slice, str-lit | - |
| intrinsic-println | T4 | tuple | str-lit, error | println |
| intrinsic-println-int | T4 | tuple | str-lit, error | println |
| intrinsic-sorted-keys | T6 | - | slice, str-rt, map | - |
| intrinsic-sprintf | T3 | - | str-rt | - |
| op-add | T0 | - | - | - |
| op-and | T0 | - | - | - |
| op-and-not | T0 | - | - | - |
| op-div | T0 | - | - | - |
| op-eq | T2 | - | str-lit | - |
| op-ge | T0 | - | - | - |
| op-gt | T0 | - | - | - |
| op-land | T0 | - | - | - |
| op-le | T0 | - | - | - |
| op-lor | T0 | - | - | - |
| op-lt | T0 | - | - | - |
| op-mul | T0 | - | - | - |
| op-ne | T0 | - | - | - |
| op-or | T0 | - | - | - |
| op-rem | T0 | - | - | - |
| op-shl | T0 | - | - | - |
| op-shr | T0 | - | - | - |
| op-sub | T0 | - | - | - |
| op-xor | T0 | - | - | - |
| sem-continue-post | T0 | - | - | - |
| sem-error-propagate | T4 | tuple | error | err-line |
| sem-int-wrap | T0 | - | - | - |
| sem-int-wrap-no-div | T0 | - | - | - |
| sem-map-order | T6 | - | map | - |
| sem-multi-assign-order | T0 | tuple | - | - |
| sem-rem-sign | T0 | - | - | - |
| sem-switch-break | T0 | - | - | - |
| stmt-assign | T0 | - | - | - |
| stmt-break | T0 | - | - | - |
| stmt-continue | T1 | - | slice | - |
| stmt-decr | T0 | - | - | - |
| stmt-expr | T4 | tuple | str-lit, error | println |
| stmt-expr-method | T0 | struct, ptr, tuple, method | - | - |
| stmt-for | T0 | - | - | - |
| stmt-for-range | T1 | - | slice | - |
| stmt-foreign | T7 | - | func, foreign | - |
| stmt-if | T0 | - | - | - |
| stmt-incr | T0 | - | - | - |
| stmt-let | T0 | - | - | - |
| stmt-lookup-else | T6 | tuple | str-lit, error, map | err-line |
| stmt-raise | T7 | - | str-lit, extern, raise | - |
| stmt-return | T2 | tuple | str-lit | - |
| stmt-return-one | T0 | - | - | - |
| stmt-send | T7 | - | chan | sent |
| stmt-switch | T2 | - | str-lit | - |
| stmt-var | T0 | - | - | - |
| type-any | T7 | - | iface | - |
| type-array-float | T5 | array | str-rt, float | - |
| type-array-int | T0 | array | - | - |
| type-bool | T0 | - | - | - |
| type-chan | T7 | - | str-lit, chan | sent |
| type-error | T4 | - | error | err-line |
| type-extern | T7 | struct, ptr, tuple | str-rt, error, extern | - |
| type-float | T5 | - | str-rt, float | - |
| type-float-compare | T5 | - | float | - |
| type-func | T7 | - | func | - |
| type-int | T0 | - | - | - |
| type-map | T6 | - | str-lit, map | - |
| type-named | T0 | - | - | - |
| type-param | T7 | - | iface, generic | - |
| type-ptr | T0 | ptr | - | - |
| type-slice | T2 | - | slice, str-lit | - |
| type-string | T2 | - | str-lit | - |
| type-tuple | T0 | tuple | - | - |
| type-tuple-error | T4 | tuple | error | err-line |
| type-unit | T2 | - | str-lit | println |
| type-unit-method | T0 | struct, ptr, method | - | - |
| unop-complement | T0 | - | - | - |
| unop-deref | T0 | ptr | - | - |
| unop-neg | T0 | - | - | - |
| unop-not | T0 | - | - | - |
| unop-plus | T0 | - | - | - |
| unop-recv | T7 | - | chan | - |
| unop-ref | T0 | struct, ptr | - | - |

## 7. Traps for a backend writer

1. The kit never exercises any of the hard integer cases except `Div(MinInt64,-1)`, `%` by -1 and `>>` of negatives, only width `platform` signed: widths 8/16/32, unsigned, shift count >= width and divide by zero are model-legal but unpinned by any case; the oracle behaviour (section 1) is the spec, so cover them with own tests, not the kit.
2. Shifts: `y >= width` is 0 (or -1 for signed `>>` of a negative) at the OPERAND width, and hardware masks the count; a negative signed count is a panic, an unsigned one is just large.
3. `idiv` traps (SIGFPE) on `MinInt / -1` and `/ 0`; the oracle yields `MinInt` and `0` for the former. Narrow widths must be re-extended after every op, including `/` and unary `-`.
4. Failure encoding is unpinned (section 3): the kit has no panicking case and no `panic` output line exists; `foreign` and `raise` are refused, not executed. The closest convention is the C runtime (stderr `panic: runtime error: ...`, exit 3, stdout flushed first), and runners treat exit 2 as "cannot print", exit 1 as failed.
5. A call with several results whose last is `error` prints `err <msg>` for a non-nil error and DROPS the other results; for a nil one it prints `ok <values>`. Errors are NOT quoted, strings in `ok` lines are `strconv.Quote`d.
6. A `println` line is written during the call, so it precedes that call `ok`/`err` line; with channel arguments `sent` lines come last.
7. The place of an assignment (including the bounds check and operand calls) is evaluated BEFORE its right-hand side, and a multi-assign loads all right-hand sides before any store: `a[i], a[j] = a[j], a[i]` and `a, b = b, a+b` must not be lowered as sequential stores.
8. Struct literals evaluate the given fields in literal order; struct, array and tuple results are values (copy on assignment, argument and return); address-taken locals must outlive the frame in general (the oracle gives every local its own heap cell), even though no kit case leaks one.
9. `for` loop variables of a three-clause loop are fresh per iteration (Go 1.22): a closure capturing them (lambda cases) sees per-iteration copies; `continue` still runs the post statement; `break` inside `switch` leaves only the switch.
10. Slice `append` growth is Go size-class rounding in the oracle (`append(nil, 5 ints)` has capacity 6), but cir/1 has no `cap` or slice expression, so capacity is not observable except as aliasing after an `append` that fits; the kit never observes it, and a doubling rule is behaviourally enough.
11. Float: per-operation rounding to f32 is required (`type-float-compare` fails otherwise); comparisons with NaN are false except `!=`; `%v` printing needs the shortest round-trip algorithm and `e+XX` exponent form from 1e21 and below 1e-4; there is no fixed-precision shortcut. Int to float32 via binary64 (double rounding) is the oracle behaviour, unverified against Go.
12. `strconv.Quote` has a table for non-ASCII that no existing runner holds: bytes >= 0x80 make the evaluator and both runners refuse; the kit is pure ASCII.
13. The `platform` int is 64-bit on every target; string `<` is byte order in the evaluator while `sorted_keys` over strings is UTF-16 order (identical on ASCII).
14. Checked integer operators (`+!` and others) are approved but not in the model yet (`.engineering/planning/design/cir1-checked-integer-operators.md`): unknown operators must be refused by a lowering, never treated as wrapping.
15. Tooling note for this document: the Write tool and a shell heredoc were blocked by the session guard, so the document and its parts were produced with `python3 -I -c` into this scratch directory only; no repository file was touched.
