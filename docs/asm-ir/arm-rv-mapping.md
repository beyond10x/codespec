# asm_ir/1 backend mapping: AArch64 (AAPCS64, Linux) and RISC-V 64 (RV64GC, LP64D, Linux)

Host x86_64, clang 23.1.1, ld.lld 23.1.1, qemu-aarch64/qemu-riscv64 11.1.2. All sequences below were assembled with `clang --target=...`, linked `ld.lld -static -e _start`, run under qemu-user, unless marked (doc) = from the ABI documents or clang -S output, not executed. Probe sources: a session scratch directory, not kept; the op corpus of `docs/asm-ir.md` § 8 replaces them.

## 0. Probes: commands and actual output

Every probe is `./build.sh <a64|rv> <name>` in the probe directory, which runs `clang --target=<aarch64|riscv64>-linux-gnu -c N.s -o N.o && ld.lld -static -e _start N.o -o N.elf && qemu-<aarch64|riscv64> ./N.elf; echo exit=$?`. `./run_all.sh` repeats all of them.

| probe | what | actual output |
|---|---|---|
| `a64_ok` | _start: write "ok 5\n", exit_group(0) | `ok 5` / `exit=0` |
| `rv_ok` | same, RV64 | `ok 5` / `exit=0` |
| `a64_ops` | 154 self-checks: every AArch64 row below with edge values (wrap, MinInt, count 64/200/2^32, NaN, saturation, big offsets, 10 int args, 9 double args, callee-saved, mmap/munmap, EBADF, EINVAL). A mutated expected value prints `FAIL: <name>` and exits 1 (verified the checker can fail) | `ok: 154` / `exit=0` |
| `rv_ops` | 169 self-checks, RV64, same coverage | `ok: 169` / `exit=0` |
| `a64_div` | guarded `go_sdiv64`/`go_srem64`: MinInt/-1 = MinInt, MinInt%-1 = 0, then 7/0 -> explicit `cbz` -> exit_group(3) | `minint/-1 ok, rem 0` / `exit=3` |
| `rv_div` | same, `beqz a1, trap` | `minint/-1 ok, rem 0` / `exit=3` |
| `a64_brk`, `rv_brk` | `brk #3` / `ebreak` as trap | `qemu: uncaught target signal 5 (Trace/breakpoint trap)`, shell `exit=133` (SIGTRAP) |
| `a64_align` | `sub sp,sp,#8; str x0,[sp]` (misaligned sp) | `exit=0`: qemu-user does NOT fault; real AArch64 EL0 does (SCTLR.SA0), do not rely on the emulator |
| `rv_align` | `ld` at odd address | `exit=0`: qemu-user allows misaligned; real RV hardware may trap or be slow |
| Go shift count 64 | `go_shl64_c64`, `go_shr64_c64`, `go_sar64_c64`, `go_shl32_c32`, ... inside `a64_ops`/`rv_ops`; raw `lsl x,x,#64` / `sll` masks to count 0 (checks `raw_lsl64_masks_to_1`, `raw_sll64_masks_to_1` pass) | included in the 154 / 169 |
| NaN == NaN | `nan_eq`, `nan_eq_f32` (fcmp+cset eq / feq.d, feq.s) = 0; `nan_ne` = 1 | included |
| `x86_probe` | host x86-64 contrast: cvttsd2si NaN and 1e30 = 0x8000000000000000; `ucomisd nan,nan; sete` = 1; `shl eax,64`(count 64 masks to 0) = 1; `idiv` MinInt/-1 -> SIGFPE | `exit=136` (SIGFPE reached only after the first four checks passed) |
| `rv_gp*` | gp relaxation: ld.lld default does not use gp; `ld.lld --relax-gp` rewrites `lui %hi/%lo` into `ld t1,-0x800(gp)`, which segfaults when _start did not set gp (exit 139) | see section 3.8 |
| `rv_far`, `rv_far2` | `beq` 8000 bytes away, same section and other section | both assembled+ran (`exit=0`): the LLVM assembler rewrites it to `bne +8; j far` itself |
| `a64_far`, `a64_far2` | `b.eq` / `cbz` 2.1 MB away | `error: fixup value out of range` (no auto-relaxation on AArch64) |
| `abi.c` -> `abi_*.s` | `clang -S -O1` of structs/narrow ints/9 args: ground truth for section 4 | see section 4 |

## 1. Conventions used in the tables

- Operand width W in {8,16,32,64}. Narrow IR values (i8/i16/i32) in registers have undefined upper bits; every op that depends on them extends first. Stores write exactly W bits; loads pick a zero- or sign-extending form.
- Naive scheme: every vreg lives in a stack slot of its own size (slot offset `o`). Binary op = load A, load B, op, store R. Load/store of a vreg: AArch64 `ldrb/ldrh/ldr w/ldr x/ldr s/ldr d` `[sp,#o]`, RV `lbu/lhu/lwu or lw/ld/flw/fld` `o(sp)`.
- AArch64 scratch: A=`x9`/`w9`, B=`x10`/`w10`, T=`x11`, T2=`x12`, address temps `x16`,`x17` (IP0/IP1, linker veneers may clobber them only across a call), FP `d16`,`d17`,`d18`. Never touch x18 (platform register), x29 (fp), x30 (lr), x8 except for syscall number / indirect result. Callee-saved x19-x28 and d8-d15 are never used by the naive backend (no save needed).
- RV64 scratch: A=`t0`, B=`t1`, R=`t2`, T=`t3`,`t4`, spare `t5`,`t6`; FP `ft0`,`ft1`,`ft2`. Never touch gp(x3), tp(x4), s0 (frame pointer), ra, sp. s1-s11, fs0-fs11 unused by the naive backend.
- Shift counts are unsigned and must already be zero-extended to 64 bits (use the count type extension row first).
- `Lx` = a unique local label per emission site (never numeric `1f` across macro expansions).

## 2. AArch64 mapping (all rows probed in `a64_ops` unless (doc))

### 2.1 Integer arithmetic, extension

