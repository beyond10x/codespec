---
format: aep.planning-md/3
id: review-result:security-writers-validate-names-pass-2
kind: review-result
status: active
title: 'Security review, pass 2: story:writers-validate-names'
relations:
- reviews: story:writers-validate-names
revision: 2
---
unit: story:writers-validate-names, security review pass 2, tree at b2abea1 plus 4 untracked secreview2 test files
verdict: NEEDS-CHANGE
cases: executed 194→202 (cargo: model, C writer, Rust writer), Go 101→188 (test and subtest runs); red 6
origin: introduced 5 / pre-existing 6 / undecided 0
wrote-outside-worktree: 1 directory, the assigned scratch directory (contents in part 6)
needs-coordinator: the right fix for S2 is probably in `adapters/c-run` (`-iquote` instead of `-I`), which is outside this unit's scope. F5 (Groovy reader) was not checked.

All of pass 1's security fixes hold, including every C-comment variant you listed. The new gaps are two module names that a toolchain treats specially (S1, S2), raw C1 control characters in `refusals.json` (S3), and some guards no test exercises.

**1. `git --no-pager diff --stat`**: empty. No tracked file changed. Four new untracked files: `adapters/c/tests/secreview2.rs`, `adapters/rust/tests/secreview2.rs`, `crates/model/tests/secreview2.rs`, `adapters/go/cmd/codespec-write-go/secreview2_test.go`.

**2. Cases added.** Each file was run alone before any suite run. Output as captured:

| Case | Now | Output when run alone |
|---|---|---|
| C `secreview2_no_doc_text_leaves_its_comment_under_any_c_standard_or_trigraph_setting` | green | 36 hostile docs, all written; gcc and clang × c99/c11/gnu11 × with and without `-trigraphs`; no marker symbol in any object |
| C `secreview2_a_module_named_like_a_c_standard_header_builds_with_the_runners_include_path` | red | `module "stdint" was written as stdint.h, which shadows <stdint.h>: …/codespec_rt.h:43:15: error: unknown type name 'uint64_t'`; the same for string, stdio, stdlib, stddef, stdbool |
| C `secreview2_refusals_json_carries_no_c1_or_format_character_from_the_document` | red | `refusals.json holds ["U+009B", "U+009D", "U+009C", "U+202E"] raw`; stderr has them escaped |
| Rust `secreview2_refusals_json_carries_no_c1_or_format_character_from_the_document` | red | the same four characters |
| Rust `secreview2_every_admitted_name_is_refused_or_compiles` | green | 15 of 25 names written; each passes `rustc --emit=metadata` |
| Rust `secreview2_no_doc_text_becomes_an_item` | green | 10 docs, all written; no marker for syn, and none in `nm` of the rustc object |
| model `secreview2_invisible_default_ignorable_characters_are_not_names` | red | `admitted: ["U+115F", "U+1160", "U+3164", "U+FFA0", "U+034F", "U+180B", "U+FE00", "U+FE0F", "U+E0100", "U+17B4"]` |
| model `secreview2_a_position_holding_a_format_character_is_refused` | green | |
| Go `TestSecreview2APackageClauseIsNeverOneTheGoCommandIgnores` | red | `module "documentation": the go command finds no package in what was written: no buildable Go source files in … (ignored: [documentation.go])` |
| Go `TestSecreview2AModuleFileIsNeverAWindowsDeviceName` | red | `module "con" was written as con.go, a device name on Windows`; the same for nul, aux, prn, com1, lpt1, CON |
| Go `TestSecreview2NoDocLineBecomesADirective` (44 subtests) | green | |
| Go `TestSecreview2NoDocBecomesACgoPreamble` | green | the file was written with `import "C"`; the docs sit before the package clause and before `func`, never on the import |
| Go `TestSecreview2EveryAdmittedNameIsRefusedOrTypeChecks` | green | 7 names written, each type-checks |
| Go `TestSecreview2AModuleNameWithThisPlatformsSuffixIsRefused` | green | Added to catch mutant A. Exit 0 on the tree, exit 1 on the mutant copy |

