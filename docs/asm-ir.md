# `asm_ir/1`: one low-level IR between `cir/1` and every machine (2026-11-25)

Status: design, decided by the planner on the owner's direction of 2026-11-25 ("so [any_lang] ->
[codespec] -> [asm_ir/1] -> [any_arch]"; "maybe design an IR in between so we have multiple
architectures"). Every decision below is reversible until `asm_ir/1` is frozen by its first
release; the two that are not reversible after that are marked.

## 1. The problem

`story:x86-64-writer` (draft) maps `cir/1` straight to x86-64 assembly. A second architecture under
that plan repeats the whole mapping: the data layout of structs, strings and slices, Go's integer
edge cases (`MinInt / -1`, a shift by the width, a negative `%`), panics, the runtime and the
driver. Measured on the host (research notes, § 12):

- the three targets disagree on what the hardware does at every edge the kit tests: `x / 0` faults
  on x86-64 (SIGFPE, exit 136), gives 0 on AArch64 and -1 on RISC-V; `MinInt / -1` faults on
  x86-64 and gives `MinInt` on the other two; every target masks a shift count, Go does not;
  float-to-int out of range gives `0x8000…` on x86-64, saturates on the other two;
- their calling conventions disagree on aggregates in five ways (AArch64 HFAs in `v` registers,
  RISC-V flattening of small float structs, the result pointer in `x8` against `a0`, a two-register
  struct split between `a7` and the stack on RISC-V only, a ninth double in a spare `a` register on
  RISC-V only); in QBE the ABI file is 30 to 44 % of each backend;
- the Linux syscall numbers and registers differ (x86-64 `write` 1 in `rax` with `syscall`;
  AArch64 and RISC-V `write` 64, in `x8` with `svc #0` and in `a7` with `ecall`).

So everything a language decides (layout, semantics, runtime, driver) is done once, in a lowering
to a small target-independent IR, and each architecture is a template expander over that IR.

```
any language --reader--> cir/1 --codespec lower--> asm_ir/1 --backend--> <arch> assembly --as, ld--> ELF
                                  (layout, semantics,          x86-64          (static, no libc;
                                   runtime, refusals)          aarch64          native or qemu-user)
                                                               riscv64
```

## 2. Decisions

| id | decision | why |
|---|---|---|
| A1 | Non-SSA: typed mutable virtual registers (vregs), basic blocks with one terminator | structured `cir/1` needs no phi insertion; a spill-everything backend needs no phi destruction. QBE and MIR accept non-SSA input. An SSA pass can come later as `asm_ir -> asm_ir` |
| A2 | Four vreg types: `i32`, `i64`, `f32`, `f64`. A pointer is `i64`, a `bool` is `i32` 0/1. `i8`/`i16` exist only as memory widths and as the `from` of `ext` | 8/16-bit wrap after each operation is then the lowering's job, written once, not three backends' (AArch64 and RISC-V have no narrow arithmetic) |
| A3 | No aggregate type in the IR. Structs and arrays live in stack slots or heap blocks; a struct argument is a pointer to a copy the caller made; a struct result goes through a hidden destination pointer; `string` is 2 words, `slice` 3, `error`/`any`/interface/`func` 2, each scalarised | no backend classifies an aggregate, so the five ABI disagreements above never reach a backend; layout is identical on every target, which byte-equal output needs |
| A4 | The calling convention is private to each backend and not part of the format. First cut on every target: each argument and result is an 8-byte cell in memory, no argument registers, every vreg in the frame | the same logic on three ISAs apart from `sp`/`fp` addressing and the call instruction. Calling into or from C is not possible in the first cut; a later `abi: "c"` on a function with a scalar signature is additive |
| A5 | No libc. The only contact with the machine is the instruction `syscall` over a closed list: `write`, `read`, `exit`, `mmap`. The backend emits `_start` | one static binary shape that runs natively and under `qemu-aarch64`/`qemu-riscv64` without a cross libc (the host has none) |
| A6 | The runtime (allocation, panic, printing, string and slice helpers, `memmove`) is written once, in `asm_ir`, by a Rust builder in the lowering crate; only functions reachable from the entry are emitted | one copy instead of three assembly dialects; every backend runs the same runtime |
| A7 | Division by zero is a precondition violation in the IR; the lowering emits the check and the call of `rt.panic_divzero` before each `sdiv`/`srem`/`udiv`/`urem` whose divisor is not a nonzero constant. `MIN / -1` is defined: quotient `MIN`, remainder 0 | three ISAs, three behaviours at zero; one check in the IR makes them agree. `MIN / -1` is free on AArch64 and RISC-V; the x86-64 backend adds one fix-up (probed: 708,170 checks, 0 failures) |
| A8 | Shifts mask the count to `log2(width)` bits, as all three ISAs do; the lowering builds Go's result for a count at or above the width with `cmp`, `sub`, `and`, no select. **Not reversible after freeze** | one instruction per backend; Rust's and Java's shift semantics lower onto the same primitive. Decided by the planner (research open point O2) |
| A9 | Immediates and float bit patterns are decimal strings in the JSON, as `cir/1` writes a literal's `value`. **Not reversible after freeze** | a JSON reader in JavaScript loses an `i64` above 2^53. Decided by the planner (open point O4) |
| A10 | `ftoi` of NaN or of a value out of the target range is unspecified; the lowering guards it where the source language defines a result (`story:ecmascript-number-semantics`) | the three ISAs give three answers (x86-64 `MIN`, AArch64 saturates with NaN 0, RISC-V saturates with NaN `MAX`); the guard is written once |
| A11 | Narrow results are kept extended by the lowering: every `i32` holding an 8- or 16-bit value is sign- or zero-extended after each operation that can leave the range. Upper bits of an `i32` held in a 64-bit register are the backend's business | the free canonical form differs (AArch64 `w` ops zero-extend; RISC-V `*w` ops and `lw` sign-extend); the IR states only the `i32` value |
| A12 | No `externs` field. A `cir/1` extern without a runtime implementation is a lowering refusal | every callee is in the document; the kernel is reached by `syscall`. Adding the field later is `asm_ir/2`. Decided by the planner (open point O5) |
| A13 | Floats are a separable feature group: the module's `features` lists `float` when any `f32`/`f64` vreg or operation occurs; a backend that does not claim it refuses the module whole | integers, structs and control flow can ship on all three targets before float printing exists |

