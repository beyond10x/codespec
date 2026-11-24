//! Code edits in `cir/1` terms (`docs/code-edit-language.md`). This crate holds the side document
//! that ties a module's nodes to its source bytes, `codespec-spans/1` ([`spans`]), the mapping
//! between its paths and cq's ids ([`ids`]), and reading a source directory with spans ([`run`],
//! the one module that starts processes and touches files).
//!
//! # `codespec-spans/1`
//!
//! One document per read module, beside its `cir/1` document, never inside it:
//!
//! ```json
//! {
//!   "format": "codespec-spans/1",
//!   "module": "example.com/orders",
//!   "cir_sha256": "<SHA-256 of the canonical cir/1 document>",
//!   "files": [{"path": "orders.go", "sha256": "…", "len": 4180, "generated": false}],
//!   "spans": [
//!     {"at": "funcs.3", "role": "decl", "file": 0, "start": 1180, "end": 1622, "line": 41, "end_line": 58},
//!     {"at": "funcs.3", "role": "name", "file": 0, "start": 1236, "end": 1240, "line": 43, "end_line": 43, "text": "Load"}
//!   ],
//!   "unspanned": [{"at": "funcs.5.body.0.x.x", "role": "name", "reason": "implicit"}],
//!   "refs": [{"target": "example.com/orders.Load", "file": 1, "start": 88, "end": 92, "text": "load", "kind": "use"}]
//! }
//! ```
//!
//! - `module` is the module's `path`, or its `name` when it has none; `cir_sha256` the SHA-256 of
//!   the `cir/1` document's canonical encoding, lower-case hex.
//! - `files`: each source file by its path relative to the directory the reader was given (no
//!   `.` or `..` part), the SHA-256 and length of its bytes, and `generated` (an edit in it is
//!   refused). Sorted by path; spans and refs name a file by its index.
//! - `spans`: `at` is a node's path ([`ids`]), `start..end` its bytes, `line` and `end_line` the
//!   1-based lines of `start` and of the last byte (a line ends at `\n`). Role `decl`: a
//!   function's whole declaration, from its doc comment or first attribute to its end, without
//!   `text`. Role `name`: the identifier token spelling the node's name, callee or field, its
//!   bytes as `text`; on a struct literal's field written in shorthand (Rust `T { x }`), `form:
//!   "shorthand"`.
//! - `unspanned`: a node that needs a span of `role` and has none: `implicit` (no token spells it,
//!   nothing to edit) or `lowered: <why>` (the reader made the node from source it cannot point
//!   at; an edit needing it is refused).
//! - `refs`: names outside `cir/1` that refer to an item of the set by its cq id: Rust `use` path
//!   segments (`kind: "use"`); an `as` alias is outside the span.
//!
//! Strict reading ([`spans::read`], given the `cir/1` module and the files) refuses, naming the
//! rule and the `at` ([`spans::SpanError`]): text that is not JSON (`json`) or not of the shape
//! above (`shape`); another `format` (`format`); an unknown field (`unknown-field`); a `module`
//! other than the document's (`module`); a `cir_sha256` other than the document's
//! (`cir-sha256`); a path outside the directory or listed twice (`file-path`), a file that cannot
//! be read (`file`), a SHA-256 or length other than the file's (`file-sha256`, `file-length`); a
//! span or ref that is not a non-empty range inside its file (`in-file`) or does not start and end
//! on UTF-8 boundaries (`utf8-boundary`); `line` and `end_line` other than its bytes' (`lines`);
//! a `name` span without `text`, a `decl` span with one, or a `text` other than the file's bytes
//! there (`text`); a `form` anywhere but a struct literal field's `name` span (`form`); an `at`
//! naming no node of the document, or a node of a kind its role does not take (`kind`: `decl`
//! takes a function, `name` a function, a struct field, a `call`, `method`, `extern`,
//! `extern_method`, `func_ref` or `field` expression, or a struct literal's field); an (`at`,
//! role) given twice, as spans or as span and `unspanned` entry (`duplicate`); a reason other
//! than `implicit` or `lowered: <why>` (`reason`); two `name` spans overlapping (`overlap`); and a
//! node without the span completeness asks for and without an `unspanned` entry (`complete`):
//! every function (`funcs` and `opaque`) a `decl` and a `name` span, every struct field, every
//! expression of those six kinds and every struct literal field a `name` span. An `at` that
//! names the wrong node of the right kind passes; verifying an edit by reading it again catches
//! it.
//!
//! The canonical encoding is two-space JSON with a final newline, the members in the order above,
//! spans and `unspanned` sorted by `at` ([`spans::at_order`]: segment by segment, numbers by value
//! before words) then role, refs by file, start, end, then target. Strict reading gives that
//! order whatever order the reader printed.
//!
//! # The adapter protocol
//!
//! `codespec-read-<lang> --spans <dir>` prints one JSON object `{"cir": <the cir/1 module>,
//! "files": […], "spans": […], "unspanned": […], "refs": […]}`, the lists as in the span
//! document; the reader computes each `at` itself. [`run::read_spanned`] strictly reads the
//! `cir/1` module, adds `format`, `module` and `cir_sha256`, and strictly reads the span document;
//! the `cir/1` module is the one a read without `--spans` gives. A reader that fails on `--spans`
//! or prints a bare module is refused: "the <lang> reader gives no spans".
//!
//! # Change sets
//!
//! Between the rows of an edit program and the files, with no process, file system or adapter
//! call: [`change::plan_with`] turns the operations of the rows (`rename`, `rename_field`:
//! [`rename`]; `retarget`: [`retarget`]; `delete`: [`delete`], which needs the files' bytes)
//! into the expected set `S'` on `cir/1` and text edits from the span documents, refusing what
//! the reader could not see ([`change::RULES`]); [`compare::compare`] compares a re-read set
//! with `S'`; [`diff::unified`] prints the edits as a unified diff. [`lang`] holds the little a
//! change set knows of a language, [`refs`] the walk that finds the names an operation renames.

pub mod change;
pub mod compare;
pub mod delete;
pub mod diff;
pub mod ids;
pub mod lang;
pub mod post;
pub mod refs;
pub mod rename;
pub mod retarget;
pub mod run;
pub mod spans;
