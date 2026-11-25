---
format: aep.planning-md/3
id: review-result:cir1-checked-integer-operators-r1
kind: review-result
status: active
title: Adversary, round 1, design:cir1-checked-integer-operators
summary: 'needs-revision: nine findings on the checked integer operators note'
relations:
- reviews: design:cir1-checked-integer-operators
revision: 1
---
needs-revision

Adversary round 1 on the `## Note` of `design:cir1-checked-integer-operators` at `71938982`
(revision 3 of the artifact). Nine findings, F1 to F9; the coordinator's decision for each is in
its message (F8 and F9 adopt the adversary's proposal for the default without `--profile`).
Revision 2 of the note answers each, tagged with its number, mapped in its § 12.

```findings
[
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":106,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F1: `v.len() - 1` was typed signed platform, so `-!` would never panic at 0; len() is now usize on the integration branch; rule: a checked op only where the operand's cir/1 type is its Rust type, else refuse"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":134,"category":"contract","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F2: `%!` dropped sem.rem.sign from features_used, so conformance/reader/rust/sem-rem-sign would lose its subject; `%!` keeps sem.rem.sign and the case is listed"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":196,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F3: package override keys are package ID specs (name@version, registry+...#name@...); match the spec forms Cargo accepts and treat a key the reader cannot resolve as unknown"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":196,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F4: precedence was wrong: a package override (manifest or config) beats a profile-level setting, and at profile level config beats manifest; write Cargo's order exactly"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":202,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F5: which .cargo/config.toml applies depends on cargo's working directory, so any config under the root that sets a profile key or rustflags, profile rustflags, and -C overflow-checks/-C debug-assertions in a rustflags source make the setting unknown"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":184,"category":"correctness","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F6: the building root decides the profile, not the crate's own manifest; codespec-run-rust and the inventory driver build the subject as a path dependency; say how conform and run use build and that the reader reads the root it is pointed at"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":295,"category":"contract","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F7: surfaces missing: .github/workflows/release-build.yml:88-89, Taskfile.yml:336, examples/edit and crates/cli/tests/fixtures/edit/rust, crates/edit/src/run.rs:1162 (reader run with --spans and no profile)"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":181,"category":"design","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F8: refusing every profile-dependent operator without --profile breaks edit and the release workflow; resolve every profile the root defines and use the value when all agree"},
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":181,"category":"design","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F9: the default: without --profile, all profiles (dev, test, release, bench, custom with inherits) agreeing gives the value and build without profile, disagreeing refuses; --profile pins one; no hard-coded debug"}
]
```