## 3. The document

One canonical JSON document per whole program (the module, its runtime and, for a run, its
driver), defined by Rust types in `crates/asm-ir`, read strictly as `cir/1` is: `deny_unknown_fields`,
nodes tagged by `k` in snake_case, ordered arrays only, a generated JSON Schema
`schemas/asm-ir-1.schema.json` drift-checked by `cargo test`, any other `format` refused.

```text
module   { format: "asm_ir/1", module: name, word: 64,
           from: { format: "cir/1", module: name, sha256: hex },
           entry: name?,                 a func with no params and no results; absent in a library
           features: [string],           sorted; "float" iff any f32/f64 vreg or op
           data: [data], funcs: [func] }
data     { name, align: 1|2|4|8|16, mutable: bool, items: [item] }
item     { k:"bytes", hex } | { k:"zero", n } | { k:"addr", sym, off }
func     { name, exported: bool, params: [type], results: [type],
           vregs: [type],                index = id; the first len(params) are the params
           slots: [{ size, align }],
           blocks: [block],              block 0 is the entry and no jump target
           origin: string? }             the cir/1 function key
block    { insns: [insn], term: term, at: string? }   at: cq node id of the cir/1 statement
operand  { k:"v", v: id } | { k:"i", i: "<decimal>" }
```

Names (functions, data) are `[A-Za-z_.][A-Za-z0-9_.]*`, unique across `data` and `funcs`; the
runtime's are prefixed `rt.`, a `cir/1` method `T.m` becomes `T.m`. Strict reading checks: every
vreg, slot, block and symbol exists; operand types match the instruction's `t`; each immediate is
in range of its type; `call` agrees with the callee's declaration; `ret` matches `results`;
`syscall` names one of the closed list with the right arity; `features` matches use; `word` is 64;
text fields carry no control or format character. A forward "assigned before read" check catches
lowering bugs.

## 4. Instructions

41 operation kinds in about 20 node kinds (`bin` carries an `op`).

