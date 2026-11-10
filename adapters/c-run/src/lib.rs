//! `codespec-run-c`: the C runner of the conformance kit (`docs/conformance.md` § 4). It compiles
//! the C `codespec-write-c` wrote, or C written by hand as a C reader reads it, with a generated
//! driver that makes the calls of a `codespec-calls/1` document and prints one line per call in
//! the harness encoding the crate doc of `codespec_conformance` defines (`ok <v>…`, `err <msg>`,
//! `sent <v>`).
//!
//! ```console
//! codespec-run-c [--build <dir>] <dir> <calls.json>
//! ```
//!
//! # How a call is made
//!
//! The written directory holds one module header `<m>.h` besides `codespec_rt.h`, its source
//! `<m>.c`, and in the header a `/* codespec: type … */` line before every type and a
//! `/* codespec: func <Name>(<types>) <results> */` line before every exported top-level
//! function, in Go's spelling of the `cir/1` types (the crate doc of `codespec-write-c`).
//! [`driver_c`] reads those lines and generates `driver.c`: each call in its own block, calling
//! `<m>_<Name>` and printing its results by their types. Arguments are checked with
//! `codespec_conformance::check_argument`, the same encoding every runner reads. A call whose
//! `func` has no `codespec: func` line is refused naming the `func`.
//!
//! | parameter | argument |
//! |---|---|
//! | an integer type | `(T)INT64_C(v)`, `INT64_MIN` for the least |
//! | `bool` | `true`, `false` |
//! | `string` | a `cs_string` of the literal |
//! | `chan T`, `chan<- T`, `T` printable | `chan`: a fresh `cs_chan` with room for 64 values, as the Go driver gives, drained after the call into one `sent <v>` line per value |
//!
//! | results | line |
//! |---|---|
//! | none | `ok` |
//! | values, the last not an `error` | `ok <v> …` |
//! | values ending in an `error` | nil: `ok` and the values before it; else `err <message>` |
//!
//! A printable value is a signed integer in decimal, `true` or `false`, a string as Go's
//! `strconv.Quote` writes it, a slice as `[` its elements separated by one space `]`, a named type
//! that is not a struct or union as the value it is defined over, and an `any` as the value it
//! holds (`<nil>` for nil), when that is printable. A string holding a byte outside ASCII is
//! refused when printed (exit 2), as `codespec-run-rust` refuses one. Any other parameter or
//! result type, and an unsigned integer result, which the kit's Go driver cannot print, is refused
//! before anything is built.
//!
//! # C written by hand
//!
//! A directory without a module header holds C written by hand, as a C reader reads it (a kit
//! case's `c/case.c`). The runner reads its `.c` files for function definitions opening at the
//! start of a line, `<type> <name>(<params>) {`, that are not `static`, and calls the one whose
//! name with its first letter upper case is the `func` (`Less` calls `less`), as the Java runner
//! does. The driver declares each called function itself and includes only `<stdbool.h>`,
//! `<stdint.h>` and `<stdio.h>`, so a function named as one of `<stdlib.h>` (`div`) is the
//! source's; every one-line or multi-line `typedef enum … Name;` of the sources, and every
//! one-line `typedef T Name;` of a `T` the table below reads as an integer or a `bool`, is copied
//! into it.
//!
//! | C type | as |
//! |---|---|
//! | a signed integer type (`long`, `int`, `int64_t`, …), a `typedef enum` name or a `typedef` of an integer | an integer |
//! | `bool`, `_Bool`, a `typedef` of one | `bool` |
//! | `const char *`, `char *` | `string`: passed as a C string literal, printed as above; a null pointer is refused (exit 2) |
//! | `void` result | no result: `ok` |
//!
//! A function's parameters are split at the commas outside parentheses, so a function pointer
//! parameter `R (*f)(P, Q)` is one parameter, spelled `R (*)(P, Q)`, and an array parameter
//! `T xs[N]` is spelled `T [N]`. Floats, pointers, arrays, function pointers and every other type
//! are what the functions `calls.json` calls use among themselves: a call of a function that takes
//! or returns one is refused, naming the type, before anything is built.
//!
//! # The build
//!
//! The build directory (`--build`, emptied first; else a directory under the system's temporary
//! directory, removed afterwards) holds `driver.c` and the program `codespec-run`, compiled with
//! `$CC` (default `cc`) as `cc -std=c11 -Wall -Werror $CFLAGS -I <written dir> -o … driver.c
//! <m>.c`. `CFLAGS` is split on whitespace; a sanitizer named there (`-fsanitize=undefined
//! -fno-sanitize-recover`) makes undefined behaviour a failed program.
//!
//! # Exit status
//!
//! | status | when |
//! |---|---|
//! | 0 | every call made and printed |
//! | 2 | refused: `calls.json` not as `read_calls` reads it, a call that does not fit the module (no such function, a wrong argument, a type outside the tables above), a value the program cannot print exactly; a usage error too |
//! | 1 | failed: neither a module header nor a C source, the build fails, or the program fails (a panic of the runtime exits 3, a sanitizer 1) |

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_conformance::{ArgKind, Calls, check_argument};

