# asm_ir/1 -> x86-64 (GNU as, AT&T, ELF, Linux, SysV) naive instruction mapping

Every sequence below that is marked [P] was assembled and run by a probe (section 12), checked against a C reference on edge values plus random inputs, with mutation runs showing the probes go red for the shift, idiv -1 guard, NaN == and < sequences. [-] = standard, not probed.

## 0. Conventions used by every row

| Item | Decision |
|---|---|
| Slot | every vreg and every addressable local owns 8-byte-aligned slot(s) at `-8*k(%rbp)`. Written `[x]` below = `-N(%rbp)` |
| Integer vreg in slot | low N bits hold the value, upper bits UNSPECIFIED. IR ints are signless; signedness is in the op (sdiv/udiv, sar/shr, sext/zext, slt/ult). Every vreg write is a full `movq %rax,[d]` (ints) / `movsd` or `movss` (floats) |
| Width-agnostic ops (add sub mul and or xor neg not andnot shl-low-bits) | read `movl [a],%eax` (W<=32; reads 4 bytes of the 8-byte slot, upper garbage harmless) or `movq` (W=64) |
| Width-sensitive ops (div rem shr sar sext zext int->float, call to C, branch) | extend at use: `movzbl/movzwl/movl` (zero) or `movsbl/movswl/movl` (sign, to 32 bit) or `movsbq/movswq/movslq` (sign, to 64 bit) |
| Compare | exact-width `cmp{b,w,l,q} [b],reg`; no extension needed |
| Scratch (all caller-saved) | rax rcx rdx rsi rdi r8-r11, xmm0-xmm15. Callee-saved rbx r12-r15 are never touched, so only rbp is saved |
| Operand notation | W in {8,16,32,64}; s = `l` for W<=32, `q` for W=64; R/C/Dx = `%eax/%ecx/%edx` (W<=32) or `%rax/%rcx/%rdx` (W=64) |
| Trap | `jz rt_trap_div0` etc. jump (not call) to a runtime stub that writes a message to fd 2 and exit_group(2) (section 11) |

## 1. Integer add/sub/mul/neg/not/and/or/xor/andnot (wrap = native)

Compute in 32-bit regs for W=8/16/32 (avoids partial-register stalls, wrap of the low W bits is automatic), in 64-bit regs for W=64. Signed and unsigned are the SAME instruction (two-s complement); imul low half is sign-agnostic.

| IR op | Sequence (W<=32 / W=64 differ only by s and register size) | Notes |
|---|---|---|
| add | `mov{s} [a],R` ; `mov{s} [b],C` ; `add{s} C,R` ; `movq %rax,[d]` | [P] all W, upper garbage on inputs |
| sub | same, `sub{s} C,R` | R = a - b. [P] |
| mul | same, `imul{s} C,R` | low-half product, signed==unsigned. [P] |
| and / or / xor | same, `and{s}` / `or{s}` / `xor{s}` | [P] |
| neg | `mov{s} [a],R` ; `neg{s} R` ; `movq %rax,[d]` | neg(MinInt)=MinInt, no trap. [P] |
| not (complement) | `mov{s} [a],R` ; `not{s} R` ; `movq %rax,[d]` | [P] |
| andnot a&^b | `mov{s} [b],C` ; `not{s} C` ; `mov{s} [a],R` ; `and{s} C,R` ; `movq %rax,[d]` | BMI1 `andn` is NOT baseline x86-64; not+and is the sequence. [P] |
| sext W->64 | `movsbq [a],%rax` / `movswq` / `movslq` / `movq` ; `movq %rax,[d]` | memory-source form, extends straight from the slot. [P] |
| zext W->64 | `movzbl [a],%eax` / `movzwl [a],%eax` / `movl [a],%eax` / `movq [a],%rax` ; `movq %rax,[d]` | every 32-bit write zero-extends to 64. [P] |
| trunc 64->W | `movq [a],%rax ; movq %rax,[d]` (nothing, upper bits unspecified) | free under the unspecified-upper-bits convention |

Re-extending a narrow result into a canonical 64-bit slot (only needed if the IR chooses "slot always canonical" instead of extend-at-use):

| Slot invariant | W=8 | W=16 | W=32 | W=64 |
|---|---|---|---|---|
| signed-canonical (sign-extended to 64) | `movsbq %al,%rax` | `movswq %ax,%rax` | `movslq %eax,%rax` (= `cltq`) | none |
| unsigned-canonical (zero-extended to 64) | `movzbl %al,%eax` | `movzwl %ax,%eax` | `movl %eax,%eax` | none |

Insert between the op and `movq %rax,[d]`. [P] for add/sub/mul variants, W=8/16/32, both flavours. Recommendation: do NOT keep slots canonical (signless IR); extend at the use that needs it. The cost of canonical slots is one extra instruction per op and the need for IR signedness on every vreg.

## 2. Division and remainder (Go semantics: zero -> trap, MinInt/-1 = MinInt, MinInt%-1 = 0)

Quotient in rax, remainder in rdx. Operands are extended to 32 or 64 bits at load.

