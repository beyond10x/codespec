# asm_ir/1: survey of low-level IRs and a recommended shape

Scope: research only. Nothing in the repository was changed.

Provenance marks:

- **[read]** read this session, from the page or file named.
- **[local]** counted or read in a local clone (QBE from git.sr.ht/~mpu/qbe, MIR from github.com/vnmakarov/mir, both shallow clones in the session scratch directory) or in this repository.
- **[memory]** not read this session; recalled from training. Treat as unverified.

Sources actually read: QBE IL doc (c9x.me/compile/doc/il.html) and QBE source tree; MIR README and local clone (`mir.h`, `HOW-TO-PORT-MIR.md`, line counts); Cranelift `docs/ir.md`; Go `cmd/compile/internal/ssa/README.md`; Go `cmd/internal/obj/link.go`; the WebAssembly core spec instruction page; the first 100 kB of LLVM LangRef (only `byval`/`byref`/`sret` reached); this repository `AGENTS.md`, `docs/design.md`, `docs/evaluator.md`, `crates/model/src/lib.rs`, `conformance/c/codespec_rt.h`. The Go `obj/doc.go` URL returned 404.

No time estimates anywhere in this file (standing rule). Sizes are lines of code only.

---

## 0. What the repository gives us

[local] Facts about `cir/1` that constrain `asm_ir/1`:

| fact | consequence for asm_ir |
|---|---|
| structured control flow only (`if`, `for`, `for_range`, `switch`, `break`, `continue`, `return`) | lowering produces a CFG by one recursive walk; no irreducible flow |
| mutable locals (`let`, `var`, `assign`, `incr`) | producing SSA needs phi insertion; producing mutable vregs does not |
| `int {width: platform\|8\|16\|32\|64, signed}`, `float {32\|64}`, `bool` | 4 machine value classes suffice (i32, i64, f32, f64); narrow ints and `bool` are normalised inside an i32/i64 |
| `string`, `slice`, `error`, `any`, `map`, `tuple`, `struct`, `array`, `func`, interfaces | all multi-word; none is a machine type |
| multi-result functions (`results: Vec<Type>`, `let` with several names) | calls need N scalar results |
| `lambda {captures}`, `func_ref`, `call_value`, `iface_call`, `to_iface` | need indirect call and function/data addresses |
| `sem.int.wrap`, `sem.rem.sign`, Go division and shift rules, run-time panics | the IR defines div/rem/shift edge cases once; a panic is `exit 3` in the C writer runtime (`codespec_rt.h`: `cs_panic` prints `panic: <msg>` to stderr, exit status 3) |
| `foreign`, `raise`, goroutines, `defer`, type switches | lowering refuses them (cir/1 refusal discipline); asm_ir has no unwinding, no threads |
| the kit has 135 cases (`conformance/cases/*`: `case.cir.json`, `calls.json`, `expected.txt`); the evaluator prints `expected.txt` on all of them (`docs/evaluator.md` § 3) | the oracle for lowering exists; asm_ir needs only its own checks, § 8 |
| determinism: ordered collections only, no maps, canonical JSON, strict reading | asm_ir copies cir/1 model discipline (serde `deny_unknown_fields`, `k` tag, schema generated from the Rust types) |

---

## 1. Survey

### 1.1 QBE IL  [read: il.html; local: source tree]

| axis | QBE |
|---|---|
| value model | typed temporaries `%t`. SSA is *allowed*, not required: "Frontends need not emit SSA. Options: use `alloc` slots with load/store, or reassign temps freely; QBE repairs non-SSA input." `phi` exists at block heads. |
| types | base `w` (i32), `l` (i64), `s` (f32), `d` (f64); no pointer type (`l`). Temporaries are base-typed only. `b`/`h` (8/16 bit) exist only in aggregates/data and as sub-word ABI types (`sb ub sh uh`) on params and returns. `l` is usable where `w` is expected. |
| calls/ABI | `call $f(w %a, l %b, ..., ...)`: every call carries full ABI types, no prototypes. `env` param for closures; `...` marks variadic. Real C ABI per target (SysV amd64, arm64, rv64) in its own pass, because QBE is meant as a C backend. |
| aggregates | `type :N = {w, l, ...}` (struct, union, opaque, with alignment). May be passed and returned *by value in the IL signature* (`:N %x`); the backend classifies. Inside a function an aggregate is only memory: `alloc4/8/16`, `blit` (constant byte count), loads and stores. Temporaries are never aggregates. |
| control flow | blocks `@L`, phis first, one terminator: `jmp`, `jnz` (word test), `ret`, `hlt`. Entry block cannot be a jump target. No switch. |
| memory ops | `storeb/h/w/l/s/d`; `loadw/l/s/d`, `loadsb/ub/sh/uh/sw/uw`; `blit`; `alloc4/8/16`. Address is a plain `l` temp (no base+offset form). |
| arithmetic | `add sub mul div neg udiv rem urem and or xor sar shr shl`; shift count is `w`, "taken modulo result width"; signed `div` truncates, `rem` takes the dividend sign; no trap semantics stated. Comparisons `c<cc><type>` return 0/1. Conversions `extsw extuw extsh extuh extsb extub exts truncd stosi stoui dtosi dtoui swtof uwtof sltof ultof cast copy`. No integer truncation op (subtyping). |
| left to backend | everything below the IL: SSA repair, copy elimination, GVN/GCM, instruction selection, ABI lowering, spilling, register allocation, frame layout, emission. |
| backend size [local, lines of C] | amd64: `sysv.c` 721 + `isel.c` 942 + `emit.c` 726 + `targ.c` 47 = 2,436. arm64: `abi.c` 852 + `isel.c` 316 + `emit.c` 679 + `targ.c` 69 = 1,916. rv64: `abi.c` 653 + `isel.c` 255 + `emit.c` 569 + `targ.c` 57 = 1,534. Shared, not counted above: `parse.c` 1,433, `rega.c` 696, `spill.c` 531, `ssa.c` 433, `cfg.c` 567, `gvn.c` 508, `mem.c` 488, `load.c` 493, `gcm.c` 460, `copy.c` 408 and more. |

Lessons: (1) the 4-type scalar model (w/l/s/d) with sub-word types only in memory and ABI is the smallest model that works, and three backends use it. (2) Non-SSA input from frontends is an explicit, working design. (3) The ABI file is 30 percent (amd64), 44 percent (arm64) and 43 percent (rv64) of each backend lines [computed from the counts above], and each backend sits on a shared SSA register allocator. A private convention with no aggregate classification and a spill-everything scheme removes both. This is an inference from line counts, not a measurement of a stripped backend.

### 1.2 Cranelift CLIF  [read: docs/ir.md; sizes from memory]

