---
format: aep.planning-md/3
id: story:rust-reader-refused-let-cascade
kind: story
status: draft
title: A refused let still binds its name, so a refusal names its own cause
summary: A refused let keeps its name bound when its type is known, so later statements are read or refused for their own reason.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: A refused let still binds its name, so a refusal names its own cause

## Outcome

When the Rust reader refuses a `let` whose binding type it can still tell (a written annotation,
or a result type it reads), the name stays bound with that type, and the statements after it are
read or refused for their own reason. Where it cannot tell the type, each later use is refused
naming the refused `let` (`` `x`, bound by the refused let at lib.rs:2 ``), not as an unknown name.

## Why

Consumer report on 0.12.0 from an async network service: 28 refusals of "a local whose let was
refused". On main `08e7fde13` (2026-11-24) the service gives 18 refusals "`x`, which is neither a
local, a constant, a static nor a function". Reproduced with a crate written from scratch:

```rust
pub fn shout(s: String) -> i64 {
    let words: Vec<String> = s.split(' ').map(|w| w.to_string()).collect();
    let n = words.len() as i64;
    let first = n + 1;
    let second = first * 2;
    second + n
}
```

Five refusals for one cause: `.split()` on String, then "`words`, which is neither a local…",
then the same for `n`, `first` and `second`. A function is refused by its first refusal either
way, so this blocks no further function; it inflates the refusal counts that rank the stories of
`epic:async-network-service-translation` and hides the second real cause of a function (here
none, but a `let` after the cascade that needs another story is reported as an unknown name).

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `shout` reads with one refusal
  (at line 2) and the statements after it in `cir/1`, red on main; a refused `let` without a
  type the reader can tell gives refusals that name it.
- `codespec measure` over the reproducers of this epic counts no "neither a local" refusal whose
  cause is a refused `let`.

## Scope

- `adapters/rust-read/src/body.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/cascade/` (not committed).
