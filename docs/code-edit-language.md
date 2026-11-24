# Code edits in cq: a query selects, an operation changes the source — design (2026-11-24)

Timo, 2026-11-24: "a unified mutation language … the query would basically select the target to
what it should be applied, and then we run the operations against this limited filtered scope …
language agnostic, kind of." Plan: `epic:code-edit-language`. Revised the same day after the
plan critics' round (§ 14).

## 1. Decisions in one table

| question | decision | section |
|---|---|---|
| notation | an `edit` item in a cq program: `edit <op>(args) :- body.`; each definite row of the body is one operation | § 3 |
| command | `codespec edit --lang <l> [--adapters <dir>] <dir>… -e <program>` or `-f <file.cq>`; dry run with a unified diff by default, `--apply` writes | § 3.3 |
| wave 1 operations | `rename`, `rename_field`, `retarget`, `delete` | § 4 |
| meaning | each operation is defined once on `cir/1`: it maps the read set to the set the edited source must read as | § 5 |
| how source changes | byte-span edits of the original files; bytes outside the edited spans stay as they were | § 6 |
| where spans live | a side document `codespec-spans/1` per read module, not inside `cir/1` | § 6.2 |
| safety | refuse what the reader could not see; verify by re-reading a cache copy holding the edit, before any write | § 7 |
| languages first | Rust and Go; Rust carries the first end-to-end slice | § 9 |
| home | codespec, not codegate | § 11 |

## 2. What exists

- **cq selects nodes already.** Every statement and expression has a node id
  `<module path>.<function key>#<index path>` (`crates/query/src/lib.rs:10-15`), which "names the
  node a rewrite would replace" (`docs/query-language.md:100`). Rows are sorted and each is
  `definite` or `possible`, a possible row naming the unknown sites it rests on
  (`docs/query-language.md:145-156`). Relations a selection needs exist: `func`, `call` (with
  `listed` for calls inside unread code), `expr` (`name`, `slot`, `parent`), `field_init`,
  `type_field`, `def`, `foreign`, `unknown`, `implements`, `reader_has`
  (`crates/query/src/facts.rs:76-260`, `:372`). Builtins `replace` and `concat` compute names
  (`crates/query/src/lib.rs:62-64`). The fact base does not hold a `foreign` statement's `src`
  (`facts.rs:234-239`).
- **`cir/1` has no span.** `Pos` is `<file>:<line>` (`crates/model/src/lib.rs:804-805`), on
  statements and declarations only; expressions have none. The Go reader writes the base file name
  (`adapters/go/cmd/codespec-read-go/front.go:130-133`), the Rust reader a path relative to the
  crate root (`adapters/rust-read/src/files.rs:151-159`). No document records its source
  directory.
- **Readers change names and shape.** The Rust reader writes names in Go spelling (`pub fn load` is
  `Load`, collisions `value_2`; `adapters/rust-read/src/lib.rs:30`). The Go reader hoists `if`
  inits and renames a colliding local, reads blank assignments, local `var` and `const`, named
  results as locals and small literal forms (branch `wave/go-reader-refusals-1`,
  `epic:go-reader-refusals`). Go `foreign.src` is `go/printer` output, not the original text
  (`front.go:135-139`). So a `cir/1` node maps back to source only through records the reader
  writes while it still holds the syntax tree.
- **The readers can give byte offsets.** Go: `token.Pos` through `fset` (`front.go:130`). Rust:
  `proc-macro2` is built with `span-locations` (`adapters/rust-read/Cargo.toml`), which gives
  `Span::byte_range()`. The Rust reader builds the model's own types, which refuse unknown fields
  (`#[serde(deny_unknown_fields)]` throughout `crates/model/src/lib.rs`), so it cannot hang a span
  on a node; the Go reader has its own IR structs (`ir.go`).
