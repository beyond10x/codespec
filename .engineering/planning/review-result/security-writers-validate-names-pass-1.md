---
format: aep.planning-md/3
id: review-result:security-writers-validate-names-pass-1
kind: review-result
status: active
title: 'Security review, pass 1: story:writers-validate-names'
relations:
- reviews: story:writers-validate-names
revision: 2
---
I found three gaps. The worst is in the C writer: a doc comment can end its own C comment, so text from the document is compiled as C (blocker, existed before this unit). Two smaller ones are in the Go writer. The shared name check, the Rust writer's doc comments and the Go writer's names and literals held up. Findings cover commit `58818b2` plus my untracked test files; I committed nothing and changed no implementation file.

```json
[
{"id":"S1","severity":"blocker","title":"C writer: a doc comment can end its own block comment, so document text becomes C code. comment() only breaks a literal star-slash; C joins lines and reads trigraphs before it looks for comments","file":"adapters/c/src/write.rs:331","evidence":"secreview1_a_doc_ending_a_line_in_star_backslash_cannot_close_its_comment and ..._in_a_trigraph_backslash_... both FAILED. Written op_add.c: '/* Add adds. *' + backslash + newline + '/ int secreview_injected = 42; /*  */'. After the line join C reads '/* Add adds. */ int secreview_injected = 42; ...'. Same failures at base 0a914ea (run from a git archive in scratch). Reachable from a Go source comment line ending in star-backslash. Sinks: :914 :925 :947 (header doc lines), :1306 (function doc). Fix: map every backslash in comment text to something else, break '??' and '/*' (a nested '/*' also lets a doc forge a '/* codespec: func ... */' line the C runner reads).","introduced_by_unit":false,"invariant":"doc comments into C block comments: the escape is incomplete (line join, ??/ trigraph, nested /*), and the module doc the unit added at adapters/c/src/write.rs:51 calls this sink escaped"},
{"id":"S2","severity":"minor","title":"Go writer: a doc line '+build EXPR' becomes a //go:build constraint at the top of the file. go/format turns any '// +build' comment anywhere in the file into one","file":"adapters/go/cmd/codespec-write-go/write.go:210","evidence":"TestSecreview1ADocCommentIsNeverABuildConstraint FAILED: the doc \"Add adds.\\n+build ignore\" became the build constraint [\"//go:build ignore\" \"// +build ignore\"]. Reproduced at base. Fix: write a doc line that constraint.IsPlusBuild accepts in another form, or refuse it.","introduced_by_unit":false,"invariant":"doc comments into Go // comments: not escaped for build-directive syntax"},
{"id":"S3","severity":"minor","title":"Go writer: a module name passes token.IsIdentifier, but <name>.go is a file the go command skips or reads as a constraint","file":"adapters/go/cmd/codespec-write-go/write.go:79","evidence":"TestSecreview1AModuleNameIsNeverAFileNameTheGoCommandSkips FAILED in 4 subtests: module \"op_add_windows\" was written as op_add_windows.go, which `go build` does not compile (matches linux/amd64: false). Same for op_add_arm64, op_add_test (a test file) and _op_add (leading _). Reproduced at base. The file is written at main.go:60. Fix: refuse a name whose file build.Default.MatchFile rejects or that ends in _test, or write a fixed file name.","introduced_by_unit":false,"invariant":"module name into the file name: validated as an identifier, not as a file name the toolchain compiles"},
{"id":"N1","severity":"note","title":"Go writer: foreign Go statements and initializers are code execution by design; the user docs never say so in those terms","file":"adapters/go/cmd/codespec-write-go/write.go:938","evidence":"Statements are spliced at write.go:938-952, initializers at :600-607. main.go:9 documents this. website/docs/concepts/pipeline.md:77 only says 'spliced back verbatim': no warning about untrusted documents, and no mention of foreign initializers. Nothing else uses that path: foreign code in another language is refused (write.go:598, :939), and the Rust and C writers refuse every foreign statement and initializer (adapters/c/src/write.rs:681, :1215). Extern import paths are quoted (write.go:172) but any path is accepted, including \"C\" (turns on cgo, though no preamble can reach it) and \"unsafe\". That gives the same power as foreign Go.","introduced_by_unit":false,"invariant":"foreign Go: verbatim by design"},
{"id":"N2","severity":"note","title":"Positions, names and refusal reasons from the document reach stderr raw, so terminal control characters pass through","file":"adapters/c/src/main.rs:127","evidence":"By reading: eprintln! of r.name, r.pos and r.reason. The Go writer does the same with w.pos and reason (%s) at write.go:224. stderr only, never the written files. The adversary's case a_refusal_message_carries_no_control_character_from_the_document covers the same thing.","introduced_by_unit":false,"invariant":"stderr: unprotected (not an output file)"},
{"id":"N3","severity":"note","title":"Unicode: rustc folds identifiers to NFC, so two different cir/1 names can become one Rust identifier","file":"adapters/rust/src/write.rs:543","evidence":"By reading: U+212B and U+00C5 both pass is_name and is_rust_ident, and rustc reads both as U+00C5. Result: shadowing or a duplicate definition, not injection. is_name lets through No/Nl characters such as U+00B2, but every writer refuses them. No character passes both checks and then ends an identifier in Rust, Go or C (the C writer is ASCII only). Keywords: Rust makes them raw or adds a _ suffix, C adds a _ suffix, Go refuses them (token.IsIdentifier). Go accepts locals named like predeclared identifiers (true, nil, len); they shadow them, which changes meaning but injects nothing.","introduced_by_unit":false,"invariant":"identifier check across languages"},
{"id":"N4","severity":"note","title":"C writer: a module named codespec_rt has its header overwritten by the runtime file written after it","file":"adapters/c/src/write.rs:796","evidence":"By reading: the files are written in the order <m>.h, <m>.c, codespec_rt.h, so the runtime replaces the module header and the build fails. The adversary's case a_module_named_like_the_runtime_header_keeps_its_own_header measures this.","introduced_by_unit":false,"invariant":"file names: collision with a fixed file name"},
{"id":"N5","severity":"note","title":"Doc comments can carry a lone CR or bidi control characters","file":"adapters/rust/src/write.rs:789","evidence":"By reading: Rust writes each doc line as its own doc attribute; secreview1_a_doc_comment_never_leaves_its_doc_comment_lines passes. rustc rejects a lone CR or bidi control in a doc comment, so the effect is a failed build. Go and C comments keep bidi controls, which can make the text display differently from what compiles (Trojan Source).","introduced_by_unit":false,"invariant":"doc comments: display only"}
]
```

