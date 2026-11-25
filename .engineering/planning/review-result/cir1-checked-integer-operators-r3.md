---
format: aep.planning-md/3
id: review-result:cir1-checked-integer-operators-r3
kind: review-result
status: active
title: Adversary, round 3, design:cir1-checked-integer-operators
summary: 'approve: one note, R3-N1, taken'
relations:
- reviews: design:cir1-checked-integer-operators
revision: 1
---
approve

Adversary round 3 on the `## Note` of `design:cir1-checked-integer-operators` at `38706745`
(revision 5 of the artifact). Every round-2 finding is answered; probes p1, p3, p7, p8 and p9
agree with Cargo. One non-blocking note, R3-N1, taken by the coordinator and answered in
revision 4 of the note (§ 5.5, test 4, § 12): only the keys and flags that chain into
`overflow-checks` make the setting unknown.

```findings
[
{"file":".engineering/planning/design/cir1-checked-integer-operators.md","line":270,"category":"design","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"R3-N1: any profile key or rustflags under the root making the setting unknown leaves a repository with [build] rustflags = [\"-C\", \"target-cpu=native\"] or [profile.release] lto = true unreadable; taken: only overflow-checks, debug-assertions, opt-level profile keys, [profile.*] rustflags, and rustflags containing -C overflow-checks, -C debug-assertions, -C opt-level or -O make it unknown; any other key or flag is ignored; a vendored crate's config counts only if it sets one of those"}
]
```