| group | node | semantics |
|---|---|---|
| constants | `const d imm`, `fconst d bits`, `mov d a` | `fconst` takes the IEEE bit pattern, never a decimal float |
| integer | `bin op t d a b`, op in `add sub mul and or xor sdiv udiv srem urem shl shr sar` | `add sub mul` wrap modulo 2^N, one op for signed and unsigned; division A7; shifts A8; negation is `sub 0 a`, complement `xor a -1`, `&^` is `and a (xor b -1)` |
| compare | `cmp cc t d a b` (cc: `eq ne slt sle sgt sge ult ule ugt uge`), `fcmp cc t d a b` (cc: `eq ne lt le gt ge`) | `d` is an `i32` 0 or 1. `fcmp` is false when an operand is NaN, except `ne`. No flags cross an instruction (RISC-V has none) |
| convert | `ext d a from signed`, `trunc d a`, `itof d a signed`, `ftoi d a signed`, `fpromote`, `fdemote`, `bitcast` | `ext` from 8, 16 or 32 bits; `ftoi` A10 |
| float | `fbin op t d a b` (op: `add sub mul div`), `fneg d a` | IEEE, round to nearest even; `fneg` flips the sign bit, so `-0.0` is right |
| memory | `load d base off m signed`, `store base off v m`, `slot_addr d slot`, `sym_addr d sym off` | `m` in `i8 i16 i32 i64 f32 f64`; little-endian; the lowering keeps every access naturally aligned and checks nil and bounds itself (no signal handler) |
| calls | `call ds callee args`, `call_ind ds fn args sig`, `syscall d name args` | the signature of a direct call is the callee's; `call_ind` carries its own (function values, interface dispatch) |
| terminators | `jmp to`, `br cond then else`, `ret vals`, `trap code` | `br` takes an `i32`; `trap` exits with `code` and prints nothing (the runtime prints first) |

Not in the IR: phi, select, `switch` (a chain of `cmp` and `br`), `blit`/`memcpy` (inline pairs or
`rt.memmove`), `fsqrt` and other math (later, additive, guarded by `features`), vectors, atomics,
threads, dynamic `alloca`, varargs, unwinding.

## 5. The lowering: `cir/1` to `asm_ir/1`

`crates/lower` (`b10x-codespec-lower`), command `codespec lower`, reads one `cir/1` module and
prints one `asm_ir/1` module, or refuses functions as every writer does: a function it cannot
lower is left out and listed with its key, position and reason; a call of a left-out function is
left out too.