| IR op | sequence (A=w9/x9, B=w10/x10, R=A) | notes |
|---|---|---|
| add/sub/mul i64 | `add x9,x9,x10` / `sub x9,x9,x10` / `mul x9,x9,x10` | wraps |
| add/sub/mul i32 | `add w9,w9,w10` / `sub w9,w9,w10` / `mul w9,w9,w10` | writing w zeroes bits 63:32 |
| add/sub/mul i8, i16 | same `w` forms; result upper bits garbage; no extend needed at the op | extend when consumed: next rows. `0x7f+1` as i8 = 0x80 -> `sxtb x9,w9` = -128 (probed) |
| neg iN | `neg x9,x9` (64) / `neg w9,w9` (8/16/32) | neg(0x80000000) = 0x80000000 (probed) |
| sext i8->i16/32 | `sxtb w9,w9` | i8->i64: `sxtb x9,w9` |
| sext i16->i32/64 | `sxth w9,w9` / `sxth x9,w9` | |
| sext i32->i64 | `sxtw x9,w9` | |
| zext i8/i16 -> any | `uxtb w9,w9` / `uxth w9,w9` | `and w9,w9,#0xff` equivalent |
| zext i32->i64 | `mov w9,w9` (alias `uxtw x9,w9`) | |
| trunc to i8/i16/i32 | none (use low bits); i32 from i64: `mov w9,w9` only if upper bits must be clean | |
| mul-high (extra) | `umulh x9,x9,x10` / `smulh x9,x9,x10` | (doc) |
| const i32 | `mov w9,#imm` if encodable, else `movz w9,#lo16` + `movk w9,#hi16,lsl #16` | |
| const i64 | `movz x9,#h0` then `movk x9,#h1,lsl #16`, `#h2,lsl #32`, `#h3,lsl #48` (skip zero halves; `movn` for mostly-ones) | `mov x9,#imm` assembles ONLY for one-instruction encodings: `mov x16,#80000` and `mov x0,#0x123456789abcdef0` are assembler errors (probed). Avoid `ldr x9,=imm` (needs literal pool/.ltorg within 1 MiB) |

### 2.2 Division and remainder (Go semantics)

Hardware: sdiv/udiv never trap; x/0 = 0; MinInt/-1 = MinInt (probed `sdiv64_minint`, `sdiv32_minint`); `msub` remainder: MinInt%-1 = 0, x%0 = x (probed). So only the zero check is needed; no MinInt/-1 special case on AArch64.

| IR op | sequence | notes |
|---|---|---|
| sdiv i64 | `cbz x10,Ltrap` ; `sdiv x9,x9,x10` | Ltrap shared per function: `brk #1` (SIGTRAP, shell exit 133) or `mov x0,#3 ; mov x8,#94 ; svc #0` (exit 3, probed) |
| srem i64 | `cbz x10,Ltrap` ; `sdiv x11,x9,x10` ; `msub x9,x11,x10,x9` | msub: x9 - x11*x10 |
| udiv/urem i64 | `cbz x10,Ltrap` ; `udiv x9,x9,x10` / `udiv x11,x9,x10 ; msub x9,x11,x10,x9` | |
| sdiv/srem i32 | w forms of the above | |
| sdiv/srem i8, i16 | `sxtb w9,w9 ; sxtb w10,w10` (`sxth`) ; `cbz w10,Ltrap` ; `sdiv w11,w9,w10` ; `msub w12,w11,w10,w9` ; result low bits | -128/-1 = +128 in 32 bits, truncates to 0x80 = -128 as required (probed `sdiv8_minint`, rem = 0). Test the zero check AFTER extension (a 0x100 pattern is a nonzero w but zero i8) |
| udiv/urem i8, i16 | `uxtb w9,w9 ; uxtb w10,w10` ; `cbz w10,Ltrap` ; `udiv`/`msub` w forms | |

### 2.3 Shifts (Go: count >= width gives 0 for shl/shr-logical, sign fill for sar). Count in x10 zero-extended. Hardware masks the count (`lsl x,x,#64` = shift by 0, probed).

| IR op | sequence | notes |
|---|---|---|
| shl i64 | `cmp x10,#64` ; `lsl x11,x9,x10` ; `csel x9,xzr,x11,hs` | hs = unsigned >= ; count 2^32 also gives 0 (probed) |
| shr (logical) i64 | `cmp x10,#64` ; `lsr x11,x9,x10` ; `csel x9,xzr,x11,hs` | |
| sar i64 | `mov x12,#63` ; `cmp x10,x12` ; `csel x12,x10,x12,lo` ; `asr x9,x9,x12` | min(count,63) |
| shl i32 (and i8/i16) | `cmp x10,#32` ; `lsl w11,w9,w10` ; `csel w9,wzr,w11,hs` | NB: w-form takes a w count register. Threshold stays 32 for i8/i16: low W bits are already 0 for count in [W,31]; re-extend/truncate after |
| shr i32 | `cmp x10,#32` ; `lsr w11,w9,w10` ; `csel w9,wzr,w11,hs` | i8/i16: `uxtb`/`uxth` the value first, same sequence (probed `go_shr8_c7/c8`) |
| sar i32 | `mov w12,#31` ; `cmp x10,x12` ; `csel x12,x10,x12,lo` ; `asr w9,w9,w12` | i8/i16: `sxtb`/`sxth` value first (probed `go_sar8_c20` = 0xff) |
| shift by constant c < W | `lsl x9,x9,#c` / `lsr` / `asr` (w forms for 32) | c >= W folded at compile time to 0 / sign fill (`asr x9,x9,#63`) |

### 2.4 Logic

| IR op | sequence | notes |
|---|---|---|
| and / or / xor | `and x9,x9,x10` / `orr x9,x9,x10` / `eor x9,x9,x10` | w forms for <= 32 |
| andnot (a &^ b) | `bic x9,x9,x10` | one instruction |
| complement | `mvn x9,x9` | = `orn x9,xzr,x9` |

### 2.5 Compare to 0/1, branches

