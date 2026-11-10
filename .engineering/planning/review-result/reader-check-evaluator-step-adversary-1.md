---
format: aep.planning-md/3
id: review-result:reader-check-evaluator-step-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on reader-check-evaluator-step
relations:
- reviews: story:reader-check-evaluator-step
revision: 2
---
The unit is red on 2 cases I wrote. Neither shows a wrong reader module passing through the evaluator step on the current kit.

```
unit: impl/reader-check-evaluator-step, commit 72d071d, tree ~/.local/state/worktree/trees/b10x/codespec/rd-evalstep
verdict: NEEDS-CHANGE
cases: executed 2→4, red 2
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: the 9 modified website/data/conformance/*.json files were already in the tree before I started; they are not mine
```

**1. `git --no-pager diff --stat`** (tracked files; none of these changes are mine)
```
 website/data/conformance/reader-{c,go,groovy,java,rust}.json, writer-{c,cir,go,rust}.json | 9 files, 9 +/9 -
```
My only change is one new, uncommitted test file: `?? crates/conformance/tests/adversary_evalstep.rs`. I changed no implementation file.

**2. Cases added** (both in `crates/conformance/tests/adversary_evalstep.rs`, both red now). Disk had 28 GB free, so I built; the output below is each case's first run.

- **`adversary_an_evaluator_stop_is_never_a_pass_even_when_expected_txt_lacks_a_final_newline`**
  - Setup: a mini kit where `op-add/expected.txt` is `"ok 5\nok -5"`, with no final newline. The Go writer refuses the module. The evaluator's runner exits 1 with a `stopped at … panic` message and prints exactly the expected text.
  - Asserts: the verdict is not `pass`.
  - Result: `test adversary_an_evaluator_stop_is_never_a_pass_even_when_expected_txt_lacks_a_final_newline ... FAILED`, with `FeatureResult { id: "op.add", claimed: true, verdict: Pass, ... runner: Some("codespec-run-cir") }`.
- **`adversary_the_guide_and_the_cli_help_name_every_program_a_reader_run_requires`**
  - Setup: `conform_adapter` for a reader, with only the stub reader, its runner and the Go writer and runner in `--adapters`. It errors with `…/adapters/codespec-write-cir does not exist`.
  - Asserts: the adapter guide, the `--adapters` help and `cli.md` name that program.
  - Result: `test adversary_the_guide_and_the_cli_help_name_every_program_a_reader_run_requires ... FAILED`, with `yet these say a reader needs only codespec-write-go and codespec-run-go besides its runner: ["website/docs/guides/add-an-adapter.md", "crates/cli/src/lib.rs (--adapters help)", "website/docs/reference/cli.md"]`.

