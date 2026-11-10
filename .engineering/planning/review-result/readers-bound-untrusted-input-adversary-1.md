---
format: aep.planning-md/3
id: review-result:readers-bound-untrusted-input-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on readers-bound-untrusted-input
relations:
- reviews: story:readers-bound-untrusted-input
revision: 1
---
**Verdict:** I broke it in two places. The Rust reader now refuses ordinary, shallow sources: about 43 doc-commented functions in a row are enough. The C reader's memory still grows with the number of `.c` files, and 16 files of 19 bytes each made it abort.

```
unit: commit 0cc4f31 on impl/readers-bound-untrusted-input (tree rd-bound, two untracked test files added)
verdict: NEEDS-CHANGE
cases: executed 7→10, red 3
origin: introduced 1 / pre-existing 1 / undecided 1
wrote-outside-worktree: 9 paths (scratch adv/, listed in 6)
needs-coordinator: none
```

### 1. Diff stat
```
 website/data/conformance/reader-c.json      | 2 +-
 ... (9 website/data/conformance/*.json, 1 line each)
?? adapters/c-read/tests/adversary_bounds.rs
?? adapters/rust-read/tests/adversary_bounds.rs
```
The 9 `website/` changes were already in the tree's git status before I started; I didn't make them. My only writes are the two untracked test files. I changed no implementation file.

### 2. Cases added (each run alone first, all red now)
- **`adapters/rust-read/tests/adversary_bounds.rs::sixty_documented_functions_read`**: 60 `/// doc` + `pub fn` items should give exit 0.
  `documented-functions: exit 0, got exit status: 1: codespec-read-rust: lib.rs:212: nested deeper than 256 levels, this reader's limit`
- **`adapters/rust-read/tests/adversary_bounds.rs::a_hundred_derived_structs_read`**: 100 `#[derive(Clone, Debug)] pub struct` items should give exit 0.
  `derived-structs: exit 0, got exit status: 1: codespec-read-rust: lib.rs:427: nested deeper than 256 levels, this reader's limit`
- **`adapters/c-read/tests/adversary_bounds.rs::sixteen_files_including_one_header_stay_within_a_fixed_memory_budget`**: 16 `.c` files that each `#include "big.h"` (60 000 `int vN;`, about 600 kB), run under `ulimit -v` 3 GiB. It should end in exit 0 or a one-line refusal.
  `exit 0 or a one-line refusal, got signal: 6 (SIGABRT): memory allocation of 224 bytes failed`

### 3. Suite run (after the cases existed)
`cargo test --no-fail-fast -p codespec-read-rust --locked --test bounds --test adversary_bounds`:
- `bounds`: `test result: ok. 4 passed; 0 failed`
- `adversary_bounds`: `test result: FAILED. 0 passed; 2 failed`
- `exit=101`

`cargo test --no-fail-fast -p codespec-read-c --locked --test bounds --test adversary_bounds`:
- `bounds`: `test result: ok. 3 passed; 0 failed`
- `adversary_bounds`: `test result: FAILED. 0 passed; 1 failed; ... finished in 45.35s`
- `exit=101`

"Before" (7) is the two `bounds.rs` files alone, my files not counted.

### 4. Findings
- **`adapters/rust-read/src/lib.rs:411` (NEEDS-CHANGE, introduced).**
  - **What's wrong:** the count only resets at a `}` when an identifier other than `else`/`as` follows it. An outer attribute starts with `#`, and so does a `///` doc comment once tokenised (`#[doc=…]`). A `}` followed by `#` therefore never resets, and each attributed or documented item adds 3 to 6 levels to the next one.
  - **Measured:** probes at 40/60/80/100 documented functions gave exit 0 at 40 and the refusal from 60 up; the first refused line is function 43.
  - **What reaches it:** any normal `lib.rs` with doc comments or `#[derive]`/`#[test]` on consecutive items.
  - **Fix:** also reset when the next token is `Punct('#')`. After a block, `#` cannot continue an expression.