| axis | CLIF |
|---|---|
| value model | SSA, **block parameters instead of phis**; entry block params are the function params. Mutable locals: the `cranelift_frontend` `Variable` API does SSA construction, or explicit stack slots when the address is taken. |
| types | `i8 i16 i32 i64 i128`, `f32 f64`, vectors; signedness per instruction, not per type. Some instructions or backends reject i8/i16. |
| calls/ABI | `(params) -> rets callconv`; params can carry `uext`/`sext` and a special kind: `sarg(N)` (by-value struct passed as pointer to N bytes), `sret` (return pointer), `vmctx`. Conventions `fast cold system_v windows_fastcall apple_aarch64 ...`; "actual call behavior is determined only by the (TargetIsa, CallConv) pair". The *frontend* decides struct passing (`sarg`/`sret`), the backend only places it. |
| aggregates | none in the IR; `explicit_slot Bytes` stack slots are untyped byte ranges; struct args are `sarg` pointers. |
| control flow | blocks with terminators; `br`, `brif` (two targets), `br_table v, default, [targets]`, `trap`, `return`. No fallthrough. |
| memory | `load/store` with flags `notrap aligned readonly volatile`; extending loads and truncating stores for 8/16/32; `stack_addr`/`stack_load`/`stack_store`; unaddressable access is UB. |
| traps | `trap` is a terminator; some instructions trap (`udiv` by zero); "a trap terminates the thread, behaviour OS/ISA specific, often a signal". Shifts: only the low B bits of the count matter (masked). The page does not list `sdiv` trap conditions [read]; that `sdiv INT_MIN/-1` traps is [memory]. |
| left to backend | register allocation, legalization, ABI lowering with prologue and epilogue, instruction selection (ISLE rules). |
| backend size | not on the page. [memory] each of x64 / aarch64 / riscv64 is several thousand lines of Rust plus ISLE rule files, larger than QBE. Not usable as a size target. |

Lessons: (1) per-instruction signedness, masked shifts and explicit traps are the design points we need; (2) `sarg`/`sret` show the clean split: the producer lowers aggregates to pointers, the backend places scalars only; (3) block params are an SSA device we do not need if we are not SSA.

### 1.3 LLVM IR and MachineIR  [partly read; mostly memory]

| axis | LLVM |
|---|---|
| value model | SSA with `phi`; `alloca` + load/store for mutable locals, then `mem2reg`. [memory] |
| types | `iN` for any N, `half float double fp128`, `ptr` (opaque), vectors, first-class structs and arrays. [memory] |
| calls/ABI | the frontend (clang) classifies and emits target-specific IR: `byval(ty)` (hidden copy made by the caller, pointer passed), `byref`, `sret(ty)` (pointer to the return struct, function returns void), `signext/zeroext`, `inreg`, coerced types (struct passed as `{i64,i64}`). [read for the byval/byref/sret text; the rest memory] |
| aggregates | first-class, but production frontends split or coerce them first, because backends do not do full classification. [memory] |
| control flow | blocks and terminators: `br`, `switch`, `ret`, `unreachable`, `invoke`. [memory] |
| undefined behaviour | `udiv/sdiv/urem/srem` by zero and `sdiv INT_MIN, -1` are UB; `shl/lshr/ashr` with count >= width give poison; `nsw/nuw` turn overflow into poison. [memory] Cautionary: IR-level UB forces every producer to guard and every consumer to guess. |
| lower levels | SelectionDAG/GlobalISel produce MachineIR: near-assembly, virtual registers then physical. TableGen plus C++ per target; backends are tens of thousands of lines. [memory] |

Lessons: take the *byval/sret* idea (aggregates become pointers plus explicit copies made by the producer); avoid poison and UB; do not copy the open-ended `iN`.

### 1.4 Go: `cmd/compile` SSA and `cmd/internal/obj`  [read: ssa README, obj link.go]

| axis | Go |
|---|---|
| SSA (generic) | values defined once, with ID, Op, type, args, aux; typed by Go types plus a `memory` pseudo-type: **a `mem` value is threaded through every memory op** to order them. Block kinds `plain`, `if`, `exit` (return; control value is memory). Ops carry the width in the name (`Add8`, `Add64`); signedness also in the name (`Less64U`, `Rsh64Ux64`) [partly memory]. |
| lowering | the `lower` pass rewrites generic ops to machine ops by rewrite rules (`_gen/*.rules`), then register allocation, then `Prog` generation. [read] |
| ABI | not in the README [read]. [memory] Go internal ABI (ABIInternal, register-based since 1.17) is private to Go code and may change; assembly uses ABI0 (stack-based) through wrappers. A private convention. |
| `obj` | `Prog` = one machine instruction in a linked list: opcode `As`, operands `From`/`To` (+ `RestArgs`), `Reg`, `Scond`, `Pos`, `Spadj`; `Addr.Type` in REG, MEM, ADDR, CONST, FCONST, SCONST, BRANCH, INDIR, TEXTSIZE. One architecture-neutral instruction *container*, a small portable opcode set (CALL, JMP, TEXT, RET), and a 2048-slot opcode block per architecture. Per-arch hooks (`LinkArch`) supply encoding; shared code does symbols, relocations, output. `LSym` is a flat `pkg.name` namespace with kind, size, data bytes, relocations. [read] |
| size | not in the sources read. [memory] `obj/x86/asm6.go` about 5k lines, `obj/arm64/asm7.go` about 7k; these include the *machine-code encoder*, which we avoid by emitting assembly text. |

Lessons: (1) `obj` is a lower layer than we want: a neutral container with per-arch opcodes makes backend authors write instruction selection per arch. We want one level above it. (2) Explicit `mem` threading is an SSA device we do not need in a statement-ordered IR: instruction order within a block *is* the memory order. (3) A private internal ABI is a respectable precedent.

### 1.5 WebAssembly (stack machine)  [read: instruction page; semantics from memory]

| axis | Wasm |
|---|---|
| value model | operand stack plus mutable `local`s (non-SSA); locals typed `i32 i64 f32 f64`. |
| types | the 4 scalars (+ v128, refs); `i8/i16` only in loads/stores and `extend8_s/16_s/32_s`. Same set as QBE. [read] |
| ops | `add sub mul div_s div_u rem_s rem_u and or xor shl shr_s shr_u rotl rotr clz ctz popcnt eqz`, compares `eq ne lt_s lt_u ...`, `wrap`, `extend_s/u`, `trunc_f*_s/u` and `trunc_sat`, `convert`, `demote/promote`, `reinterpret`; loads `load8_s/u load16_s/u load32_s/u`, stores `store8/16/32`. [read] |
| semantics | shift count masked mod width; `div_s`/`div_u`/`rem` by zero trap; `div_s INT_MIN/-1` traps; float-to-int `trunc` traps out of range, `trunc_sat` saturates and maps NaN to 0. [memory] `unreachable` traps unconditionally. [read] |
| control | **structured**: `block loop if else br br_if br_table return`; `call`, `call_indirect` (typed). [read] |
| aggregates | none; linear memory; the producer lays out structs, a shadow stack in linear memory holds addressable locals. [memory] |
| size | small engines (interpreter or baseline compiler) are the usual reference; not measured here. |