| IR op | sequence | notes |
|---|---|---|
| cmp i64 | `cmp x9,x10` ; `cset w11,<cc>` | cc: eq ne; signed lt le gt ge; unsigned lo ls hi hs (all probed) |
| cmp i32 | `cmp w9,w10` ; `cset w11,<cc>` | |
| cmp i8/i16 signed | `sxtb w9,w9` ; `cmp w9,w10,sxtb` ; `cset w11,<cc>` | second operand extended by the cmp itself (probed `i8_lt_s`); eq/ne alternatively mask both |
| cmp i8/i16 unsigned | `uxtb w9,w9` ; `cmp w9,w10,uxtb` ; `cset w11,<cc>` | (probed `i8_lo_u`) |
| cmp with 0 | `cmp x9,#0 ; cset w11,eq` | |
| store bool | `strb w11,[sp,#o]` | bool = i8 0/1 |
| br cond,T,F | `ldrb w9,[sp,#o]` ; `cbnz w9,LT` ; `b LF` (omit `b` if LF is next) | range: cbz/cbnz/b.cond +-1 MiB (assembler error beyond, probed 2.1 MB), tbz +-32 KiB, b/bl +-128 MiB. Emitter knows the function size: if > 512 KiB use `cbz w9,Lskip ; b LT ; Lskip: b LF` |
| fused cmp+br (optional) | `cmp x9,x10 ; b.<cc> LT ; b LF` | |

### 2.6 Floating point (d = f64, s = f32; same mnemonics)

| IR op | sequence | notes |
|---|---|---|
| fadd/fsub/fmul/fdiv | `fadd d16,d16,d17` / `fsub` / `fmul` / `fdiv` (s regs for f32) | IEEE, default FPCR: RNE, no traps |
| fsqrt | `fsqrt d16,d16` | |
| fneg / fabs | `fneg d16,d16` / `fabs d16,d16` | pure sign-bit ops, NaN payload kept |
| fmin/fmax (extra) | `fmin`/`fmax` propagate NaN (matches Go min/max) | (doc); RISC-V returns the non-NaN operand |
| fcmp -> 0/1 | `fcmp d16,d17` ; `cset w11,<cc>` | NaN sets NZCV = 0011 (unordered). Correct cc: **eq** (false on NaN), **ne** (TRUE on NaN, Go-correct), **a<b: mi**, **a<=b: ls**, **a>b: gt**, **a>=b: ge**. Do NOT use lt/le: they are true on NaN (probed `nan_lt_WRONG`=1, `nan_le_WRONG`=1) |
| f32 compare | `fcmp s16,s17` ; same cc | |
| f64<->f32 | `fcvt s16,d16` / `fcvt d16,s16` | |
| bits move | `fmov x9,d16` / `fmov d16,x9` / `fmov w9,s16` / `fmov s16,w9` | float constants via integer reg + fmov |
| float -> int signed | `fcvtzs x9,d16` (i64) / `fcvtzs w9,d16` (i32) / `fcvtzs w9,s16` | rounds toward zero. Out of range SATURATES, NaN -> 0, no trap, no flag needed (probed: 1e30 -> 0x7fff..., -1e30 -> 0x8000..., NaN -> 0, i32 3e9 -> 0x7fffffff) |
| float -> int unsigned | `fcvtzu x9,d16` / `fcvtzu w9,d16` | negative -> 0, huge -> all ones, NaN -> 0 (probed) |
| float -> i8/i16 | `fcvtzs w9,d16` then take low bits (wraps) | unspecified in Go; saturating to the narrow range would need clamp code |
| int -> float | `scvtf d16,x9` / `scvtf d16,w9` / `ucvtf d16,x9` / `ucvtf d16,w9` (s16 for f32) | i8/i16: `sxtb`/`uxtb` first. i64->f32 rounds once (probed 0x7fff...ffff -> 0x5f000000) |

### 2.7 Memory, addressing

| IR op | sequence | notes |
|---|---|---|
| load u8/u16/u32/i64 | `ldrb w9,[sp,#o]` / `ldrh w9,..` / `ldr w9,..` / `ldr x9,..` | zero-extending (probed) |
| load sext i8/i16/i32 | `ldrsb x9` (or `w9`) / `ldrsh` / `ldrsw x9` | |
| load f32/f64 | `ldr s16,[..]` / `ldr d16,[..]` | |
| store 8/16/32/64, f32/f64 | `strb w9` / `strh w9` / `str w9` / `str x9` / `str s16` / `str d16` | narrow store does not touch neighbours (probed) |
| stack slot offset | scaled unsigned imm12: `[sp,#o]` needs o % size == 0 and o <= 4095*size (u8: 4095, u64: 32760); `ldur/stur` unscaled -256..255 | pick slot alignment = size |
| large offset | `movz x16,#lo ; movk x16,#hi,lsl #16 ; add x16,sp,x16 ; ldr x9,[x16]` or `add x16,sp,#H,lsl #12 ; add x16,x16,#L ; ldr x9,[x16]` (o = H*4096+L, H,L < 4096) | both probed with o = 79984 |
| address of stack slot | `add x9,sp,#o` (o <= 4095, or `#H,lsl #12` then `#L`) | |
| address of data label | `adrp x9,sym ; add x9,x9,:lo12:sym` | +-4 GiB; folded form `adrp x9,sym ; ldr x11,[x9,:lo12:sym]` (probed on .data, .bss, .rodata) |
| indirect call | `ldr x16,[sp,#o] ; blr x16` | |
| unaligned access | allowed for normal memory on AArch64 (not atomics) | RV may trap: IR must assume natural alignment |

### 2.8 Prologue, epilogue, calls (probed `sum10`, `fsum9`, `clobber`)