- **No edit command exists.** `codespec pass` rewrites one module (`crates/passes/src/lib.rs:1-15`)
  and `codespec write` emits a whole module in the writer's formatting. `story:cir-rewrite` (draft)
  planned "cq selects, `cir/1` template replaces, written through the Go writer", and
  `epic:codebase-transformations` excluded "formatting- or comment-preserving edits of the original
  files". This design takes that up as its own epic (§ 12).
- **codegate** (`~/projects/editor`, Go only): operations are Go-specific structs
  (`operations.go:7-45`: `RenameSymbol`, `AddGoParameter`, `RenameGoStructField` …), selected by a
  `SymbolSelector` of name, kind and container (`internal/core/types.go:489-499`), compiled per
  backend into byte-offset `TextEdit`s (`changeset.go:25-47`). Overlapping edits are refused
  (`changeset.go:154-186`); each changed file is run through `go/format` (`changeset.go:141-146`,
  `internal/lang/goast/edits.go:107-114`); a stale source is caught by a hash of the symbol's bytes
  (`edits.go:386-404`). Its rename skips an occurrence whose length differs from the name
  (`edits.go:364-370`) instead of refusing, and field rename is unsupported (`edits.go:415-416`).
  Validation is parse and type check (`internal/core/types.go:260-266`), not a proof that the
  change is the one asked for. The CLI runs one operation as JSON or flags, dry run by default
  (`cmd/codegate/operation.go`).

Prior art taken: span edits with an overlap refusal (LSP `WorkspaceEdit`, codegate); a query that
binds and a rewrite that uses the bindings (Semgrep autofix, ast-grep `rewrite`, Coccinelle's
metavariables, Comby templates); refactorings that refuse when a reference cannot be resolved
(gopls and rust-analyzer rename); a lossless tree that keeps formatting (OpenRewrite's LST,
jscodeshift with recast). Not taken: text-pattern matching as the selector (Comby, ast-grep) —
cq already selects typed nodes; a full lossless tree per language — spans give the same
guarantee for the edited bytes at a fraction of the cost.

## 3. The notation

### 3.1 Edit items

A cq program gains one item kind:

```
edit    = "edit" op "(" term ("," term)* ")" ":-" body "."
```

The body is an ordinary cq body: it may use rules of the program, record patterns, negation,
aggregates and builtins. Each row of the body binds the operation's arguments, and is one
operation. A program may hold several `edit` items; their operations form one change set. An edit
program holds no `?-` query; `codespec query` refuses a program with an `edit` item, naming it.

Only definite rows edit. A possible row is refused with the unknown sites it rests on: an edit
never acts on "might".

The target is named in `cir/1` terms (a function id, a type id, a node id). The new name is written
in the source language's spelling, as it is to appear in the file. A relation
`source_name(node, text)`, built from the span documents (§ 6.2), gives the source spelling, so a
query can select by what the file says (`load`) where `cir/1` says `Load`.

### 3.2 Examples

Wave 1 (§ 4). Examples 1, 2 and 4 name exported functions, so they run with `--closed-world`
(§ 7.1):

```
// 1. Rename a function and every reference to it, in every module of the set.
edit rename(F, "LoadAll") :- func{id: F, module: "example.com/orders", name: "Load", recv: ""}.

// 2. Rename many: every exported Get… of a module to Fetch….
edit rename(F, New) :- func{id: F, module: "example.com/orders", name: N, exported: true},
                       N ~ "^Get[A-Z]", replace(N, "Get", "Fetch", New).

// 3. Rename a struct field: declaration, every access, every keyed literal.
edit rename_field(T, "qty", "quantity") :- type{id: T, name: "line", path: "example.com/orders"}.

// 4. Point every call of Load at LoadCached (same module, same signature).
edit retarget(Site, G) :- call{site: Site, callee: "example.com/orders.Load"},
                          func{id: G, module: "example.com/orders", name: "LoadCached"}.

// 5. Delete unexported functions nothing calls.
edit delete(F) :- func{id: F, module: "example.com/orders", kind: "func", exported: false},
                  !calls(_, F).
```

Later waves (§ 4.2); a template is source text of the module's language, `$X` the source text of
the node bound to the rule variable `X`:

```
// 6. Replace an expression pattern.
edit replace(E, "strings.Contains($S, $T)") :-
    expr{id: E, kind: "binop", op: ">=", x: X, y: Y}, expr{id: Y, kind: "lit", value: "0"},
    call{site: X, callee: "strings.Index"},
    expr{parent: X, slot: "args", index: 0, id: S}, expr{parent: X, slot: "args", index: 1, id: T}.

// 7. Wrap a dropped error (rule dropped_error of docs/query-language.md § 6).
edit replace_stmt(S, "if err := $C; err != nil {\n\treturn err\n}") :-
    dropped_error(C), expr{id: C, stmt: S}, stmt{id: S, kind: "expr"}.

// 8. Add a parameter with a default at every call site.
edit add_param(F, 0, "ctx", "context.Context", "context.Background()") :-
    func{id: F, module: "example.com/orders", name: "Fetch"}.

// 9. Move a function to another module; imports and qualifiers follow.
edit move(F, "example.com/orders/store") :- func{id: F, name: "SaveLine", module: "example.com/orders"}.

// 10. Change visibility (Go: the name's case and every reference; Rust: `pub`).
outside_call(F) :- calls{site: S, target: F}, call{site: S, func: G}, func{id: F, module: M},
                   func{id: G, module: N}, M != N.
edit set_visibility(F, "private") :- func{id: F, module: "example.com/orders", exported: true},
                                     !outside_call(F).
```

### 3.3 The command

```console
codespec edit --lang go --closed-world ./orders ./api \
  -e 'edit rename(F, "LoadAll") :- func{id: F, name: "Load"}.'
codespec edit --lang rust --adapters target/adapters ./crates/ledger -f renames.cq --apply
```

- Each directory is read with `codespec-read-<lang> --spans` (§ 6.3) into one set;
  `--adapters <dir>` names the directory of the adapter programs, as for `codespec read`. The fact
  base is built over the set as `codespec query` builds it, with `source_name` added.
- Default: dry run. Standard output is a unified diff (`--- a/<path>`, `+++ b/<path>`, three
  lines of context, paths relative to the current directory) that `git apply --check` accepts;
  `--format json` prints `codespec-edit-report/1` (rows with their status, files with their
  SHA-256 before and after, the diff). A dry run is verified too (§ 7.3).
- `--apply` writes the files after verification, each by a write to a temporary file in its
  directory and a rename, and only when every file still has the SHA-256 it had when read.
- `--closed-world` says no code outside the inputs refers to their exported names (§ 7.1).
- Exit 0: every row planned (dry run) or written (`--apply`), or no row. Exit 1: one or more rows
  refused or verification failed; nothing is written, every refusal is printed. Exit 2: an error
  (program, reader, input).

Why an `edit` item and not a pipeline (`codespec query … | codespec edit …`) or a separate file
format: the selection and the operation are one statement that a reviewer reads at once, the rule
variables are in scope for the arguments (example 2) and for templates (examples 6, 7), and the
grammar, the explanation printer and the possible/definite rule are cq's own. A `.cq` file is the
file form; there is no second format.

## 4. Operations

### 4.1 Wave 1

| operation | target | what changes in the source |
|---|---|---|
| `rename(F, New)` | a function or method id | the declaration's name and every reference: calls, method calls, function values, calls from other modules of the set (`extern`, `extern_method`), Rust `use` paths (the path segment, not an `as` alias) |
| `rename_field(T, Old, New)` | a struct type id and a field's `cir/1` name | the field's declaration, every field access, every keyed literal field; a Rust shorthand `T { x }` becomes `T { new: x }` |
| `retarget(Site, G)` | a call node and a function id of the callee's module with the same signature (and receiver type for a method) | the callee's name token at that call |
| `delete(F)` | a function or method id | the declaration, from its doc comment or first attribute to its end, with its line break |