| IR op | Sequence | Notes |
|---|---|---|
| sdiv/srem W=64 | `movq [a],%rax ; movq [b],%rcx ; testq %rcx,%rcx ; jz rt_trap_div0 ; cmpq $-1,%rcx ; je .Lm1 ; cqto ; idivq %rcx ; jmp .Ldn ; .Lm1: negq %rax  (div)  /  xorl %edx,%edx  (rem) ; .Ldn: movq %rax,[d]  (div)  /  movq %rdx,[d]  (rem)` | unguarded `idiv` on MinInt/-1 raises #DE = SIGFPE (exit 136), shown by the mutation run. neg(MinInt)=MinInt is exactly the Go result. [P] |
| sdiv/srem W=32 | `movl [a],%eax ; movl [b],%ecx ; testl %ecx,%ecx ; jz rt_trap_div0 ; cmpl $-1,%ecx ; je .Lm1 ; cltd ; idivl %ecx ; jmp .Ldn ; .Lm1: negl %eax / xorl %edx,%edx ; .Ldn: movq %rax or %rdx,[d]` | same shape, 32-bit. [P] |
| sdiv/srem W=8 | `movsbl [a],%eax ; movsbl [b],%ecx ; testl %ecx,%ecx ; jz rt_trap_div0 ; cltd ; idivl %ecx ; movq %rax or %rdx,[d]` | -128/-1 = 128 fits in 32 bit; the low 8 bits (0x80) are the wrapped Go result, so NO -1 check for W=8/16. [P] |
| sdiv/srem W=16 | as W=8 with `movswl` | [P] |
| udiv/urem W=64 | `movq [a],%rax ; movq [b],%rcx ; testq %rcx,%rcx ; jz rt_trap_div0 ; xorl %edx,%edx ; divq %rcx ; movq %rax or %rdx,[d]` | no overflow case possible. [P] |
| udiv/urem W=32 | same with `movl`, `divl %ecx` | [P] |
| udiv/urem W=8/16 | `movzbl/movzwl [a],%eax ; movzbl/movzwl [b],%ecx ; testl %ecx,%ecx ; jz rt_trap_div0 ; xorl %edx,%edx ; divl %ecx ; movq %rax or %rdx,[d]` | extend to 32, use divl. [P] |
| div by constant nonzero, non -1 | skip the checks | front-end or peephole may drop `testq`/`cmpq` |

Notes: do not use `idiv r/m8` / `div r/m8` (quotient in al, remainder in ah, AH encoding clash with REX; slower). Both `jz` targets share one stub per module. A division latency of 20-90 cycles dominates; the checks are irrelevant. Alternative for the -1 case without a branch: none worth it.

## 3. Shifts (Go semantics: count >= W gives 0 for shl/shr and sign-fill for sar)

x86 masks the count (to 5 bits for 8/16/32-bit operand size, 6 bits for 64) so count 32 means 0, not "shift out everything". IR count is an UNSIGNED integer of any width, zero-extended into ecx/rcx (Go: a negative signed count panics; that is a front-end check, not part of the shift op). W=8/16 compute in a 32-bit register, so the threshold is 32 for them; shl/shr results for W=8/16 are correct for counts 8..31 because the vacated/shifted-out bits are zero in the low W bits (shr input is zero-extended first).

| IR op | Sequence (branchless, cmov) | Notes |
|---|---|---|
| shl W=64 | `movq [a],%rax ; movq [b],%rcx ; shlq %cl,%rax ; xorl %edx,%edx ; cmpq $64,%rcx ; cmovaeq %rdx,%rax ; movq %rax,[d]` | count loaded as 64-bit so huge counts (2^32, 2^63) compare correctly. [P] |
| shl W=32/16/8 | `movl [a],%eax ; movl [b],%ecx  (W=16: movzwl, W=8: movzbl) ; shll %cl,%eax ; xorl %edx,%edx ; cmpl $32,%ecx ; cmovael %edx,%eax ; movq %rax,[d]` | threshold 32 for all three. [P] |
| shr W=64 | `movq [a],%rax ; movq [b],%rcx ; shrq %cl,%rax ; xorl %edx,%edx ; cmpq $64,%rcx ; cmovaeq %rdx,%rax ; movq %rax,[d]` | [P] |
| shr W=32/16/8 | `movl/movzwl/movzbl [a],%eax ; movl/movzwl/movzbl [b],%ecx ; shrl %cl,%eax ; xorl %edx,%edx ; cmpl $32,%ecx ; cmovael %edx,%eax ; movq %rax,[d]` | value MUST be zero-extended at load. [P] |
| sar W=64 | `movq [a],%rax ; movq [b],%rcx ; movl $63,%edx ; cmpq $63,%rcx ; cmovaq %rdx,%rcx ; sarq %cl,%rax ; movq %rax,[d]` | clamp count to W-1 = min(count,63); sar by 63 gives pure sign fill. [P] |
| sar W=32/16/8 | `movl/movswl/movsbl [a],%eax ; movl/movzwl/movzbl [b],%ecx ; movl $31,%edx ; cmpl $31,%ecx ; cmoval %edx,%ecx ; sarl %cl,%eax ; movq %rax,[d]` | value MUST be sign-extended at load; clamp to 31 is enough for 8/16 too. [P] |
| shift by constant c | c < W: `shl{s} $c,R` etc., no checks. c >= W: shl/shr -> `xorl %eax,%eax`; sar -> `sar{s} $W-1 (31 or 63),R` | fold in the backend |
| branch variant | `cmp{s} $32|64,C ; jae .Lz ; shl{s} %cl,R ; jmp .Ld ; .Lz: xorl %eax,%eax ; .Ld:` | same result; cmov is shorter and not data-dependent. `cmovae` is baseline (P6+) |
| rotate | not required | `rol/ror` mask the count natively |

## 4. Comparisons and branches

Compare at the exact width, no extension: `mov{b,w,l,q} [a],{%al,%ax,%eax,%rax} ; cmp{b,w,l,q} [b],{%al,...} ; set<cc> %al ; movzbl %al,%eax ; movq %rax,[d]`. [P] all 10 predicates at W=8/16/32/64. The result is a 0/1 i8 (upper bits zero here, but the IR must treat them as unspecified).

| IR pred | cc (set / j / cmov) | | IR pred | cc |
|---|---|---|---|---|
| eq | `e` | | ult | `b` |
| ne | `ne` | | ule | `be` |
| slt | `l` | | ugt | `a` |
| sle | `le` | | uge | `ae` |
| sgt | `g` | | sge | `ge` |

