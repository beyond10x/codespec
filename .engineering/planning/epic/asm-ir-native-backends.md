---
format: aep.planning-md/3
id: epic:asm-ir-native-backends
kind: epic
status: draft
title: asm_ir/1 and native backends for x86-64, AArch64 and RISC-V 64
summary: cir/1 lowered once into asm_ir/1, a target-independent low-level IR, and written by three template backends as static Linux binaries, held to the conformance kit.
relations:
- informed_by: epic:ecmascript-to-native
- informed_by: epic:low-level-readers
revision: 2
---
# Epic: `asm_ir/1` and native backends for x86-64, AArch64 and RISC-V 64

## Outcome

Any language a reader lifts into `cir/1` runs as a static Linux binary on three architectures,
through one target-independent low-level IR:

```
any language --reader--> cir/1 --codespec lower--> asm_ir/1 --codespec-write-<arch>--> .s --as, ld.lld--> ELF
                                                                x86-64 (native)
                                                                aarch64 (qemu-aarch64)
                                                                riscv64 (qemu-riscv64)
```

- `asm_ir/1` (`crates/asm-ir`): a canonical, strictly read JSON document of typed virtual registers,
  basic blocks, stack slots, loads, stores, calls and four syscalls; no aggregates, no libc.
- `codespec lower` (`crates/lower`): `cir/1` to `asm_ir/1`, owning data layout, Go integer
  semantics, panics, the runtime (written once in `asm_ir`) and refusals.
- `codespec-write-x86-64`, `-aarch64`, `-riscv64` (`adapters/asm`): template expanders, one per
  architecture; `codespec-run-asm` (`adapters/asm-run`) runs a kit case on each.
- Each writer is held to the conformance kit by `codespec conform` against `expected.txt`, and the
  three against each other.

Design: [docs/asm-ir.md](../../../docs/asm-ir.md), with the research notes under `docs/asm-ir/`.

## Why Now

Asked by the owner on 2026-11-25: "create a writer for assembler for this particular architecture"
(the host, x86-64), then "maybe design an IR in between so we have multiple architectures", then
"so [any_lang] -> [codespec] -> [asm_ir/1] -> [any_arch]". The three targets disagree on division
by zero, `MinInt / -1`, shift counts, float-to-int and the passing of aggregates (probed on the
host, `docs/asm-ir/x86-64-mapping.md`, `docs/asm-ir/arm-rv-mapping.md`); a direct writer per
architecture would repeat every one of those decisions.

## Done When

- All 11 stories that decompose this epic are implemented.
- `codespec conform --role writer` passes 122 of the kit's 145 cases on x86-64 (groups 1 to 7 of
  `docs/asm-ir.md` § 5), and the cross-check of `story:asm-kit-conformance` holds the AArch64 and
  RISC-V 64 outputs byte-equal to it on every case the three claim.
- The `asm_ir` interpreter prints `expected.txt` for the 78 cases of group 1.
- Every gate step ends within 2 minutes; no test runs over 60 s.

## Out of Scope

- The 23 kit cases that need function values, interfaces, `any`, channels, generics, externs,
  `foreign` or `raise`: a later epic.
- Register allocation and any optimisation; a C-ABI entry point; a fourth architecture; another
  operating system.
- WebAssembly: `story:wasm-writer` lowers from `cir/1` directly (`docs/asm-ir.md` § 9).
- The bench, the containers and `intrinsic.math` of `epic:ecmascript-to-native`:
  `story:x86-64-writer` builds them on `codespec-write-x86-64` from this epic.