mod plain;

/// Why the calls cannot be made; the text names the file or the call.
pub type Error = String;

/// How a run ends without printing every call: refused (exit 2) or failed (exit 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The calls do not fit the module, or a value cannot be printed exactly.
    Refused(Error),
    /// Reading the module, the build or the program failed.
    Failed(Error),
}

impl Failure {
    /// The runner's exit status for it: 2 refused, 1 failed.
    pub fn exit_code(&self) -> u8 {
        match self {
            Failure::Refused(_) => 2,
            Failure::Failed(_) => 1,
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Refused(e) => write!(f, "refused: {e}"),
            Failure::Failed(e) => f.write_str(e),
        }
    }
}

/// A `cir/1` type as the header's `codespec:` lines spell it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Int { bits: u32, signed: bool },
    Bool,
    String,
    Error,
    Any,
    Slice(Box<Ty>),
    Chan(Box<Ty>),
    Named(String),
    Other(String),
}

/// What a declared type is, for printing.
enum Decl {
    /// A newtype or enum: printed as the type it is defined over.
    Over(Ty),
    /// A struct or union.
    Composite,
}

fn parse_ty(s: &str) -> Ty {
    let s = s.trim();
    let int = |name: &str| -> Option<Ty> {
        let (signed, rest) = match name.strip_prefix("uint") {
            Some(r) => (false, r),
            None => (true, name.strip_prefix("int")?),
        };
        let bits = match rest {
            "" | "64" => 64,
            "8" => 8,
            "16" => 16,
            "32" => 32,
            _ => return None,
        };
        Some(Ty::Int { bits, signed })
    };
    if let Some(t) = int(s) {
        return t;
    }
    match s {
        "bool" => return Ty::Bool,
        "string" => return Ty::String,
        "error" => return Ty::Error,
        "any" => return Ty::Any,
        _ => {}
    }
    if let Some(e) = s.strip_prefix("[]") {
        return Ty::Slice(Box::new(parse_ty(e)));
    }
    if let Some(e) = s
        .strip_prefix("chan<- ")
        .or_else(|| s.strip_prefix("chan "))
    {
        return Ty::Chan(Box::new(parse_ty(e)));
    }
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Ty::Named(s.to_string());
    }
    Ty::Other(s.to_string())
}