| IR op | Sequence | Notes |
|---|---|---|
| condbr c, T, F (c is i8/bool) | `cmpb $0,[c] ; jne .LT ; jmp .LF` (omit `jmp` when F is the next block) | only the low byte of the slot is defined. [-] |
| fused icmp+condbr (peephole) | `mov{s} [a],R ; cmp{s} [b],R ; j<cc> .LT ; jmp .LF` | optional when the icmp has one use, same block. [-] |
| br L | `jmp .LL` (omit if next block) | [-] |
| switch | compare chain `cmpl $k,%eax ; je .Lk` per case, then `jmp default` | [-] |
| select c,x,y | `movq [y],%rax ; movq [x],%rcx ; cmpb $0,[c] ; cmovneq %rcx,%rax ; movq %rax,[d]` | cmov has no 8-bit form: select on 64-bit moves, any W. [-] |
| zext i1 / bool to int | already 0/1 from `movzbl %al,%eax` | |

## 5. Floating point (SSE2 is baseline x86-64)

f64 slot: `movsd`, suffix `sd`; f32 slot: `movss`, suffix `ss` (4 bytes of the 8-byte slot, upper bytes unspecified). All sequences below exist for both; shown for f64 with the f32 variant in the notes.

| IR op | Sequence (f64) | f32 variant / notes |
|---|---|---|
| fadd/fsub/fmul/fdiv | `movsd [a],%xmm0 ; movsd [b],%xmm1 ; addsd %xmm1,%xmm0 ; movsd %xmm0,[d]` (`subsd` `mulsd` `divsd`) | movss/addss...; AT&T order `op src,dst` so a-b is `subsd %xmm1,%xmm0` with a in xmm0. Divide by zero gives inf/NaN, never traps (MXCSR default masks all). [P] |
| fsqrt | `sqrtsd [a],%xmm0 ; movsd %xmm0,[d]` | `sqrtss`. sqrt(-1) = 0xfff8000000000000 (negative quiet NaN). [P] |
| fneg | `movq [a],%rax ; btcq $63,%rax ; movq %rax,[d]` | f32: `movl [a],%eax ; btcl $31,%eax ; movl %eax,[d]`. Integer-domain sign flip, no constant in memory, exact for NaN and -0. [P] |
| fneg (SSE form) | `movsd [a],%xmm0 ; movabsq $0x8000000000000000,%rax ; movq %rax,%xmm1 ; xorpd %xmm1,%xmm0 ; movsd %xmm0,[d]` | f32: `movl $0x80000000,%eax ; movd %eax,%xmm1 ; xorps`. A `.rodata` mask works only if 16-byte aligned (`xorpd mem` faults otherwise); the immediate route avoids that. [P] |
| fabs | `movq [a],%rax ; btrq $63,%rax ; movq %rax,[d]` | f32: `btrl $31,%eax`. SSE form: mask 0x7fffffffffffffff + `andpd`. [P] |
| fcmp eq | `movsd [a],%xmm0 ; ucomisd [b],%xmm0 ; sete %al ; setnp %cl ; andb %cl,%al ; movzbl %al,%eax` | unordered sets ZF=PF=CF=1, so ZF alone is wrong; PF=0 required. f32: `ucomiss`. [P] with NaN, +-0, inf |
| fcmp ne | `movsd [a],%xmm0 ; ucomisd [b],%xmm0 ; setne %al ; setp %cl ; orb %cl,%al ; movzbl %al,%eax` | NaN != anything is true. [P] |
| fcmp lt (a<b) | `movsd [b],%xmm0 ; ucomisd [a],%xmm0 ; seta %al ; movzbl %al,%eax` | swap operands (compares b with a: b>a) so unordered (CF=1) makes `seta` false, no parity test. [P] |
| fcmp le (a<=b) | `movsd [b],%xmm0 ; ucomisd [a],%xmm0 ; setae %al ; movzbl %al,%eax` | unordered CF=1 -> `setae` false. [P] |
| fcmp gt | `movsd [a],%xmm0 ; ucomisd [b],%xmm0 ; seta %al ; movzbl %al,%eax` | [P] |
| fcmp ge | `movsd [a],%xmm0 ; ucomisd [b],%xmm0 ; setae %al ; movzbl %al,%eax` | [P] |
| fused fcmp+condbr | lt/le/gt/ge: same compare then `ja`/`jae`; eq: `ucomisd ; jne .LF ; jp .LF ; jmp .LT`; ne: `ucomisd ; jne .LT ; jp .LT ; jmp .LF` | never use `jb/jbe/je` alone on ucomisd results. [-] |
| f32 -> f64 | `cvtss2sd [a],%xmm0 ; movsd %xmm0,[d]` | exact. [P] |
| f64 -> f32 | `cvtsd2ss [a],%xmm0 ; movss %xmm0,[d]` | rounds to nearest-even; overflow -> inf. [P] |
| sitofp i8/i16 | `movsbl|movswl [a],%eax ; cvtsi2sdl %eax,%xmm0 ; movsd %xmm0,[d]` | f32: `cvtsi2ssl`. [P] |
| sitofp i32 | `cvtsi2sdl [a],%xmm0 ; movsd %xmm0,[d]` | memory form needs the `l` suffix. [P] |
| sitofp i64 | `cvtsi2sdq [a],%xmm0 ; movsd %xmm0,[d]` | f32: `cvtsi2ssq`. Optional `xorps %xmm0,%xmm0` first removes a false dependency (perf only). [P] |
| uitofp u8/u16 | `movzbl|movzwl [a],%eax ; cvtsi2sdl %eax,%xmm0` | [P] |
| uitofp u32 | `movl [a],%eax ; cvtsi2sdq %rax,%xmm0` | zero-extend to 64 and use the signed 64-bit convert. [P] |
| uitofp u64 | `movq [a],%rax ; testq %rax,%rax ; js 1f ; cvtsi2sdq %rax,%xmm0 ; jmp 2f ; 1: movq %rax,%rcx ; shrq %rcx ; andl $1,%eax ; orq %rax,%rcx ; cvtsi2sdq %rcx,%xmm0 ; addsd %xmm0,%xmm0 ; 2: movsd %xmm0,[d]` | halve with sticky low bit (round-to-odd), convert, double: correctly rounded. f32: `cvtsi2ssq`, `addss`. [P] 20000 random + edge values, bit-exact vs C |
| fptosi f64->i64 | `cvttsd2siq [a],%rax ; movq %rax,[d]` | truncates toward zero. Out of range or NaN -> 0x8000000000000000 ("integer indefinite"), no trap. [P] |
| fptosi f64->i32 | `cvttsd2sil [a],%eax ; movq %rax,[d]` | out of range or NaN -> 0x80000000. [P] |
| fptosi f64->i8/i16 | `cvttsd2sil [a],%eax ; movq %rax,[d]` | convert to 32 then truncate (Go amd64 does the same): 300.0 -> 0x2c, 1e30 -> 0. [P] |
| fptoui f64->u8/u16 | as i8/i16 | [P] |
| fptoui f64->u32 | `cvttsd2siq [a],%rax ; movl %eax,%eax ; movq %rax,[d]` | 64-bit convert, keep low 32: in range for 0<=x<2^32; 5e9 wraps to 0x2a05f200, -1.0 -> 0xffffffff. [P] |
| fptoui f64->u64 | `movsd [a],%xmm0 ; movabsq $0x43e0000000000000,%rax ; movq %rax,%xmm1 ; ucomisd %xmm1,%xmm0 ; jae 1f ; cvttsd2siq %xmm0,%rax ; jmp 2f ; 1: subsd %xmm1,%xmm0 ; cvttsd2siq %xmm0,%rax ; btcq $63,%rax ; 2: movq %rax,[d]` | 0x43e0... = 2^63 as f64; f32 uses `movl $0x5f000000,%eax ; movd %eax,%xmm1`, `ucomiss`, `subss`, `cvttss2siq`. NaN: jae not taken -> 0x8000000000000000; 1e30 -> 0; -1.0 -> 0xffffffffffffffff. [P] in range, bit-exact vs C |