Lessons: (1) mutable typed locals plus explicit loads/stores is a proven non-SSA scalar IR; (2) the producer owns struct layout and the shadow stack; (3) structured control suits a *stack* machine and a validator, but is not simpler for a register backend than a CFG, and cir/1 is already structured, so lowering flattens it in one walk. Wasm names what must trap instead of leaving it open; that list is the useful part.

### 1.6 MIR (vnmakarov/mir)  [read: README; local: source]

| axis | MIR |
|---|---|
| value model | **non-SSA** typed registers (locals and args); MIR builds SSA internally in its generator (Braun construction) and register-allocates. |
| types | register values: 64-bit integer, `float`, `double`, `long double`. Memory operands: `i8 u8 i16 u16 i32 u32 i64 u64 f d ld`, extended to 64 on load. 32-bit ops (`ADDS`, `DIVS`, ...) read only the low 32 bits; the upper half of the result is undefined. Int registers are 64-bit; 32-bit ops are separate opcodes. |
| calls | `proto` (signature), `call proto, func, res..., args...`; multiple results allowed (machine-defined combinations). ABI lowering in a "machinize" pass inside the generator. Block (aggregate) args: types `BLK`..`BLK+4` and `RBLK` (return block), carrying a size. |
| control | labels and jumps; `BEQ BNE BLT...` fused compare-and-branch, `SWITCH`, `RET`, `ALLOCA`. Overflow-setting `ADDO SUBO MULO` with `BO/BNO`. |
| instruction set | [local `mir.h`] `MOV FMOV DMOV LDMOV`, `EXT8/16/32 UEXT8/16/32`, `F2I D2I`, `ADDR`, `ADD ADDS FADD DADD`, `DIV DIVS UDIV UDIVS FDIV DDIV`, `MOD MODS UMOD UMODS`, `ADDO..UMULOS`, `BEQ..`, `CALL`, `SWITCH`, `RET`, `ALLOCA`. |
| backend size [local, lines of C] | generator per target: `mir-gen-x86_64.c` 3,217, `mir-gen-aarch64.c` 2,761, `mir-gen-riscv64.c` 3,007; common machine-dependent file (thunks, wrappers, interpreter shims) `mir-x86_64.c` 984, `mir-aarch64.c` 949, `mir-riscv64.c` 1,177. Shared generator `mir-gen.c` 10,085, core `mir.c` 7,056. README [read]: generator about 23.4k lines, interpreter 11.3k, c2mir 54.8k. `HOW-TO-PORT-MIR.md` [local] gives "about 3K C lines" of machine-dependent code per target. |
| left to backend | register allocation (priority-based linear scan on SSA; fast mode at -O0), ABI, isel, frame, thunks. |

Lessons: (1) **MIR is the closest precedent to our constraints**: non-SSA typed registers in, one backend file per target. Its per-target size is about 3k lines *with* an optimizing allocator and a full C ABI. (2) 64-bit-only integer registers with width-suffixed ops is the alternative to QBE w/l split; either works. We take w/l because it maps 1:1 onto `w`/`x` registers on arm64, `*w` instructions on riscv64 and 32-bit operand size on x86.

### 1.7 libFirm and others  [memory only]

- **libFirm**: sea-of-nodes SSA graph, modes (`Is Iu Ls Lu F D P b`), `Phi`, `Proj`, memory edge threaded like Go `mem`; real isel through a graph-matching generator. Heavier than we want. The pointer mode `P` is the one point worth noting; we do not take it.
- **harec (Hare) and other young compilers targeting QBE**: chosen because the IL is small and non-SSA input is accepted. Same lesson as 1.1.
- **chibicc, tcc**: naive C compilers emitting x86-64 assembly text, spill-everything or push/pop style. The right *shape* for our backends (a template per op, scratch registers, every value in a frame slot). Sizes not checked.
- **Plan 9 compilers, Go pre-SSA backend**: tree-based isel directly from a typed expression tree. Matches the shape of cir/1 more than a CFG, but gives no portability layer; rejected.

### 1.8 Comparison summary

| IR | value model | scalar types | aggregates in IR | calls | control | regalloc, ABI, isel, frame | backend lines |
|---|---|---|---|---|---|---|---|
| QBE | typed temps; SSA optional (non-SSA accepted) | w l s d (+b h in memory/ABI) | named aggregate types, by value in signature, backend classifies | full ABI types per call, C ABI | blocks + `jmp jnz ret hlt` | all in backend | 1.5k to 2.4k per target + shared passes [local] |
| CLIF | SSA, block params | i8..i128 f32 f64 | none (`sarg`/`sret` pointers) | signature with callconv + `sarg/sret/uext/sext` | blocks + `brif br_table trap` | all in backend | several k [memory] |
| LLVM | SSA, phi | iN, float, ptr | first-class, split by frontends; `byval/sret` | attributes | blocks + `switch unreachable` | all in backend | tens of k [memory] |
| Go SSA | SSA values + threaded `mem` | Go types | via memory ops | private ABI | blocks plain/if/exit | all in backend | n/a |
| Wasm | stack + mutable locals | i32 i64 f32 f64 | none | typed `call`/`call_indirect` | structured | engine | small engines [memory] |
| MIR | non-SSA typed regs (SSA internal) | i64 f d ld regs; 8/16/32 in memory | `BLK` args (size) | `proto`, multi-result | labels, fused cmp+branch, `SWITCH` | all in backend | about 3k per target [local] |

---

## 2. Recommendation overview

`asm_ir/1` is a **non-SSA, typed-virtual-register, basic-block, scalar-only** IR with explicit stack slots, a private calling convention, and no aggregates. It is QBE type set and Wasm locals in a MIR-shaped function. Data layout, aggregate copying, string/slice/map/interface representation, panic checks, Go integer edge cases and the runtime library live in the **lowering** (the runtime is written once, in asm_ir itself). Each backend is a template expander: every vreg lives in a frame slot, every instruction loads its operands into fixed scratch registers, computes, and stores. The backend hard parts are ISA-specific immediate and displacement ranges, x86 `div`/shift/`fcmp` constraints, and the three syscall conventions.


---

## 3. Point 1: value model

### Recommendation

| decision | choice |
|---|---|
| function body | list of basic blocks, block 0 is the entry; each block is a list of instructions and exactly one terminator |
| values | **mutable virtual registers**, assigned any number of times (non-SSA), declared per function with a type |
| vreg types | `i32`, `i64`, `f32`, `f64` only. No `i8`/`i16` vregs. Pointers are `i64`. `bool` is `i32` 0/1. |
| addressable storage | **explicit stack slots** `{size, align}` per function; `slot_addr` yields the address as an `i64` |
| memory | explicit `load`/`store` with a memory width and sign flag; address = vreg + constant offset |
| operands | a vreg or a constant (immediate) on any integer source operand; no memory operands |