**3. Suite runs, after the cases existed:**
- `cargo test -p b10x-codespec-model -p codespec-write-c -p codespec-write-rust --no-fail-fast`: exit 101, 198 passed, 4 failed, 6 ignored. All 4 failures are mine.
- The same with `-- --skip secreview2`: exit 0, 194 passed, 6 ignored.
- `go test ./cmd/codespec-write-go -count=1 -v`: exit 1, 188 runs, 10 FAIL lines (my 2 tests and their 8 subtests).
- The same with `-skip TestSecreview2`: exit 0, 101 runs.
- Disk: 32 GB free before, 30 GB after.

**4. Findings** (tree b2abea1)

- **S1** (warning, NEEDS-CHANGE, pre-existing), `adapters/go/cmd/codespec-write-go/write.go:89`. A module named `documentation` is written as `package documentation`, and the go command ignores that file (`go/build/build.go:942`, `cmd/go/internal/modindex/read.go:549`).
  - Measured: the red case in row 9 of the table.
  - What reaches it: any reader that names a module `documentation` (a Rust crate, a Java package). The base had no module-name check, so this predates the unit. It is a neighbour of pass-1 S3 that `plainGoFile` misses: the problem is the package clause, not the file name. The commit message's claim ("a module name whose file the go command would skip … is refused") does not hold.
  - Fix: refuse `m.Name == "documentation"`.
  - Breaks: "a file the toolchain treats specially".
