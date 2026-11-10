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
//! | `cs_box(&desc, &v, sizeof(T))`, `desc` a `static const cs_type desc = {CS_K_…, "name", NULL};` of `T`'s kind and Go's name of `T` (`int`, `int64`), or for a newtype `{CS_K_NAMED, "Name", &<desc of the type it is over>}` | the value as `any` |
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
//! **Order.** C leaves the order of a call's arguments and an operator's operands unspecified (gcc
//! goes right to left), Go goes left to right. A call, an operator, an assignment, a `return`, a
//! `send` or a lookup with more than one operand that makes a call, an `append`, or an operation
//! that can panic (an integer `/` or `%`, an index of anything but a map, a shift by a signed
//! count: `cs_div_*`, `cs_rem_*`, `cs_slice_at`, `cs_shift_count` in runtime form) is refused
//! (`&&` and `||` aside, which C orders); the C writer refuses the same for calls. So is an
//! assignment to a place whose operands read a global, memory or a local whose address the
//! function takes, from a value that makes a call (the call may write it), and a multiple
//! assignment whose later target reads what an earlier one assigns (C reads it after, Go before;
//! a target written or read through a pointer counts as touching every variable), and an
//! assignment or `++` through `*cs_map_get` or `*cs_chan_recv`, whose value is no place of the
//! map or channel. A call of a module function whose `codespec: func` line types its parameters
//! and results checks each argument and the returned value against it, a call of any other
//! function each argument against its C parameter type; a receive from a
//! send-only channel and a send on a receive-only one are refused.
//!
//! **Maps.** The runtime walks a map in insertion order, `cir/1` in none. A loop over a map is read
//! as `range` only when no order of its entries changes its result: each statement accumulates
//! into its own integer variable, neither key nor value, with `+ * & | ^` (or `++`, `--`), from
//! a value that makes no call and reads no accumulator and nothing through a pointer (which may be an
//! accumulator under another name). Any other loop over a map is refused,
//! and so is a `cs_map_put` in it, through the map or another variable holding it.
//!
//! **Widths.** `int64_t` holds `int` and `int64`, `uint64_t` `uint` and `uint64`: the `codespec:
//! func` line says which for a parameter or result, a local takes its initializer's, and a
//! counter of a `range` is `int64_t`.
//!
//! **The C writer's layout.** The writer is the definition of runtime form; what it writes for
//! the kit reads back (`task c-round-trip`). A directory with one header beside `codespec_rt.h`
//! holding `codespec:` lines is the writer's module `<m>`: its functions are `<m>_<Name>`, read
//! as `<Name>`, the module is named `<m>`, and a function without a line is unexported and typed
//! by its C types (an exported one, with an upper-case first letter, needs its line). A name
//! whose form without the prefix already names another declaration, a parameter or a local keeps
//! its prefix, with a refusal. Besides the
//! forms above it writes, and the reader reads:
//!
//! | the C writer | cir/1 |
//! |---|---|
//! | `static` helpers `<m>_cs_append_<T>`, `_send_`, `_put_`, `_get_`, `_find_`, `_box_`, each the writer's exact text around one runtime call taking `&v` | that runtime call, with `v` |
//! | `{ cs_slice cs_rN = xs; for (int64_t cs_iN = 0, cs_nN = cs_rN.len; cs_iN < cs_nN; cs_iN++) { T v = ((T *)cs_rN.p)[cs_iN]; … } }`, likewise over `cs_map_len` with `cs_map_key`/`cs_map_val` and over an `int64_t` | `range`; a body using `cs_rN`, `cs_iN` or `cs_nN` is refused |
//! | `{ T i = e; for (; c; post) { … } }` | `for` with `init` |
//! | `(void)f(x);` | `expr` |
//! | `cs_shl_T(x, cs_shift_count(n))`, `cs_shl_T(x, (uint64_t)u)`, `cs_shr_T` | `<<`, `>>`; a signed count not through `cs_shift_count` is refused |
//! | `((cs_string){0})`, `((cs_slice){0})` where a type is wanted | `zero` of it |
//! | `typedef int64_t <m>_ID;` under `/* codespec: type ID int */` (`int64`: over `int64`), and its wrapping arithmetic `(<m>_ID)((uint64_t)a + (uint64_t)b)` | `newtype` `ID` over `int`; `typedef uint64_t` under a line saying `uint` is refused |
//!
//! The C runner compiles each `.c` file as its own translation unit, as the reader reads it, so a
//! macro of one file is never seen in another by either.
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
//!
//! # Limits
//!
//! A source is untrusted: what it can cost the reader is bounded, and a source past a bound is
//! refused, exit 1 and the reason on stderr, not a crash.
//!
//! | bound | limit |
//! |---|---|
//! | sources | the `.c` and `.h` files of the directory up to [`MAX_SOURCE_BYTES`], 16 MiB together |
//! | clang's JSON | up to [`MAX_JSON_BYTES`], 64 MiB per `.c` file, and [`MAX_JSON_TOTAL_BYTES`], 256 MiB, for all `.c` files of the directory together; read from clang's stdout as a stream and refused, clang stopped, one byte past either (`clang's JSON AST of f.c is over 67108864 bytes, this reader's limit`, `clang's JSON ASTs of the .c files up to f.c are over 268435456 bytes together, this reader's limit`). clang indents each level two more spaces, so the JSON of a deep expression grows with the square of its source: `a + a + … + a` of 1000 operands, 2 kB, prints about 150 MB. A header counts once per `.c` file that includes it, as clang prints it for each |
//! | depth | that JSON up to 4096 levels of arrays and objects, checked by a scan before it is read; it is read, and the reader's passes recurse, on a thread with a 256 MiB stack |
//!
//! Memory: every file's tree is held until the module is assembled, about 6.4 bytes per byte of
//! JSON (measured: a file whose JSON is 42 MB peaks at 313 MiB resident, each further one adds
//! 259 MiB), so the trees stay under about 1.7 GiB and the reader under about 2 GiB. Not bounded
//! by the reader: clang's own memory and time while it parses and before it prints, and the
//! module the reader builds beyond what the trees bound.
//!
//! The tree read from the JSON is linear in its size: each value and each string of it is one of
//! the JSON's, but for the `file` and `line` completed into each location, and those share the
//! file name clang printed once rather than copy it. `json::tests` holds the budget, no more
//! values and bytes of text than the JSON has bytes, on a header included through a 2 kB path,
//! which before cost every location in it that path again. Some of the reader's passes walk a
//! subtree once per level around it: the check that no two operands of a statement have an effect
//! whose order C leaves unspecified
//! (`body::ordered`, `body::has_effect`) runs for each statement and each node over what lies
//! below it. Their work is at most the tree's size times its depth, which the depth limit bounds.

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