### Why non-SSA

| criterion | SSA (CLIF, LLVM, Go) | non-SSA vregs (MIR input, QBE input, Wasm locals) |
|---|---|---|
| producing from structured cir/1 | `let x = ..; if c { x = .. }; use x` needs phi or block-param insertion (Braun et al. on the fly, or alloca + mem2reg): one more algorithm, with sealed-block or dominance bookkeeping, in the lowering | `x` is one vreg; `assign` is a computation or `mov` into it. No extra algorithm. `for` variables, `break`, `continue` need no repair |
| consuming naively (spill everything) | phis must be destructed into parallel copies (lost-copy and swap problems), in every backend or a shared pass | a vreg is a frame cell; no phi, no parallel copy, no critical-edge splitting |
| consuming with a register allocator later | easier | still possible: liveness over vregs is standard linear-scan input; MIR does it |
| validation | single definition + dominance | type check only; optionally a forward "definitely assigned" dataflow check |
| reference interpreter | needs phi handling on edges | a vreg array |
| main risk | wrong phi placement is a silent miscompile | an uninitialised read: avoided because lowering zero-initialises every declared variable (cir/1 `var` is already the zero value) |

SSA is better for optimizing; this first cut does not optimize. If a later optimizer wants SSA it can be an `asm_ir -> asm_ir` pass (and `asm_ir/2` only if phis or block params become part of the format).

### Why only i32/i64/f32/f64 for vregs (a deviation from the lean toward i8/i16/i32/i64)

- First-class `i8`/`i16` arithmetic forces every backend to implement correct 8/16-bit wrap after every op. x86 has partial-register ops, but arm64 and riscv64 have none (they need `sxtb`/`uxth` or `slli`+`srai` after each op): 3 backends times N ops of fix-up.
- With only i32/i64 vregs the *lowering* owns narrow-int semantics: it computes in i32 and applies `ext` (from 8 or 16) after each op whose result can leave the range (Go `uint8 + uint8` wraps). The invariant "a narrow value is kept sign- or zero-extended to its register width" is stated once, in the lowering. QBE (`extsb`, `extub`) and Wasm (`i32.extend8_s`) do this.
- `i8`/`i16` still appear: as *memory widths* of `load`/`store`, and as the `from` of `ext`.
- Pointers are `i64`. A distinct `ptr` type adds conversion noise (hashing, slice arithmetic) and nothing else, since all three targets are LP64. The module has a fixed `word: 64`; a reader refuses another value, leaving room for `asm_ir/2`.
- The i32/i64 split matches arm64 `w`/`x` registers, riscv64 `*w` instructions and x86 operand size: one template per op with a size parameter.

Go `int` (`platform` width) lowers to `i64`.

### Stack slots vs vregs: who decides

The lowering puts a scalar local in a **vreg** when its address is never taken. It puts a local in a **slot** when it is an aggregate (struct, array), when its address is taken (`&x`, a pointer-receiver method call on a value, capture by reference), or when it is a call-argument or return copy. This needs a syntactic scan for `&` and captures; cir/1 `lambda.captures` already lists captured locals. Heap allocation (`new`, escaping values) goes through the runtime `alloc`.

### Frame (backend side, not in the IR)

The backend computes: saved fp and return address, the slot area (each slot at its alignment), one 8-byte cell per vreg, the outgoing call area. Offsets are never in the document. A frame larger than the immediate range is the backend problem (materialise the offset in a scratch register). Risk: spill-everything frames are large, so deep recursion in a test program uses more stack than a real compiler would. Not measured.

---

## 4. Point 2: the instruction list

Notation: `d` destination vreg; `a`, `b` operands (vreg or immediate, immediate typed by the instruction `t`); `t` is `i32`/`i64` (integer ops) or `f32`/`f64` (float ops). Every instruction is one JSON object tagged `"k"`.

### 4.1 Constants and moves

| op | operands | semantics |
|---|---|---|
| `const` | `d:i32\|i64`, `imm` | `d = imm`. Signed integer in the type range (i32: -2^31..2^31-1; i64: any i64; an unsigned constant is written as its two-complement value) |
| `fconst` | `d:f32\|f64`, `bits` | `d` = the IEEE pattern `bits` (u32 for f32; u64 written as i64 for f64). Avoids decimal float spelling in the IR |
| `mov` | `d`, `a` | copy; `d` and `a` have the same type |

### 4.2 Integer arithmetic (`t` = `i32` or `i64`)

| op | semantics |
|---|---|
| `add sub mul` | two-complement, **wrap** modulo 2^N. One op serves signed and unsigned (identical bits). No overflow-trapping variant in v1 |
| `and or xor` | bitwise. `a &^ b` is `and a, (xor b, -1)` in lowering; neg is `sub 0, a`; not is `xor a, -1` (so no `neg`/`not` ops) |
| `sdiv srem` | signed, truncated toward zero; remainder takes the dividend sign (`sem.rem.sign`). **Divisor must be nonzero** (precondition; with zero the behaviour is unspecified and a backend may fault). `MIN / -1` is **defined**: quotient `MIN`, remainder `0` (Go). |
| `udiv urem` | unsigned; divisor must be nonzero |
| `shl shr sar` | `shl` left, `shr` logical right, `sar` arithmetic right. **Count = low log2(N) bits of `b`** (mod 32 or mod 64): masked, as all three ISAs do. Go semantics for count >= width are produced by the lowering (below) |

Division decision (trap semantics): the IR does **not** trap on divide-by-zero. The lowering emits `cmp.eq b, 0` and a `br` to a block that calls `rt.panic_divzero` before every `sdiv/srem/udiv/urem` whose divisor is not a nonzero constant. Reasons:

- x86 `idiv`/`div` raise SIGFPE on zero (and on `MIN/-1`), arm64 `sdiv` returns 0 and riscv64 `div` returns -1 for zero, so "what the hardware does" differs three ways. A precondition plus an explicit guard in the IR makes all three agree with no backend special case. [memory: instruction behaviours]
- the panic message and exit code (3) are runtime policy and live in one place.
- `MIN / -1` is defined in the IR, so the x86 backend emits one short fix-up (compare the divisor to -1; `neg` for the quotient, zero for the remainder) instead of the lowering guarding it; arm64 and riscv64 produce `MIN` and `0` natively. [memory: hardware behaviour]

Shift decision: masked primitives in the IR, Go semantics in the lowering.

