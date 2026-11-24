---
format: aep.planning-md/3
id: review-result:cir1-checked-integer-operators-r2
kind: review-result
status: active
title: Adversary, round 2, design:cir1-checked-integer-operators
summary: 'needs-revision: six findings on revision 2 of the checked integer operators note'
relations:
- reviews: design:cir1-checked-integer-operators
revision: 1
---
needs-revision

Adversary round 2 on the `## Note` of `design:cir1-checked-integer-operators` at `52875083`
(revision 4 of the artifact). F1 to F9 of round 1 hold. Six new findings (two blockers, two
warnings, two notes), probes under `target/adv/r2/` of the adversary's tree. The coordinator also
adopted one usability change: a one-line standard-error summary when profile-dependent operators
are refused. Revision 3 of the note answers each, tagged R2-B1 to R2-N2, mapped in its § 12.

```findings
[
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":203,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"R2-B1: a crate without [workspace] under a workspace that does not list it (probe p9; reader.rs spans-*, whole_crate.rs, bounds.rs, adversary_bounds.rs, edit's read_spanned) must never fail the read; a failed root lookup makes the setting unknown and the refusal names --root and --profile"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":204,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"R2-B2: membership follows Cargo: members with globs and exclude plus path dependencies of members under the root (probe p1); undecidable membership makes the setting unknown"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":243,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"R2-W1: a .cargo/config(.toml) anywhere under the root applies when cargo runs there (probe p7); read every one under the root, any profile key or rustflags makes the setting unknown"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":271,"category":"contract","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"R2-W2: callers missing: Taskfile.yml:687-688 (examples-inventory byte compare; inventory has + - * +=) and every reader test file that reads crates (evaluation.rs, whole_crate.rs, adversary1/2.rs and the rest) with what each passes"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":237,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"R2-N1: two package keys matching one crate make Cargo refuse the manifest (probe p3); the reader treats it as unknown"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":237,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"R2-N2: version.workspace resolves from the crate's own workspace, independent of --root"}
]
```