Out-of-range float->int is the one place x86 is not "natural": results above are x86-specific. The IR must say "unspecified value, no trap" (as Go does) or define saturation and pay a compare+cmov sequence per conversion; do not leave it undefined behaviour in the IR sense, because the backends will disagree (arm64 `fcvtzs` saturates and maps NaN to 0).

## 6. Load, store, addresses, aggregates

| IR op | Sequence | Notes |
|---|---|---|
| load i8 / i16 / i32 / i64 (ptr in slot) | `movq [p],%rax ; movzbl (%rax),%ecx` / `movzwl (%rax),%ecx` / `movl (%rax),%ecx` / `movq (%rax),%rcx` ; `movq %rcx,[d]` | zero-extending form is fine (upper bits unspecified anyway); use `movsbq/movswq/movslq` only if the IR has signed loads. [P] |
| store i8 / i16 / i32 / i64 | `movq [p],%rax ; movq [v],%rcx ; movb %cl,(%rax)` / `movw %cx,(%rax)` / `movl %ecx,(%rax)` / `movq %rcx,(%rax)` | store EXACTLY the width, never wider: the neighbours are live data. [P] (immediates form `movb $0x7f,32(%rdi)` etc. also probed) |
| load/store f32, f64 | `movss (%rax),%xmm0 ; movss %xmm0,[d]` / `movsd`; store `movss %xmm0,(%rax)` / `movsd %xmm0,(%rax)` | f64 value in an f32 store is a bug: convert first (probe caught it). [P] |
| load/store with constant offset | `movq [p],%rax ; movl 12(%rax),%ecx` | fold offset into the displacement |
| load/store of a stack slot directly | `movl -40(%rbp),%eax` / `movb %cl,-40(%rbp)` | no lea needed when the slot is accessed by name |
| address of stack slot | `leaq -16(%rbp),%rax ; movq %rax,[d]` | [P] |
| address of global/data label | `leaq sym(%rip),%rax ; movq %rax,[d]` | RIP-relative, works in static non-PIE and PIE (local symbols); `.data` vs `.rodata` vs `.bss` are the section choices. [P] |
| call-address of function | `leaq fn(%rip),%rax` | [-] |
| memcpy, constant n (aggregate copy) | `leaq [dst],%rdi ; leaq [src],%rsi ; movl $N,%ecx ; rep movsb` | `rep movsb` is fast on all modern x86 (ERMS/FSRM); DF must be clear (it is at function entry per ABI). rdi/rsi/rcx are scratch. Pointer operands: `movq [p],%rdi`. [P] (40-byte slot-to-slot, and 0..129 bytes with offsets) |
| memcpy, runtime n (non-overlapping) | `movq [dst],%rdi ; movq [src],%rsi ; movq [n],%rcx ; rep movsb` | [P] |
| memmove (overlap allowed) | `movq [dst],%rdi ; movq [src],%rsi ; movq [n],%rcx ; cmpq %rsi,%rdi ; jbe .Lf ; leaq (%rsi,%rcx),%rax ; cmpq %rax,%rdi ; jae .Lf ; 1: testq %rcx,%rcx ; jz 2f ; decq %rcx ; movb (%rsi,%rcx),%al ; movb %al,(%rdi,%rcx) ; jmp 1b ; .Lf: rep movsb ; 2:` | backward byte loop only when dst is inside (src, src+n). Do not use `std ; rep movsb` (slow, and DF must be restored). [P] overlap offsets -70..+70 |
| memzero / zero-init aggregate | `movq [dst],%rdi ; movq [n],%rcx ; xorl %eax,%eax ; rep stosb` | [P] |
| small constant copy (n<=32, optional) | `movq (%rsi),%rax ; movq %rax,(%rdi)` per 8 bytes + tail | not needed for naive |