- Go: for count >= width, `<<` and unsigned `>>` give 0, signed `>>` gives all sign bits; a negative signed count panics (`codespec_rt.h` `cs_shift_count`).
- Masked is native on all three ISAs (x86 `shl r, cl`, arm64 `lslv`, riscv64 `sll/sllw`), so each backend template is one instruction.
- Lowering for a **constant count** (the common case) decides statically: `count >= W` becomes `const 0` (or `sar x, W-1`). For a **variable count** the lowering emits `lt = cmp.ult count, W`, `m = sub 0, lt` (all ones when in range, else 0), then for `shl`/`shr`: `r = shl x, count; d = and r, m`; for `sar` clamp the count first, `c2 = or (and count, m), (and (W-1), (xor m, -1))`, then `d = sar x, c2`. No `select` is needed, so no backend needs `cmov`/`csel`.
- The alternative (Go semantics in the instruction) pushes a compare and a conditional into three backends (x86 `cmov`, arm64 `csel`, riscv64 a branch) and hides the rule from the per-op conformance corpus. Rejected for the first cut (open point O2).

### 4.3 Comparison

| op | operands | semantics |
|---|---|---|
| `cmp` | `cc`, `t`, `d:i32`, `a`, `b` | `d = 1` if the relation holds, else `0`. `cc` in `eq ne slt sle sgt sge ult ule ugt uge` (`gt`/`ge` are redundant with swapped operands; they cost the lowering nothing and the backend a table row) |
| `fcmp` | `cc`, `t:f32\|f64`, `d:i32`, `a`, `b` | IEEE: `cc` in `eq ne lt le gt ge`; **false if either operand is NaN**, except `ne`, which is true. Backend hard bits: x86 `ucomisd` + parity flag for `eq`/`ne`; arm64 condition codes `mi`/`ls`; riscv64 has `feq flt fle` natively (`ne` is `feq` + `xori`, `gt` swaps) [memory] |

No `select`/`cmov` and no fused compare-and-branch in v1: `cmp` + `br` is two templates and costs only speed. A fused `br_cmp` terminator can be added later, additively.

### 4.4 Conversions

| op | operands | semantics |
|---|---|---|
| `ext` | `d:i32\|i64`, `a`, `from: 8\|16\|32`, `signed` | sign- or zero-extend the low `from` bits of `a` to the width of `d`; `from` is at most the width of `a`. `ext` from 32 into an i64 `d` is the widening `i64 <- i32`. Bits above `from` fully defined |
| `trunc` | `d:i32`, `a:i64` | low 32 bits |
| `itof` | `d:f32\|f64`, `a:i32\|i64`, `signed` | integer to float, round to nearest even. Unsigned i64 to f64 is the awkward case on x86 (halving trick); backend burden, a known sequence |
| `ftoi` | `d:i32\|i64`, `a:f32\|f64`, `signed` | round toward zero. **Out-of-range and NaN: unspecified** in v1 (x86 gives `MIN`; arm64 and riscv64 saturate [memory]). No kit case may rely on it. Open point O3 |
| `fpromote` / `fdemote` | `f32 -> f64` / `f64 -> f32` | IEEE conversion, round to nearest even |
| `bitcast` | `d`, `a` | same-width reinterpretation `i32<->f32`, `i64<->f64` |

### 4.5 Float arithmetic (`t` = `f32` or `f64`)

`fadd fsub fmul fdiv`: IEEE, round to nearest even, no traps, no flags. `fneg`: sign-bit flip (not `0 - x`, which differs for `-0.0`). `fsqrt` and other `math` functions are not in v1 (no kit case needs them). Float support is a separable group: a backend may refuse a module whose `features` list names `float`, and say so (the refusal discipline of cir/1 writers).

### 4.6 Memory

| op | operands | semantics |
|---|---|---|
| `load` | `d`, `base`, `off`, `m`, `signed` | `m` in `i8 i16 i32 i64 f32 f64`. `d` is `i32`/`i64` for an integer `m`, `f32`/`f64` for a float `m`; a narrower `m` is extended per `signed` to the width of `d` (`i32` into `i64` allowed). `base` is an `i64` vreg, `off` a constant i32 |
| `store` | `base`, `off`, `v`, `m` | store the low `m` bits (or the float) of `v`; `v` is a vreg or an integer immediate |
| `slot_addr` | `d:i64`, `slot` | address of stack slot `slot` (index into the function `slots`), aligned as declared |
| `sym_addr` | `d:i64`, `sym`, `off` | address of a data object or function plus a constant offset (static, position-dependent executable; each backend uses its idiom: x86 `lea sym(%rip)`, arm64 `adrp`+`add`, riscv64 `auipc`+`addi` or `la`) |

Rules: little-endian on all three targets. The IR makes no promise about unaligned access: **the lowering keeps scalar accesses naturally aligned**. A nil dereference is checked in the lowering (`cmp.eq p, 0`, then `rt.panic_nil`), like division, not by a signal handler; an out-of-bounds slice index likewise.

There is no `blit`/`memcpy` instruction: the lowering emits inline `load`/`store` pairs for small constant sizes and calls `rt.memmove` (written in asm_ir) otherwise. This removes the one op that would need a per-ISA loop or a runtime call.

### 4.7 Calls

| op | operands | semantics |
|---|---|---|
| `call` | `ds: [vreg]`, `callee: sym`, `args: [operand]`, `sig` | direct call; `sig = {params:[t], results:[t]}` is carried **on the call** (QBE style), no prototype table. The validator checks it against the callee declaration. 0..N results |
| `call_ind` | `ds`, `fn: vreg i64`, `args`, `sig` | indirect call through a function address obtained with `sym_addr` or loaded from a closure or method table (function values, lambdas, interface dispatch). Type safety is the lowering; `sig` is for the backend |
| `syscall` | `d:i64`, `name`, `args: [operand]` | a Linux system call from a **closed list**: `write(fd, ptr, len)`, `read(fd, ptr, len)`, `exit(code)` (never returns; the lowering follows it with `trap`), `mmap(len)` (anonymous private read+write; returns the pointer or negative errno). Returns the raw kernel value (`-errno` on failure). See § 6 |

Calling convention: **backend-private, not part of the format.** All calls are asm_ir to asm_ir; the only entities outside are `_start` (emitted by the backend) and the kernel. Recommended first-cut convention (non-normative): every argument and result is an 8-byte cell in memory; arguments in the caller outgoing area at `[sp + 8*i]`, results written by the callee into cells the caller reserved just above the arguments; no argument registers, no callee-saved registers (every vreg is in the frame, so the callee may clobber anything). Cost: nothing is passed in registers; each call site is a load and a store per argument. Benefit: no ABI classification, the same logic on all three ISAs apart from `sp`/`fp` addressing and the call/return instruction (`call`/`ret`; `bl`/`ret` with `lr` saved; `jal`/`ret` with `ra` saved). Argument and result counts are unbounded. A float travels as its bit pattern in the cell.

### 4.8 Terminators

| op | operands | semantics |
|---|---|---|
| `jmp` | `to: block` | unconditional |
| `br` | `cond: vreg i32`, `then`, `else` | `cond != 0` goes to `then` |
| `ret` | `vals: [operand]` | return the function declared results, in order |
| `trap` | `code: u8` | terminate the process with exit status `code`, no message (the lowering prints first via `rt.panic`) |

