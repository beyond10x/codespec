//! The C reader: the C11 sources of one directory to one `cir/1` module, from the typed AST clang
//! prints as JSON (`clang -Xclang -ast-dump=json -fsyntax-only -std=c11`, one run per `.c` file,
//! `clang` from `PATH`). Every expression's type is clang's (`qualType`, through typedefs), with
//! one exception, stated in "C in runtime form": in a directory holding the kit's runtime header
//! `codespec_rt.h`, the reader recognises the names that header declares in the forms that section
//! lists, and takes the element and key types of `cs_slice`, `cs_map` and `cs_chan`, which C does
//! not record, from the `codespec: func` lines and the forms that section names. A runtime name
//! used outside those forms is refused, never guessed. Nothing else is guessed: a statement outside
//! the subset is kept as a `foreign` statement in `c` (its source, the reason and the calls it
//! makes), a declaration outside it is left out or kept as an opaque function, each with a refusal
//! at its position. Positions are `<file>:<line>`, the file relative to the source directory.
//!
//! The module is named after the directory, every character other than a letter, a digit or `_`
//! read as `_`, and `_` put before a leading digit (`zlib-1.3.1` is `zlib_1_3_1`). A parameter
//! without a name is `_`. Every `.c` file of the directory is
//! read, in name order; a header is read where a `.c` file includes it, and only a header in the
//! directory declares the module's types, variables and functions.
//!
//! # Declarations
//!
//! | C | cir/1 |
//! |---|---|
//! | a function definition | function; `static` is unexported, any other exported |
//! | a variadic function definition | opaque, with a refusal |
//! | a function whose parameter or result type is outside the subset | opaque, with a refusal |
//! | `struct T { … }`, `typedef struct { … } T` | `struct`, fields in order |
//! | `enum E { … }`, `typedef enum { … } E` | `enum` over `int` (32-bit, signed), values as clang computes them |
//! | `enum { A = 1, … }` without a name | a `const` of type `int` per enumerator |
//! | a file-scope variable | `var`; an initializer outside the subset is kept as foreign source |
//! | `typedef T Name;` in the source directory, `T` an integer, `_Bool` or floating-point type | `newtype` `Name` over `T`: a value clang types `Name` is of that newtype, distinct from `T` |
//! | a second such `typedef` of `Name` to another type (in another file), one named as a struct, union or enum of the module | refused, and so is a value of it: a module has one type of a name |
//! | any other `typedef` | nothing: the type it names stands for it |
//! | `union`, a bit field, an unnamed struct | refused |
//! | a function the module calls and does not define | `extern` with its declared signature: path `libc` when a header outside the directory declares it, else the declaring file |
//!
//! # Types
//!
//! | C | cir/1 |
//! |---|---|
//! | `_Bool` (`bool`) | `bool` |
//! | `signed char`, `short`, `int`, `long`, `long long` and their `unsigned` forms, `int8_t`…`uint64_t` | `int` of width 8, 16, 32, 64, 64 (LP64), signed or not |
//! | `const char *` | `string`: read where a string literal or a parameter of that type flows; nothing indexes, compares or walks it |
//! | `struct T`, `enum E` the directory declares (a field of `struct T` may point to `T`), a newtype `typedef` | `named` |
//! | `float`, `double` | `float` of 32, 64 bits |
//! | `T *`, `const T *` (but `const char *`) | `ptr` to `T` |
//! | `T[N]` | `array` of `N` `T` |
//! | `R (*)(P, …)` | `func` from `P, …` to `R`, no result for `void` |
//! | `void` (a result) | no result; a call of it is typed `unit` |
//! | variadic extern parameters (`...`) | a last parameter `[]any`, the extern `variadic` |
//! | plain `char`, `long double`, `void *`, a pointer to or an array of `const char *`, a pointer to an array, an array without a constant length, a function pointer without a prototype or to a variadic function, `volatile`, `_Atomic` | refused |
//!
//! # Statements
//!
//! | C | cir/1 |
//! |---|---|
//! | `T x;` or `T x = 0;` (a zero initializer: `0`, a null pointer constant, `{0}`, `{}`) | `var` |
//! | `T x = e;` | `let`, `e` of type `T` |
//! | `x = e;`, `x op= e;` (`x` a variable, a field, `a[i]` or `*p`) | `assign`; `x &= ~e` is `&^=` |
//! | `x++;`, `++x;`, `x--;`, `--x;` | `incr`, `decr` |
//! | `f(…);` | `expr` |
//! | `if`, `else` | `if` |
//! | `for (init; cond; inc)` | `for`: `init` a declaration (`let`) or an assignment, `inc` an assignment, `++` or `--`. `&` of a variable `init` declares is refused: C has one such variable for all iterations, Go one per iteration |
//! | `while (c)`, `for (;;)`, `while (1)` | `for` with or without a condition |
//! | `do { … } while (c);` | `for { …; if !c { break } }`; a `continue` in it is refused, as it would run `c` |
//! | `switch` on an integer, `case` values constant | `switch`; stacked labels are one case; a case's trailing `break` leaves the switch, which a `cir/1` case does by ending. A case that falls through, or a `break` elsewhere in a case, is refused |
//! | `break`, `continue` in a loop | `break`, `continue` |
//! | `return`, `return e` | `return` |
//! | a label | its statement: only the `goto` that jumps to it is refused |
//! | `goto`, inline assembly, a block with declarations of its own (but a switch in runtime form) | refused |
//!
//! # Expressions
//!
//! Every expression's type is clang's type of it, but for comparisons and logic.
//!
//! | C | cir/1 |
//! |---|---|
//! | an integer literal; an integer constant converted to the type its context needs | `lit` of that type, under `-` when negative |
//! | a floating-point literal (`0.25`, `1e21f`) | `float` of its type: the shortest decimal that reads back to clang's value of it |
//! | `-` of a floating-point literal (`-0.0`), an operator or comparison whose operands are all floating-point constants (`0.1 + 0.2`) | refused: the Go writer writes them as Go constant expressions, which Go evaluates exactly (`-0.0` is positive zero, `0.1 + 0.2 == 0.3` true), not in binary floating point as C does; read once the Go writer writes them as C evaluates them |
//! | `"…"` (printable ASCII, the escapes `\n \t \r \\ \"`) | `lit` of type `string`, spelled as Go spells it |
//! | a null pointer constant (`0`, `NULL`, `(void *)0`) where a pointer or function pointer is expected or compared with | `nil` of that type |
//! | a parameter or local, a file-scope variable | `var`, `global` |
//! | an enumerator where its enum is expected, an anonymous enum's enumerator | `const` |
//! | the name of a function the module defines, as a value | `func_ref` |
//! | `+ - * / % & | ^ << >>` on one integer type or newtype over one (`<<` and `>>` with any count type), `+ - * /` on one floating-point type | `binop`; `x & ~y` is `&^` |
//! | unary `- +` on a number, `~` on an integer | `unop` `- + ^` |
//! | `&x` of a variable, a field, an element or a struct literal `(struct T){…}`; `*p` | `unop` `&`, `*` |
//! | `(T)x` for a newtype `T`, `x` of the type it is defined over | `conv` |
//! | `< > <= >= == !=` on one number, enum or newtype type, `== !=` on pointers or a function pointer and `NULL`, `&& || !` | `binop`, `unop` of type `bool`, read where C converts them to `_Bool` or tests them; an integer tested is `x != 0` |
//! | `f(…)` | `call` of a function the module defines, else `extern` |
//! | `f(…)` through a function pointer (a local, a parameter, a field) | `call_value` |
//! | `s.f`, `p->f` | `field`; through a pointer, of the pointer, as `cir/1` reads a field through one |
//! | `a[i]` of an array | `index` |
//! | `(struct T){…}`, `{…}` initializing a struct | `struct`, the fields it writes |
//! | `(struct T){0}`, `(struct T){}`: a struct literal of zeros only, but under `&` | `zero` of `T` |
//!
//! An implicit or explicit conversion between two integer or floating-point types, between an
//! integer and a floating-point type, a cast from a newtype, a `_Bool` used as an integer and a
//! comparison used as an `int` are refused: `cir/1` has no such conversion. So are pointer
//! arithmetic (indexing a pointer among it), ordering pointers, comparing two function pointers,
//! `&` of a function or of a non-struct literal, `?:`, `sizeof` (but in the runtime forms), the comma operator, an assignment
//! or `++` inside an expression, an array initialized with values other than zero, character
//! literals and a function the module does not define used as a value.
//!
//! # C in runtime form
//!
//! C has no slice, map, channel, `any`, error or length-carrying string. The kit's C sources write
//! them with the runtime the C writer writes beside its output, `conformance/c/codespec_rt.h`
//! (byte-equal to the writer's, `tests/reader.rs`). A directory is in runtime form when it holds
//! a `codespec_rt.h` and one of its `.c` files has the line `#include "codespec_rt.h"`. A
//! `codespec_rt.h` other than the kit's is refused at its first line, and none of the forms below
//! is read: they mean what they mean only with that header.
//!
//! **Types.** `int64_t` is `int` (of the platform's width, 64 bits, as the writer writes `int`),
//! `cs_string` `string`, `cs_error` `error`, `cs_any` `any`. A `cs_slice`, `cs_map *` or `cs_chan *`
//! records no element or key type, so it has the type
//!
//! - of its parameter or result in the function's `codespec: func` line;
//! - of its initializer, for a local initialized with a value of a known type (`cs_map_new`, a
//!   call, another variable);
//! - for a local `cs_slice` declared empty (`cs_slice_make()`, `{0}`), of every use that pins it:
//!   `cs_append` and `cs_slice_at` with `sizeof(T)`, an argument or a returned value whose
//!   `codespec: func` line types it. A slice no use pins, or whose uses pin different types, is
//!   refused.
//!
//! Any other runtime type (`cs_sb`, `cs_type` outside the globals below) is refused.
//!
//! **Signatures.** Every function definition has a `/* codespec: func <Name>(<types>) <results> */`
//! line, in Go's spelling of the `cir/1` types, as the C writer writes it into its header; the line
//! types the function and each C type is checked to hold its type. A function of several results
//! returns `typedef struct { T0 r0; T1 r1; … } <Name>_result;`. A function without the line, a
//! second line of one name, or a C type that does not hold the line's type is refused.
//!
//! | C in runtime form | cir/1 |
//! |---|---|
//! | `(cs_string){"…", n}`, `n` the literal's length in bytes | `lit` of type `string` |
//! | `s.n` of a `cs_string`, `xs.len` of a `cs_slice` | `len` |
//! | `cs_string_eq(a, b)`, `cs_string_cat(a, b)` | `==`, `+` on strings |
//! | `(T)((uint64_t)a op (uint64_t)b)`, `op` one of `+ - *`, `T` signed; `(T)((uint64_t)0 - (uint64_t)a)` | `binop` `op`, `unop` `-`: they wrap, as `cir/1` integers do |
//! | `cs_div_T(a, b)`, `cs_rem_T(a, b)` | `/`, `%` |
//! | `cs_slice_make()` where a declaration or assignment gives a slice type | `make` of it |
//! | `cs_append(xs, &v, sizeof(T))`, `&(T){e}` for `&v` | `append(xs, v)` |
//! | `*(T *)cs_slice_at(xs, i, sizeof(T))` | `index` |
//! | `cs_map_new(sizeof(K), sizeof(V), CS_KEY_…)`, the key kind `K`'s | `make` of `map[K]V` |
//! | `*(V *)cs_map_put(m, &k) = e;` (only assigned to) | `assign` to `m[k]` |
//! | `*(V *)cs_map_get(m, &k, sizeof(V))` | `index` |
//! | `cs_map_len(m)`, `cs_map_sorted_keys(m)` | `len`, `sorted_keys` |
//! | `cs_chan_send(ch, &v);` | `send` |
//! | `*(T *)cs_chan_recv(ch)` | `unop` `<-` |
//! | `cs_box(&desc, &v, sizeof(T))`, `desc` a `static const cs_type desc = {CS_K_…, "name", NULL};` of `T`'s kind | the value as `any` |
//! | `static struct cs_error_s X_s = {{"…", n}, NULL, NULL};` and `cs_error const X = &X_s;` | `error_var` `X` |
//! | a chain from `cs_sb_new()` of `cs_sb_bytes(b, "…", n)`, `cs_fmt_str`, `cs_fmt_int`, `cs_fmt_bool`, `cs_fmt_error`, ended by `cs_sb_string`, `cs_sb_error` or `cs_sb_println` | `sprintf`, `errorf` (`%s %d %t %v`), `println` (values separated by one space) |
//! | `<Name>_result t = <Name>(…); T0 x0 = t.r0; …` (`t` used no further) | `let x0, … := <Name>(…)` |
//! | `return (<Name>_result){e0, e1, …};` | `return e0, e1, …` |
//! | `T cs_t1 = e1; … x1 = cs_t1; …` (in order, the temporaries used no further) | `assign` `x1, … = e1, …` |
//! | `V *p = cs_map_find(m, &k); if (p == NULL) { … return …; } V v = *p;` (`p` used no further) | `lookup_else` |
//! | `{ T cs_s1 = tag; if (cs_s1 == a \|\| …) { … } else if … else { … } }` | `switch tag`; a `break` in a case leaves the loop around, as in `cir/1` |
//! | `for (int64_t i = 0; i < xs.len; i++) { T v = *(T *)cs_slice_at(xs, i, sizeof(T)); … }` | `for _, v := range xs` |
//! | `for (int64_t i = 0; i < cs_map_len(m); i++) { K k = *(K *)cs_map_key(m, i); V v = *(V *)cs_map_val(m, i); … }` | `for k, v := range m` |
//!
//! An error compares only with `NULL` or a sentinel: C compares the pointers, which Go's `==`
//! on those does too. A loop over a slice or map is read as `range` only when its body neither uses
//! the counter nor assigns the container, which Go evaluates once. A near miss of each form (a wrong
//! length or size, a cast another type than the container holds, `cs_map_put` read as a value, a
//! printed line ending in a separator, a temporary used later) is refused with its reason,
//! `tests/runtime.rs`.
//!
//! # What the reader claims, and what it does not
//!
//! `codespec-read-c --describe` claims each feature whose conformance case has a hand-written C
//! source, `conformance/cases/<case>/c/case.c`. `tests/cases.rs` holds every claim: the source reads
//! without a refusal and with its subject, and passed and written as Go runs as the reference does.
//! Running the source as written is step 1 of `codespec conform`, with the C runner.
//!
//! Where a feature's first case needs a slice, an error or a string, its C source is in a variant
//! case that holds the same subject without them, and the first case has none: `stmt.expr` and
//! `type.unit` in `stmt-expr-method` and `type-unit-method` (a `void` function called on a
//! pointer to a struct), `type.float` in `type-float-compare`, `type.array` in `type-array-int`,
//! `expr.nil` in `expr-nil-ptr`, `expr.zero` in `expr-zero-struct`. The C source of
//! `expr-func-value` adds the step after the call, as C has no method value. The C sources of a
//! slice, a map, a channel, an `any`, an error, a tuple, a string operation or an intrinsic are in
//! runtime form ("C in runtime form"), as is `sem-int-wrap`'s: signed overflow is undefined in C,
//! and the runtime form's conversion through `uint64_t` is the wrap `cir/1` integers have.
//!
//! The vocabulary not claimed:
//!
//! | feature | why |
//! |---|---|
//! | `fact.*` | computed by `codespec pass`, never by a reader |
//! | `stmt.foreign` | what a reader writes for what it cannot read; a corpus case of it would hold the construct the reader refuses, which `codespec conform` counts only for that subject |
//! | `decl.variadic` | a variadic C definition reads its arguments through `va_list`, `va_start` and `va_arg` of `<stdarg.h>`, one at a time and with no count; `cir/1` has only a slice parameter for them, so reading one is a convention of its own, and the first case's subject is a slice |
//! | `expr.spread` | it passes a slice to a variadic parameter, which no C definition the reader reads has |
//! | `decl.interface`, `expr.iface_call`, `expr.to_iface` | C has no interface, and the runtime and the C writer no form of one |
//! | `decl.type_params`, `expr.instantiate`, `type.param` | C has no type parameter, and the runtime and the C writer no form of one |
//! | `expr.lambda` | C has no closure, and the runtime and the C writer no form of one |
//! | `expr.method`, `decl.embedded` | C has no method and no promoted field; reading a function `T_m(T *self, …)` as a method would be a naming convention this crate doc does not state |
//! | `expr.to_error` | a value of a type of the module as an error is `cs_error_of(fn, &v)`, a function giving the message and a copy of the value, a form the reader does not read |
//! | `decl.union` | a C `union` has no tag, a `cir/1` union does: refused |
//! | `expr.extern`, `expr.extern_method`, `type.extern` | their cases call Go's standard library (`strings`, `strconv`), which C has not; a C function the module does not define is read as an `extern` of `libc`, which no case of them holds |