- **S2** (warning, NEEDS-CHANGE, pre-existing), `adapters/c/src/write.rs:813`. The module name becomes the file name `<m>.h`. The C runner compiles with `-I <out>` (`adapters/c-run/src/lib.rs:673`), so `#include <stdint.h>` in `codespec_rt.h` (and glibc's own includes) finds the module's header instead. The build fails; I found no case where it succeeds with a changed meaning.
  - Measured: the red C case in row 2. At the base, `stdint` was also written as `stdint.h` (`git show 0a914ea`, line 403).
  - Fix: `-iquote` in c-run, or refuse names of standard headers (also `features`, `stdarg`, `limits`).
- **S3** (note, CONFIRMED, pre-existing), `adapters/rust/src/write.rs:350` and `adapters/c/src/main.rs:114`. `refusals.json` holds raw C1 controls (U+009B is CSI) and bidi characters from the document's foreign `reason`, because serde_json escapes only C0. The unit escapes stderr but not this file.
  - Fix: pass reasons through `printable`, or have strict reading refuse control characters in `reason`.
- **S4** (note, CONFIRMED, pre-existing), `crates/model/src/lib.rs:263`. `is_name` admits invisible default-ignorable characters: Hangul fillers (Lo), variation selectors and the combining grapheme joiner (Mn). Rust and Go write them (`ㅤ`, `a\u{fe0f}`), so two names can look identical. This is display only, the same family as N5.
- **S5** (note, INFEASIBLE, pre-existing), `write.go:89`. Windows device names (`con`, `nul` …) become `con.go`, and in the C writer `con.h`/`con.c`. This only matters on Windows before 11, and nobody is shown running the writers there.
- **Mutants the suite does not catch** (all in this unit's new code; all note):
  - `write.go:82`, the recover path's position: never reached any more, because the empty-assign refusal comes first.
  - `write.go:245`, host GOOS/GOARCH instead of `codespec`: now caught by my green case.
  - `lib.rs:303`, Cf escaping in `printable`: no test checks a Cf character in a message.
  - `lib.rs:1485/1489`, `check_text`'s extern-expression branch, `unordered_iteration` and extern `type`: adversary1's case still passes through `printable`.
  - `write.rs:1693`, the blank-name uniqueness loop: no test names a parameter or local `_cs_p<i>`.

**Pass-1 findings:**
- Security pass 1:
  - S1 closed (36 docs × 12 compiler settings).
  - S2 closed.
  - S3 closed for file-name suffixes; its neighbour is open as the new S1.
  - N1 and N2 closed.
  - N3, N4, N5 are assigned to the distinct-names draft and were not re-checked.
- Adversary pass 1:
  - F1, F4, F8 and F11 closed.
  - F12 closed, but the recover-path position is untested (mutant row above).
  - F5 not checked.
  - F2, F3, F6, F7, F9, F10 and F13 are assigned elsewhere.

**Sink table, updated:**

| Sink | Status |
|---|---|
| C doc comment `write.rs:337` | escaped, complete (checked with compilers) |
| C banner and include guard | validated name |
| C file names `write.rs:813` | validated as an identifier; shadows standard headers (S2) |
| Go doc comment `write.go:223` | complete; no directive, `+build`, `//line` or cgo preamble reachable |
| Go file name and package clause `write.go:89` | `documentation` open (S1); device names (S5) |
| Go recover `write.go:78` | escaped by `run()`; position untested |
| Rust blank parameters `write.rs:1665` | no document text reaches them |
| `Cargo.toml` | `name` only; validated identifier, cannot break the TOML string |
| `refusals.json`, Rust and C | C0 escaped, C1 and Cf raw (S3) |
| stderr, all three writers and strict reading | escaped |
| CLI | prints only the strictly-read name, positions, extern names and `ReadError` (escaped) |

**Mutants:**
- Go, run on a scratch copy with secreview2 removed:
  - Caught: C (empty-assign refusal), E (`+build` check), F (`_test` suffix), G (whole `plainGoFile`), H (`printable`).
  - Survived: A (host platform), B (recover position), D (`go:build` check, which is equivalent because every line starts with `// `).
- Rust, C and model, reasoned from the tests' assertions and not run:
  - Every C `comment()` transform is caught. The `/*` break is caught through `-Wcomment` under `-Wall -Werror` in `writer.rs`.
  - `is_name`: the format-character, currency-sign and XID guards are caught. The control-character guard is equivalent.
  - `check_assigns` is caught.
  - The survivors are the ones listed in part 4.

**5. Checked and could not fault:**
- No Rust doc line becomes an item (CR, U+2028, U+0085, VT, FF, `*/`).
- Every name class `is_name` admits is refused or compiles in all three writers; C refuses everything non-ASCII.
- NFC normalisation cannot produce a keyword (KELVIN SIGN folds to `K`, and no keyword holds a `K`).
- `is_format_char` matches Unicode 16 Cf exactly.
- Positions and extern names with Cf or C1 characters are refused.
- No path traversal is possible. Following a symlink already in the output directory needs write access to that directory, not just a document.

**6. Written outside the worktree.** Everything is under `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/sec/writers/secreview2/`:
- the pass-1 records (`pass1-sec.txt`, `pass1-adv.txt`)
- scripts: `cprobe.sh`, `cf.py`, `why.py`, `alone.sh`, `alone2.sh`, `gomut.sh`, `gomut2.sh`, `suite.sh`
- directories: `cprobe/`, `why/`, `rsanity/`, and `gomut/` (symlinks plus a Go test target)
- logs: `alone*.log`, `gomut*.log`, `suite-*.log`, `df-*.txt`

Inside the worktree's `target/`: `tmp/secreview2{,-c,-rust}/` and `go-writer-test/TestSecreview2*`. I took no worktree lease.

```findings
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 89
  category: integrity
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: a module named documentation is written as package documentation, which go/build and the go command ignore, so the written package is empty
- file: adapters/c/src/write.rs
  line: 813
  category: integrity
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: a module named like a C standard header is written as <name>.h, which the runner's -I out makes the header the runtime and libc include
- file: adapters/rust/src/write.rs
  line: 350
  category: integrity
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: refusals.json carries C1 controls and bidi characters from a foreign statement's reason raw, since serde_json escapes C0 only
- file: adapters/c/src/main.rs
  line: 114
  category: integrity
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the C writer's refusals.json carries C1 controls and bidi characters from the document raw, though its stderr escapes them
- file: crates/model/src/lib.rs
  line: 263
  category: integrity
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: is_name admits invisible default-ignorable letters and marks (Hangul fillers, variation selectors, CGJ), which Rust and Go write as look-alike names
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 89
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: Windows device names (con, nul, aux, prn, com1, lpt1) are written as <name>.go and <name>.h, which no use on Windows was shown to reach
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 82
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: removing the recovered panic's position survives the suite, because the only known panic is now refused before it happens
- file: adapters/go/cmd/codespec-write-go/write.go
  line: 245
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: matching file names against the host platform survived the committed suite; the added TestSecreview2AModuleNameWithThisPlatformsSuffixIsRefused catches it
- file: crates/model/src/lib.rs
  line: 303
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: dropping format-character escaping from printable survives, since no test checks a Cf character in a message
- file: crates/model/src/lib.rs
  line: 1485
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: check_text's extern-expression branch, its unordered_iteration key and the extern type key can each be removed without a test failing
- file: adapters/rust/src/write.rs
  line: 1693
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the loop that keeps a blank parameter's _cs_p<i> name distinct from other parameters and locals can be removed without a test failing
```