No `switch` and no `unreachable`: a cir/1 `switch` is a chain of `cmp` + `br` (what each backend would emit for a table-less `switch` anyway), and `trap` serves as `unreachable`. A jump-table `switch` is a later additive optimisation.

### 4.9 Instruction count

Operation kinds: `const fconst mov` (3); integer binary `add sub mul and or xor sdiv udiv srem urem shl shr sar` (13); `cmp fcmp` (2); `ext trunc itof ftoi fpromote fdemote bitcast` (7); `fadd fsub fmul fdiv fneg` (5); `load store slot_addr sym_addr` (4); `call call_ind syscall` (3); terminators `jmp br ret trap` (4). Total 41. In JSON the binary ops can share one node kind with an `op` field, so the Rust model has about 20 variants; a backend has about 41 small templates.

### 4.10 Deliberately not in the IR

Phi, select, overflow-trapping arithmetic, rotates and bit counts, `fsqrt`, vectors, atomics, threads, `switch`/jump tables, `blit`, dynamic `alloca`, `va_arg`, TLS, aggregate types, a `ptr` type, exceptions and unwinding. Each can be added later if additive and guarded by the module `features` list (as cir/1 does with `features.rs`); a feature a backend does not claim is a named refusal.


---

## 5. Point 3: aggregates

### Recommendation: aggregates exist only in the lowering. asm_ir has no struct or array type, no aggregate parameter, no aggregate return.

| cir/1 construct | in asm_ir |
|---|---|
| struct, array value | bytes in a stack slot (or a heap block); the lowering computes size, alignment and field offsets (natural alignment, one fixed rule in the lowering crate) |
| field read/write | `slot_addr` or pointer + `load`/`store` at `off` |
| struct copy (`a = b`, passing, returning) | inline `load`/`store` pairs (up to about 64 bytes) or `call rt.memmove` |
| struct literal | evaluate into a destination address (destination-passing: `lower_into(expr, dst_ptr)`) on a zeroed slot |
| struct parameter | **passed as a pointer** (`i64`) to a copy the caller made: caller allocates a slot, copies, passes the address (the callee may mutate its copy, preserving Go value semantics). The idea of LLVM `byval`, CLIF `sarg`, QBE aggregate args, but written as an ordinary copy plus pointer, so no backend classifies anything |
| struct result | **sret**: the caller passes a destination pointer as a hidden first parameter; the function returns no value for it |
| `string` | 2 words `(ptr, len)`, **scalarised**: a string parameter is 2 `i64` params, a result 2 `i64` results, a local 2 vregs. `len` counts bytes (Go; the Java UTF-16 `len` is an open item in `docs/evaluator.md` § 3) |
| `slice` | 3 words `(ptr, len, cap)`, scalarised the same way |
| `error`, `any`, interface values | 2 words `(type descriptor ptr, data word)`, scalarised; the descriptor is a data object (method table for `iface_call` dispatch through `call_ind`, a type id for printing) |
| `tuple`, multi-result | N results |
| `func` value, closure | 2 words `(code ptr, env ptr)`; a `lambda` becomes a function taking `env` as an extra first parameter, captures live in a heap or slot record the env points to |
| `map` | an opaque pointer to a runtime-managed structure; operations are calls into the runtime (asm_ir, § 6); insertion order kept so `sem.map.order` and `sorted_keys` can be reproduced |
| named `enum`, `newtype` | the underlying type |
| `union` (tagged variants) | tag word + payload bytes sized by the largest variant |
| generics (`param`, `targs`) | the lowering monomorphises or refuses; asm_ir sees concrete types only |

Why in the lowering and not in the backends:

1. Per-arch ABI classification of structs (SysV eightbytes on amd64, HFA/HVA on arm64, flattened-float rules on riscv64) is the most error-prone and largest part of a real backend: 30 to 44 percent of each QBE backend by line count [local, § 1.1]. Our calls never leave the module, so there is no C ABI to honour; a private convention that never classifies aggregates removes this from all three backends permanently.
2. Layout is a *language-level* decision (Go layout, string header shape) that must be identical on all three targets for byte-equal output (the kit compares byte for byte). Doing it once in the lowering guarantees that.
3. An asm_ir without aggregates has a trivially checkable type system (4 scalar types).

Cost: more instructions in the output (copies are explicit) and no interop with C code taking structs by value. Neither matters for a non-optimizing first cut with no libc. If C interop is ever required, add an extern with `abi: "c"` restricted to scalar signatures (ints and pointers in registers); still no struct classification.

---

## 6. Point 4: runtime services

### Recommendation: a runtime written once **in asm_ir**, included in every module, whose only contact with the machine is the closed `syscall` instruction.

| service | expression |
|---|---|
| process start | the backend emits `_start`: align the stack, `call` the module `entry` function (no args, no results), then `exit(0)`. A few instructions per ISA; `entry` is named in the document |
| exit with a code | `syscall exit(code)` followed by `trap` |
| write to stdout/stderr | `syscall write(fd, ptr, len)`; `rt.write_all` loops on short writes (an asm_ir function) |
| allocation | `rt.alloc(n)`: bump allocator over chunks from `syscall mmap(len)`; **no free** (programs are short; non-optimizing, deterministic); memory is zero-filled (anonymous mmap), so `make`/`zero` need no memset on fresh blocks |
| panic | `rt.panic(msg_ptr, msg_len)`: `write(2, "panic: ")`, the message, newline, `exit(3)`. Status 3 matches `cs_panic` in the C writer runtime (`conformance/c/codespec_rt.h`). Wrappers `rt.panic_divzero`, `rt.panic_nil`, `rt.panic_index` hold constant messages in `data` |
| `trap code` | bare `exit(code)` for paths that already printed |
| printing (`println`, `sprintf` with `%d %s %v`, `errorf`) | asm_ir functions: `rt.fmt_i64`, `rt.fmt_u64`, `rt.write_str`, a string builder over `rt.alloc`; the same verbs the evaluator supports |
| strings | `rt.str_eq`, `rt.str_cmp` (byte order; the Java UTF-16 order for `sorted_keys` is a separate function), `rt.str_cat` |
| slices | `rt.append` (growth policy fixed and documented); index checks inline in lowered code |
| maps | `rt.map_*` over the key kinds in use (int, string, struct by bytes), insertion-ordered like `cs_map` in the C header and `MapStore` in the evaluator |
| memory | `rt.memmove`, `rt.memzero`, `rt.memeq` as byte or word loops |

Why asm_ir for the runtime (not hand-written assembly, a C runtime, or libc):