Chosen because each is useful alone, each replaces identifier tokens or whole declarations only
(so the per-language rendering is a keyword check and Rust's shorthand), and each has an expected
`cir/1` that can be computed exactly (§ 5), so each is proved, not trusted.

### 4.2 Later

| wave | operations | needs |
|---|---|---|
| 2 | `ensure_import`, `remove_import`; `retarget` to another module | import ranges in the spans (`story:edit-imports`) |
| 2 | `replace(E, template)`, `replace_stmt(S, template)`, `insert_before/after(S, template)`, with the imports a template needs | expression and statement spans; reading a template as a fragment (`story:edit-replace-template`, after `story:edit-imports`) |
| 2 | `add_param(F, i, name, type, default)`, `remove_param(F, i)` | parameter-list and argument-list spans |
| 3 | `rename` of a type, a local, a parameter; `rename` of an interface method with every implementation | spans of type references and locals |
| 3 | `move(F, module)`, `set_visibility(F, v)` | imports, qualifiers, per-language visibility |
| 3 | C and Java readers giving spans | reader work |

## 5. Meaning on `cir/1`

An operation is a function from the read set `S` (the `cir/1` documents) to the expected set `S'`:
the documents the edited source must read as. It is defined once, in Rust, in a new crate
`crates/edit` (`b10x-codespec-edit`), for every language. The crate's modules are pure (no process,
no file) except `run`, which holds the pipeline of § 3.3 (readers, fact base, edit rows, cache copy,
re-read, comparison, apply); `codespec edit` and the MCP server (`epic:agent-mcp-server`) both call
`codespec_edit::run`, and `crates/cli` stays a thin wrapper.