mod body;
mod json;
mod rtbody;
mod runtime;
mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use codespec_model::{
    ConstDecl, ErrorVar, Expr, Extern, Field, ForeignInit, Func, IntWidth, Module, Opaque, Param,
    Reader, Refusal, Type, TypeDecl, VarDecl, Variant,
};

use body::Body;
use json::J;
use types::{Tables, Typedef, Unit, int};

/// The text of every `.c` and `.h` file of the source directory, by name.
pub struct Source {
    files: BTreeMap<String, Vec<u8>>,
}

/// The location `l` stands for: its expansion for a location inside a macro.
fn at(l: &J) -> &J {
    match l.get("expansionLoc") {
        J::Null => l,
        e => e,
    }
}

fn file_name(f: &str) -> String {
    f.strip_prefix("./").unwrap_or(f).to_string()
}

/// The file of a location, when it has one.
pub(crate) fn file_of(l: &J) -> Option<String> {
    let f = at(l).s("file");
    (!f.is_empty()).then(|| file_name(f))
}

/// A file of the source directory: clang runs there on relative names, so every other file is
/// absolute or a `<built-in>` buffer.
pub(crate) fn in_dir(f: &str) -> bool {
    !f.is_empty()
        && !f.starts_with('/')
        && !f.starts_with('<')
        && !f.contains("..")
        && f != runtime::RT_FILE
}