## 7. Function prologue, epilogue, frame, alignment

Layout (high to low): `16+8n(%rbp)` stack args n=0,1,.. | `8(%rbp)` return address | `(%rbp)` saved rbp | `-8k(%rbp)` slots k=1..S | `0..8m-8(%rsp)` outgoing stack-arg area (m = max stack args over all calls in the function).

```
f:   pushq %rbp
     movq %rsp,%rbp
     subq $FRAME,%rsp          # FRAME = (8*S + 8*m + 15) & -16
     movq %rdi,-8(%rbp) ...    # spill register params (ints movq; f64 movsd %xmm0,..; f32 movss)
     ...
     movq [ret],%rax           # or movsd [ret],%xmm0
     leave                     # = movq %rbp,%rsp ; popq %rbp
     ret
```

| Rule | Detail |
|---|---|
| Alignment | At function entry `rsp % 16 == 8` (call pushed 8). After `pushq %rbp`, `rsp % 16 == 0`. FRAME a multiple of 16 keeps `rsp % 16 == 0` at every `call`. Because stack args are written with `movq` into the preallocated outgoing area (never `push`), alignment is static, no per-call padding. [P] callees assert `testb $15,%spl` and the call probe passes. |
| `_start` | `rsp % 16 == 0` at the first instruction (kernel guarantee, probed: exit code = rsp&15 = 0), so `call main` directly gives the callee the right alignment |
| Callee-saved | rbx rbp r12-r15 (plus rsp; MXCSR control bits and x87 CW; DF clear on entry and exit). The naive backend uses none except rbp, so no save/restore. A future register allocator must push/pop the ones it uses (and keep alignment) |
| Red zone | 128 bytes below rsp are usable in leaf functions; do not use them (signal handlers are fine, but it buys nothing for a naive backend and breaks if a call is added) |
| Large frames | stack grows on fault; main-thread limit 8 MiB (`ulimit -s`). Overflow = SIGSEGV, exit 139, no message. Frames over about 4 KiB should touch each page in descending order (`orq $0,(%rsp)` loop) so the guard gap (1 MiB default) cannot be jumped; overflow detection with a message needs a `sigaltstack` SIGSEGV handler (out of scope) |
| Over-aligned locals (>16) | `andq $-N,%rsp` after the `subq` and address locals off rsp instead of rbp, or give the IR an alignment-padded slot; asm_ir/1 should cap slot alignment at 16 |
| Params on the stack | the incoming slot at `16+8n(%rbp)` is the parameter vreg storage (callee owns it); no copy |
| Variadic callee | not needed (no libc); calls to C varargs set `%al` (section 8) |

## 8. Calls (System V AMD64)

| Item | Rule |
|---|---|
| Int/pointer args | rdi rsi rdx rcx r8 r9, then stack |
| Float args (f32/f64) | xmm0-xmm7, then stack. Int and float counters are INDEPENDENT; an arg that does not fit goes to the stack, later args may still use free registers |
| Stack args | 8 bytes each, at `0(%rsp)`, `8(%rsp)`... at the call instruction, in ARGUMENT order (not by type), lowest address = first stack arg. The callee sees them at `16(%rbp)`, `24(%rbp)`... f32 on the stack occupies 8 bytes (value in the low 4) |
| Return | ints/ptr: rax (second int rdx); f32/f64: xmm0 (second xmm1); void: nothing |
| Call | `call sym` (direct) or `movq [fp],%r11 ; call *%r11` (r11 is free, r10 is the static-chain reg, rax is needed below) |
| `%al` | number of vector registers used (0-8), required for variadic C callees; set `movl $NXMM,%eax` right before every call to unknown or C code (cheap, always safe) |
| Caller sequence | (1) for each stack arg `movq [x],%rax ; movq %rax,8n(%rsp)`; (2) for each reg arg `movq [x],%rdi` / `movsd [x],%xmm0`; (3) `movl $NXMM,%eax`; (4) `call`; (5) result: `movq %rax,[d]` / `movsd %xmm0,[d]` / `movss %xmm0,[d]` |
| Clobbered by call | rax rcx rdx rsi rdi r8-r11, xmm0-15 (all xmm are caller-saved on SysV), flags. Nothing lives in registers across a call in the naive backend, so nothing to save |
| Narrow ints to/from C | the ABI leaves bits above the type undefined, but clang/gcc callers extend i8/i16 args to 32 bits (signext/zeroext per the C type) and callees rely on it; bool is 0/1 zero-extended to 8 bits. Narrow RETURN values: upper bits of eax are undefined, extend yourself. So calls into C need a per-param `signext`/`zeroext` attribute in the IR, or the IR allows only i32/i64/ptr/f32/f64 across the C boundary |
| IR-to-IR calls | the unspecified-upper-bits convention applies, no extension needed |
| Multi-value return | optional. Rule "ints take rax then rdx, floats take xmm0 then xmm1, in order" matches SysV for {i64,i64}, {f64,f64}, {i64,f64}, {f64,i64}; it does NOT match {i32,i32} or {f32,f32} (SysV packs these in one register), so restrict multi-value returns to 64-bit scalars if C compatibility matters |
| Indirect/closure | `call *%r11`; Go closure ctx register (rdx in Go ABI0) is not SysV; keep the context as an explicit first argument |