| item | sequence | notes |
|---|---|---|
| prologue | `stp x29,x30,[sp,#-16]!` ; `mov x29,sp` ; `sub sp,sp,#N` | N % 16 == 0, includes slots + outgoing-args area. N > 4095: `sub sp,sp,#H,lsl #12 ; sub sp,sp,#L` or `movz/movk x16 ; sub sp,sp,x16` (probed) |
| epilogue | `mov sp,x29` ; `ldp x29,x30,[sp],#16` ; `ret` | non-leaf MUST save x30 (bl overwrites it). No red zone on AArch64 |
| incoming stack args | `[x29,#16+8k]` | 8-byte slots |
| outgoing stack args | store at `[sp,#8k]` before `bl` (area reserved in the prologue, sp constant) | |
| call | `bl sym` (+-128 MiB; beyond that the linker inserts thunks (doc)) | args: `ldr x0..x7`, `ldr d0..d7`; result in x0/x1, d0.. |
| callee-saved | x19-x28, x29, d8-d15 (low 64 bits only); x30 is saved by callee only if it calls | probed (`x19_preserved`, `d8_preserved`) |
| sp alignment | 16 bytes whenever sp is used as a base; hardware faults otherwise (not emulated by qemu-user, probed `a64_align`) | |

## 3. RISC-V 64 mapping (RV64GC = IMAFDC; no Zbb/Zba; all rows probed in `rv_ops` unless (doc))

Key facts: no flags register (compare writes a GPR, branches compare two GPRs); the 32-bit `*w` ops sign-extend their result to 64 bits (the ABI form of an i32 is "sign-extended"); `sext.w rd,rs` = `addiw rd,rs,0`. Immediates are 12-bit signed (-2048..2047): `sb t0,4095(s2)` is an assembler error (probed).

### 3.1 Integer arithmetic, extension

| IR op | sequence (A=t0, B=t1, R=t2) | notes |
|---|---|---|
| add/sub/mul i64 | `add t2,t0,t1` / `sub` / `mul` | |
| add/sub/mul i32 | `addw t2,t0,t1` / `subw` / `mulw` | result sign-extended 32 -> 64 for free (0x7fffffff+1 = 0xffffffff80000000, probed) |
| add/sub/mul i8, i16 | `add`/`sub`/`mul` (64-bit), upper bits garbage | extend when consumed |
| neg i64 / i32 / narrow | `neg t2,t0` (= `sub t2,x0,t0`) / `negw t2,t0` / `neg` | neg(MinInt64) = MinInt64 |
| sext i8 -> any | `slli t0,t0,56 ; srai t0,t0,56` | no `sext.b` (Zbb) |
| sext i16 | `slli t0,t0,48 ; srai t0,t0,48` | |
| sext i32 -> i64 | `sext.w t0,t0` | free if the value came from a `*w` op or `lw` |
| zext i8 | `andi t0,t0,255` (alias `zext.b`, accepted by clang, equals andi) | |
| zext i16 | `slli t0,t0,48 ; srli t0,t0,48` | `zext.h` is Zbb |
| zext i32 -> i64 | `slli t0,t0,32 ; srli t0,t0,32` | `zext.w` is Zba; `lwu` for loads |
| trunc | none | |
| mul-high (extra) | `mulhu t2,t0,t1` / `mulh` | (doc) |
| const | `li t0,imm` (assembler expands lui/addi/slli, up to 8 instructions, any 64-bit value incl. 0x8000000000000000 probed) | |

### 3.2 Division and remainder (Go semantics)

Hardware: never traps. x/0: div/divu/divw = -1 (all ones), rem/remu = x (probed `div0_hw`, `rem0_hw`, `divu0_hw`, `divw0_hw`). MinInt/-1: div = MinInt, rem = 0 (probed for i64 and i32 `divw`/`remw`). So like AArch64: only the zero check is needed.

| IR op | sequence | notes |
|---|---|---|
| sdiv / srem i64 | `beqz t1,Ltrap` ; `div t2,t0,t1` / `rem t2,t0,t1` | Ltrap: `ebreak` (SIGTRAP, exit 133) or `li a0,3 ; li a7,94 ; ecall` (exit 3, probed) |
| udiv / urem i64 | `beqz t1,Ltrap` ; `divu` / `remu` | |
| sdiv / srem i32 | operands sign-extended (`sext.w`) ; `beqz t1,Ltrap` ; `divw` / `remw` | |
| udiv / urem i32 | `sext.w`-or-not (divuw/remuw read only the low 32 bits) ; the zero check must test the LOW 32 bits: `sext.w t3,t1 ; beqz t3,Ltrap` ; `divuw` / `remuw` | `divuw` result is sign-extended (0xffffffff/2 = 0x7fffffff, probed) |
| sdiv / srem i8, i16 | `sext8/16` both ; zero check ; `divw` / `remw` ; low bits | -128/-1 = +128, truncates to -128 (probed). No special case |
| udiv / urem i8, i16 | `andi`/`zext16` both ; zero check ; `divuw` / `remuw` | |

### 3.3 Shifts (branch-free; count in t1 zero-extended; hardware masks: `sll` by 64 = by 0, probed)

| IR op | sequence | notes |
|---|---|---|
| shl i64 | `sltiu t3,t1,64` ; `sll t2,t0,t1` ; `neg t3,t3` ; `and t2,t2,t3` | t3 = -1 if count < 64 else 0 |
| shr (logical) i64 | `sltiu t3,t1,64` ; `srl t2,t0,t1` ; `neg t3,t3` ; `and t2,t2,t3` | |
| sar i64 | `sltiu t3,t1,64` ; `addi t3,t3,-1` ; `or t3,t1,t3` ; `sra t2,t0,t3` | t3 = 0 if count < 64 else -1; or-ing gives count 63 (low 6 bits all ones) for count >= 64 |
| shl i32 (and i8/i16) | `sltiu t3,t1,32` ; `sllw t2,t0,t1` ; `neg t3,t3` ; `and t2,t2,t3` | threshold 32 also for i8/i16; sllw sign-extends the 32-bit result |
| shr i32 | `sltiu t3,t1,32` ; `srlw t2,t0,t1` ; `neg t3,t3` ; `and t2,t2,t3` | i8/i16: zero-extend the value first |
| sar i32 | `sltiu t3,t1,32` ; `addi t3,t3,-1` ; `or t3,t1,t3` ; `sraw t2,t0,t3` | i8/i16: sign-extend first. (probed all: c64, c63, c200, c2^32, c32, c31, c40, i8 c1/c7/c8/c20/c40) |
| shift by constant c < W | `slli`/`srli`/`srai` (`slliw`/`srliw`/`sraiw` for 32) | c >= W folded: 0 or `srai t2,t0,63` |