**Sink table**
module name -> Cargo.toml `name = "…"` — adapters/rust/src/write.rs:340 (checked at :125) — validated
every Rust identifier (types, functions, fields, variants, locals, params, layout receiver, facts) — adapters/rust/src/write.rs:532-568 `checked_ident` — validated
Rust string, enum and const literals — adapters/rust/src/write.rs:1756, :1303, :1450 `Literal::string` — escaped
Rust int and bool literals — adapters/rust/src/write.rs:1759, :1777 — validated (parsed)
Rust sprintf/errorf format — adapters/rust/src/write.rs:4102-4144 (braces doubled, then `Literal::string`) — escaped
Rust doc comments (module, types, functions) — adapters/rust/src/write.rs:789-797 — escaped
Rust floats, externs, interface calls, foreign — adapters/rust/src/write.rs:3448, :3495-3504 — refused
Rust refusals.json — adapters/rust/src/write.rs:344 — escaped (JSON)
Rust file names — adapters/rust/src/main.rs:103 — fixed names
C module name -> file names, prefix, header guard, banner — adapters/c/src/write.rs:415, :797, :808, :893 (checked at :407) — validated
every C name (`each_name`, then `c_name`) — adapters/c/src/write.rs:407, :181 — validated
C strings, error messages, type descriptions — adapters/c/src/write.rs:315 (used at :636, :1032, :1385) — escaped (complete: quote, backslash, ?, octal)
C sprintf pieces — adapters/c/src/write.rs:2024 — escaped
C int and rune literals — adapters/c/src/write.rs:1078 — validated (parsed)
C `codespec:` header lines — adapters/c/src/write.rs:918, :951, :1297 — validated (extern types refused by `cty` first)
C doc comments — adapters/c/src/write.rs:331 (used at :914, :925, :947, :1306) — unprotected (S1)
C externs, interface calls, function refs, lambdas, foreign, foreign var initializers — adapters/c/src/write.rs:1658-1679, :1215, :681 — refused
C refusals.json / stderr — adapters/c/src/main.rs:109 / :127 — escaped / unprotected (N2)
Go module name -> package clause — adapters/go/cmd/codespec-write-go/write.go:79, :154 — validated
Go module name -> file name — adapters/go/cmd/codespec-write-go/main.go:60 — unprotected for file-name meaning (S3)
every Go identifier (including extern names, extern type names, import aliases, type params, int width and float bits) — adapters/go/cmd/codespec-write-go/write.go:189 `checkNode` — validated
Go sorted_keys helper text — adapters/go/cmd/codespec-write-go/sorted_keys.go:48-59 (`typeSrc` checked at :170) — validated
Go `lit` and float literals — adapters/go/cmd/codespec-write-go/write.go:1061-1071, float.go:14 — validated
Go error messages — adapters/go/cmd/codespec-write-go/write.go:583 `strconv.Quote` — escaped
Go import paths — adapters/go/cmd/codespec-write-go/write.go:172, :184 — escaped (any path accepted, N1)
Go operators, array length — adapters/go/cmd/codespec-write-go/write.go:834, :1095, :365 — validated
Go doc comments (module, types, functions, interfaces) — adapters/go/cmd/codespec-write-go/write.go:152, :196, :210 — line ends handled; build-directive syntax unprotected (S2)
Go foreign Go statement / initializer — adapters/go/cmd/codespec-write-go/write.go:938 / :600 — verbatim by design
Go foreign code in other languages — adapters/go/cmd/codespec-write-go/write.go:598, :939 — refused
model module path — crates/model/src/lib.rs:465 — validated (no writer writes it)