Probe: 7 int + 9 double (6 int regs + 1 stack int; 8 xmm + 1 stack double; stack order int7 then d9 and, in the second signature, d9 then int7).

### Aggregates by pointer only: recommendation

Recommend: asm_ir/1 has no by-value aggregate parameters or returns. Aggregate params are passed as a pointer to a caller-owned copy; aggregate returns use a hidden first pointer parameter (caller allocates the result slot, passes it in rdi, callee writes it and may return the pointer in rax). Why: SysV classification (per-eightbyte INTEGER/SSE/MEMORY, 16-byte split across rdi:rsi / xmm0:xmm1 / mixed, alignment and padding rules, unions, nested arrays, long double X87 class) is the only genuinely complicated part of the ABI, and a naive backend gains nothing from it for IR-to-IR calls.

Cost when calling C: (a) C functions taking or returning a struct BY VALUE need a C-side wrapper taking a pointer (a 3-line shim compiled by cc), or asm_ir/1 later gains an explicit `abi` attribute for the 2 easy cases; (b) structs larger than 16 bytes passed by value are the EASY case: class MEMORY, copied into the outgoing stack area at `8n(%rsp)` (16-aligned if the type needs it) with `rep movsb` ("byval"); a C function returning one takes the result address in rdi and returns it in rax, which is exactly the hidden-pointer convention above, so large-struct calls interoperate with C unchanged; (c) only structs of 16 bytes or less need classification: {i64,i64}, {i64,f64}, {f64,f64}, {i32,i32} etc. map to rdi/rsi, rdi/xmm0, xmm0/xmm1, one packed rdi. Runtime services here are raw syscalls, so no libc struct-by-value call is needed (the one realistic case, `div_t`/`timespec`, does not occur). Pointers to aggregates, ints, floats and large-by-pointer cover every C function the runner needs.


## 9. Runtime without libc (Linux x86-64)

| Item | Detail |
|---|---|
| Syscall ABI | `syscall`: number in rax; args rdi rsi rdx **r10** r8 r9 (r10, NOT rcx); result in rax; clobbers rcx and r11; error = rax in [-4095,-1] (test with `cmpq $-4095,%rax ; jae err`) |
| Numbers (x86-64 only; aarch64 and riscv64 use the generic table: write 64, exit_group 94, mmap 222, so the numbers must live in the backend, not in portable IR) | read 0, write 1, open 2, close 3, mmap 9, munmap 11, brk 12, exit 60, **exit_group 231**, clock_gettime 228, getrandom 318 |
| `_start` | `xorl %ebp,%ebp ; movq (%rsp),%rdi (argc) ; leaq 8(%rsp),%rsi (argv; envp follows argc+1 pointers) ; call main ; movl %eax,%edi ; call rt_exit` (rsp is already 16-aligned, no `andq` needed) |
| Link | `as f.s -o f.o && ld.lld -static -z noexecstack f.o -o f` or `cc -nostdlib -static f.o -o f`; give every .s a `.section .note.GNU-stack,"",@progbits` line or the linker may warn about an executable stack |
| rt_exit | `movl $231,%eax ; syscall ; hlt` (code in edi; exit_group ends all threads, plain exit 60 only the calling thread) |
| write(1,"ok 5\n",5) | `movl $1,%eax ; movl $1,%edi ; leaq msg(%rip),%rsi ; movl $5,%edx ; syscall` [P] |
| rt_write_all(fd,buf,len) | `1: testq %rdx,%rdx ; jz 3f ; movl $1,%eax ; syscall ; cmpq $-4,%rax ; je 1b ; testq %rax,%rax ; jle 3f ; addq %rax,%rsi ; subq %rax,%rdx ; jmp 1b ; 3: ret` : retries EINTR, handles short writes (pipes), gives up on error. rdi (fd) survives the syscall. [P] |
| rt_trap(msg,len) | `movq %rsi,%rdx ; movq %rdi,%rsi ; movl $2,%edi ; call rt_write_all ; movl $2,%edi ; jmp rt_exit` : message to stderr, exit code 2 (what a Go panic returns). Stub per trap: `leaq msg_div0(%rip),%rdi ; movl $LEN,%esi ; jmp rt_trap`. `ud2` as the alternative gives SIGILL (exit 132, no message). [P] stderr text and rc=2 |
| Syscall wrapper for IR | `ir_syscall6(n,a1..a6)`: `movq %rdi,%rax ; movq %rsi,%rdi ; movq %rdx,%rsi ; movq %rcx,%rdx ; movq %r8,%r10 ; movq %r9,%r8 ; movq 8(%rsp),%r9 ; syscall ; ret` (the 7th SysV argument is on the stack). [P] |

| mmap bump allocator | state `heap_cur`, `heap_end` in `.bss` (8 bytes each, zero = empty). alloc(size): `addq $15,%rdi ; jc oom ; andq $-16,%rdi ; (min 16) ; movq heap_cur(%rip),%rax ; leaq (%rax,%rdi),%rdx ; jc grow ; cmpq heap_end(%rip),%rdx ; ja grow ; movq %rdx,heap_cur(%rip) ; ret`. grow: chunk = max(size, 64 MiB) rounded up to 4096, then `xorl %edi,%edi ; movq chunk,%rsi ; movl $3,%edx ; movl $0x22,%r10d ; movq $-1,%r8 ; xorl %r9d,%r9d ; movl $9,%eax ; syscall` ; `cmpq $-4095,%rax ; jae oom` ; cur = base+size, end = base+chunk. PROT_READ\|PROT_WRITE = 3; MAP_PRIVATE\|MAP_ANONYMOUS = 0x22 (add MAP_NORESERVE 0x4000 for a huge chunk). Memory is zero-filled and 16-aligned. The tail of the old chunk is abandoned. Not thread-safe. [P] 24-byte block, 100 MiB block (forces a second mmap, checked with strace), zero fill, alignment, no overlap |
| Heap exhaustion | `panic: out of memory` via rt_trap, exit 2 |
| Stack overflow, SIGFPE, SIGSEGV | no handler: the process dies by signal (exit 139 / 136); only division is checked in software |