- Hand-written is 3 copies of N functions in 3 assembly dialects. In asm_ir it is one copy, and **every backend and the asm_ir interpreter run the same runtime**.
- The module is self-contained: nothing but `ld -static` on the assembler output. No libc, no dynamic loader, so the same binary shape runs under qemu-user on each ISA.
- The runtime is built by a Rust *builder* in the lowering crate (a function-builder API emitting the model types), not parsed from text, so the same validator checks it. Only functions reachable from `entry` are emitted, so output stays small and deterministic.
- `syscall` names the call (closed enum), not a number. Numbers differ per ISA: x86-64 `write`=1, `exit`=60, `mmap`=9; aarch64 and riscv64 use the generic table, `write`=64, `exit`=93, `mmap`=222 [memory]. Register conventions differ too: x86-64 `rax` (number), `rdi rsi rdx r10 r8 r9`, instruction `syscall`; aarch64 `x8`, `x0..x5`, `svc #0`; riscv64 `a7`, `a0..a5`, `ecall` [memory]. Each backend maps 4 names to numbers and emits one sequence: about 15 lines.
- A closed list (write, read, exit, mmap) means the interpreter implements exactly four. `getrandom` or `clock_gettime` later is additive.

The harness side: the kit runs a `calls.json` through a driver. The runner generates the `entry` function (as the C runner generates a C driver), in cir/1 or directly in asm_ir, calling the lowered functions and printing in the harness encoding through the runtime print functions. asm_ir knows nothing about `codespec-calls/1`.

Externs of cir/1 (`strings.ToUpper`, `fmt.*`, `os.*`): those the evaluator has a built-in for are implemented as runtime asm_ir functions where the kit needs them; any other extern is a **lowering refusal** (reported by the lowering, in the style of `foreign` plus `refusals`), not an `externs` entry in the document (§ 7). Same rule as evaluator E4: never past the unknown.

---

## 7. Point 5: the JSON document

### Recommendation: one canonical JSON document `asm_ir/1` per whole program (program + runtime), defined by Rust types in a new crate, strict reading, no maps.

Conventions copied from cir/1 [local: `crates/model/src/lib.rs`, `AGENTS.md`]: `#[serde(deny_unknown_fields)]`, tagged nodes with field `k` in snake_case, ordered `Vec`s only, canonical encoding (fixed field order), a generated JSON Schema (`schemas/asm-ir-1.schema.json`) drift-checked, any other format string refused.

```text
module {
  format:   "asm_ir/1"
  module:   string                      name (same identifier rules as cir/1)
  word:     64                          pointer width in bits; reading refuses anything else
  from:     { format: "cir/1", module: string, sha256: string }   provenance, like cir_sha256 of codespec-spans/1
  entry:    string                      a func with params [] and results []
  features: [string]                    sorted; "float" present iff any f32/f64 vreg or op
  data:     [ data ]                    lowering order, unique names
  funcs:    [ func ]                    reachability order from entry, unique names
}

data {
  name:    string                       unique across data and funcs
  align:   1 | 2 | 4 | 8 | 16
  mutable: bool                         false: read-only (string literals); true: writable (globals)
  items:   [ item ]
}
item = { k:"bytes", hex:string }        lowercase hex, even length
     | { k:"zero",  n:u64 }
     | { k:"addr",  sym:string, off:i64 }   one 8-byte absolute pointer; needs a relocation

func {
  name:    string
  params:  [ "i32"|"i64"|"f32"|"f64" ]   vregs 0..len(params)-1 receive them, in order
  results: [ type ]
  vregs:   [ type ]                      index = id; the first len(params) entries equal params
  slots:   [ { size:u32, align:1|2|4|8|16 } ]
  blocks:  [ block ]                     block 0 = entry; ids are indices
  origin:  string                        optional, skipped when empty: the cir/1 function key
}

block {
  insns: [ insn ]
  term:  terminator
  at:    string                          optional, skipped when empty: cq node id of the cir/1 statement
}

operand = { k:"v", v:u32 }              a vreg id
        | { k:"i", i:i64 }              an immediate

insn examples
  { k:"const", d:3, imm:-1 }
  { k:"bin",  op:"add", t:"i64", d:5, a:{k:"v",v:3}, b:{k:"i",i:8} }
  { k:"cmp",  cc:"slt", t:"i32", d:6, a:{k:"v",v:1}, b:{k:"v",v:2} }
  { k:"load", d:7, base:4, off:16, m:"i64", signed:false }
  { k:"store", base:4, off:0, v:{k:"v",v:7}, m:"i32" }
  { k:"slot_addr", d:8, slot:0 }
  { k:"sym_addr",  d:9, sym:"str.3", off:0 }
  { k:"call", ds:[10,11], callee:"rt.str_cat", args:[...], sig:{params:[...], results:[...]} }
  { k:"syscall", d:12, name:"write", args:[...] }
terminator
  { k:"jmp", to:2 } | { k:"br", cond:6, then:3, else:4 } | { k:"ret", vals:[...] } | { k:"trap", code:3 }
```

What strict reading enforces (the asm_ir analogue of the checks in cir/1 `Module::from_json`):

- every vreg id, slot id, block id and symbol exists; names are unique across `data` and `funcs`;
- each instruction operand types match its `t` and the vreg table; each immediate is in range of its type;
- each block has exactly one terminator; block 0 is not a jump target (the QBE rule; it simplifies the prologue);
- `call` arity and types agree with the callee declaration; `call_ind` is checked only against its own `sig`;
- `ret` matches `results`; `entry` exists with the required signature;
- `load`/`store` `m` is compatible with the `d`/`v` type; `ext.from` is within the width; `syscall` name is in the closed list and its arity is right;
- `features` matches use; `word` is 64;
- text fields (`name`, `sym`, `hex`) free of control and format characters (the cir/1 rule);
- optional, cheap, catches lowering bugs early: every vreg read is assigned on every path from entry (forward dataflow).

Why `data` carries relocatable items: string headers, method tables, type descriptors and global initialisers hold pointers to other symbols; an `addr` item avoids a run-time initialiser for static data. A global whose initialiser is not constant becomes a lowering-generated `init` function called from `entry` before user code.

Why vregs, slots and blocks are indices, not names: smaller, unambiguous, no name-collision rules, determinism by construction (the lowering counters). A text dump (`codespec asm-ir print`, to be written) prints `v3`, `slot0`, `b2` for humans. `origin` and `at` bridge back to cir/1 for diagnostics and are not semantics; skipped when empty, so a runtime function costs nothing.

Immediates and the JSON number range: immediates are JSON integers in the signed i64 range. The Rust reader handles all of i64/u64 exactly; a JavaScript consumer loses precision above 2^53. If any non-Rust tool will read asm_ir, switch immediates and `bits` to decimal strings before `asm_ir/1` is frozen; this is the one encoding decision that cannot change afterwards (open point O4).

`externs`: **not in `asm_ir/1`.** An `externs` list matters only if a later stage links against something outside the document. Here every callee is defined in the document (program plus runtime) and the only outside entity is the kernel, reached by the closed `syscall` list. A cir extern with no runtime implementation is refused by the lowering and reported there. If a link step appears later, `externs: [{name, params, results}]` would be a change a strict `asm_ir/1` reader refuses, so strictly `asm_ir/2`; choose now whether to reserve the field as always empty (open point O5).