/// `m` with every name of its own declarations (types, constants, errors, variables, functions)
/// that starts with the C writer's prefix `px` named without it, wherever it is used.
fn unprefix(m: Module, px: &str) -> Module {
    let mut names = BTreeSet::new();
    names.extend(m.types.iter().map(|d| d.name().to_string()));
    names.extend(m.consts.iter().map(|d| d.name.clone()));
    names.extend(m.errors.iter().map(|d| d.name.clone()));
    names.extend(m.vars.iter().map(|d| d.name.clone()));
    names.extend(m.funcs.iter().map(|d| d.name.clone()));
    names.extend(m.opaque.iter().map(|d| d.name.clone()));
    names.retain(|n| n.len() > px.len() && n.starts_with(px));
    let Ok(mut v) = serde_json::to_value(&m) else {
        return m;
    };
    // A name without the prefix that is already the name of a declaration, a parameter or a
    // local would name two things: such a name keeps its prefix, with a refusal.
    fn idents(v: &serde_json::Value, out: &mut BTreeSet<String>) {
        match v {
            serde_json::Value::Object(o) => {
                if o.get("k").and_then(|k| k.as_str()) == Some("field") {
                    return o
                        .iter()
                        .filter(|(k, _)| k.as_str() != "name")
                        .for_each(|(_, x)| idents(x, out));
                }
                for (k, x) in o {
                    match (k.as_str(), x) {
                        ("fields", _) => {}
                        ("name" | "key" | "val", serde_json::Value::String(s)) => {
                            out.insert(s.clone());
                        }
                        ("names", serde_json::Value::Array(a)) => {
                            out.extend(a.iter().filter_map(|s| s.as_str().map(String::from)));
                        }
                        _ => idents(x, out),
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| idents(x, out)),
            _ => {}
        }
    }
    let mut taken = BTreeSet::new();
    idents(&v, &mut taken);
    let mut refusals = Vec::new();
    names.retain(|n| {
        let bare = &n[px.len()..];
        let free = !taken.contains(bare);
        if !free {
            refusals.push(Refusal {
                pos: decl_pos(&v, n).unwrap_or_default(),
                what: n.clone(),
                reason: format!(
                    "{n} without the C writer's prefix {px} is {bare}, which names another declaration, a parameter or a local: it keeps its prefix"
                ),
            });
        }
        free
    });
    fn walk(v: &mut serde_json::Value, px: &str, names: &BTreeSet<String>) {
        match v {
            serde_json::Value::String(s) => {
                let (kind, n) = match s.split_once(':') {
                    Some((k, n)) if names.contains(n) => (format!("{k}:"), n.to_string()),
                    _ => (String::new(), s.clone()),
                };
                if names.contains(&n) {
                    *s = format!("{kind}{}", &n[px.len()..]);
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(|x| walk(x, px, names)),
            serde_json::Value::Object(o) => o.values_mut().for_each(|x| walk(x, px, names)),
            _ => {}
        }
    }
    walk(&mut v, px, &names);
    let mut out: Module = serde_json::from_value(v).unwrap_or(m);
    out.refusals.extend(refusals);
    out
}

/// The position of the declaration of the module named `n`.
fn decl_pos(v: &serde_json::Value, n: &str) -> Option<String> {
    ["types", "consts", "errors", "vars", "funcs", "opaque"]
        .iter()
        .flat_map(|k| v[*k].as_array().into_iter().flatten())
        .find(|d| d["name"] == n)
        .and_then(|d| d["pos"].as_str().map(String::from))
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

/// Read every `.c` file of `dir` into one module, within the limits the crate doc states.
pub fn read_dir(dir: &Path) -> Result<Module, String> {
    // The AST nests a level per operand; read it on a stack that holds one `json::MAX_DEPTH` deep.
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

/// The most bytes of `.c` and `.h` files the reader reads from one directory, together.
pub const MAX_SOURCE_BYTES: u64 = 16 << 20;

/// The most bytes of JSON AST the reader takes from clang for one `.c` file. clang indents each
/// level two more spaces, so the JSON of a deep expression grows with the square of its source
/// (`a + a + … + a` of 1000 operands, 2 kB, prints about 150 MB): the reader reads it as a stream
/// and stops, refusing the file, one byte past this.
pub const MAX_JSON_BYTES: u64 = 64 << 20;

/// The most bytes of JSON AST the reader takes from clang for all `.c` files of the directory
/// together. Every file's tree is held until the module is assembled, at about 6.4 bytes of
/// memory per byte of JSON (measured: a file whose JSON is 42 MB peaks at 313 MiB resident, and
/// each further such file adds 259 MiB), so this keeps the trees under about 1.7 GiB and the
/// reader, with one file's JSON being read, under about 2 GiB.
pub const MAX_JSON_TOTAL_BYTES: u64 = 256 << 20;

/// What clang prints on stderr that the reader keeps for its message.
const STDERR_KEPT: u64 = 64 << 10;

/// `clang`'s JSON AST of `file`, run in `dir`, read up to `limit` bytes; `None` past it, clang
/// stopped.
fn clang(dir: &Path, file: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = Command::new("clang")
        .args(["-Xclang", "-ast-dump=json", "-fsyntax-only", "-std=c11"])
        .arg(file)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => "clang not found".to_string(),
            _ => format!("cannot run clang: {e}"),
        })?;
    let (Some(mut out), Some(mut err)) = (child.stdout.take(), child.stderr.take()) else {
        return Err("cannot run clang: no pipes".into());
    };
    let (json, read, stderr) = std::thread::scope(|s| {
        let stderr = s.spawn(move || {
            let mut kept = Vec::new();
            let _ = (&mut err).take(STDERR_KEPT).read_to_end(&mut kept);
            let _ = std::io::copy(&mut err, &mut std::io::sink());
            kept
        });
        let mut json = Vec::new();
        let read = (&mut out).take(limit + 1).read_to_end(&mut json);
        if read.is_err() || json.len() as u64 > limit {
            let _ = child.kill();
        }
        drop(out);
        (json, read, stderr.join().unwrap_or_default())
    });
    let status = child.wait().map_err(|e| format!("cannot run clang: {e}"))?;
    if json.len() as u64 > limit {
        return Ok(None);
    }
    read.map_err(|e| format!("clang's JSON AST of {file}: {e}"))?;
    if !status.success() {
        return Err(format!(
            "clang rejects {file}:\n{}",
            String::from_utf8_lossy(&stderr)
        ));
    }
    Ok(Some(json))
}

fn read(dir: &Path) -> Result<Module, String> {
    use std::io::Read;
    let mut names = Vec::new();
    let mut files = BTreeMap::new();
    let mut left = MAX_SOURCE_BYTES;
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !e.path().is_file() || !(name.ends_with(".c") || name.ends_with(".h")) {
            continue;
        }
        let mut text = Vec::new();
        std::fs::File::open(e.path())
            .and_then(|f| f.take(left + 1).read_to_end(&mut text))
            .map_err(|e| format!("{name}: {e}"))?;
        left = left.checked_sub(text.len() as u64).ok_or_else(|| {
            format!(
                "{}: the .c and .h files are over {MAX_SOURCE_BYTES} bytes, this reader's limit",
                dir.display()
            )
        })?;
        files.insert(name.clone(), text);
        if name.ends_with(".c") {
            names.push(name);
        }
    }
    names.sort();
    if names.is_empty() {
        return Err(format!("{}: no .c file", dir.display()));
    }
    // Every file's tree is held until the module is assembled: the JSON of all of them shares one
    // budget, as well as each file having its own.
    let mut asts = Vec::new();
    let mut json_left = MAX_JSON_TOTAL_BYTES;
    for f in &names {
        let limit = MAX_JSON_BYTES.min(json_left);
        let Some(json) = clang(dir, f, limit)? else {
            return Err(if limit == MAX_JSON_BYTES {
                format!(
                    "clang's JSON AST of {f} is over {MAX_JSON_BYTES} bytes, this reader's limit"
                )
            } else {
                format!(
                    "clang's JSON ASTs of the .c files up to {f} are over {MAX_JSON_TOTAL_BYTES} bytes together, this reader's limit"
                )
            });
        };
        json_left -= json.len() as u64;
        asts.push(json::parse(&json)?);
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
    /// `/* codespec: type <Name> <type> */` of the C writer's header, by the C name.
    type_lines: BTreeMap<String, String>,
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
            type_lines: BTreeMap::new(),
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
        // The C writer's layout: one header `<m>.h` beside the runtime holds the `codespec:`
        // lines, and every name of the module is `<m>_<name>`.
        let headers: Vec<&String> = self
            .src
            .files
            .keys()
            .filter(|f| f.ends_with(".h") && f.as_str() != runtime::RT_FILE)
            .collect();
        if let [h] = headers.as_slice()
            && String::from_utf8_lossy(&self.src.files[*h]).contains("/* codespec: ")
        {
            self.tables.prefix = format!("{}_", h.trim_end_matches(".h"));
        }
        for (name, text) in &self.src.files {
            if name.as_str() == runtime::RT_FILE {
                continue;
            }
            let mut lines = BTreeMap::new();
            if let Err(e) = runtime::sig_lines(&String::from_utf8_lossy(text), &mut lines) {
                self.refusals.push(Refusal {
                    pos: format!("{name}:1"),
                    what: "codespec: func".into(),
                    reason: e,
                });
            }
            let px = if name.ends_with(".h") {
                self.tables.prefix.clone()
            } else {
                String::new()
            };
            for l in String::from_utf8_lossy(text).lines() {
                if let Some((t, go)) = l
                    .trim()
                    .strip_prefix("/* codespec: type ")
                    .and_then(|r| r.strip_suffix("*/"))
                    .and_then(|r| r.trim().split_once(' '))
                {
                    self.type_lines.insert(format!("{px}{t}"), go.trim().into());
                }
            }
            for (f, l) in lines {
                let key = format!("{px}{f}");
                if self.sig_lines.insert(key.clone(), l).is_some() {
                    self.refusals.push(Refusal {
                        pos: format!("{name}:1"),
                        what: "codespec: func".into(),
                        reason: format!("a second codespec: func line of {key}"),
                    });
                }
            }
        }
    }

    /// A helper the C writer writes around one runtime call, `static … <m>_cs_<op>_<type>(…)`,
    /// its text exactly the writer's: the runtime function it calls, and the C type of the
    /// value it takes. `None` for any other function.
    fn helper(&self, n: &J) -> Option<(String, String)> {
        let px = &self.tables.prefix;
        let name = n.s("name");
        let rest = name.strip_prefix(px.as_str())?.strip_prefix("cs_")?;
        if px.is_empty() || n.s("storageClass") != "static" {
            return None;
        }
        let (op, suffix) = rest.split_once('_')?;
        let params: Vec<&J> = n
            .inner()
            .iter()
            .filter(|p| p.kind() == "ParmVarDecl")
            .collect();
        let ty = |i: usize| {
            params
                .get(i)
                .map(|p| p.get("type").s("qualType").to_string())
        };
        let (runtime, t, text) = match op {
            "append" => {
                let t = ty(1)?;
                let text = format!(
                    "static cs_slice {name}(cs_slice s, {t} v) {{ return cs_append(s, &v, sizeof v); }}"
                );
                ("cs_append", t, text)
            }
            "send" => {
                let t = ty(1)?;
                let text =
                    format!("static void {name}(cs_chan *c, {t} v) {{ cs_chan_send(c, &v); }}");
                ("cs_chan_send", t, text)
            }
            "put" => {
                let t = ty(1)?;
                let text = format!(
                    "static void *{name}(cs_map *m, {t} k) {{ return cs_map_put(m, &k); }}"
                );
                ("cs_map_put", t, text)
            }
            "get" => {
                let t = ty(1)?;
                let text = format!(
                    "static void *{name}(const cs_map *m, {t} k, size_t vsz) {{ return cs_map_get(m, &k, vsz); }}"
                );
                ("cs_map_get", t, text)
            }
            "find" => {
                let t = ty(1)?;
                let text = format!(
                    "static void *{name}(const cs_map *m, {t} k) {{ return cs_map_find(m, &k); }}"
                );
                ("cs_map_find", t, text)
            }
            "box" => {
                let t = ty(0)?;
                let text = format!(
                    "static cs_any {name}({t} v) {{ return cs_box(&{px}cs_type_{suffix}, &v, sizeof v); }}"
                );
                ("cs_box", t, text)
            }
            _ => return None,
        };
        let have = src_of(self.src, n);
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        (squash(&have) == squash(&text)).then(|| (runtime.to_string(), t))
    }

    /// A name of the module as `cir/1` names it: without the C writer's prefix.
    fn bare_name<'n>(&self, c: &'n str) -> &'n str {
        c.strip_prefix(self.tables.prefix.as_str())
            .filter(|_| !self.tables.prefix.is_empty())
            .unwrap_or(c)
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
        let prefix = self.tables.prefix.clone();
        let name = match prefix.strip_suffix('_') {
            Some(m) => m.to_string(),
            None => name,
        };
        let module = Module {
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
            persistence: Vec::new(),
        };
        if prefix.is_empty() {
            module
        } else {
            unprefix(module, &prefix)
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
                    // In runtime form `int64_t` is `int` unless the type's `codespec: type` line
                    // says `int64`; `uint64_t` is `uint64`, and a line saying `uint` is refused:
                    // C has no spelling of `uint` apart from `uint64`.
                    let line = self.type_lines.get(n.s("name")).cloned();
                    let line = line.as_deref();
                    let q = t.s("qualType").trim();
                    if self.tables.runtime && q == "uint64_t" && line == Some("uint") {
                        let name = n.s("name").to_string();
                        let why = "a newtype over uint, which runtime form spells uint64_t as it does uint64".to_string();
                        self.tables.bad_typedefs.insert(name.clone(), why.clone());
                        self.refuse(n, format!("typedef {name}"), why);
                        continue;
                    }
                    let target = match t.s("desugaredQualType") {
                        _ if self.tables.runtime && q == "int64_t" && line != Some("int64") => {
                            "int64_t"
                        }
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
                    if let Some(h) = self.helper(n) {
                        self.tables.helpers.insert(name.into(), h);
                        continue;
                    }
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
                let bad = || {
                    "a cs_type other than {CS_K_…, \"name\", NULL} or {CS_K_NAMED, \"Name\", &<cs_type of its type>}".to_string()
                };
                if !kind.starts_with("CS_K_") || parts.len() != 3 {
                    return Err(bad());
                }
                let tname = bare(&parts[1]);
                let tname = (tname.kind() == "StringLiteral")
                    .then(|| tname.s("value"))
                    .and_then(|v| v.strip_prefix('"')?.strip_suffix('"'))
                    .filter(|v| v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
                    .ok_or_else(bad)?
                    .to_string();
                // `CS_K_NAMED` carries the `cs_type` of the type it is defined over, every
                // other kind none.
                let elem = bare(&parts[2]);
                let elem = if kind == "CS_K_NAMED" {
                    let target = (elem.kind() == "UnaryOperator" && elem.s("opcode") == "&")
                        .then(|| {
                            bare(&elem.inner()[0])
                                .get("referencedDecl")
                                .s("name")
                                .to_string()
                        })
                        .filter(|d| self.tables.descs.contains_key(d))
                        .ok_or_else(bad)?;
                    Some(target)
                } else if null_constant(&parts[2]) {
                    None
                } else {
                    return Err(bad());
                };
                self.tables.descs.insert(name.clone(), (kind, tname, elem));
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
            None if why.is_none()
                && self.tables.runtime
                && self
                    .bare_name(&name)
                    .starts_with(|c: char| c.is_ascii_uppercase()) =>
            {
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
        b.addressed = body::addressed(n);
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
            raises: Vec::new(),
            body: stmts,
            doc: String::new(),
            pos: pos_of(self.src, n),
        });
    }
}