## 10. Integer to decimal printing

Recommendation: write `print_i64`/`print_u64` ONCE in asm_ir/1 and lower it in every backend. Reasons: it exercises udiv/urem, i8 loads and stores, stack slots, address-of-slot, loops and the write intrinsic on every target, so it doubles as a backend smoke test; a per-backend hand-written copy is N copies of the MinInt and 20-digit edge cases. What must stay per-backend asm: `_start`, `rt_exit`, the raw syscall, and the trap stubs. Give asm_ir/1 either one `syscall(nr, a1..a6)` intrinsic with per-target numbers, or three fixed intrinsics `rt.write`, `rt.exit`, `rt.mmap` (preferred: portable runtime IR never sees a syscall number).

Algorithm for the IR version: 24-byte stack buffer; store newline at the end; magnitude = v (u64) or `0 - v` for negative i64 (neg of MinInt gives 0x8000000000000000, which is the correct u64 magnitude of 9223372036854775808, so no special case); `do { buf[--p] = 48 + urem(m,10); m = udiv(m,10) } while (m != 0)`; prepend `-` if negative; `rt.write(1, &buf[p], end-p)`. 20 digits + sign + newline = 22 bytes, buffer 24 is enough.

Reference x86-64 implementation (hand-written, used by probe 5; r9 = sign carrier, magnitude in rdi):
```
rt_print_i64: movq %rdi,%r9 ; testq %rdi,%rdi ; jns rt_print_core ; negq %rdi ; jmp rt_print_core
rt_print_u64: xorl %r9d,%r9d
rt_print_core: pushq %rbp ; movq %rsp,%rbp ; subq $32,%rsp
  movb $10,-1(%rbp) ; leaq -1(%rbp),%r8 ; movq %rdi,%rax ; movl $10,%ecx
1: xorl %edx,%edx ; divq %rcx ; addl $48,%edx ; decq %r8 ; movb %dl,(%r8) ; testq %rax,%rax ; jnz 1b
  testq %r9,%r9 ; jns 2f ; decq %r8 ; movb $45,(%r8)
2: movq %rbp,%rdx ; subq %r8,%rdx ; movq %r8,%rsi ; movl $1,%edi ; call rt_write_all ; leave ; ret
```
[P] prints 0, -1, -9223372036854775808, 9223372036854775807, 1234567890123 and u64 18446744073709551615 correctly. Found while probing: r8 is clobbered by this routine and by any call, so a caller must not keep a value there across it; the naive backend keeps nothing in registers across calls, so this cannot happen there.


## 11. Pitfalls the IR design must account for

1. idiv faults (SIGFPE) on MinInt/-1 and on /0: both guards are mandatory for W=32/64 sdiv/srem; W=8/16 need only the zero check when computed in 32 bits.
2. Shift counts are masked by hardware (5/6 bits): the IR shift op must define count >= W (Go: 0 / sign fill) and the backend must emit the cmp+cmov. Count type must be unsigned (or the IR needs a separate negative-count trap); a 64-bit count must be compared as 64 bits.
3. Narrow ints: pick "upper bits unspecified, extend at use" and state it in the spec; ops that must extend are exactly div, rem, shr, sar, sext, zext, int->float, and calls into C.
4. Float compares: `ucomisd` alone is wrong for == and != (parity), and for < <= when written as `setb/setbe` (unordered sets CF); use operand swap + `seta/setae`.
5. float->int out of range and NaN produce the x86 "indefinite" value (INT_MIN pattern) and u64 conversions need a branch; i8/i16 convert via 32 bits. The IR must say unspecified-no-trap or saturating.
6. sqrt(-1) and inf-inf produce the NEGATIVE quiet NaN 0xfff8000000000000 on x86; other targets produce positive NaN. NaN bit patterns and payloads are not portable: the IR should not promise them, and golden output must print floats via comparison or classification, not raw bits. `fneg`/`fabs` through bit ops preserve payloads.
7. Alignment: 16 bytes at every `call`; use a fixed outgoing area, never `push`, to keep it static. `xorpd mem` and `movaps mem` fault on unaligned memory, so do not use `.rodata` masks without `.balign 16`.
8. `syscall` clobbers rcx/r11 and takes the 4th arg in r10; SysV calls take it in rcx.
9. Calls into C need `signext`/`zeroext` on i8/i16/i1 params and results (callers extend to 32 bits, callees assume it); `%al` must hold the xmm count for variadic callees.
10. Aggregates: by-pointer only; large by-value is the free MEMORY case; small by-value structs are the only classification work and can be deferred.
11. Exit codes: trap = message on fd 2 + exit_group(2); signals (SIGSEGV on stack overflow, SIGFPE if a guard is missing) are the failure mode the runner should map to "crash".
12. Syscall numbers are per-architecture: runtime IR must not hardcode them; use `rt.write`/`rt.exit`/`rt.mmap` intrinsics.
13. Wide immediates: only `movabsq` takes a 64-bit immediate; `cmpq $imm`, `addq $imm`, `movq $imm,mem` take sign-extended 32-bit immediates, so a 64-bit constant is `movabsq $c,%rax` then use the register.
14. Partial-register writes (`movw [a],%ax`, `setcc %al`) keep the old upper bits: always follow `setcc` with `movzbl %al,%eax`; writes to 32-bit registers zero the upper half, 8/16-bit writes do not.