- `rename(F, New)`: let `o` be `F`'s `cir/1` name in module `M`. `S'` is `S` with `o` replaced by
  `o'` in `F`'s declaration, `M.layout`, every `call`, `method` and `func_ref` that resolves to `F`,
  every `extern`/`extern_method` expression of another module naming `M.path` and `o` (and the
  receiver type), the `externs` entries naming it (sorted again), `entries`, `transactional` and
  `linknames`. For a method, also the `MethodEntry` of `methods` (its `name` and its `target`
  call ref's name), with the method set sorted by name again (`crates/model/src/lib.rs:768-784`).
  `o'` is the reader's spelling of `New`, taken from the re-read declaration: Go keeps `New`, the
  Rust reader may write it in Go spelling. Verification requires one `o'` everywhere and `o'` used
  nowhere else in `S`. Positions do not change: a name holds no line break.
- `rename_field(T, Old, New)`: the same over `types[T].fields`, `field` expressions on a value of
  `T` (or a pointer to it) and `struct` literal fields of `T`.
- `retarget(Site, G)`: the call at `Site` names `G` instead of its callee; nothing else changes.
- `delete(F)`: `F` leaves `funcs` (or `opaque`), `layout` and its type's method set; an `externs`
  entry no remaining call names leaves too. Every position after the deleted lines in that file
  moves up by their count.

Several rows: their operations are applied to `S` in row order; two operations whose text edits
overlap are refused unless they are byte-identical, which are merged (example 2 can name one
function twice).

## 6. Applying to source

### 6.1 Span edits, not re-emitting

The writer re-emits a module in its own formatting (`codespec write`); comments outside `doc`,
blank lines and the author's layout would be lost. So the source changes by byte-span edits of
the original files. A text edit is `(file, start, end, replacement)`; edits apply from the end of a
file to its start; bytes outside every edited span stay as they were. The per-language part is
small: whether the new name is an identifier and not a keyword of the language, and Rust's field
shorthand (wave 2: reading a template as a fragment). After the edits, a Go file that was
`gofmt`-clean before is run through `gofmt`, which realigns the whitespace around a renamed field
or argument and nothing else. Its changes become text edits too: the formatted and the span-edited
text must have the same characters outside whitespace, and each whitespace run that differs is one
edit, merged with the span edits. Other files are not reformatted.

### 6.2 `codespec-spans/1`, beside `cir/1`

The spans are a side document, one per read module, not an optional field of `cir/1`. Reasons:

- The bytes of `cir/1` are compared across readers and runs: `examples-check` compares every
  checked-in read, `examples-series` compares a Rust re-read of written code, the fact-base cache is
  keyed by the documents' SHA-256. Byte offsets in the document would change all of them on a
  whitespace edit of the source.
- A pass or a writer produces `cir/1` that has no source; spans in it would be wrong or empty, and
  `lookup-guard` would have to carry them through its rewrite.
- Only `codespec edit` needs them, and only for the modules it edits.
- The existing plan decided the same (`epic:agent-mcp-server`; `story:reader-function-spans`).

The format and its strict reading live in `crates/edit/src/spans.rs` (not in `crates/model`, which
defines `cir/1` alone). The document:

```json
{
  "format": "codespec-spans/1",
  "module": "example.com/orders",
  "cir_sha256": "<SHA-256 of the canonical cir/1 document>",
  "files": [{"path": "orders.go", "sha256": "…", "len": 4180, "generated": false}],
  "spans": [
    {"at": "funcs.3", "role": "decl", "file": 0, "start": 1180, "end": 1622, "line": 41, "end_line": 58},
    {"at": "funcs.3", "role": "name", "file": 0, "start": 1236, "end": 1240, "line": 43, "end_line": 43, "text": "Load"},
    {"at": "funcs.5.body.2.rhs.0", "role": "name", "file": 0, "start": 2051, "end": 2055, "line": 77, "end_line": 77, "text": "Load"}
  ],
  "unspanned": [{"at": "funcs.5.body.0.x.x", "role": "name", "reason": "implicit"}],
  "refs": [{"target": "example.com/orders.Load", "file": 1, "start": 88, "end": 92, "text": "load", "kind": "use"}]
}
```

- `at` is the node's path in the canonical JSON of the `cir/1` document. Below `funcs.<i>.body` it
  is the cq index path (`funcs.5.body.2.rhs.0` is node `<path>.<key of funcs[5]>#2.rhs.0`).
  `crates/edit` owns the two-way mapping between `at` paths and cq node ids
  (`story:edit-spans-format`).
- Roles in wave 1: `decl` (a function's whole declaration, from its doc comment or first attribute
  to its end) and `name` (the identifier token that spells the node's name, callee or field).
  `line` and `end_line` are the 1-based lines of `start` and of the span's last byte. `refs` are
  names outside `cir/1`: Rust `use` path segments naming an item of the set (`kind: "use"`); an
  `as` alias is not part of the span.
- Completeness: every function has a `decl` span, and every node of a kind wave 1 renames
  (functions, struct fields, `call`, `method`, `extern`, `extern_method`, `func_ref`, `field`,
  `struct` literal fields) a `name` span; each one that has not has an `unspanned` entry for that
  role. `implicit`: no token spells the name there (a positional literal, a promoted field's hidden
  hop): nothing to edit. `lowered: <why>`: the reader made the node from source it cannot point at
  (a desugared `?`, a named result read as a local): an edit needing it is refused.
- Strict reading, given the `cir/1` document and the files: the format, no unknown field,
  `cir_sha256` equal, each file's SHA-256 and length equal, every span inside its file and on UTF-8
  boundaries, `line` and `end_line` those of its bytes, `text` equal to the file's bytes there,
  every `at` naming a node of the document of a kind its role allows, no two `name` spans
  overlapping, and the completeness rule above. An `at` that names the wrong node of the right
  kind passes strict reading; verification (§ 7.3) catches it, because the edit then lands on
  the wrong token and the re-read differs from `S'`.
- Paths are relative to the directory given to `codespec read`; a Go file starting with
  `// Code generated … DO NOT EDIT.` is `generated`, and an edit in it is refused.
- Open for later roles: `node` (a statement's or expression's whole text, wave 2 templates), import
  ranges (`story:edit-imports`). A planned code quality assessor would want `node` extents of
  statements for physical size; that epic is not decided, and wave 1 neither emits `node` spans
  nor adds a relation for them. Adding a role is additive to the format.

### 6.3 Adapter protocol

- `codespec-read-<lang> --spans <dir>` prints one JSON object
  `{"cir": <the cir/1 module>, "files": […], "spans": […], "unspanned": […], "refs": […]}`; the
  reader computes each `at` itself. The Go reader records spans on its own IR (`ir.go`) and
  computes paths while it encodes; the Rust reader threads the node's path through its builders
  (`body.rs`, `calls.rs`) and records the span where a node is placed in its parent, because the
  model's types take no extra member. `codespec read --spans <out>` (and `codespec edit`)
  strict-reads the `cir/1`, adds `module` and `cir_sha256`, strict-reads the span document and
  writes both. The `cir/1` is byte-identical to a read without `--spans`.
- A reader without spans fails on `--spans` or prints a bare module; `codespec edit` then refuses
  the language, naming the reader. No manifest field is added: the support matrix does not show
  spans in wave 1.

## 7. Safety

### 7.1 Refusals before any edit

Every refusal is computed in Rust in `crates/edit`, over the `cir/1` set and the span documents (the
fact base has no `foreign` `src`); cq only selects. A row is refused, with the reason and the site,
when:

- a node it must edit has no span, or an `unspanned` entry `lowered`;
- a reference to the target is inside code the reader did not read: a listed call naming it (a
  `foreign` statement's, an opaque function's or an initializer's call list), a `named` call ref
  whose name is the target's source spelling, a `foreign` statement whose `src` holds that
  spelling as a word, or a `foreign` statement or opaque function in a module whose reader does not
  fill call lists; the reader could not see it, so the edit could not be complete;
- a `name` span of a reference has a `text` other than the target's source spelling and no `refs`
  alias explains it (a Rust call through `use … as fetch` is spelled `fetch` and is left alone;
  its `use` path segment is edited);
- the target is a method of a type that implements an interface of the set with a method of that
  name: renaming or deleting it breaks the interface; wave 3 renames both;
- the target is exported and `--closed-world` is not given: callers outside the inputs are not
  read;
- the new name is not an identifier of the language, is a keyword, changes Go exportedness (that is
  `set_visibility`), or is the spelling of a declaration of the module or of a local or parameter
  of a body that references the target (it would capture the reference);
- `delete`: anything in the set references the target, or it is `main`, `init` or an entry;
- `retarget`: the new callee is in another module (wave 2) or its signature differs;
- the edit lands in a `generated` file;
- two edits overlap and differ (§ 5).

### 7.2 What is not touched

Comments, string literals and doc text that mention the old name are not changed, as in gopls
and rust-analyzer. Test files the reader does not read (Go `_test.go`, Rust `#[cfg(test)]`) are
not changed; `--build` (§ 7.4) is what catches a test that names a deleted or renamed function.

### 7.3 Verified, not trusted

Before anything is written, `codespec edit` copies the module root of the inputs (Go: the
directory of the `go.mod` above them; Rust: the Cargo workspace root, the nearest directory above
the crate whose `Cargo.toml` has `[workspace]`, else the crate directory), without `.git` and
`target`,
into its cache directory, writes the edited files there, and reads the copied input directories
again with `--spans`. A relative `replace` path in the copy's `go.mod` is rewritten to the absolute
path of the original. The re-read set must equal `S'` (§ 5) byte for byte in canonical encoding,
positions included, and the new span documents must pass strict reading. A difference refuses the
whole change set and prints the first differing path of `cir/1` with both values. A dry run runs
the same verification; `--no-verify` does not exist.

Why a copy and not an overlay inside the readers: `go/build` resolves module imports only when no
file-system hook is set (`/usr/lib/go/src/go/build/build.go:1136-1145`, `importGo` returns
`errNoModules` once `OpenFile` or `ReadDir` is set), and `parser.ParseDir` reads the disk directly
(`front.go:68`). A copy keeps every reader's resolution as it is on the original, and the build
check (§ 7.4) runs in the same copy. The copy is made once per run; its time on the examples is
reported by `story:edit-slice-rust-rename`.

### 7.4 Build check

`--build` also compiles the copy, after the re-read matched (§ 7.3). The copy lives under
`$CODESPEC_CACHE_DIR/edit` (else `$XDG_CACHE_HOME/codespec/edit`, else `~/.cache/codespec/edit`)
and is removed after the run. Go runs `go build ./...` and `go vet ./...` over the whole copied
`go.mod` root, not only the input packages (test files included), with `GOWORK=off`; Rust runs
`cargo check --all-targets --quiet --message-format short` in the copied workspace root, its target
directory inside the copy. Each command is bounded at 10 minutes. Both run through the host's
build pool where one exists (`cargo` and `go` shims). A failure gives the status `unverified`: the
reason `build failed: <root>: <command> exited with …` and the first 20 lines of the compiler's
output, exit 1, nothing written.

## 8. Determinism

Rows are sorted (cq § 7); operations apply in row order; text edits are sorted by file path, then
start; `gofmt`'s whitespace changes are edits too (§ 6.1); the diff is generated from the edits themselves (hunks around each edited span, three lines
of context), not by a diff algorithm; the report has no time field. The same sources and program
give the same bytes, and running a program again on its own output changes nothing (the rows that
selected the old names are gone).

## 9. Languages for wave 1: Rust and Go

- Reader maturity: Go reads 115 and Rust 113 of 128 features verified (`website/data/support-matrix.json`),
  the two highest; both read real code in this workspace (the Go reader a 35-package service,
  `epic:go-reader-refusals`; the Rust reader whole crates, `story:rust-reader-whole-crate`).
- Byte offsets are at hand in both (§ 2).
- Rust's Go spelling is the test that the design is language-agnostic: the selector and the
  verification work on `cir/1` names, the edit on source spellings, and nothing in `crates/edit`
  knows a language beyond its identifier rules.
- Rust goes first: the Go reader is being changed on `wave/go-reader-refusals-1`, so the first
  end-to-end slice (`story:edit-slice-rust-rename`) is a Rust rename, and the Go reader's spans
  start after that branch is merged (§ 13).
- Java next (105 features read, no writer needed for span edits); C after (its reader is Rust on
  clang's JSON AST, which has offsets). Groovy, Elixir, Gleam and Odin read too little of real code
  to rename safely.

## 10. What it does not do

- Edit code the reader did not read (foreign statements, opaque bodies, macros' insides): refused,
  naming the site.
- Text search and replace, or edits of comments and strings.
- Re-format code outside the edited spans (Go's `gofmt` of a file that was `gofmt`-clean aside).
- Prove that a change keeps behaviour beyond the `cir/1` equality: a rename is behaviour-preserving
  by construction; a template edit (wave 2) is proved local, and behaviour is the evaluator's
  differential run (`story:mcp-verify-change`).
- Edit two languages in one run (wave 1: one `--lang`), or files outside the inputs.
- Serve an editor (no LSP); the MCP server (`epic:agent-mcp-server`) calls the same crate.

## 11. Home: codespec

The selector (cq), the typed model (`cir/1`), name resolution (the readers, on `go/types` and
`syn`) and the re-read that verifies an edit are all here; an edit needs every one of them. codegate
resolves references per backend in Go and has one backend for Go; a second language there means a
second resolver. codegate can call `codespec edit` as a backend later. Timo confirmed codespec on
2026-11-24.

## 12. Relation to the existing plan

| artifact | status | decision |
|---|---|---|
| `epic:codebase-transformations` | active | kept for instrumentation, mutation testing and patterns; its Outcome and Out of Scope now say that a rewrite lands in the original files through `epic:code-edit-language`. |
| `story:cir-patterns` | draft | left. cq is the selector; a source-language pattern lowered to cq (`docs/query-language.md:298`) stays a later convenience and blocks nothing here. |
| `story:cir-rewrite` | draft | superseded by `story:edit-replace-template`: the template replaces the original file's span instead of a module re-emitted by the Go writer. Archived after wave 1 (D4). |
| `story:reader-function-spans` | draft | superseded by `story:edit-spans-format` with the reader span stories (function `decl` spans), `story:edit-imports` (import ranges) and `story:edit-spans-c-reader`. Archived after wave 1 (D4). |
| `story:reach-at-positions` | draft | depends on `story:edit-spans-format` and `story:edit-spans-go-reader`; its `--spans` reads `codespec-spans/1` `decl` spans. |
| `story:reader-indirect-call-sites` | draft | depends on `story:edit-spans-c-reader` (same C adapter) and `story:edit-spans-format` (`docs/design.md`). |
| `story:reader-indirect-sites-go`, `story:reader-indirect-sites-rust` | draft | depend on `story:edit-spans-go-reader` and `story:edit-spans-rust-reader` (same adapters). |
| `story:mutation-testing`, `story:instrumentation` | draft | restated on `codespec edit` templates; depend on `story:edit-replace-template` (instrumentation also on `story:edit-imports`). |
| `story:mcp-rewrite-preview` | draft | restated: previews are `codespec edit` dry runs through `crates/edit`; depends on `story:edit-replace-template`. |
| `story:mcp-apply-in-place` | draft | restated: applies a preview through `crates/edit`; depends on `story:edit-cli-verified-apply`, `story:edit-imports` and `story:edit-spans-c-reader`. |

## 13. Plan and hold

One wave, on the integration branch `wave/code-edit-language`, in this order (stories of one line
run side by side):

1. `story:edit-spans-format`
2. `story:edit-cq-items`, `story:edit-spans-rust-reader`, `story:edit-engine-rename`
3. `story:edit-slice-rust-rename` (first end-to-end proof), `story:edit-engine-delete-retarget`
4. `story:edit-spans-go-reader`, held: it starts only after the pull request of
   `wave/go-reader-refusals-1` is merged to `main` and the integration branch has merged
   `origin/main`
5. `story:edit-cli-verified-apply`, then `story:edit-build-and-format`, then
   `story:edit-examples-and-docs`

## 14. Revision after the plan critics (2026-11-24)

Four critics (acceptance, scope, parallel safety, design) returned 34 findings. Taken here: spans
live in `crates/edit`, not `crates/model` (no `sha2` in the model, no manifest change); the reader
prints `at` paths itself (the Rust reader's model types refuse extra members); re-reading uses a
cache copy instead of an in-reader overlay (`go/build` drops module resolution under hooks); every
refusal is Rust in `crates/edit`; method renames include method sets; Rust `use` aliases; `node`
spans and the `extent` relation left out of wave 1; a Rust slice before the full command; the Go
reader hold; tests in existing test binaries. Each story names the findings it answers.

Round 2 (17 warnings): the pipeline is `codespec_edit::run` so `crates/mcp` can call it (§ 5);
`ids` maps functions, types and fields as well as body nodes; Rust copies and builds the workspace
root (§ 7.3); `gofmt` changes are merged text edits (§ 6.1); checkable cases for the `go.mod`
rewrite, the JSON report, possible rows and the "not yet" exits; a sh fixture reader stands in for
a reader without spans; the refusal examples are counted per language; the later drafts and
`epic:agent-mcp-server` are restated against the cache copy and `codespec edit`.

## 15. Open questions

Defaults taken by the coordinator on 2026-11-24:

- **D1, where spans live.** A: the side document `codespec-spans/1` (taken). B: an optional `span`
  field on `cir/1` nodes.
- **D2, exported targets.** A: refuse unless `--closed-world` (taken). B: edit and warn.
- **D3, Go formatting.** A: `gofmt` a Go file after the edit when it was `gofmt`-clean before
  (taken). B: never reformat.
- **D4, the superseded drafts.** A: archive `story:cir-rewrite` and `story:reader-function-spans`
  when wave 1 is implemented (taken). B: archive now.