/// A null pointer constant in an initializer: `NULL`, `0`, `(T *)0`, or a field left out.
fn null_constant(n: &J) -> bool {
    let mut n = n;
    while matches!(
        n.kind(),
        "ImplicitCastExpr" | "ParenExpr" | "ConstantExpr" | "CStyleCastExpr"
    ) {
        n = &n.inner()[0];
    }
    n.kind() == "ImplicitValueInitExpr" || body::constant(n) == Some(0)
}

/// Where a node begins, `<file>:<line>`.
pub(crate) fn pos_of(_src: &Source, n: &J) -> String {
    let mut l = at(n.get("range").get("begin"));
    if l.get("offset") == &J::Null {
        l = at(n.get("loc"));
    }
    format!(
        "{}:{}",
        file_name(l.s("file")),
        l.get("line").int().unwrap_or(0)
    )
}

/// The source text of a node, with the `;` that ends it.
pub(crate) fn src_of(src: &Source, n: &J) -> String {
    let (b, e) = (
        at(n.get("range").get("begin")),
        at(n.get("range").get("end")),
    );
    let Some(text) = src.files.get(&file_name(b.s("file"))) else {
        return String::new();
    };
    let start = b.get("offset").int().unwrap_or(0) as usize;
    let mut end =
        (e.get("offset").int().unwrap_or(0) + e.get("tokLen").int().unwrap_or(0)) as usize;
    if text.get(end) == Some(&b';') {
        end += 1;
    }
    text.get(start..end.max(start))
        .map(|t| String::from_utf8_lossy(t).into_owned())
        .unwrap_or_default()
}