## 12. Probe log

Host: x86_64 Manjaro, GNU as 2.47, ld.lld 23.1.1, gcc. Probe files were in a session scratch directory, not kept; the op corpus of `docs/asm-ir.md` § 8 replaces them. Compiling steps ran through `slot`. Generated asm is slot-based (-O0 shaped): args spilled to `-8(%rbp)`, `-16(%rbp)`, result `-24(%rbp)`.

| # | Probe | Command | Actual output |
|---|---|---|---|
| 1 | integer ops: add sub mul and or xor andnot neg not sext zext, sdiv srem udiv urem, shl shr sar, 10 compares, W=8/16/32/64, canonical-extension variants, Go shift and MinInt/-1 cases, div-by-zero trap, upper-garbage inputs | `python3 -I gen_int.py && as int_ops.s -o int_ops.o && slot gcc -O1 -static -o int_test int_test.c int_ops.o && ./int_test` | `int probe: 708170 checks, 0 failures` |
| 1m | mutation: drop `cmovaeq` from shl64 | assemble mut1.s, link with int_test.c, run | `FAIL shl64 a=1 b=ffffffff got=8000000000000000 want=0` ... `3221 failures` |
| 1m | mutation: drop the `-1` guard from sdiv64 | `./mut2.bin; echo rc=$?` | no output, `rc=136` (SIGFPE) |
| 2 | floats: add sub mul div sqrt neg abs (bit and SSE forms), 6 compares with NaN/inf/+-0/denormals, f32 and f64, all int<->float conversions incl. u64, f32<->f64 | `python3 -I gen_flt.py && as flt_ops.s -o flt_ops.o && slot gcc -O1 -static -o flt_test flt_test.c flt_ops.o -lm && ./flt_test` | `float probe: 448603 checks, 0 failures`; `oor: d->i64(1e30)=8000000000000000 d->i64(NaN)=8000000000000000 d->i32(1e30)=80000000 d->i32(NaN)=80000000 d->i8(1e30)=0 d->i8(300.0)=2c`; `oor: d->u64(1e30)=0 d->u64(NaN)=8000000000000000 d->u64(-1.0)=ffffffffffffffff d->u32(-1.0)=ffffffff d->u32(5e9)=2a05f200 f->i64(1e30f)=8000000000000000 d->i64(-1e30)=8000000000000000`; `sqrt(-1)=fff8000000000000  inf-inf=fff8000000000000` |
| 2m | mutation: `==` without parity test, `<` as `setb` | `./mutf` | `13 FAIL d_eq`, `12 FAIL d_lt`, `70 failures` |
| 3 | `_start` writes "ok 5\n", exit 0 | `as ok5.s -o ok5.o && ld.lld -static -z noexecstack ok5.o -o ok5 && ./ok5; echo rc=$?` | `ok 5` / `rc=0`. Also `cc -nostdlib -static ok5.o -o ok5_cc && ./ok5_cc` gives `ok 5`, rc=0. strace: `write(...) = 5`, `exit_group(0)`. 1168-byte static ELF |
| 3b | rsp alignment at `_start` | `as rsp.s -o rsp.o && ld.lld -static rsp.o -o rsp && ./rsp; echo $?` | `0` (rsp mod 16 = 0) |
| 4 | call with 7 int + 9 double args, both directions, signature A (i1..i7, d1..d9) and C (interleaved, d9 before i7), asm and C callees, asm callee asserts 16-byte alignment | `python3 -I gen_call.py && as call.s -o call.o && slot gcc -O2 -static -o call_test call_test.c call.o && ./call_test` | `expected 744.25`, then `744.25 ok` for each of C->asm_callee_a, C->asm_callee_c, asm_caller_a->C, asm_caller_a->asm, asm_caller_c->C, asm_caller_c->asm; rc=0 |
| 5 | runtime: print_i64/u64, mmap bump alloc (64 MiB chunk, then 100 MiB forcing a second mmap), div-by-zero trap | `as rt.s -o rt.o && ld.lld -static -z noexecstack rt.o -o rt && ./rt; echo rc=$?` | stdout (one per line) `0 -1 -9223372036854775808 9223372036854775807 1234567890123 18446744073709551615 0 0 0 77 0 1234605616436508552`; stderr `panic: integer divide by zero`; `rc=2`. `./rt 1 2` (no trap) rc=0. strace: `mmap(NULL, 67108864, ...)`, `mmap(NULL, 104857600, ...)`, `exit_group(2)` |
| 6 | memcpy/memmove/memzero (`rep movsb`, `rep stosb`, backward loop), 40-byte aggregate copy, `leaq` of global and stack slot, loads and stores of every width incl. f32/f64 | `as mem.s -o mem.o && slot gcc -O1 -static -o mem_test mem_test.c mem.o && ./mem_test` | `mem probe: 0 failures (memcpy/memmove/memzero/agg40/lea global+slot/loads+stores of every width)` |
| 7 | `ir_syscall6` wrapper (7th arg on stack, r10 shuffle) | `as sc.s -o sc.o && slot gcc -static -o sc_test sc_test.c sc.o && ./sc_test` | `syscall6 ok` / `r=12 ebadf=-9` |

Harness bugs fixed during probing (none changed a documented sequence): a missing 64-bit declaration in probe 1; probe 6 stored an f64 with `movss` (caught, see section 6); probe 5 kept a value in r8 across `rt_print_u64` (section 10).

Not probed: `condbr`/`switch`/`select` sequences (section 4), `rep movsb` performance, the stack-probe loop, `sigaltstack`, floating-point exceptions flags (MXCSR is left at default, all masked).