**Tests, runs and what else was checked**
`git diff --stat` is empty: my only changes are new, untracked test files. Added: adapters/c/tests/secreview1.rs (3 cases, 2 red), adapters/go/cmd/codespec-write-go/secreview1_test.go (2 tests with 6 subtests, all red), crates/model/tests/secreview1.rs (1 green), adapters/rust/tests/secreview1.rs (1 green).
`each_name` visits every name of every checked-in document outside its documented exclusions (model test, green). None of those exclusions reaches output unchecked: C refuses externs, interfaces and foreign code and ignores layout and facts; Rust checks every identifier; Go checks every identifier in the printed tree.
Strict reading is not the only guard: the Rust and C binaries call `Module::from_json`; the Go binary decodes JSON loosely, and its own checks hold without strict reading, except doc comments (S2) and the file name (S3).
Each file alone: C cases exit 101 (2 red); Go `go test -run TestSecreview1` exit 1; model exit 0; Rust exit 0. C at base 0a914ea, from a git archive in scratch: exit 101, same 2 red. Go at base: exit 1, same failures.
Suite (`cargo test --no-fail-fast`, model + Rust writer + C writer, `--target-dir target/secreview`): executed 192 -> 197, exit 101 both times. Red: my 2 C cases plus 12 failures in the other reviewer's adversary1 files, which also fail with my cases skipped.
`go test ./cmd/codespec-write-go/` (counting test and subtest runs): 90 -> 98, exit 0 without my cases, exit 1 with them.
Written outside the worktree, all under ~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/sec/writers/secreview1/: logs (*.log), scripts (alone.sh, suite.sh, suite2.sh), base/ and base-full/ (exports of 0a914ea) and base-target/ (their build).
Nothing written by these tests was built or run. Disk: 37 GB free at the end.