- **`adapters/c-read/src/lib.rs:566` (CONFIRMED, pre-existing).**
  - **What's wrong:** the 64 MiB JSON limit applies per `.c` file, but every file's parsed tree stays in `asts` until the module is assembled. The 16 MiB source limit doesn't help: the 16 `.c` files are 19 bytes each.
  - **Measured:** peak RSS was 312 MiB with 1 file, 1091 MiB with 4, and an abort near 2.7 GB with 16.
  - **Origin:** the base `c64cc86:adapters/c-read/src/lib.rs:492-494` also keeps every tree, so the behaviour is older than this unit. But the crate doc this unit added at :252 says "what it can cost the reader is bounded", and that is false for a directory.
  - **What reaches it:** any source directory with several `.c` files that include a large header.
- **Java reader with deep unbracketed nesting (INFEASIBLE, undecided; no file:line).**
  - **What's wrong:** with 200 000 `a + …` terms, ternaries or `else if`s, `codespec-read-java` neither ends nor refuses within 170 s.
  - **Measured:** `java-binary`, `java-ternary` and `java-elseif` all hit `timeout` after 170 s.
  - **Why INFEASIBLE:** no timing claim exists to test against, and I didn't run the base.

### 5. Attacked and not broken
- **Rust count missing real nesting:** string, raw-string and char literals and plain comments are not counted. The resets on `;`, `,`, `=>` and `}`+identifier didn't let deeper nesting through in any construction I tried. Flat `match`/`format!` lowering doesn't deepen the module.
- **Rust over-counting:** `&&`, `->` and method chains count about twice their real nesting, and `'static` counts as a keyword. This is documented and conservative; I didn't write a test for it.
- **C JSON depth scan:** it handles escaped quotes and brackets inside strings correctly.
- **C source-size sum:** the `take(left+1)` / `checked_sub` arithmetic refuses exactly one byte past the limit.
- **C clang kill:** stderr is drained on its own thread, then `wait` reaps clang, so no zombie and no pipe deadlock. `#include "/dev/zero"` reads as empty (exit 0).
- **C self-include:** refused, but as clang's 409-line message ("clang rejects f.c:" + stderr). That format predates this unit and isn't a limit refusal.
- **JVM readers:** Java unary, Groovy binary and Groovy `else if` chains each give exit 1 with one line. Groovy unary is a parse error, also one line.
- **Not attempted:** C deep unary/ternary/`else if` near the JSON limit, the macro-expansion bomb, and the 16-file case without the memory cap.

### 6. Paths written outside the worktree
In `/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/bound/adv/`: `probe_rust.py`, `probe_c.py`, `probe_jvm.py`, `multiply16.log`, `jvm.log`, `rust-adv.log`, `c-adv.log`, `rust-suite.log`, `c-suite.log`. I removed the probe source directories.

Inside the tree, under `target/` (gitignored), the tests left `target/c-read-tests/adversary-bounds` (768K) and `target/rust-read-tests/adversary-bounds-*` (36K).

### 7. Findings block

```findings
- file: "adapters/rust-read/src/lib.rs"
  line: 411
  category: "acceptance"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "The nesting count never resets at a block followed by an attribute or doc comment, so 60 documented functions or 100 derived structs are refused as nested deeper than 256 levels (adversary_bounds.rs: lib.rs:212 and lib.rs:427)."
- file: "adapters/c-read/src/lib.rs"
  line: 566
  category: "acceptance"
  severity: "warning"
  verdict: "CONFIRMED"
  origin: "pre-existing"
  message: "Every .c file's tree is kept until assembly, so memory grows with the file count under the per-file 64 MiB JSON limit; 16 files of 19 bytes including one 600 kB header abort with SIGABRT under 3 GiB, while the new crate doc says the cost is bounded."
- file: "adapters/java/src/codespec/read/Reader.java"
  line: 670
  category: "judgement"
  severity: "note"
  verdict: "INFEASIBLE"
  origin: "undecided"
  message: "codespec-read-java neither ends nor refuses within 170 s on 200 000-term binary, ternary or else-if chains; there is no timing claim to test against and the base was not run."
```