/// `s` split at the commas outside brackets and parentheses.
fn split_top(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// One exported function as its `codespec: func` line gives it.
struct Sig {
    params: Vec<Ty>,
    results: Vec<Ty>,
}

/// The module header as the runner reads it.
struct Header {
    /// `<m>_`.
    px: String,
    /// `<m>.h`.
    file: String,
    types: BTreeMap<String, Decl>,
    funcs: BTreeMap<String, Sig>,
}

impl Header {
    fn read(base: &str, text: &str) -> Result<Header, Error> {
        let mut h = Header {
            px: format!("{base}_"),
            file: format!("{base}.h"),
            types: BTreeMap::new(),
            funcs: BTreeMap::new(),
        };
        for line in text.lines() {
            let Some(rest) = line
                .strip_prefix("/* codespec: ")
                .and_then(|r| r.strip_suffix(" */"))
            else {
                continue;
            };
            if let Some(t) = rest.strip_prefix("type ") {
                let (name, under) = t.split_once(' ').unwrap_or((t, ""));
                let d = match under {
                    "struct" | "union" => Decl::Composite,
                    u => Decl::Over(parse_ty(u)),
                };
                h.types.insert(name.to_string(), d);
            } else if let Some(f) = rest.strip_prefix("func ") {
                let bad = || format!("{}: cannot read the line {line:?}", h.file);
                let (name, rest) = f.split_once('(').ok_or_else(bad)?;
                let mut depth = 1;
                let mut end = None;
                for (i, c) in rest.char_indices() {
                    match c {
                        '(' | '[' => depth += 1,
                        ')' | ']' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let end = end.ok_or_else(bad)?;
                let params = split_top(&rest[..end])
                    .iter()
                    .map(|p| parse_ty(p))
                    .collect();
                let res = rest[end + 1..].trim();
                let results = if res.is_empty() {
                    Vec::new()
                } else if let Some(inner) = res.strip_prefix('(').and_then(|r| r.strip_suffix(')'))
                {
                    split_top(inner).iter().map(|p| parse_ty(p)).collect()
                } else {
                    vec![parse_ty(res)]
                };
                h.funcs.insert(name.to_string(), Sig { params, results });
            }
        }
        Ok(h)
    }

    fn cty(&self, t: &Ty) -> Result<String, Error> {
        Ok(match t {
            Ty::Int { bits, signed } => format!("{}int{bits}_t", if *signed { "" } else { "u" }),
            Ty::Bool => "bool".into(),
            Ty::String => "cs_string".into(),
            Ty::Error => "cs_error".into(),
            Ty::Any => "cs_any".into(),
            Ty::Slice(_) => "cs_slice".into(),
            Ty::Chan(_) => "cs_chan *".into(),
            Ty::Named(n) => format!("{}{n}", self.px),
            Ty::Other(s) => return Err(format!("type `{s}` has no C form the runner knows")),
        })
    }

    /// Whether a value of `t` can be printed; the reason when not.
    fn printable(&self, t: &Ty) -> Result<(), Error> {
        match t {
            Ty::Int {
                signed: false,
                bits,
            } => Err(format!(
                "unsigned `uint{bits}` cannot be printed: the kit's Go driver prints signed integers only"
            )),
            Ty::Int { .. } | Ty::Bool | Ty::String | Ty::Any => Ok(()),
            Ty::Slice(e) => self.printable(e),
            Ty::Named(n) => match self.types.get(n) {
                Some(Decl::Over(u)) => self.printable(u),
                Some(Decl::Composite) => Err(format!("the struct `{n}` cannot be printed")),
                None => Err(format!("the type `{n}` is not declared in the header")),
            },
            Ty::Error | Ty::Chan(_) | Ty::Other(_) => Err(format!("{t:?} cannot be printed")),
        }
    }

    /// Statements appending the value `v` of type `t` to the builder `b`.
    fn show(&self, t: &Ty, v: &str, out: &mut String, depth: usize) -> Result<(), Error> {
        let ind = "    ".repeat(depth + 2);
        match t {
            Ty::Int { signed: true, .. } => {
                writeln!(out, "{ind}cs_fmt_int(b, (int64_t)({v}));").unwrap()
            }
            Ty::Bool => writeln!(out, "{ind}cs_fmt_bool(b, {v});").unwrap(),
            Ty::String => writeln!(out, "{ind}show_string(b, {v});").unwrap(),
            Ty::Any => writeln!(out, "{ind}show_any(b, {v});").unwrap(),
            Ty::Named(n) => match self.types.get(n) {
                Some(Decl::Over(u)) => self.show(u, v, out, depth)?,
                _ => return Err(format!("the struct `{n}` cannot be printed")),
            },
            Ty::Slice(e) => {
                self.printable(e)?;
                let ct = self.cty(e)?;
                let (s, i) = (format!("s{depth}"), format!("i{depth}"));
                writeln!(out, "{ind}{{\n{ind}    cs_slice {s} = {v};\n{ind}    cs_sb_bytes(b, \"[\", 1);\n{ind}    for (int64_t {i} = 0; {i} < {s}.len; {i}++) {{\n{ind}        if ({i} > 0) {{\n{ind}            cs_sb_bytes(b, \" \", 1);\n{ind}        }}").unwrap();
                self.show(e, &format!("(({ct} *){s}.p)[{i}]"), out, depth + 2)?;
                writeln!(
                    out,
                    "{ind}    }}\n{ind}    cs_sb_bytes(b, \"]\", 1);\n{ind}}}"
                )
                .unwrap();
            }
            other => {
                self.printable(other)?;
            }
        }
        Ok(())
    }
}

/// `codespec_driver.h`, written beside `driver.c`: the functions that print a value. In a
/// header, so a function a driver does not call is no warning (clang warns about an unused
/// `static inline` function of the main file).
pub const DRIVER_H: &str = r#"/* codespec_driver.h: written by codespec-run-c. DO NOT EDIT. */
#ifndef CODESPEC_DRIVER_H
#define CODESPEC_DRIVER_H

#include "codespec_rt.h"

/* Go's strconv.Quote for ASCII; a string with a byte outside ASCII is refused, exit 2. */
static inline void show_string(cs_sb *b, cs_string s) {
    cs_sb_bytes(b, "\"", 1);
    for (int64_t i = 0; i < s.n; i++) {
        unsigned char c = (unsigned char)s.p[i];
        switch (c) {
        case '\a': cs_sb_bytes(b, "\\a", 2); break;
        case '\b': cs_sb_bytes(b, "\\b", 2); break;
        case '\f': cs_sb_bytes(b, "\\f", 2); break;
        case '\n': cs_sb_bytes(b, "\\n", 2); break;
        case '\r': cs_sb_bytes(b, "\\r", 2); break;
        case '\t': cs_sb_bytes(b, "\\t", 2); break;
        case '\v': cs_sb_bytes(b, "\\v", 2); break;
        case '\\': cs_sb_bytes(b, "\\\\", 2); break;
        case '"': cs_sb_bytes(b, "\\\"", 2); break;
        default:
            if (c >= 0x20 && c <= 0x7e) {
                char one = (char)c;
                cs_sb_bytes(b, &one, 1);
            } else if (c < 0x80) {
                const char *hex = "0123456789abcdef";
                char e[4] = {'\\', 'x', hex[c >> 4], hex[c & 15]};
                cs_sb_bytes(b, e, 4);
            } else {
                fflush(stdout);
                fprintf(stderr, "codespec-run-c: a string outside ASCII cannot be printed\n");
                exit(2);
            }
        }
    }
    cs_sb_bytes(b, "\"", 1);
}

static inline void refuse_any(const cs_type *t) {
    fflush(stdout);
    fprintf(stderr, "codespec-run-c: an any holding %s cannot be printed\n", t->name);
    exit(2);
}

/* The size of a value of `t`, when the runner can print one. */
static inline size_t size_of(const cs_type *t) {
    switch (t->kind) {
    case CS_K_INT8: return sizeof(int8_t);
    case CS_K_INT16: return sizeof(int16_t);
    case CS_K_INT32: return sizeof(int32_t);
    case CS_K_INT64: return sizeof(int64_t);
    case CS_K_BOOL: return sizeof(bool);
    case CS_K_STRING: return sizeof(cs_string);
    case CS_K_SLICE: return sizeof(cs_slice);
    case CS_K_NAMED: return size_of(t->elem);
    default: refuse_any(t);
    }
    return 0;
}

static inline void show_typed(cs_sb *b, const cs_type *t, const void *p) {
    switch (t->kind) {
    case CS_K_INT8: cs_fmt_int(b, *(const int8_t *)p); return;
    case CS_K_INT16: cs_fmt_int(b, *(const int16_t *)p); return;
    case CS_K_INT32: cs_fmt_int(b, *(const int32_t *)p); return;
    case CS_K_INT64: cs_fmt_int(b, *(const int64_t *)p); return;
    case CS_K_BOOL: cs_fmt_bool(b, *(const bool *)p); return;
    case CS_K_STRING: show_string(b, *(const cs_string *)p); return;
    case CS_K_NAMED: show_typed(b, t->elem, p); return;
    case CS_K_SLICE: {
        const cs_slice *s = p;
        size_t esz = size_of(t->elem);
        cs_sb_bytes(b, "[", 1);
        for (int64_t i = 0; i < s->len; i++) {
            if (i > 0) {
                cs_sb_bytes(b, " ", 1);
            }
            show_typed(b, t->elem, (const char *)s->p + (size_t)i * esz);
        }
        cs_sb_bytes(b, "]", 1);
        return;
    }
    default: refuse_any(t);
    }
}

static inline void show_any(cs_sb *b, cs_any a) {
    if (a.t == NULL) {
        cs_sb_bytes(b, "<nil>", 5);
        return;
    }
    show_typed(b, a.t, a.p);
}

#endif
"#;

/// A C string literal of `s`, printable ASCII without `"` or `\`.
fn c_lit(s: &str) -> String {
    format!("\"{}\"", s.replace('?', "\\?"))
}

/// The runner's `driver.c` for the module header `h_text` of the module `base`, making `calls`.
/// Refuses a call of a function the header does not list, a wrong argument count or form, and a
/// parameter or result type outside the crate doc's tables.
pub fn driver_c(base: &str, h_text: &str, calls: &Calls) -> Result<String, Error> {
    let h = Header::read(base, h_text)?;
    let mut s = format!(
        "/* Code generated by codespec-run-c from calls.json. DO NOT EDIT. */\n#include \"{}\"\n#include \"codespec_driver.h\"\n\nint main(void) {{\n",
        h.file
    );
    for call in &calls.calls {
        let at = |e: String| format!("{}: {e}", call.func);
        let sig = h
            .funcs
            .get(&call.func)
            .ok_or_else(|| format!("no exported function {:?} in {}", call.func, h.file))?;
        if sig.params.len() != call.args.len() {
            return Err(at(format!(
                "takes {} arguments, the call passes {}",
                sig.params.len(),
                call.args.len()
            )));
        }
        s.push_str("    {\n");
        let mut args = Vec::new();
        let mut chans = Vec::new();
        for (i, (a, p)) in call.args.iter().zip(&sig.params).enumerate() {
            match p {
                Ty::Int { .. } => {
                    check_argument(a, ArgKind::Int).map_err(at)?;
                    let v = if a == &i64::MIN.to_string() {
                        "INT64_MIN".to_string()
                    } else {
                        format!("INT64_C({a})")
                    };
                    args.push(format!("({}){v}", h.cty(p)?));
                }
                Ty::Bool => {
                    check_argument(a, ArgKind::Bool).map_err(at)?;
                    args.push(a.clone());
                }
                Ty::String => {
                    check_argument(a, ArgKind::String).map_err(at)?;
                    args.push(format!(
                        "((cs_string){{{}, {}}})",
                        c_lit(&a[1..a.len() - 1]),
                        a.len() - 2
                    ));
                }
                Ty::Chan(elem) => {
                    check_argument(a, ArgKind::Chan).map_err(at)?;
                    h.printable(elem).map_err(at)?;
                    let ct = h.cty(elem).map_err(at)?;
                    writeln!(
                        s,
                        "        cs_chan *ch{i} = cs_chan_make(64, sizeof({ct}));"
                    )
                    .unwrap();
                    args.push(format!("ch{i}"));
                    chans.push((format!("ch{i}"), (**elem).clone()));
                }
                other => {
                    return Err(at(format!(
                        "a parameter of type {other:?} cannot be passed"
                    )));
                }
            }
        }
        let callee = format!("{}{}({})", h.px, call.func, args.join(", "));
        let values: Vec<(String, Ty)> = match sig.results.as_slice() {
            [] => {
                writeln!(s, "        {callee};").unwrap();
                Vec::new()
            }
            [t] => {
                writeln!(s, "        {} r = {callee};", h.cty(t).map_err(at)?).unwrap();
                vec![("r".to_string(), t.clone())]
            }
            ts => {
                writeln!(s, "        {}{}_result r = {callee};", h.px, call.func).unwrap();
                ts.iter()
                    .enumerate()
                    .map(|(i, t)| (format!("r.r{i}"), t.clone()))
                    .collect()
            }
        };
        let (plain, err) = match values.split_last() {
            Some(((v, Ty::Error), rest)) => (rest.to_vec(), Some(v.clone())),
            _ => (values.clone(), None),
        };
        let mut ok =
            String::from("        cs_sb *b = cs_sb_new();\n        cs_sb_bytes(b, \"ok\", 2);\n");
        for (v, t) in &plain {
            h.printable(t).map_err(at)?;
            ok.push_str("        cs_sb_bytes(b, \" \", 1);\n");
            h.show(t, v, &mut ok, 0).map_err(at)?;
        }
        ok.push_str("        cs_sb_println(b);\n");
        match err {
            Some(e) => {
                let indented: String = ok.lines().map(|l| format!("    {l}\n")).collect();
                writeln!(
                    s,
                    "        if ({e} != NULL) {{\n            cs_sb *b = cs_sb_new();\n            cs_sb_bytes(b, \"err \", 4);\n            cs_fmt_error(b, {e});\n            cs_sb_println(b);\n        }} else {{\n{indented}        }}"
                )
                .unwrap();
            }
            None => s.push_str(&ok),
        }
        for (ch, elem) in chans {
            let ct = h.cty(&elem).map_err(at)?;
            let mut show = String::new();
            h.show(&elem, "v", &mut show, 1).map_err(at)?;
            writeln!(
                s,
                "        while (cs_chan_len({ch}) > 0) {{\n            {ct} v = *({ct} *)cs_chan_recv({ch});\n            cs_sb *b = cs_sb_new();\n            cs_sb_bytes(b, \"sent \", 5);\n{show}            cs_sb_println(b);\n        }}"
            )
            .unwrap();
        }
        s.push_str("    }\n");
    }
    s.push_str("    return 0;\n}\n");
    Ok(s)
}

/// What a directory holds: C `codespec-write-c` wrote, or C written by hand.
enum Layout {
    /// The module's name: the stem of the one header besides `codespec_rt.h`.
    Written(String),
    /// No module header: the `.c` files, sorted.
    Plain(Vec<PathBuf>),
}

fn layout(dir: &Path) -> Result<Layout, Error> {
    let mut headers = Vec::new();
    let mut sources = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in entries {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.extension().is_some_and(|x| x == "h")
            && p.file_name().is_some_and(|n| n != "codespec_rt.h")
            && let Some(stem) = p.file_stem()
        {
            headers.push(stem.to_string_lossy().to_string());
        } else if p.extension().is_some_and(|x| x == "c") {
            sources.push(p);
        }
    }
    sources.sort();
    match headers.as_slice() {
        [one] => Ok(Layout::Written(one.clone())),
        [] if !sources.is_empty() => Ok(Layout::Plain(sources)),
        [] => Err(format!(
            "{}: no module header and no C source",
            dir.display()
        )),
        _ => Err(format!("{}: more than one module header", dir.display())),
    }
}

/// The exit status the program uses for a value it cannot print exactly.
const PROGRAM_REFUSED: i32 = 2;

/// Compile the C in `dir` with a driver for `calls` in `build` (emptied first) and run it; its
/// standard output and error are the caller's. A directory with a module header is C
/// `codespec-write-c` wrote ([`driver_c`]); one without is C written by hand.
pub fn run(dir: &Path, calls: &Calls, build: &Path) -> Result<(), Failure> {
    let dir =
        std::path::absolute(dir).map_err(|e| Failure::Failed(format!("{}: {e}", dir.display())))?;
    let (driver, sources) = match layout(&dir).map_err(Failure::Failed)? {
        Layout::Written(base) => {
            let header = dir.join(format!("{base}.h"));
            let text = std::fs::read_to_string(&header)
                .map_err(|e| Failure::Failed(format!("{}: {e}", header.display())))?;
            let driver = driver_c(&base, &text, calls).map_err(Failure::Refused)?;
            (driver, vec![dir.join(format!("{base}.c"))])
        }
        Layout::Plain(sources) => {
            let texts = plain::read_sources(&sources).map_err(Failure::Failed)?;
            let driver = plain::driver_c(&texts, calls).map_err(Failure::Refused)?;
            (driver, sources)
        }
    };
    let bin = compile(&dir, &sources, driver, build).map_err(Failure::Failed)?;
    let status = Command::new(&bin)
        .status()
        .map_err(|e| Failure::Failed(format!("{}: cannot start: {e}", bin.display())))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(PROGRAM_REFUSED) => Err(Failure::Refused(
            "the program met a value it cannot print exactly".into(),
        )),
        _ => Err(Failure::Failed(format!("the program: {status}"))),
    }
}

/// Write `driver` into `build` (emptied first), compile it with `sources` and return the program.
fn compile(
    dir: &Path,
    sources: &[PathBuf],
    driver: String,
    build: &Path,
) -> Result<PathBuf, Error> {
    let build = std::path::absolute(build).map_err(|e| format!("{}: {e}", build.display()))?;
    if build.exists() {
        std::fs::remove_dir_all(&build).map_err(|e| format!("{}: {e}", build.display()))?;
    }
    std::fs::create_dir_all(&build).map_err(|e| format!("{}: {e}", build.display()))?;
    let driver_h = build.join("codespec_driver.h");
    std::fs::write(&driver_h, DRIVER_H).map_err(|e| format!("{}: {e}", driver_h.display()))?;
    let driver_path = build.join("driver.c");
    std::fs::write(&driver_path, driver).map_err(|e| format!("{}: {e}", driver_path.display()))?;
    let bin = build.join("codespec-run");
    let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    let cflags = std::env::var("CFLAGS").unwrap_or_default();
    let out = Command::new(&cc)
        .args(["-std=c11", "-Wall", "-Werror"])
        .args(cflags.split_whitespace())
        .arg("-I")
        .arg(dir)
        .arg("-o")
        .arg(&bin)
        .arg(&driver_path)
        .args(sources)
        .output()
        .map_err(|e| format!("{}: cannot start: {e}", cc.to_string_lossy()))?;
    if !out.status.success() {
        return Err(format!(
            "{}: {}\n{}",
            cc.to_string_lossy(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_lines_are_read_as_go_types() {
        let h = Header::read(
            "m",
            "/* codespec: type Color string */\n/* codespec: type Span struct */\n/* codespec: func Two(int, map[string]int) (int, string) */\n/* codespec: func Emit(chan<- any, []Color) error */\n",
        )
        .unwrap();
        let two = &h.funcs["Two"];
        assert_eq!(
            two.params[0],
            Ty::Int {
                bits: 64,
                signed: true
            }
        );
        assert_eq!(two.params[1], Ty::Other("map[string]int".into()));
        assert_eq!(
            two.results,
            [
                Ty::Int {
                    bits: 64,
                    signed: true
                },
                Ty::String
            ]
        );
        let emit = &h.funcs["Emit"];
        assert_eq!(emit.params[0], Ty::Chan(Box::new(Ty::Any)));
        assert_eq!(
            emit.params[1],
            Ty::Slice(Box::new(Ty::Named("Color".into())))
        );
        assert_eq!(emit.results, [Ty::Error]);
        assert!(h.printable(&Ty::Named("Color".into())).is_ok());
        assert!(h.printable(&Ty::Named("Span".into())).is_err());
    }
}
