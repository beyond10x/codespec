---
format: aep.planning-md/3
id: review-result:asmir-acceptance-round-2
kind: review-result
status: active
title: 'asm_ir plan: acceptance critic, round 2'
relations:
- reviews: story:asm-ir-model
- reviews: story:asm-ir-lower-core
- reviews: story:asm-x86-64-backend
- reviews: story:asm-x86-64-writer
- reviews: story:asm-aarch64-backend
- reviews: story:asm-riscv64-backend
- reviews: story:asm-kit-conformance
- reviews: story:asm-ir-lower-slices-strings-errors
- reviews: story:asm-ir-lower-floats
- reviews: story:asm-ir-lower-maps
- reviews: story:asm-ir-interpreter
- reviews: epic:asm-ir-native-backends
revision: 1
---
needs-revision

story:asm-x86-64-writer — the Outcome promises that `codespec write --lang x86-64` finds the writer by name and that the gate runs the conform command as a part of `scripts/check-parts.txt`, and no Done When bullet checks either, so both can be missing at "done" (the conform bullet runs the command by hand; the part bullet only states a time) — .engineering/planning/story/asm-x86-64-writer.md:35 and :44
story:asm-kit-conformance — the Outcome promises that `website/data/status.json` lists the three writers and that "one site page links `docs/asm-ir.md`", and the only site bullet ("`codespec-docs generate --check` covers the three writers; the support matrix lists their claims") checks neither the status file nor the page link — .engineering/planning/story/asm-kit-conformance.md:39

Round 1 re-check (all 9 findings):
- Landed: x86-64-backend 41-operation test and the two header/feature bullets (lines 40, 20); aarch64 and riscv64 "fail, never pass by skipping"; kit-conformance CI install now says "installed by the backend stories"; the exit 0/2/1 and `--asm-ir` paths moved to x86-64-writer with a test each; lower-floats checked-in `float-format.txt` written by a Go program; interpreter now names `index.txt` and `group1.txt`; the epic has a Done When with counts (122 of 145, 78, 2 min / 60 s).
- Counts agree across the set: 78 + 33 (7 + 11 + 2 + 13) = 111; + 4 = 115; + 7 = 122.

What I read: 11 of 11 ids (the epic and its 10 stories), each in full with `aep plan artifact show <id>`, plus `aep plan artifact show review-result:asmir-acceptance-round-1`, the critic rubric, and `grep -n` over the two story files for the line citations.

What I could not establish: none that sets the verdict. Not findings: the interpreter's `index.txt` set grows as the aarch64 and riscv64 stories add modules, but its test iterates the file, so it stays checkable. "The 78 drivers are built in one batch" (x86-64-writer) is checkable only by reading the code. Out of lane, not setting the verdict: `story:asm-ir-interpreter` depends only on `story:asm-x86-64-writer`, but the AArch64 and RISC-V modules it would also run are added by later stories (design/parallel-safety); several stories write `adapters/asm`, `adapters/asm-run` and `scripts/ci-tools.sh` (shared surfaces).

```findings
- file: .engineering/planning/story/asm-x86-64-writer.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises that `codespec write --lang x86-64` finds the writer by name and that the gate runs `codespec conform --lang x86-64 --role writer` as a part of `scripts/check-parts.txt`, and no Done When bullet checks either"
- file: .engineering/planning/story/asm-kit-conformance.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises that `website/data/status.json` lists the three writers and one site page links `docs/asm-ir.md`, and the Done When bullet on `codespec-docs generate --check` and the support matrix checks neither"
```