### 3.4 Logic

| IR op | sequence | notes |
|---|---|---|
| and / or / xor | `and` / `or` / `xor t2,t0,t1` | |
| andnot | `not t3,t1` (= `xori t3,t1,-1`) ; `and t2,t0,t3` | `andn` is Zbb, not baseline |
| complement | `xori t2,t0,-1` (= `not`) | for i32 the result stays sign-extended |

### 3.5 Compare to 0/1, branches

| IR op | sequence | notes |
|---|---|---|
| eq / ne | `xor t2,t0,t1 ; seqz t2,t2` / `xor t2,t0,t1 ; snez t2,t2` | `seqz` = `sltiu rd,rs,1`, `snez` = `sltu rd,x0,rs` |
| lt / gt signed | `slt t2,t0,t1` / `slt t2,t1,t0` | |
| le / ge signed | `slt t2,t1,t0 ; xori t2,t2,1` / `slt t2,t0,t1 ; xori t2,t2,1` | |
| lo / hi unsigned | `sltu t2,t0,t1` / `sltu t2,t1,t0` | |
| ls / hs unsigned | `sltu t2,t1,t0 ; xori t2,t2,1` / `sltu t2,t0,t1 ; xori t2,t2,1` | |
| cmp i32 | operands `sext.w` first, then the 64-bit sequences | signed AND unsigned order of sign-extended i32 equals the 32-bit order (probed `i32_lo_u_on_sext`, `i32_hi_u_on_sext`), so no zext needed for unsigned i32 |
| cmp i8/i16 | signed: sext both then slt family; unsigned: zext both then sltu family; eq/ne: `xor ; andi t2,t2,255 ; seqz` | (probed) |
| cmp with 0 | `seqz t2,t0` / `snez t2,t0` / `sltz`/`sgtz` | |
| br cond,T,F | `lbu t0,o(sp)` ; `beqz t0,Lskip` ; `j LT` ; `Lskip: j LF` (omit last `j` if LF is next) | **always branch over a `j`**: conditional branches reach only +-4 KiB (`beq/bne/blt/bge/bltu/bgeu`), `j` (jal x0) +-1 MiB, `call`/`tail` (auipc+jalr) +-2 GiB. The LLVM assembler auto-relaxes an out-of-range `beq` into `bne +8 ; j` even across sections (probed), GNU toolchains and hand-computed size estimates differ, so emit the inverted form explicitly: the size is fixed and predictable |
| fused cmp+br (optional) | `blt t0,t1,Lskip ; ...` (inverse cond) `; j LT` | only gt/le need operand swap: `bgt a,b` = `blt b,a` |

### 3.6 Floating point (D: `.d`, F: `.s`)

| IR op | sequence | notes |
|---|---|---|
| fadd/fsub/fmul/fdiv | `fadd.d ft2,ft0,ft1` / `fsub.d` / `fmul.d` / `fdiv.d` (`.s` for f32) | dynamic rounding = RNE at process start (doc); results are canonical NaN (0x7ff8...), not payload-propagating (doc) |
| fsqrt | `fsqrt.d ft2,ft0` | |
| fneg / fabs | `fneg.d ft2,ft0` (= `fsgnjn.d ft2,ft0,ft0`) / `fabs.d ft2,ft0` (= `fsgnjx.d`) | |
| fmin/fmax (extra) | `fmin.d`/`fmax.d` return the NON-NaN operand | (doc); Go min/max (NaN-propagating) needs fix-up code |
| a==b | `feq.d t2,ft0,ft1` | 0 on NaN (probed `nan_eq`, `nan_eq_f32` with `feq.s`) |
| a!=b | `feq.d t2,ft0,ft1 ; xori t2,t2,1` | 1 on NaN, Go-correct |
| a<b / a<=b | `flt.d t2,ft0,ft1` / `fle.d t2,ft0,ft1` | 0 on NaN |
| a>b / a>=b | `flt.d t2,ft1,ft0` / `fle.d t2,ft1,ft0` | swap operands; 0 on NaN (all probed incl. NaN on either side, -0 == 0) |
| f64<->f32 | `fcvt.s.d ft1,ft0` / `fcvt.d.s ft1,ft0` | |
| bits move | `fmv.x.d t0,ft0` / `fmv.d.x ft0,t0` / `fmv.x.w` / `fmv.w.x` | `fmv.x.w` SIGN-extends the 32 bits. f32 values in FP regs are NaN-boxed; `flw`/`fmv.w.x` box correctly |
| float -> i64 signed | `fcvt.l.d t2,ft0,rtz` | rtz required (default dyn mode would round-to-nearest). Out of range SATURATES; **NaN -> +MaxInt (0x7fff...)**, not 0 (probed) |
| float -> i32 signed | `fcvt.w.d t2,ft0,rtz` | saturates, sign-extended result; NaN -> 0x7fffffff (doc) |
| float -> u64 / u32 | `fcvt.lu.d t2,ft0,rtz` / `fcvt.wu.d t2,ft0,rtz` | negative -> 0; huge and NaN -> all ones; **`fcvt.wu.d` result is sign-extended** (3e9 -> 0xffffffffb2d05e00; use low 32 bits) (probed) |
| float -> i8/i16 | `fcvt.w.d` then low bits | |
| deterministic variant (saturate, NaN -> 0 as on AArch64) | `feq.d t3,ft0,ft0 ; fcvt.l.d t2,ft0,rtz ; neg t3,t3 ; and t2,t2,t3` | probed `f2i_nan_fixup0`; +3 instructions |
| int -> float | `fcvt.d.l ft0,t0` / `fcvt.d.lu` / `fcvt.d.w` / `fcvt.d.wu` / `fcvt.s.l` ... | i8/i16: sext/zext first; `.w` forms read the low 32 bits. Add explicit `rne` rounding operand if you do not trust frm |

### 3.7 Memory, addressing