| cir/1 | asm_ir/1 |
|---|---|
| `int` 8/16/32 | `i32`, kept extended per A11; 64 and `platform`: `i64` |
| `bool` | `i32` 0/1 |
| `float` 32/64 | `f32`/`f64` |
| `ptr`, `map`, `chan` | `i64` address |
| `named` struct, `array` | bytes at an address: natural alignment, fields in declaration order, one rule in the lowering |
| `string` | `(ptr, len)`; a literal is a read-only `data` object |
| `slice` | `(ptr, len, cap)`; `append` doubles the capacity (no case observes it; the evaluator uses Go's size classes) |
| `error`, `any`, interface, `func` | `(descriptor, word)`; descriptor a `data` object |
| named newtype, enum | its underlying type |
| several results | several results |
| a local whose address is taken, or that is an aggregate | a stack slot (or a heap block when it escapes the function); else a vreg |

Integer semantics are those of the evaluator, which is the oracle (`docs/evaluator.md`): `+ - *`
and negation wrap; `/` and `%` truncate toward zero with the remainder's sign from the dividend,
`MinInt / -1` is `MinInt`, division by zero panics; a shift by the width or more gives 0 (or -1
shifting a negative right), a negative signed count panics; a conversion truncates or extends. A
panic writes `panic: <message>` to stderr and exits 3, as `cs_panic` in the C runtime does
(`adapters/c/src/codespec_rt.h`); an index out of range and a nil dereference panic the same way.

Groups, in order, with the kit cases each brings (of 145; `docs/asm-ir/cir-semantics.md` § 6,
cumulative, each case counted at the highest group it needs):

| group | constructs | cases | total |
|---|---|---|---|
| 1 | integers of every width, `bool`, calls, `if`, `for`, `switch`, `break`, `continue`, compound assignment, `incr`/`decr`, constants, newtypes and enums, module variables, structs, pointers, methods, fixed arrays, `zero`, tuples | 78 | 78 |
| 2 | slices of numbers (`make`, `append`, `len`, index, `for_range`) | 7 | 85 |
| 3 | string literals, string `==`, `len`, printing a string result | 11 | 96 |
| 4 | strings built at run time (`sprintf`, `errorf`) | 2 | 98 |
| 5 | errors: sentinel `errvar`, `nil`, `to_error`, the `err <msg>` line | 13 | 111 |
| 6 | floats (A13), shortest round-trip printing for both widths | 4 | 115 |
| 7 | maps, insertion-ordered | 7 | 122 |
| later | function values and lambdas, `any`, interfaces, channels, generics (monomorphised), externs | 23 | 145 |

What the kit does not pin, and the op corpus of § 8 must: the kit's integers are all signed
`platform` (64-bit), so widths 8, 16 and 32, unsigned arithmetic, a shift by the width or more and
division by zero are reached by no case; no case panics; no case observes a slice's capacity, so
the growth rule is free (doubling) until `cap` or a slice expression enters `cir/1`. The kit's
hard integer edges are `MinInt64 / -1`, `% -1` and `>>` of a negative. Six error cases contain no
string expression, yet print `err <msg>` from `errors[].message`: group 5 needs the static data of
group 3. Floats need `f32` rounding after each operation (`type-float-compare` fails without it).
An operator outside the model's list is refused, never wrapped.

Refused with a reason, always: `foreign`, `raise`, an extern without a runtime implementation.

## 6. The runtime

Built by `crates/lower`'s builder as `asm_ir` functions, emitted when reachable:

| function | does |
|---|---|
| `rt.alloc(n) -> p` | bump allocation over 1 MiB chunks from `syscall mmap`, zero-filled, never freed; a failed `mmap` panics "out of memory" |
| `rt.panic(ptr, len)` | writes `panic: `, the message and a newline to fd 2, `syscall exit 3` |
| `rt.panic_divzero`, `rt.panic_index`, `rt.panic_nil`, `rt.panic_shift` | `rt.panic` with the evaluator's message |
| `rt.write_all(fd, ptr, len)` | loops on a short write |
| `rt.fmt_i64`, `rt.fmt_u64`, `rt.write_quoted` | decimal and Go `strconv.Quote` output for the driver and `println` |
| `rt.memmove`, `rt.memeq`, `rt.str_eq`, `rt.append` | byte loops |

## 7. Backends

Each backend is a library `emit(&asm_ir::Module) -> String` and a writer program that composes the
lowering with it. Every vreg lives in a frame cell; each instruction loads its operands into fixed
caller-saved scratch registers, computes and stores. Output is deterministic: document order,
labels from function and block indices, no address, path or time.

| target | assembler, link, run | what is not a one-line template (research notes) |
|---|---|---|
| x86-64 | GNU `as` 2.47, AT&T; `ld.lld -static -e _start`; native | `idiv`/`div` in `rax:rdx` with the `-1` fix-up; a variable shift's count in `cl`; `fcmp eq` is `sete` and `setnp`, `lt` is `seta` with swapped operands; unsigned `i64` to float; syscall argument 4 in `r10` |
| aarch64 | `clang --target=aarch64-linux-gnu -c`; `ld.lld`; `qemu-aarch64` | constants by `movz`/`movk` (`mov x16,#80000` does not assemble); scaled 12-bit load/store offsets, larger through a scratch register; `adrp`+`add`; `sdiv`+`msub` for `rem`; NaN-correct conditions `eq ne mi ls gt ge`, never `lt`/`le`; `b.cond` reaches ±1 MiB |
| riscv64 | `clang --target=riscv64-linux-gnu -c`; `ld.lld` without `--relax-gp`; `qemu-riscv64` | conditional branches reach ±4 KiB: always an inverted branch over a `j`; `lla`, never `la` (a GOT load under clang); 12-bit offsets; no flags (`slt`, `sltu`, `seqz`); `feq`/`flt`/`fle` |

Writer programs `codespec-write-x86-64`, `codespec-write-aarch64`, `codespec-write-riscv64`, one
crate `adapters/asm`: each reads `cir/1` on stdin and writes `<out>/<module>.s`, beside it
`<module>.asm-ir.json` (the lowered module) and `refusals.json`; `--describe` prints its manifest;
`--asm-ir` reads an `asm_ir/1` module instead of `cir/1`. The `.s` file's first line is
`# codespec: arch <target> asm_ir/1`.

The runner `codespec-run-asm` (`adapters/asm-run`) reads the target from that line, generates a
driver as `asm_ir` (the `entry` function making each call of `calls.json` with literal arguments
and printing its result through the runtime, as `codespec-run-c` generates `driver.c`), writes it
with the same backend, assembles and links with the host's `as`/`clang` and `ld.lld`, and runs the
ELF natively or under `qemu-<target>`. Exit status as `codespec-run-c`: 0 printed, 2 refused,
1 failed.

The containers of `story:native-run-harness` replace the host tools when that story lands; the
runner's tool lookup is one function.

## 8. Checking

1. **Kit, against the evaluator.** `codespec conform --lang <target> --role writer` for each of
   the three targets: each claimed case's output byte-equal to `expected.txt`. A case the writer
   refuses is listed in its manifest's `not`.
2. **Backend against backend.** For every case all three claim, the three outputs are byte-equal.
   A case passing on two targets and failing on the third is a backend bug.
3. **Per-instruction corpus.** `crates/asm-ir/tests/ops/`: one `asm_ir` function per operation with
   boundary operands (`MIN / -1`, shift by 0, W-1, W, `-0.0`, NaN compares, `ext` of each width,
   immediates at each ISA's range edge, a frame larger than 4 KiB), its expected output written
   by hand; every backend runs it. It catches the immediate-range bugs the kit never reaches.
4. **An `asm_ir` interpreter** (later story) separates a lowering bug from a backend bug when all
   three targets fail alike.

Budget: one container or qemu call per case is about 10 ms for a static binary; the kit's 145
cases on three targets stay inside the 120 s step when the build of drivers is batched per target.
Not measured yet; the first backend story measures it.

## 9. What this changes in the plan

- `story:x86-64-writer` keeps its outcome (an x86-64 writer under `epic:ecmascript-to-native`,
  with the bench target and the container) and is written through `asm_ir/1`: its writer is
  `codespec-write-x86-64` of this design, its mapping section is § 7 here. Its dependency on the
  bench and the container stays; this design does not need either.
- `story:wasm-writer` is not changed: WasmGC is structured and garbage-collected and lowers from
  `cir/1` directly; going through `asm_ir/1` would throw away the structure it needs.
- `story:reader-asm` reads `cc -S -O0` output; this design's output is not in its table and
  nothing here depends on it.

## 10. Story plan

Under `epic:asm-ir-native-backends`:

| story | owns | depends on |
|---|---|---|
| `story:asm-ir-model` | `crates/asm-ir`: types, strict reading, canonical JSON, schema, a builder, a text printer; the skeletons and workspace registration of the other three crates | — |
| `story:asm-ir-lower-core` | `crates/lower`, `codespec lower`: group 1 (78 cases), the runtime of § 6 | model |
| `story:asm-x86-64-backend` | `adapters/asm` (x86-64 `emit`, all 41 operations), `codespec-run-asm --module`, the op corpus | model; beside lower-core, no shared file |
| `story:asm-x86-64-writer` | `codespec-write-x86-64`, the driver, `codespec-run-asm <dir> <calls.json>`, the x86-64 `conform` part: the first `cir/1` to ELF | lower-core, x86-64 backend |
| `story:asm-aarch64-backend` | `adapters/asm` (aarch64), its emulator in CI | x86-64 writer (shared files) |
| `story:asm-riscv64-backend` | `adapters/asm` (riscv64), its emulator in CI | aarch64 backend (shared files) |
| `story:asm-kit-conformance` | the AArch64 and RISC-V 64 writers and `conform` parts, the cross-check, the site's support matrix | x86-64 writer, riscv64 backend |
| `story:asm-ir-lower-slices-strings-errors` | groups 2 to 5 (33 cases, 111 in total) | x86-64 writer |
| `story:asm-ir-lower-floats` | group 6 (4 cases, 115), float printing | slices-strings-errors |
| `story:asm-ir-lower-maps` | group 7 (7 cases, 122) | floats |
| `story:asm-ir-interpreter` | § 8.4 | x86-64 writer |

## 11. Not decided here

- Register allocation; a C-ABI entry point; a fourth target. Each is behind the same IR.
- Whether `codespec-bench` uses this writer for its x86-64 rows: that is `story:x86-64-writer`'s.

## 12. Sources

Research notes of 2026-11-25, checked in beside this file under `docs/asm-ir/`:

- `ir-survey.md`: QBE IL, Cranelift CLIF, LLVM, Go SSA and `cmd/internal/obj`, WebAssembly, MIR;
  the recommendation this design takes, with its open points O1–O8.
- `x86-64-mapping.md`: the instruction table for x86-64 and its probes (all five passed; the
  integer probe 708,170 checks, the float probe 448,603, 0 failures each).
- `arm-rv-mapping.md`: the tables for AArch64 and RISC-V 64 and their probes under qemu-user
  (154 and 169 checks, all passing).
- `cir-semantics.md`: the evaluator's semantics the lowering keeps, with `file:line`, and the kit
  coverage per representation.
