# Wave plan: `asm_ir/1` and native backends

Epic: `epic:asm-ir-native-backends`. Design: `docs/asm-ir.md`. Critic panel: two rounds
(`review-result:asmir-*-round-1`, `-round-2`), every finding recorded `fixed`; no third round
(rule of `aep:planning` § 7), so the round-2 fixes are not re-reviewed.

Derived by hand from the `depends_on` edges and the typed `scope` of the 11 stories; `aep plan
artifact waves --status active` cannot derive it yet because no story is accepted. Every pair in
one wave below has disjoint typed scope and no edge between them.

| wave | story | objective | state |
|---|---|---|---|
| 1 | story:asm-ir-model | `crates/asm-ir`: the `asm_ir/1` types, strict reading, schema, builder; the three skeleton crates registered | proposed |
| 2 | story:asm-ir-lower-core | `codespec lower`: group 1 (78 kit cases) and the runtime | planned |
| 2 | story:asm-x86-64-backend | x86-64 `emit`, `codespec-run-asm --module`, the op corpus | planned |
| 3 | story:asm-x86-64-writer | `codespec-write-x86-64`: the first `cir/1` to ELF; `conform x86-64` in the gate | planned |
| 4 | story:asm-aarch64-backend | AArch64 under `qemu-aarch64` | planned |
| 4 | story:asm-ir-lower-slices-strings-errors | groups 2 to 5, 111 cases | planned |
| 5 | story:asm-riscv64-backend | RISC-V 64 under `qemu-riscv64` | planned |
| 5 | story:asm-ir-lower-floats | group 6, 115 cases | planned |
| 6 | story:asm-kit-conformance | AArch64 and RISC-V 64 writers, the cross-check, the site data | planned |
| 6 | story:asm-ir-lower-maps | group 7, 122 cases | planned |
| 7 | story:asm-ir-interpreter | the `asm_ir` interpreter | planned |

`story:x86-64-writer` (epic `epic:ecmascript-to-native`) comes after wave 7 and after the
container and bench stories it depends on; it is not in this plan.

## Wave 1 (proposed, waiting for the owner's word)

The owner chose on 2026-11-26 about 00:10 CET (relayed by the host operator): "B: PR the plan
only". The plan is committed with every story in `draft`; no story is accepted and wave 1 does not
run until the owner says so.

- Unit: `story:asm-ir-model`, N = 1. Wave 1 is one story because every later story builds on its
  crate and its workspace registration.
- Scope (typed, cited from the body): `crates/asm-ir`, `schemas/asm-ir-1.schema.json`,
  `crates/cli`, `crates/lower`, `adapters/asm`, `adapters/asm-run`, `Cargo.toml`, `Cargo.lock`,
  `scripts/check-parts.txt`, `AGENTS.md`.
- Dispatch: `aep:implementor` (model sonnet, as the owner asked on 2026-11-25: "do basic stuff
  with sonnets"), then `aep:adversary` (model sonnet) on the green unit.
- Terminal move: `active -> implemented`; evidence `test_result` from `task check` on the merged
  head.
- Pre-flight: build and test time not measured; the story's own budget is 60 s per test and
  120 s for the step.

Commits approval authorises: (1) one planning commit on `feat/x86-64-writer` holding
`docs/asm-ir.md`, `docs/asm-ir/*`, the store's new and changed artifacts and evidence, and this
page; (2) one unit commit for `story:asm-ir-model`; (3) one closing store commit (evidence, move
to `implemented`, this page); (4) push of the branch as `b10x-bot[bot]` and one pull request to
`main`. Nothing else.

| unit | branch | head | worktree | stage |
|---|---|---|---|---|
| story:asm-ir-model | feat/x86-64-writer | — | codespec-x86-64-writer (managed) | proposed |