| IR op | sequence | notes |
|---|---|---|
| load u8/u16/u32/i64 | `lbu` / `lhu` / `lwu` / `ld t0,o(sp)` | |
| load sext i8/i16/i32 | `lb` / `lh` / `lw` | **`lw` sign-extends**: natural for the i32 canonical form |
| load f32/f64 | `flw ft0,o(sp)` / `fld` | |
| store | `sb` / `sh` / `sw` / `sd` / `fsw` / `fsd` | |
| stack slot offset | `o(sp)` with -2048 <= o <= 2047 | |
| large offset | `li t3,o ; add t3,sp,t3 ; ld t0,0(t3)` or `lui t3,%hi_part ; add t3,t3,sp ; ld t0,lo(t3)` | probed with o = 79984 (both) |
| address of stack slot | `addi t0,sp,o` (|o| <= 2047) else `li t0,o ; add t0,sp,t0` | |
| address of data label | `lla t0,sym` (= `auipc t0,%pcrel_hi(sym) ; addi t0,t0,%pcrel_lo(L)`) | **use `lla`, not `la`**: with `clang --target=riscv64-linux-gnu`, `la` assembles as a GOT load (`R_RISCV_GOT_HI20`: `auipc a1,..; ld a1,-0x53e(a1)`, 176 of them in `rv_ops.o`) and the linker has to build a .got. Explicit form: `L: auipc t0,%pcrel_hi(sym)` then `addi t0,t0,%pcrel_lo(L)` (the %pcrel_lo operand is the LABEL of the auipc, unique per site); folded: `auipc t3,%pcrel_hi(sym) ; ld t2,%pcrel_lo(L)(t3)` (probed all four) |
| indirect call | `ld t0,o(sp) ; jalr ra,0(t0)` | |
| unaligned access | may trap or be slow on real hardware (qemu-user allows, probed) | assume natural alignment; bytes otherwise |

### 3.8 Prologue, epilogue, calls (probed `sum10`, `fsum9`, `fsum9s`, `clobber`)

| item | sequence | notes |
|---|---|---|
| prologue | `addi sp,sp,-16` ; `sd ra,8(sp)` ; `sd s0,0(sp)` ; `addi s0,sp,16` ; `addi sp,sp,-N` | N % 16 == 0; N > 2048: `li t0,N ; sub sp,sp,t0`. The ra/s0 stores happen before the large subtract so their offsets stay small |
| epilogue | `addi sp,s0,-16` ; `ld ra,8(sp)` ; `ld s0,0(sp)` ; `addi sp,sp,16` ; `ret` | `ret` = `jalr x0,0(ra)`. Non-leaf MUST save ra. No red zone |
| incoming stack args | `0(s0)`, `8(s0)`, ... | s0 = caller sp at entry |
| outgoing stack args | `sd t0,8k(sp)` before `call` | |
| call | `call sym` (auipc ra + jalr ra, +-2 GiB, linker relaxes to `jal` when near) | `call` clobbers only ra; `tail` clobbers t1 |
| callee-saved | s0-s11 (x8,x9,x18-x27), fs0-fs11, sp. gp and tp are reserved: never write | |
| gp | ld.lld (default) never uses gp; `ld.lld --relax-gp` and GNU ld rewrite absolute `%hi/%lo` accesses to gp-relative, and process start leaves gp = 0 (probed: segfault, exit 139). Emit only pcrel pairs (`lla`), or have _start run `.option push ; .option norelax ; lla gp,__global_pointer$ ; .option pop` | |
| sp alignment | 16 bytes (ABI; hardware does not check) | |
| clang default arch | `rv64i2p1_m2p0_a2p1_f2p2_d2p2_c2p0_zicsr2p0_zmmul1p0_zaamo1p0_zalrsc1p0_zca1p0_zcd1p0` (rv64gc, ABI lp64d); `zext.b`/`sext.w`/`not`/`neg`/`seqz` are aliases of base instructions | compressed instructions are emitted automatically by the assembler, branch ranges above are unaffected |

## 4. Calling convention (ground truth: `clang -S -O1` of `abi.c`, files `abi_aarch64-linux-gnu.s`, `abi_riscv64-linux-gnu.s`; plus probes)

| case | AArch64 (AAPCS64) | RV64 (LP64D) | x86-64 SysV (for contrast, doc) |
|---|---|---|---|
| int/ptr args | x0-x7 | a0-a7 | rdi rsi rdx rcx r8 r9 (6) |
| float args | v0-v7 (d/s) | fa0-fa7 | xmm0-xmm7 |
| register counters | independent int and FP counters | independent; **when fa0-fa7 are used up, further floats go into free a-registers, then the stack** (probed: 9th double in a2 with `fsum9`; with all of a0-a7 busy it is on the stack, `fsum9s`); `abi.c g9`: 9th double arrives in a0 | independent |
| stack args | 8-byte slots at `[sp+8k]` at entry, in order; a float beyond v7 goes to the stack (probed `g9`) | same, `0(sp)` at entry | same |
| results | x0, x1 (two ints); v0..v3 | a0, a1; fa0, fa1 | rax, rdx; xmm0, xmm1 |
| narrow int args | upper bits UNSPECIFIED; callee extends itself (`g11`: `w0,uxtb`, `w2,sxth`) | caller must extend: signed types sign-extended, unsigned char/short zero-extended, **unsigned int sign-extended to 64** (`g11`: callee adds raw registers) | unspecified (callee extends) |
| struct <= 16 B all-int | 1-2 x regs (`S16` in x0,x1; `char[3]` in x0) | 1-2 a regs (same) | INTEGER eightbytes in regs |
| struct > 16 B | caller makes a copy, passes ADDRESS in an x reg (`S24` in x0) | same (`S24` pointer in a0) | by value on the stack (memory class) |
| homogeneous float struct | HFA, up to 4 members of one float type, in consecutive v regs (`D2` in d0,d1; `F3` in s0,s1,s2) | at most 2 fp fields (or 1 fp + 1 int) go to fa/a regs (`D2` fa0,fa1; `{long,double}` a0+fa0); `F3` (3 floats, 12 B) is packed into a0,a1 as integers | classified per eightbyte (SSE/INTEGER) |
| `{long,double}` | x0,x1 (not an HFA) | a0 + fa0 | rdi + xmm0 |
| struct result > 16 B | caller passes buffer address in **x8** (`r2`: `stp x0,x0,[x8]`) | hidden pointer is the FIRST argument in **a0**, real args shift by one (`r2`: `sd a1,0(a0)`) | hidden pointer in rdi, returned in rax |
| struct result <= 16 B | x0,x1 / v0.. | a0,a1 / fa0,fa1 / a0+fa0 | rax,rdx / xmm0 / mixed |
| 2-register struct with one register left | whole struct to the stack, x7 stays unused (`g7`: 7 longs then `S16` -> stack, probed in clang output) | **split: first half in a7, second half on the stack** (`g7`: `a7` + `0(sp)`) | whole struct on the stack |
| call-site sp | 16-byte aligned, no red zone | 16-byte aligned, no red zone | 16-byte aligned at the call, 128-byte red zone |
| return address | x30 (lr) | ra (x1) | on the stack |