Suggested placement (not required): `crates/asm_ir` (types, strict reading, canonical JSON, schema, validator, reference interpreter), `crates/lower` (cir/1 to asm_ir plus the runtime builder), `adapters/<isa>-asm` (one backend per ISA; reads asm_ir on stdin, prints assembly: the same adapter boundary as every writer).

---

## 8. Backends: what they do and how they are checked

### Template (all three ISAs)

Per function: prologue (save fp and return address, reserve the frame); `blocks` in document order with labels `.L<func>_<block>`; an epilogue per `ret`. Per instruction: load operands into scratch registers, op, store to the vreg cell. Scratch registers fixed, e.g. x86-64 `rax rcx rdx r10 r11` + `xmm0 xmm1`; aarch64 `x9..x13` + `d16 d17`; riscv64 `t0..t3` + `ft0 ft1` [memory: all caller-saved, so free to clobber in a spill-everything scheme].

### What each ISA makes non-trivial

| ISA | non-trivial pieces |
|---|---|
| x86-64 | `idiv/div` use `rax:rdx` and fault on `MIN/-1` (the `-1` fix-up of § 4.2); variable shifts need the count in `cl`; `fcmp` parity; unsigned i64 to f64; `ext` via `movsx/movzx`, 32-bit ops zero the upper half; `sym_addr` RIP-relative; frame displacement is 32-bit (no range problem) |
| aarch64 | immediate ranges: `add/sub` 12-bit (optionally shifted), `ldr/str` 12-bit scaled or 9-bit unscaled, logical immediates bitmask-encoded (materialise with `movz/movk` in a scratch register unless trivially encodable); `adrp`+`add` for `sym_addr`; 32-bit `w` ops zero the upper half; `msub` for `rem`; `fcvtzs/fcvtzu` saturate (fine: out-of-range unspecified); `sp` must stay 16-byte aligned at every `sp`-based access |
| riscv64 | 12-bit immediates (`lui`+`addi`, the `li` pseudo handles it); `*w` instructions sign-extend 32-bit results; `div/rem` native with `MIN/-1` giving `MIN`/0; no condition flags (`slt/sltu/xor+sltiu`); `fcmp` native `feq flt fle`; unsigned int-to-float native `fcvt.d.lu`; `ecall` [memory for all three rows] |

### Size

No backend was written; this is judgement, not measurement. QBE per-target backends are 1.5k to 2.4k lines *with* real ABI classification and a shared SSA register allocator behind them [local]; MIR generators are about 3k per target *with* an optimizing allocator [local]. Removing aggregate ABI (§ 5), the register allocator and instruction selection leaves 41 operation templates, constant and offset materialisation, the frame, the syscall mapping and `_start`. That fits the "few hundred to about 2000 lines of Rust" target; the immediate-range helpers (aarch64, riscv64) and the x86 special cases are where lines accumulate. A register allocator added later sits behind the same IR.

### Checking: three comparisons, each isolating one stage

1. **Lowering vs evaluator.** A new asm_ir interpreter (in `crates/asm_ir`: a vreg array, slot memory, flat byte memory, the 4 syscalls simulated) runs the lowered module; its output must equal the cir/1 evaluator output and the kit `expected.txt` on every case the lowering claims (the shape of `eval-kit` and `eval-diff`). A failure here is a lowering or runtime bug.
2. **Backend vs interpreter.** The assembly runs under `qemu-x86_64`, `qemu-aarch64`, `qemu-riscv64` and must equal the asm_ir interpreter output byte for byte, exit status included. A failure here is a backend bug and cannot be a lowering bug, because both read the same asm_ir document. This is the main reason to build the interpreter: without it a failing qemu run does not say which stage broke.
3. **Per-instruction conformance corpus for backends.** A small generated asm_ir set, one function per op with boundary operands (`MIN/-1`, shifts by 0, W-1 and W via the masked form, `-0.0`, NaN compares, `ext` of each width, immediates at each ISA range edge), run by interpreter and qemu. About 41 ops times a few operands; it catches immediate-range bugs the kit never hits.

Deterministic output: a backend iterates `data` and `funcs` in document order, never a hash map; labels derive from function and block indices; no timestamps or paths in the assembly.

---

## 9. Open points (decided here, reversible unless marked)

| id | question | default taken here | reversibility |
|---|---|---|---|
| O1 | `i8`/`i16` as vreg types | no; memory widths and `ext` sources only | adding vreg types later is additive if `features` guards it |
| O2 | shift semantics | masked in the IR, Go semantics in the lowering | switching to Go semantics in the IR changes meaning: `asm_ir/2`; decide before freezing |
| O3 | `ftoi` out of range and NaN | unspecified; no kit case depends on it | defining it later (saturate, NaN to 0, like `trunc_sat`) only narrows an unspecified region; x86 then needs a fix-up |
| O4 | immediates as JSON integers or strings | integers (Rust consumers only) | **not reversible after freeze** without `asm_ir/2`; decide now if a non-Rust tool will read it |
| O5 | reserve an `externs` field | omit | adding it later is `asm_ir/2` strictly; reserving it empty costs one field |
| O6 | block and vreg numbering | allocation order of the lowering single recursive walk | numbering is output, fixed per lowering version; a change that renumbers changes bytes, as a writer change does today |
| O7 | allocator never frees | bump, no free | a runtime function, replaceable without touching the format |
| O8 | no `switch`, `select`, fused compare-and-branch | omit | all additive later |

## 10. Risks

- **Frame size and recursion depth.** Spill-everything with a cell per vreg and no slot reuse makes frames large; a recursive kit case could exhaust the guest stack under qemu-user. Mitigations: the lowering reuses vregs of dead expression temporaries (a simple pool), or the runner raises the stack limit. Not measured.
- **Runtime surface.** Maps, `sprintf`, `append` growth and string ordering (UTF-16 vs bytes for `sorted_keys`) all live in asm_ir. This is real work in the lowering crate, not in the backends, and is the largest single item. Staging suggestion: ints, bools, strings, structs, slices and `println` first; maps, closures and interfaces later; refuse until then.
- **Float printing** (`%v` of a float) needs a shortest-round-trip algorithm in asm_ir; refuse floats in printing in v1 or print integers only.
- **qemu-user fidelity.** Assumed to implement the Linux syscalls used (write, exit, mmap) on all three ISAs [memory]; verify with the first backend before relying on it.
- **Unverified claims.** All `[memory]` entries (LLVM UB list, instruction edge behaviour, syscall numbers and register conventions, Cranelift and Go backend sizes, libFirm, chibicc, qemu behaviour) were not checked against a source this session. The QBE, MIR, Cranelift `ir.md`, Go SSA README, `obj/link.go` and Wasm instruction-page facts marked [read] or [local] were.