/// Read every `.c` file of `dir` into one module.
pub fn read_dir(dir: &Path) -> Result<Module, String> {
    // The AST nests a level per operand; read it on a stack that holds a deep one.
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .name("c-read".into())
            .stack_size(256 << 20)
            .spawn_scoped(s, || read(dir))
            .expect("a thread to read on")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// `clang`'s JSON AST of `file`, run in `dir`.
fn clang(dir: &Path, file: &str) -> Result<Vec<u8>, String> {
    let out = Command::new("clang")
        .args(["-Xclang", "-ast-dump=json", "-fsyntax-only", "-std=c11"])
        .arg(file)
        .current_dir(dir)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => "clang not found".to_string(),
            _ => format!("cannot run clang: {e}"),
        })?;
    if !out.status.success() {
        return Err(format!(
            "clang rejects {file}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(out.stdout)
}

fn read(dir: &Path) -> Result<Module, String> {
    let mut names = Vec::new();
    let mut files = BTreeMap::new();
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !e.path().is_file() || !(name.ends_with(".c") || name.ends_with(".h")) {
            continue;
        }
        files.insert(
            name.clone(),
            std::fs::read(e.path()).map_err(|e| format!("{name}: {e}"))?,
        );
        if name.ends_with(".c") {
            names.push(name);
        }
    }
    names.sort();
    if names.is_empty() {
        return Err(format!("{}: no .c file", dir.display()));
    }
    let mut asts = Vec::new();
    for f in &names {
        asts.push(json::parse(&clang(dir, f)?)?);
    }
    let src = Source { files };
    let module = dir
        .canonicalize()
        .ok()
        .and_then(|d| d.file_name().map(|n| module_name(&n.to_string_lossy())))
        .unwrap_or_else(|| "c".into());
    Ok(Assembly::new(&src).run(&asts, module))
}

/// The module name of a source directory: every character but a letter, a digit and `_` read as
/// `_`, and `_` before a leading digit, so the name is an identifier (`zlib-1.3.1` is
/// `zlib_1_3_1`).
fn module_name(dir: &str) -> String {
    let mut s: String = dir
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

/// A parameter's name: `_` for one without, as the Go reader writes an unnamed parameter.
fn param_name(p: &J) -> String {
    match p.s("name") {
        "" => "_".into(),
        n => n.into(),
    }
}

/// The module being built, over every unit.
struct Assembly<'s> {
    src: &'s Source,
    tables: Tables,
    types: Vec<TypeDecl>,
    consts: Vec<ConstDecl>,
    vars: Vec<VarDecl>,
    funcs: Vec<Func>,
    opaque: Vec<Opaque>,
    externs: Vec<Extern>,
    layout: Vec<String>,
    refusals: Vec<Refusal>,
    /// Each typedef name of the source directory read for a newtype, with the position and the
    /// type of its first typedef.
    typedef_at: BTreeMap<String, (String, String)>,
    errors: Vec<ErrorVar>,
    /// The `codespec: func` lines, by function name.
    sig_lines: BTreeMap<String, runtime::SigLine>,
    /// The message of each `static struct cs_error_s` of the runtime form, by name.
    error_structs: BTreeMap<String, String>,
}

/// The typedef names an anonymous record or enum declaration gets, by declaration id.
fn anonymous_names(ast: &J) -> BTreeMap<String, String> {
    fn tag_decl(n: &J) -> Option<&J> {
        let d = n.get("decl");
        if matches!(d.s("kind"), "RecordDecl" | "EnumDecl") {
            return Some(d);
        }
        n.inner().iter().find_map(tag_decl)
    }
    let mut out = BTreeMap::new();
    for n in ast.inner().iter().filter(|n| n.kind() == "TypedefDecl") {
        if let Some(d) = tag_decl(n)
            && d.s("name").is_empty()
        {
            out.entry(d.s("id").to_string())
                .or_insert_with(|| n.s("name").to_string());
        }
    }
    out
}

impl<'s> Assembly<'s> {
    fn new(src: &'s Source) -> Assembly<'s> {
        Assembly {
            src,
            tables: Tables::default(),
            types: Vec::new(),
            consts: Vec::new(),
            vars: Vec::new(),
            funcs: Vec::new(),
            opaque: Vec::new(),
            externs: Vec::new(),
            layout: Vec::new(),
            refusals: Vec::new(),
            typedef_at: BTreeMap::new(),
            errors: Vec::new(),
            sig_lines: BTreeMap::new(),
            error_structs: BTreeMap::new(),
        }
    }

    /// Whether the directory is in runtime form, and its `codespec: func` lines; a
    /// `codespec_rt.h` other than the kit's is refused, and nothing of it read.
    fn runtime_form(&mut self) {
        let includes = self.src.files.iter().any(|(name, text)| {
            name.as_str() != runtime::RT_FILE
                && String::from_utf8_lossy(text)
                    .lines()
                    .any(|l| l.trim() == "#include \"codespec_rt.h\"")
        });
        let Some(rt) = self.src.files.get(runtime::RT_FILE) else {
            return;
        };
        if !includes {
            return;
        }
        if rt.as_slice() != runtime::RT.as_bytes() {
            self.refusals.push(Refusal {
                pos: format!("{}:1", runtime::RT_FILE),
                what: runtime::RT_FILE.into(),
                reason: "a codespec_rt.h other than the runtime of the kit: its forms are read only with that header".into(),
            });
            return;
        }
        self.tables.runtime = true;
        for (name, text) in &self.src.files {
            if name.as_str() == runtime::RT_FILE {
                continue;
            }
            if let Err(e) = runtime::sig_lines(&String::from_utf8_lossy(text), &mut self.sig_lines)
            {
                self.refusals.push(Refusal {
                    pos: format!("{name}:1"),
                    what: "codespec: func".into(),
                    reason: e,
                });
            }
        }
    }

    /// The types of every `codespec: func` line, once the module's types are read.
    fn resolve_sigs(&mut self) {
        let lines = std::mem::take(&mut self.sig_lines);
        for (name, l) in &lines {
            let ps: Result<Vec<Type>, String> =
                l.params.iter().map(|p| self.tables.go(p)).collect();
            let rs: Result<Vec<Type>, String> =
                l.results.iter().map(|p| self.tables.go(p)).collect();
            match (ps, rs) {
                (Ok(ps), Ok(rs)) => {
                    self.tables.sigs.insert(name.clone(), (ps, rs));
                }
                (Err(e), _) | (_, Err(e)) => self.refusals.push(Refusal {
                    pos: String::new(),
                    what: format!("codespec: func {name}"),
                    reason: e,
                }),
            }
        }
        self.sig_lines = lines;
    }

    fn refuse(&mut self, n: &J, what: String, reason: String) {
        self.refusals.push(Refusal {
            pos: pos_of(self.src, n),
            what,
            reason,
        });
    }

    fn run(mut self, asts: &[J], name: String) -> Module {
        self.runtime_form();
        for n in asts.iter().flat_map(|a| a.inner()) {
            if matches!(n.kind(), "RecordDecl" | "EnumDecl")
                && !n.s("name").is_empty()
                && in_dir(&file_of(n.get("loc")).unwrap_or_default())
            {
                let tag = match n.kind() {
                    "EnumDecl" => "enum",
                    _ => n.s("tagUsed"),
                };
                self.tables
                    .tags
                    .entry(n.s("name").into())
                    .or_insert_with(|| tag.into());
            }
        }
        let mut units = Vec::new();
        let mut defs = Vec::new();
        for ast in asts {
            let (unit, funcs) = self.declarations(ast);
            units.push(unit);
            defs.push(funcs);
        }
        self.resolve_sigs();
        for ((ast, unit), funcs) in asts.iter().zip(&units).zip(defs) {
            let decls: BTreeMap<String, &J> = ast
                .inner()
                .iter()
                .filter(|n| n.kind() == "FunctionDecl")
                .map(|n| (n.s("id").to_string(), n))
                .collect();
            for f in funcs {
                self.function(unit, &decls, f);
            }
        }
        self.externs
            .sort_by(|a, b| (&a.path, &a.name).cmp(&(&b.path, &b.name)));
        self.externs
            .dedup_by(|a, b| a.path == b.path && a.name == b.name);
        let read: BTreeSet<String> = self.funcs.iter().map(|f| f.name.clone()).collect();
        self.layout
            .retain(|l| l.strip_prefix("func:").is_none_or(|f| read.contains(f)));
        Module {
            format: codespec_model::FORMAT.into(),
            name,
            reader: Reader {
                language: "c".into(),
                name: "codespec-read-c".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            doc: String::new(),
            types: self.types,
            consts: self.consts,
            errors: self.errors,
            vars: self.vars,
            funcs: self.funcs,
            externs: self.externs,
            layout: self.layout,
            facts: None,
            refusals: self.refusals,
            path: String::new(),
            opaque: self.opaque,
            methods: Vec::new(),
            linknames: Vec::new(),
            inits: Vec::new(),
            interfaces: Vec::new(),
            entries: Vec::new(),
            beans: Vec::new(),
            injects: Vec::new(),
            transactional: Vec::new(),
        }
    }

    /// The types, constants and variables of one unit, and its function definitions to read.
    fn declarations<'j>(&mut self, ast: &'j J) -> (Unit, Vec<&'j J>) {
        let mut unit = Unit {
            anon: anonymous_names(ast),
            ..Unit::default()
        };
        let mut defs = Vec::new();
        for n in ast.inner() {
            let file = file_of(n.get("loc")).unwrap_or_default();
            let ours = in_dir(&file);
            match n.kind() {
                "TypedefDecl" => {
                    let t = n.get("type");
                    let target = match t.s("desugaredQualType") {
                        "" => t.s("qualType"),
                        d => d,
                    };
                    let named = unit
                        .anon
                        .iter()
                        .find(|(_, name)| *name == n.s("name"))
                        .map(|(_, name)| name.clone());
                    let newtype = ours && named.is_none();
                    unit.typedefs.insert(
                        n.s("name").into(),
                        Typedef {
                            target: target.into(),
                            named,
                        },
                    );
                    if newtype {
                        self.newtype(&unit, n, target);
                    }
                }
                "EnumDecl" if ours => self.enum_decl(&mut unit, n),
                "RecordDecl" if ours && n.is_true("completeDefinition") => self.record(&unit, n),
                "VarDecl" if ours => self.global(&mut unit, n),
                "FunctionDecl" if ours && n.inner().iter().any(|k| k.kind() == "CompoundStmt") => {
                    let name = n.s("name");
                    if !self.tables.funcs.insert(name.into()) {
                        self.refuse(
                            n,
                            format!("function {name}"),
                            "a second definition of the name".into(),
                        );
                        continue;
                    }
                    self.layout.push(format!("func:{name}"));
                    defs.push(n);
                }
                _ => {}
            }
        }
        (unit, defs)
    }

    fn tag_name(unit: &Unit, n: &J) -> Option<String> {
        match n.s("name") {
            "" => unit.anon.get(n.s("id")).cloned(),
            name => Some(name.into()),
        }
    }

    fn enum_decl(&mut self, unit: &mut Unit, n: &J) {
        let name = Self::tag_name(unit, n);
        let i32t = int(IntWidth::W32, true);
        let mut next: i128 = 0;
        let mut variants = Vec::new();
        for c in n.inner().iter().filter(|c| c.kind() == "EnumConstantDecl") {
            unit.enum_of.insert(c.s("id").into(), name.clone());
            let v = c
                .inner()
                .first()
                .and_then(|e| e.s("value").parse::<i128>().ok())
                .unwrap_or(next);
            next = v + 1;
            let value = if v < 0 {
                Expr::Unop {
                    op: "-".into(),
                    x: Box::new(Expr::Lit {
                        value: (-v).to_string(),
                        t: i32t.clone(),
                    }),
                    t: i32t.clone(),
                }
            } else {
                Expr::Lit {
                    value: v.to_string(),
                    t: i32t.clone(),
                }
            };
            variants.push((
                c,
                Variant {
                    name: c.s("name").into(),
                    value,
                },
            ));
        }
        match name {
            Some(name) => {
                if !self.tables.enums.insert(name.clone()) {
                    return;
                }
                self.layout.push(format!("type:{name}"));
                self.types.push(TypeDecl::Enum {
                    name,
                    exported: true,
                    under: i32t,
                    variants: variants.into_iter().map(|(_, v)| v).collect(),
                    doc: String::new(),
                    pos: pos_of(self.src, n),
                });
            }
            None => {
                for (c, v) in variants {
                    if !self.tables.consts.insert(v.name.clone()) {
                        continue;
                    }
                    self.layout.push(format!("const:{}", v.name));
                    self.consts.push(ConstDecl {
                        name: v.name,
                        t: i32t.clone(),
                        value: v.value,
                        pos: pos_of(self.src, c),
                    });
                }
            }
        }
    }

    /// A typedef of the source directory: a newtype when it names an integer, `_Bool` or
    /// floating-point type; any other typedef stands for the type it names. A second typedef of
    /// the name to another type, and a newtype named as a struct, union or enum of the module, are
    /// refused: a module has one type of a name.
    fn newtype(&mut self, unit: &Unit, n: &J, target: &str) {
        let name = n.s("name").to_string();
        let under = self.tables.spelled(unit, target).ok();
        let newtype = matches!(
            under,
            Some(Type::Int { .. } | Type::Bool | Type::Float { .. })
        );
        if let Some((first, was)) = self.typedef_at.get(&name).cloned() {
            let differs = match self.tables.spelled(unit, &was).ok() {
                Some(t) => under.as_ref() != Some(&t),
                None => was != target,
            };
            if differs && (newtype || self.tables.newtypes.contains_key(&name)) {
                self.refuse(
                    n,
                    format!("typedef {name}"),
                    format!(
                        "a second typedef of {name}, as {target}, where {first} has it as {was}: a module has one type of a name"
                    ),
                );
            }
            return;
        }
        self.typedef_at
            .insert(name.clone(), (pos_of(self.src, n), target.into()));
        let Some(under) = under.filter(|_| newtype) else {
            return;
        };
        if let Some(tag) = self.tables.tags.get(&name) {
            let why = format!(
                "a typedef of {target} named as the {tag} {name} of the module: a module has one type of a name"
            );
            self.tables.bad_typedefs.insert(name.clone(), why.clone());
            self.refuse(n, format!("typedef {name}"), why);
            return;
        }
        self.tables.newtypes.insert(name.clone(), under.clone());
        self.layout.push(format!("type:{name}"));
        self.types.push(TypeDecl::Newtype {
            name,
            exported: true,
            tparams: Vec::new(),
            under,
            doc: String::new(),
            pos: pos_of(self.src, n),
        });
    }

    fn record(&mut self, unit: &Unit, n: &J) {
        let tag = n.s("tagUsed");
        let Some(name) = Self::tag_name(unit, n) else {
            self.refuse(n, tag.to_string(), format!("an unnamed {tag}"));
            return;
        };
        if self.tables.structs.contains_key(&name) || self.tables.refused.contains_key(&name) {
            return;
        }
        // The struct of the results of a function its codespec: func line gives several.
        if self.tables.runtime
            && let Some(f) = name.strip_suffix("_result")
            && self.sig_lines.get(f).is_some_and(|l| l.results.len() > 1)
        {
            let fields = n
                .inner()
                .iter()
                .filter(|k| k.kind() == "FieldDecl")
                .map(|k| {
                    (
                        k.s("name").to_string(),
                        k.get("type").s("qualType").to_string(),
                    )
                })
                .collect();
            self.tables.result_types.insert(name.clone(), f.to_string());
            self.tables.result_fields.insert(name, fields);
            return;
        }
        let mut why = None;
        let mut fields = Vec::new();
        // Declared before its fields are read: a field may point to the struct itself.
        self.tables.structs.insert(name.clone(), Vec::new());
        if tag != "struct" {
            why = Some(format!(
                "a {tag}: cir/1 has no untagged union, and C does not say which member a value holds"
            ));
        }
        for f in n.inner().iter().filter(|f| f.kind() == "FieldDecl") {
            if why.is_some() {
                break;
            }
            if f.is_true("isBitfield") {
                why = Some(format!("field {} is a bit field", f.s("name")));
                break;
            }
            match self.tables.ty(unit, f.get("type")) {
                Ok(t) => fields.push(Field {
                    name: f.s("name").into(),
                    t,
                    embedded: false,
                }),
                Err(e) => why = Some(format!("field {} is {e}", f.s("name"))),
            }
        }
        if let Some(why) = why {
            self.tables.structs.remove(&name);
            self.tables.refused.insert(name.clone(), why.clone());
            self.refuse(n, format!("{tag} {name}"), why);
            return;
        }
        self.tables.structs.insert(
            name.clone(),
            fields.iter().map(|f| f.name.clone()).collect(),
        );
        self.layout.push(format!("type:{name}"));
        self.types.push(TypeDecl::Struct {
            name,
            exported: true,
            tparams: Vec::new(),
            fields,
            doc: String::new(),
            pos: pos_of(self.src, n),
        });
    }

    fn global(&mut self, unit: &mut Unit, n: &J) {
        let name = n.s("name").to_string();
        unit.globals.insert(n.s("id").into(), name.clone());
        let init = n.inner().iter().find(|k| !k.kind().ends_with("Attr"));
        if (n.s("storageClass") == "extern" && init.is_none())
            || self.tables.globals.contains_key(&name)
        {
            return;
        }
        if self.tables.runtime
            && let Some(r) = self.runtime_global(n, init)
        {
            if let Err(reason) = r {
                self.refuse(n, format!("var {name}"), reason);
            }
            return;
        }
        let t = match self.tables.ty(unit, n.get("type")) {
            Ok(t) => t,
            Err(e) => {
                self.refuse(n, format!("var {name}"), format!("its type is {e}"));
                return;
            }
        };
        let (mut value, mut foreign) = (None, None);
        if let Some(init) = init {
            let decls = BTreeMap::new();
            let mut b = Body::new(&self.tables, unit, self.src, &decls, Vec::new());
            match b.value(init) {
                Ok(e) if *e.t() == t => value = Some(e),
                r => {
                    let reason = r.err().unwrap_or_else(|| {
                        "an initializer whose type is not the declared one".into()
                    });
                    self.refuse(n, format!("var {name}"), reason.clone());
                    foreign = Some(ForeignInit {
                        lang: "c".into(),
                        src: src_of(self.src, init),
                        reason,
                    });
                }
            }
        }
        self.tables.globals.insert(name.clone(), t.clone());
        self.layout.push(format!("var:{name}"));
        self.vars.push(VarDecl {
            name,
            t,
            init: value,
            foreign,
            pos: pos_of(self.src, n),
        });
    }

    /// A file-scope variable of the runtime's own types: a sentinel error's `static struct
    /// cs_error_s X_s = {{"…", n}, NULL, NULL};` and `cs_error const X = &X_s;`, a `static const
    /// cs_type` describing a type an `any` holds. `None` for any other variable.
    fn runtime_global(&mut self, n: &J, init: Option<&J>) -> Option<Result<(), String>> {
        let ty = n.get("type").s("qualType").trim().to_string();
        let name = n.s("name").to_string();
        let bare = |j: &J| {
            let mut j = j;
            while matches!(j.kind(), "ImplicitCastExpr" | "ParenExpr" | "ConstantExpr") {
                j = &j.inner()[0];
            }
            j.clone()
        };
        match ty.as_str() {
            "struct cs_error_s" => Some((|| {
                let init = init.map(bare).ok_or("a cs_error_s without its message")?;
                let parts = init.inner();
                let msg = parts
                    .first()
                    .map(bare)
                    .ok_or("a cs_error_s without its message")?;
                let null = |j: Option<&J>| j.is_some_and(null_constant);
                if parts.len() != 3 || !null(parts.get(1)) || !null(parts.get(2)) {
                    return Err(
                        "a cs_error_s other than {{\"message\", length}, NULL, NULL}".into(),
                    );
                }
                let s = msg.inner();
                let lit = s
                    .first()
                    .map(bare)
                    .ok_or("a cs_error_s without its message")?;
                if lit.kind() != "StringLiteral" || s.len() != 2 {
                    return Err("a cs_error_s whose message is not a string literal".into());
                }
                let text = body::string_literal(lit.s("value"))?;
                let bytes = lit
                    .get("type")
                    .s("qualType")
                    .strip_prefix("char[")
                    .and_then(|r| r.strip_suffix(']'))
                    .and_then(|r| r.parse::<i128>().ok())
                    .map(|n| n - 1);
                if bytes.is_none() || body::constant(&s[1]) != bytes {
                    return Err("a cs_error_s whose length is not its message's".into());
                }
                let message = text[1..text.len() - 1].to_string();
                if message.contains('\\') {
                    return Err("a cs_error_s whose message holds an escape".into());
                }
                self.error_structs.insert(name.clone(), message);
                Ok(())
            })()),
            "const cs_error" | "cs_error const" | "cs_error" => Some((|| {
                if ty == "cs_error" {
                    return Err(
                        "a cs_error variable: a sentinel error is cs_error const, any other is not read".into(),
                    );
                }
                let init = init.map(bare).ok_or("a sentinel error without its value")?;
                let target = if init.kind() == "UnaryOperator" && init.s("opcode") == "&" {
                    bare(&init.inner()[0])
                        .get("referencedDecl")
                        .s("name")
                        .to_string()
                } else {
                    String::new()
                };
                let message = self
                    .error_structs
                    .get(&target)
                    .cloned()
                    .ok_or("a sentinel error other than the address of its cs_error_s")?;
                self.tables.errors.insert(name.clone(), message.clone());
                self.layout.push(format!("error:{name}"));
                self.errors.push(ErrorVar {
                    name: name.clone(),
                    message,
                    pos: pos_of(self.src, n),
                });
                Ok(())
            })()),
            "const cs_type" => Some((|| {
                let init = init.map(bare).ok_or("a cs_type without its value")?;
                let parts = init.inner();
                let kind = parts
                    .first()
                    .map(|k| bare(k).get("referencedDecl").s("name").to_string())
                    .unwrap_or_default();
                let elem_null = parts.get(2).is_some_and(null_constant);
                if !kind.starts_with("CS_K_") || parts.len() != 3 || !elem_null {
                    return Err("a cs_type other than {CS_K_…, \"name\", NULL}".into());
                }
                self.tables.descs.insert(name.clone(), kind);
                Ok(())
            })()),
            _ => None,
        }
    }

    fn function(&mut self, unit: &Unit, decls: &BTreeMap<String, &J>, n: &J) {
        let name = n.s("name").to_string();
        let body = n
            .inner()
            .iter()
            .find(|k| k.kind() == "CompoundStmt")
            .expect("a definition");
        let mut params = Vec::new();
        let mut why = None;
        if n.is_true("variadic") {
            why = Some("a variadic function definition".to_string());
        }
        let pdecls: Vec<&J> = n
            .inner()
            .iter()
            .filter(|p| p.kind() == "ParmVarDecl")
            .collect();
        let sig = Body::new(&self.tables, unit, self.src, decls, Vec::new())
            .rt_signature(n, &name, &pdecls);
        let mut sig_results = None;
        match sig {
            Some(Ok((ps, rs))) if why.is_none() => {
                for (p, t) in pdecls.iter().zip(ps) {
                    params.push((p.s("id").to_string(), param_name(p), t));
                }
                sig_results = Some(rs);
            }
            Some(Err(e)) if why.is_none() => why = Some(format!("its codespec: func line: {e}")),
            None if why.is_none() && self.tables.runtime => {
                why = Some(
                    "a function in runtime form without a codespec: func line, which alone types it"
                        .into(),
                );
            }
            _ => {}
        }
        for p in &pdecls {
            if why.is_some() || sig_results.is_some() {
                break;
            }
            match self.tables.ty(unit, p.get("type")) {
                Ok(t) => params.push((p.s("id").to_string(), param_name(p), t)),
                Err(e) => why = Some(format!("parameter {} is {e}", p.s("name"))),
            }
        }
        let result = match (&why, &sig_results) {
            (None, Some(_)) => None,
            (None, None) => match self.tables.result(unit, n.get("type").s("qualType")) {
                Ok(t) => Some(t),
                Err(e) => {
                    why = Some(format!("its result is {e}"));
                    None
                }
            },
            (Some(_), _) => None,
        };
        let mut b = Body::new(&self.tables, unit, self.src, decls, params.clone());
        b.results = match (&sig_results, &result) {
            (Some(rs), _) => rs.clone(),
            (None, Some(Type::Unit) | None) => Vec::new(),
            (None, Some(t)) => vec![t.clone()],
        };
        if self.tables.runtime && why.is_none() {
            b.pin_locals(body);
        }
        if let Some(why) = why {
            let calls = b.calls(body);
            self.refuse(n, format!("function {name}"), why.clone());
            self.opaque.push(Opaque {
                name,
                recv: None,
                pos: pos_of(self.src, n),
                reason: why,
                no_body: false,
                calls,
            });
            return;
        }
        let stmts = b.block(body);
        self.refusals.append(&mut b.refusals);
        self.externs.append(&mut b.externs);
        let results = std::mem::take(&mut b.results);
        self.funcs.push(Func {
            exported: n.s("storageClass") != "static",
            name,
            tparams: Vec::new(),
            recv: None,
            params: params
                .into_iter()
                .map(|(_, name, t)| Param {
                    name,
                    t,
                    variadic: false,
                })
                .collect(),
            results,
            body: stmts,
            doc: String::new(),
            pos: pos_of(self.src, n),
        });
    }
}