Consequence for the IR: the ABIs disagree on aggregates in five independent ways (HFA vs flattening, a7 split, x8 vs a0 for the result pointer, fp-to-gpr overflow, narrow-int extension duty). **asm_ir/1 should pass and return aggregates only by pointer** (caller owns a copy for by-value semantics; results larger than the scalar results via an explicit out-pointer parameter). Scalars (i8..i64, f32, f64, ptr) map identically on both targets: up to 8 int + 8 fp register args, rest on the stack in order; up to 2 scalar results (a0/a1 or x0/x1, fa0/fa1 or v0/v1). Since asm_ir/1 code is never linked against C, the cross-language quirks (RV extension duty, fp overflow into a-regs) are irrelevant, but a backend that follows the standard ABI exactly for scalars costs nothing and keeps the option of calling C later. A rule that makes both consistent without ABI knowledge: more than 8 args of one class go to the stack for both targets; do not use the RV fp-to-a-reg overflow (a self-consistent internal convention is allowed because both caller and callee are generated).

## 5. Linux syscalls without libc, _start

| item | AArch64 | RV64 | x86-64 (contrast) |
|---|---|---|---|
| instruction | `svc #0` | `ecall` | `syscall` |
| number register | x8 | a7 | rax |
| args | x0..x5 | a0..a5 | rdi rsi rdx **r10** r8 r9 |
| result | x0; -4095..-1 = -errno | a0; same | rax; same |
| preserved | everything except x0 (x28/s11 counters survived all probes) | everything except a0 | **rcx and r11 clobbered** |
| read | 63 | 63 | 0 |
| write | **64** (probed) | **64** (probed) | 1 |
| openat | 56 | 56 | 257 (also `open` = 2; absent on arm64/riscv) |
| close | 57 | 57 | 3 |
| lseek | 62 | 62 | 8 |
| exit (thread) | 93 | 93 | 60 |
| exit_group | **94** (probed) | **94** (probed) | 231 |
| mmap | **222** (probed) | **222** (probed) | 9 |
| munmap | 215 (probed) | 215 (probed) | 11 |
| mprotect | 226 | 226 | 10 |
| brk | 214 | 214 | 12 |
| mremap / madvise | 216 / 233 | 216 / 233 | 25 / 28 |
| clock_gettime | 113 | 113 | 228 |
| nanosleep / clock_nanosleep | 101 / 115 | 101 / 115 | 35 / 230 |
| getrandom | 278 | 278 | 318 |
| rt_sigaction / rt_sigprocmask | 134 / 135 | 134 / 135 | 13 / 14 |
| kill / tgkill / getpid / gettid | 129 / 131 / 172 / 178 | same | 62 / 234 / 39 / 186 |
| futex / clone / execve / wait4 | 98 / 220 / 221 / 260 | same | 202 / 56 / 59 / 61 |
| fstat / newfstatat | 80 / 79 | 80 / 79 | 5 / 262 |
| ioctl / writev / readv | 29 / 66 / 65 | same | 16 / 20 / 19 |

Source of numbers: `/usr/include/asm-generic/unistd.h` (arm64 and riscv64 share the generic table) and `/usr/include/asm/unistd_64.h` (x86-64). Every number in the x86-64 column was checked against that header. arm64/riscv64 have no `open`, `stat`, `fork`, `pipe`, `dup2`, `poll`, `arch_prctl`: only `openat`, `newfstatat`, `clone`, `pipe2`, `dup3`, `ppoll`.
mmap: `x0=0 (addr), x1=len, x2=3 (PROT_READ|PROT_WRITE), x3=0x22 (MAP_PRIVATE|MAP_ANONYMOUS), x4=-1 (fd), x5=0 (offset)`; same values on all three. Error test: `cmn x0,#4095 ; b.hi err` (AArch64), `li t3,-4096 ; bltu t3,a0,err` (RV). Probed: mmap 4096 B read/write/zero-filled, munmap -> 0, `write(99,..)` -> -9, `mmap(len 0)` -> -22 on both. Use exit_group (94), not exit (93): exit ends only the calling thread. write may be short: loop on the returned count.

_start (both probed; entry state is only guaranteed for sp):

| | AArch64 | RV64 |
|---|---|---|
| stack at entry | sp 16-aligned; `[sp]` = argc, `[sp+8]` = argv[0].. | same |
| minimal entry | `mov x29,#0 ; mov x30,#0 ; ... ; mov x0,#0 ; mov x8,#94 ; svc #0` | `mv s0,zero ; mv ra,zero ; ... ; li a0,0 ; li a7,94 ; ecall` |
| "ok 5" write | `mov x0,#1 ; adrp x1,msg ; add x1,x1,:lo12:msg ; mov x2,#5 ; mov x8,#64 ; svc #0` | `li a0,1 ; lla a1,msg ; li a2,5 ; li a7,64 ; ecall` (probe used `la`, which works only because the GOT path happens to link; use `lla`) |
| calling a function from _start | sp is already 16-aligned, `bl main` directly | same, `call main` |
| contrast | x86-64: rsp 16-aligned at _start but 8 mod 16 at a function entry after `call` | |