**3. Targeted run, after the cases existed** (script `adv/suite.sh`, log `adv/suite.log`)
- `--test conform -- a_case_the_go_writer_refuses a_reader_that_turns`, my file excluded: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out`, exit 0.
- `--test conform --test adversary_evalstep -- … adversary_`: `test result: FAILED. 0 passed; 2 failed`, exit 101. Cargo stopped at the first failing binary, so `conform` did not run again in this second call.

**4. Findings**

- **The guide and CLI help name too few programs** (`conform.rs:1680`).
  - What happens: a reader run now looks up `codespec-write-cir` and `codespec-run-cir` before any case runs. It fails even when the Go writer would write every case.
  - What is wrong: three texts still say a reader needs only the Go pair: `website/docs/guides/add-an-adapter.md:97`, `crates/cli/src/lib.rs:106` and `website/docs/reference/cli.md:126`.
  - Who hits it: anyone who builds an `--adapters` directory by following the guide. The release archive does ship the two evaluator programs, so someone who unpacks it is unaffected.
  - Fix: update the three texts (and AGENTS.md:288), or look the two programs up only when the Go writer first refuses a case.
  - NEEDS-CHANGE, introduced.
- **A stopped evaluator run can be reported `pass`** (`conform.rs:895`).
  - Cause: the prefix test needs output ending in `\n`. When `expected.txt` has no final newline and the output equals it, the exit-1 branch falls through to `compared`, which returns Pass. The module doc says a stop is `refused` or `wrong_output`, never a pass.
  - Who hits it: nobody today. I checked every `conformance/cases/*/expected.txt` and all end in a newline, so I built this state myself.
  - INFEASIBLE, introduced.
- **The support-matrix note is wrong for an evaluator stop** (`crates/docs/src/support.rs:251`).
  - What happens: when the evaluator runs the module and stops, the note reads "the Go writer refuses it and the evaluator does not run it: codespec-run-cir: stopped at …". The evaluator did run it.
  - Why the tests miss it: the docs tests only build entries with `runner: None`.
  - CONFIRMED (read from the code), introduced.
- **A `panic`, `blocked` or `step budget` stop counts as `refused`, not as a failure** (`conform.rs:898`).
  - What happens: the evaluator can reproduce a reader module that would crash or hang under Go. If that happens before any wrong line, an unclaimed feature counts as `refused`, not `fail`. The Go path reports the same kind of crash as `run_refused`/`run_failed`, which is a failure.
  - Effect: limited to the report summary and the note text. A claimed feature still fails, and the matrix cell shows "refused" either way. The story's acceptance asks for "an evaluator stop is `refused`", so this is a design question, not a broken rule.
  - CONFIRMED, introduced.

**5. What I attacked and could not break**
- A wrong module cannot reach `pass` through the stop path while `expected.txt` ends in a newline: the prefix check and `compared` close it.
- A stop after a wrong line is `wrong_output`.
- An evaluator exit 1 without `stopped at` is `run_failed`. Exit 2, a missing function or arity mismatch, is `run_refused`. Both are failures.
- A claimed feature whose evaluator run stopped is `fail`.
- A case the Go writer writes or times out on never reaches the evaluator.
- A missing evaluator program is an error, never a pass.
- `runner` uses serde default and skip-if-none, and old reports read back.
- Report order is deterministic (`parallel` keeps input order, and `find_map` follows it).
- `stopped at ` in stderr cannot come from the program itself: the evaluator's handlers do not write to stderr.

**6. Paths written outside the worktree**
- `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/evalstep/adv/` (directory)
- `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/evalstep/adv/run1.log`
- `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/evalstep/adv/suite.sh`
- `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/evalstep/adv/suite.log`

Inside the worktree, the tests also wrote `target/conform-tests/adversary-evalstep-{stop-pass,adapters}/`.

**7. Findings block**
```findings
- file: "crates/conformance/src/conform.rs"
  line: 1680
  category: "contract"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "a reader run now requires codespec-write-cir/codespec-run-cir in --adapters, but the adapter guide (add-an-adapter.md:97), the --adapters help (cli/src/lib.rs:106) and cli.md:126 still name only the Go pair | cargo test -p b10x-codespec-conformance --test adversary_evalstep: conform_adapter errors '.../adapters/codespec-write-cir does not exist'; texts silent: [add-an-adapter.md, cli/src/lib.rs --adapters help, reference/cli.md]"
- file: "crates/conformance/src/conform.rs"
  line: 895
  category: "correctness"
  severity: "note"
  verdict: "INFEASIBLE"
  origin: "introduced"
  message: "an evaluator exit 1 with 'stopped at' is reported pass when stdout equals an expected.txt lacking a final newline; no current kit case lacks one | adversary_an_evaluator_stop_is_never_a_pass_even_when_expected_txt_lacks_a_final_newline: FeatureResult { id: \"op.add\", claimed: true, verdict: Pass, runner: Some(\"codespec-run-cir\") }; every conformance/cases/*/expected.txt ends in a newline"
- file: "crates/docs/src/support.rs"
  line: 251
  category: "docs"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "the matrix note says 'the evaluator does not run it' for an evaluator stop, where it ran the module and stopped; no docs test covers runner Some | code path: evaluate() returns refused(Reason::Refused, stop text) for exit 1 + 'stopped at', verdicts() sets runner, refusal() matches Some(Reason::Refused) if e.runner.is_some(); support.rs tests build runner: None only"
- file: "crates/conformance/src/conform.rs"
  line: 898
  category: "correctness"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "a panic/blocked/step-budget stop of a wrong reader module before any wrong line is 'refused' for an unclaimed feature, where the Go path reports such a crash as a failure | code path: Some(1) if stderr contains 'stopped at ' and prefix -> refused(...) regardless of the stop kind (crates/eval/src/lib.rs stop kinds panic, blocked, step budget); effect limited to summary and note, claimed features still fail, and story acceptance asks for stop -> refused"
```

**Tests written and each one's own result line**
- `test adversary_an_evaluator_stop_is_never_a_pass_even_when_expected_txt_lacks_a_final_newline ... FAILED`
- `test adversary_the_guide_and_the_cli_help_name_every_program_a_reader_run_requires ... FAILED`
