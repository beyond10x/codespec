---
title: Code edits
sidebar_position: 5
description: How codespec edit changes Go and Rust source with a cq program, which rows it refuses, and how each change is read again and compared with the expected cir/1 before anything is written.
lede: A cq program selects what to change, an edit operation says how. codespec edit applies it to the source bytes, reads the result again, and writes it only when the result is exactly the cir/1 it expected.
source: docs/code-edit-language.md, crates/edit/src/lib.rs, crates/cli/src/lib.rs, examples/edit, scripts/examples-edit.sh, commit 1dbbc9fe
---

# Code edits

## The notation

An edit program is a cq program with `edit` items. The body of an item is an ordinary cq body over
the fact base `codespec query --schema` prints. Each row of the body is one operation, and all
rows of a program form one change set:

```
edit rename(F, "Fetch") :- func{id: F, module: "example.com/depot/stock", name: "Load"}.
```

The target is named in `cir/1` terms: a function id, a type id, a call site. The new name is
written as it is to appear in the file. Only definite rows edit. A row that rests on code the fact
base cannot see is refused, with the sites it rests on. `codespec query` refuses a program that
holds an `edit` item.

```console
codespec edit --lang go --closed-world ./stock ./front -f rename.cq
codespec edit --lang rust ./yard -e 'edit delete(F) :- func{id: F, name: "spare"}, !calls(_, F).' --apply
```

Each input directory is read with `codespec-read-<lang> --spans`: the `cir/1` module and a
`codespec-spans/1` document that ties each function, field, call and field access to its bytes in
the source. Go and Rust readers give spans.

## Operations

| operation | target | what changes in the source |
|---|---|---|
| `rename(F, New)` | a function or method | the declaration's name and every reference: calls, method calls, function values, calls from other modules of the set, Rust `use` paths |
| `rename_field(T, Old, New)` | a struct type and a field name | the field's declaration, every access, every keyed literal field; a Rust shorthand `T { x }` becomes `T { new: x }` |
| `retarget(Site, G)` | a call and a function of the callee's module with the same signature | the callee's name at that call |
| `delete(F)` | a function or method nothing references | the declaration, from its doc comment or first attribute to its end |

The source changes by byte-span edits of the original files, so comments, blank lines and layout
outside the edited spans stay as they were. A Go file that was `gofmt`-clean before the edit is
run through `gofmt`, and its whitespace changes join the diff; a change outside whitespace is
refused.

## Refusals

A refused row changes nothing; the other rows of the program are planned and checked again, and
the run still exits 1 with nothing written. A row is refused, naming the rule and the site, when:

| rule | when |
|---|---|
| `target` | the target names nothing of the set |
| `listed`, `foreign`, `unread`, `named` | a reference to the target is in code the reader did not read: a call inside a Go `go` statement or a Rust macro, kept as a `foreign` statement, or an unresolved call with the target's spelling |
| `unread` | code the reader refused (a Rust `macro_rules!` body, a `static`'s initializer) holds the target's name as a word, or a `rename` targets a function the reader kept opaque |
| `trait` | the target is a Rust method in an `impl <Trait> for T` whose trait no input declares (`Default`, `Display`) |
| `reflect` | the name is visible to reflection, even with `--closed-world`: a field of a Go struct with a tag, or whose values go to a function outside the inputs (`fmt.Sprintf("%+v", …)`) or become an interface; a field of a Rust struct that derives anything but `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and `Default` (`Debug`, `Serialize`) or has an attribute macro; an exported Go method whose type's values go outside the inputs or become an interface the inputs do not declare (`fmt.Stringer`) |
| `spelling` | a reference is spelled otherwise and no Rust `use … as` alias explains it |
| `exported` | the target is exported and `--closed-world` is not given: callers outside the inputs are not read |
| `interface` | the target is a method that implements an interface or trait of the set |
| `collision` | the new name is a declaration of the module, or a local or parameter of a body that references the target |
| `generated` | the edit lands in a generated file (Go: a `// Code generated … DO NOT EDIT.` header) |
| `overlap` | two edits overlap and differ |
| `lowered`, `no-span` | a node to edit has no span: the reader built it from source it cannot point at |
| `name`, `exportedness` | the new name is not an identifier of the language, is a keyword, or changes Go exportedness |
| `embedded` | `rename_field` of an embedded field, which is named after its type |
| `referenced`, `main` | `delete`: something of the set references the target, or it is `main` or `init` |
| `source`, `lines` | `delete`: the file's bytes differ from those read, or the declaration shares its first or last line with other text |
| `signature`, `module`, `unexported` | `retarget`: the new callee's signature differs (its receiver too: Rust `&self` against `self`, Go `T` against `*T`), it is in another module, or it is unexported and the call is in another module |

Comments, string literals and doc text that mention the old name are not changed. Test files the
reader does not read (Go `_test.go`, Rust `#[cfg(test)]`) are not changed either; `--build`
catches a test that still names the old function.

## Verification

Before anything is printed, `codespec edit` copies the module root of the inputs (Go: the
directory of the `go.mod` above them; Rust: the Cargo workspace root) without `.git` and `target`
into `$CODESPEC_CACHE_DIR/edit`, writes the edited files there and reads the copied directories
again. The set read must equal the expected `cir/1` byte for byte, positions included; otherwise
the first differing path is printed and nothing is written. A dry run is checked the same way.

| flag | what it adds |
|---|---|
| none | a dry run: the unified diff on standard output, paths relative to the current directory |
| `--apply` | writes each file through a temporary file and a rename, only when every file read still has the SHA-256 it was read with; every temporary file is written before the first rename, and a failed write leaves every file as read (exit 2) |
| `--build` | compiles the checked copy first: Go with `go build ./...` and `go vet ./...` over the `go.mod` root (`GOWORK=off`), Rust with `cargo check --all-targets`; a failure is status `unverified` with the compiler's first 20 lines |
| `--format json` | prints `codespec-edit-report/1`: each row's status and reasons, each edited file's SHA-256 before and after, the diff |

Exit 0: every row planned or written, or no row. Exit 1: a row refused, a check failed or a file
changed since it was read; nothing written. Exit 2: an error in the program, a reader or an input.

## The examples

`examples/edit/go` holds a Go module with two packages, `examples/edit/rust` a Rust crate. Each
holds one program per operation and one per refusal its source can show: six for Go and five for
Rust. Each program has its flags (`<name>.args`) and its expected diff (`<name>.diff`) or refusal
(`<name>.refusal`). The Go `rename_field` program renames a field of a `gofmt`-aligned struct:

```diff
--- a/source/stock/stock.go
+++ b/source/stock/stock.go
@@ -3,8 +3,8 @@
 
 // Item is one kind of good on the shelves.
 type Item struct {
-	Name  string
+	Name     string
-	Count int
+	Quantity int
 }
 
 // Load gives the number of goods on the shelves.
```

`task examples-edit`, a part of the gate, runs every program as a dry run and compares its output
and exit code. It checks that no file of the sources changed, then applies each operation to a copy
under `target/` with `--build` and runs it again, which must change nothing. Each refusal runs with
`--apply` on a copy and must leave every file as it was. A self-test first runs the Go `rename`
program once; its output must match the diff as checked in and must not match a copy that differs
in one byte.

The flags are in the [CLI reference](/docs/reference/cli).