## 6. Differences the IR design must account for (x86-64 vs AArch64 vs RV64)

| topic | x86-64 | AArch64 | RV64 | IR consequence |
|---|---|---|---|---|
| x/0 | `div`/`idiv` raise SIGFPE | returns 0 | returns -1 (all ones) | IR: division by zero TRAPS; every backend emits the explicit zero check (probed on ARM/RV; the hardware result is never observable) |
| MinInt/-1 | `idiv` raises SIGFPE (probed exit 136) | = MinInt, rem 0 (probed) | = MinInt, rem 0 (probed) | IR: wraps (= MinInt, rem 0). Only the x86 backend needs a `cmp -1` special case (branch around idiv, `neg` result / zero remainder); ARM/RV nothing |
| remainder | idiv gives rdx | `sdiv`+`msub` (no rem instruction) | `rem` | one IR op `srem`; no shared helper |
| shift count | masked to 5/6 bits (8/16-bit shifts also mask to 5 bits, `shl eax,64` = shl by 0, probed) | masked to 5/6 bits | masked to 5/6 bits | IR: count >= width gives 0 / sign fill. Cheapest: AArch64 `cmp;csel`, RV 3 branch-free instructions (`sltiu;neg;and` / `sltiu;addi;or`), x86 `cmp;sbb;and` or `cmov`. i8/i16 use the 32-bit sequence |
| narrow ints | 8/16-bit ops exist, partial registers | only w/x forms | only 32/64-bit forms, `*w` sign-extend to 64 | IR: bits above the width are UNDEFINED in registers; explicit `sext`/`zext`/`trunc` ops. Free canonical forms differ (x86/AArch64 w-ops zero-extend, RV `*w` ops and `lw` sign-extend), so IR must not promise either. Compare, div, shr, convert, widen and branch each extend their inputs |
| flags | EFLAGS | NZCV | **none**: compares produce a GPR, branches compare two GPRs | IR cmp yields an i8 0/1 vreg, then `br`. Fusing is a backend peephole. RV has only lt/ge/eq/ne branches (gt/le by operand swap) |
| float compare with NaN | `ucomisd` sets ZF=PF=CF=1 when unordered, so `sete` is TRUE for NaN (probed) and needs `setnp`/`and` | `fcmp` sets 0011: use eq/ne/mi/ls/gt/ge; **never lt/le** | `feq/flt/fle` give 0 for NaN; `ne` = `feq`+`xori 1` | IR: all float compares except `ne` are false on NaN, `ne` true; one table of condition codes per target |
| float -> int out of range / NaN | `cvttsd2si`: 0x8000000000000000 for NaN and out of range (probed); no unsigned 64-bit form | saturates, NaN -> 0 | saturates, **NaN -> MaxInt** (unsigned: all ones) | IR: out-of-range and NaN results are UNSPECIFIED (Go says implementation-defined), no trap; if determinism is wanted choose saturate+NaN=0 and pay the fix-up on RV (+3 instr) and x86 |
| fp min/max | NaN rules differ | NaN-propagating | returns non-NaN | do not add fmin/fmax to the IR without defining NaN behaviour |
| conditional branch range | rel32 +-2 GiB | b.cond/cbz +-1 MiB, tbz +-32 KiB (no auto-relax: assembler error, probed) | **+-4 KiB** | emit blocks as `inverse-cond over j` on RV always; on AArch64 `cbnz L ; b L2` with a function-size guard (> 512 KiB use the inverted form) |
| immediates | imm32 sign-extended | `mov` takes one-instruction encodings only (assembler error otherwise, probed), `movz/movk` | 12-bit; `li` expands | IR carries 64-bit constants; each backend materializes them |
| memory offsets | disp32 | scaled imm12 (u8 4095 .. u64 32760), unscaled +-256 | +-2047 | frame layout: put small slots first; any offset beyond the limit goes through a scratch address register (probed 79984) |
| load/store architecture | memory operands | yes | yes | IR load/store only; no memory-operand ops |
| data address | RIP-relative `lea` | `adrp`+`add :lo12:` | `lla` (auipc+addi). `la` = GOT load with clang defaults | one IR op `addr sym` |
| link register | return address on stack | x30 | ra | non-leaf functions save x30 / ra in every prologue; naive backend: always |
| stack | 16-byte aligned at call, red zone 128 B | 16-byte aligned always, no red zone, hardware fault otherwise (qemu-user does not fault) | 16-byte aligned (ABI), no red zone | frame size multiple of 16 on all; no red-zone use |
| unaligned access | fine | fine (normal memory) | may trap/slow on hardware | IR requires natural alignment for load/store (slots aligned to size) |
| aggregates in calls | SysV classification | HFA, x8 result pointer | a0 hidden pointer, a7 split, fp-to-gpr overflow | IR passes/returns aggregates by pointer only |
| syscall | rax, rdi rsi rdx r10 r8 r9, clobbers rcx r11 | x8, x0..x5 | a7, a0..a5 | numbers differ from x86-64 (section 5); a per-target `syscall` intrinsic lowers number and up to 6 args |
| trap | `ud2` (SIGILL) | `brk #1` (SIGTRAP, 133) | `ebreak` (SIGTRAP, 133) | IR `trap`: either a signal or the explicit exit 3 stub; prefer the exit_group(3) stub for portability of the observable exit code |
| process entry | rsp%16 == 0 | sp%16 == 0 | sp%16 == 0; set gp only if linking with relaxing GNU ld | same `_start` shape on all three |

Probe-environment caveats: qemu-user does not emulate AArch64 SP alignment faults, RISC-V misaligned-access traps, or real FPCR/frm state beyond the defaults; all numeric results above (division, saturation, NaN) are architecturally defined behaviour, not emulator quirks. Not probed: tbz range, thunks, stack arguments of sub-8-byte size, frames > 4 KiB on RV prologues (the `li/sub` form), fmin/fmax.
