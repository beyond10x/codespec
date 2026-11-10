//! C written by hand, as a C reader reads it: the functions of the `.c` files of a directory
//! without a module header, called by the names `calls.json` gives (the crate doc, "C written by
//! hand").

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use codespec_conformance::{ArgKind, Calls, check_argument};

use crate::{Error, c_lit};

/// What a C type is to the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Bool,
    Str,
    Void,
    Other,
}

/// A C type as the source spells it, and its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CTy {
    spelling: String,
    kind: Kind,
}

/// One function definition with external linkage.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fun {
    name: String,
    ret: CTy,
    params: Vec<CTy>,
}

/// The functions and the typedefs of the sources that name an integer or a `bool`.
#[derive(Debug, Default)]
struct Source {
    /// By the name a call gives: the C name with its first letter upper case.
    funcs: BTreeMap<String, Fun>,
    /// Every `typedef enum … Name;` and `typedef <integer or bool type> Name;`, as written.
    typedefs: Vec<String>,
    /// The name each of those typedefs declares, and what it is to the runner.
    names: BTreeMap<String, Kind>,
}

const SIGNED: &[&str] = &[
    "signed char",
    "short",
    "short int",
    "signed short",
    "signed short int",
    "int",
    "signed",
    "signed int",
    "long",
    "long int",
    "signed long",
    "signed long int",
    "long long",
    "long long int",
    "signed long long",
    "signed long long int",
    "int8_t",
    "int16_t",
    "int32_t",
    "int64_t",
];

/// `s` with its tokens separated by one space, `*` a token of its own.
fn tokens(s: &str) -> String {
    s.replace('*', " * ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The name a call gives the C function `name`: its first letter upper case, as the Java runner
/// exports a method.
fn exported(name: &str) -> String {
    let mut c = name.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

fn ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `decl` split into its type and the identifier ending it.
fn split_decl(decl: &str) -> Option<(String, String)> {
    let decl = decl.trim();
    let start = decl.trim_end_matches(ident).len();
    let name = &decl[start..];
    let ty = tokens(&decl[..start]);
    if name.is_empty() || ty.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some((ty, name.to_string()))
}

impl Source {
    fn read(texts: &[String]) -> Source {
        let mut s = Source::default();
        for text in texts {
            s.read_one(text);
        }
        s
    }

    fn read_one(&mut self, text: &str) {
        let mut typedef: Option<String> = None;
        for line in text.lines() {
            if let Some(t) = typedef.as_mut() {
                t.push('\n');
                t.push_str(line);
                if line.contains('}') && line.trim_end().ends_with(';') {
                    let t = typedef.take().unwrap();
                    self.typedef(t);
                }
                continue;
            }
            if line.starts_with("typedef enum") {
                if line.contains('}') && line.trim_end().ends_with(';') {
                    self.typedef(line.to_string());
                } else {
                    typedef = Some(line.to_string());
                }
                continue;
            }
            if let Some(decl) = line
                .strip_prefix("typedef ")
                .and_then(|l| l.trim_end().strip_suffix(';'))
            {
                self.scalar_typedef(line, decl);
                continue;
            }
            if let Some(f) = definition(line) {
                self.funcs.insert(exported(&f.name), f);
            }
        }
        let names = self.names.clone();
        for f in self.funcs.values_mut() {
            for t in std::iter::once(&mut f.ret).chain(f.params.iter_mut()) {
                if t.kind == Kind::Other
                    && let Some(k) = names.get(&t.spelling)
                {
                    t.kind = *k;
                }
            }
        }
    }

    /// A `typedef enum … Name;`: `Name` is an integer.
    fn typedef(&mut self, t: String) {
        let name = t
            .trim_end()
            .trim_end_matches(';')
            .trim_end()
            .rsplit(|c: char| !ident(c))
            .next()
            .unwrap_or("")
            .to_string();
        if !name.is_empty() {
            self.names.insert(name, Kind::Int);
            self.typedefs.push(t);
        }
    }

    /// A one-line `typedef <type> Name;` of a signed integer type, `bool` or a name typedef'd so
    /// before: `Name` is what that type is. Any other typedef is not read.
    fn scalar_typedef(&mut self, line: &str, decl: &str) {
        let Some((ty, name)) = split_decl(decl) else {
            return;
        };
        let k = match kind(&ty) {
            Kind::Other => self.names.get(&ty).copied().unwrap_or(Kind::Other),
            k => k,
        };
        if matches!(k, Kind::Int | Kind::Bool) {
            self.names.insert(name, k);
            self.typedefs.push(line.trim_end().to_string());
        }
    }
}

fn kind(spelling: &str) -> Kind {
    match spelling {
        "bool" | "_Bool" => Kind::Bool,
        "const char *" | "char const *" | "char *" => Kind::Str,
        "void" => Kind::Void,
        s if SIGNED.contains(&s) => Kind::Int,
        _ => Kind::Other,
    }
}

fn cty(spelling: String) -> CTy {
    CTy {
        kind: kind(&spelling),
        spelling,
    }
}

/// The function a line opening a definition at the start of the line defines, when it has
/// external linkage: `<type> <name>(<params>) {`.
fn definition(line: &str) -> Option<Fun> {
    let line = line.trim_end();
    if !line.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') || !line.ends_with('{') {
        return None;
    }
    let first = line.split(|c: char| !ident(c)).next()?;
    if matches!(
        first,
        "static" | "typedef" | "extern" | "inline" | "_Noreturn"
    ) {
        return None;
    }
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    if close < open || !line[close + 1..].trim().eq("{") {
        return None;
    }
    let (ret, name) = split_decl(&line[..open])?;
    let inner = line[open + 1..close].trim();
    let mut params = Vec::new();
    if !(inner.is_empty() || inner == "void") {
        for p in split_params(inner) {
            params.push(param(&p)?);
        }
    }
    Some(Fun {
        name,
        ret: cty(ret),
        params,
    })
}

/// A parameter list split at its commas outside parentheses and brackets.
fn split_params(list: &str) -> Vec<String> {
    let (mut out, mut depth, mut cur) = (Vec::new(), 0i32, String::new());
    for c in list.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out
}

/// One parameter's type: `T name`; a function pointer `R (*name)(…)`, spelled `R (*)(…)`; an
/// array `T name[N]`, spelled `T [N]`. Only the first is a type the runner may pass.
fn param(p: &str) -> Option<CTy> {
    let p = p.trim();
    if let Some(open) = p.find("(*") {
        let rest = &p[open + 2..];
        let close = rest.find(')')?;
        if !rest[..close].trim().chars().all(ident) {
            return None;
        }
        return Some(CTy {
            spelling: format!("{} (*){}", tokens(&p[..open]), rest[close + 1..].trim()),
            kind: Kind::Other,
        });
    }
    if let Some(open) = p.find('[') {
        let (ty, _) = split_decl(&p[..open])?;
        return Some(CTy {
            spelling: format!("{ty} {}", p[open..].trim()),
            kind: Kind::Other,
        });
    }
    let (ty, _) = split_decl(p)?;
    Some(cty(ty))
}

/// The C sources of `dir`, read.
pub(crate) fn read_sources(files: &[PathBuf]) -> Result<Vec<String>, Error> {
    files
        .iter()
        .map(|f| std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display())))
        .collect()
}

/// `codespec_run_quote`: Go's `strconv.Quote` of a C string of ASCII; 2 for a byte outside ASCII
/// or a null pointer, which the runner cannot print exactly.
const QUOTE: &str = r#"static int codespec_run_quote(const char *s) {
    if (s == NULL) {
        fflush(stdout);
        fprintf(stderr, "codespec-run-c: a null string cannot be printed\n");
        return 2;
    }
    putchar('"');
    for (; *s != '\0'; s++) {
        unsigned char c = (unsigned char)*s;
        switch (c) {
        case '\a': fputs("\\a", stdout); break;
        case '\b': fputs("\\b", stdout); break;
        case '\f': fputs("\\f", stdout); break;
        case '\n': fputs("\\n", stdout); break;
        case '\r': fputs("\\r", stdout); break;
        case '\t': fputs("\\t", stdout); break;
        case '\v': fputs("\\v", stdout); break;
        case '\\': fputs("\\\\", stdout); break;
        case '"': fputs("\\\"", stdout); break;
        default:
            if (c >= 0x20 && c <= 0x7e) {
                putchar(c);
            } else if (c < 0x80) {
                printf("\\x%02x", c);
            } else {
                fflush(stdout);
                fprintf(stderr, "codespec-run-c: a string outside ASCII cannot be printed\n");
                return 2;
            }
        }
    }
    putchar('"');
    return 0;
}
"#;

/// The runner's `driver.c` for C written by hand in `texts`, making `calls`. The driver includes
/// no header of the sources and only `<stdbool.h>`, `<stdint.h>` and `<stdio.h>`, so a function
/// named as a function of `<stdlib.h>` (`div`) is called as the source defines it.
pub(crate) fn driver_c(texts: &[String], calls: &Calls) -> Result<String, Error> {
    let src = Source::read(texts);
    let mut protos: BTreeMap<String, String> = BTreeMap::new();
    let mut body = String::new();
    let mut quote = false;
    for call in &calls.calls {
        let at = |e: String| format!("{}: {e}", call.func);
        let f = src.funcs.get(&call.func).ok_or_else(|| {
            format!(
                "no exported function {:?} in the C sources (a function `{}` or `{}` with external linkage)",
                call.func,
                call.func,
                lower_first(&call.func)
            )
        })?;
        if f.params.len() != call.args.len() {
            return Err(at(format!(
                "takes {} arguments, the call passes {}",
                f.params.len(),
                call.args.len()
            )));
        }
        let mut args = Vec::new();
        for (a, p) in call.args.iter().zip(&f.params) {
            args.push(match p.kind {
                Kind::Int => {
                    check_argument(a, ArgKind::Int).map_err(at)?;
                    if a == &i64::MIN.to_string() {
                        format!("({})INT64_MIN", p.spelling)
                    } else {
                        format!("({})INT64_C({a})", p.spelling)
                    }
                }
                Kind::Bool => {
                    check_argument(a, ArgKind::Bool).map_err(at)?;
                    a.clone()
                }
                Kind::Str => {
                    check_argument(a, ArgKind::String).map_err(at)?;
                    c_lit(&a[1..a.len() - 1])
                }
                Kind::Void | Kind::Other => {
                    return Err(at(format!(
                        "a parameter of type `{}` cannot be passed",
                        p.spelling
                    )));
                }
            });
        }
        let params = if f.params.is_empty() {
            "void".to_string()
        } else {
            f.params
                .iter()
                .map(|p| p.spelling.clone())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let sep = if f.ret.spelling.ends_with('*') {
            ""
        } else {
            " "
        };
        protos.insert(
            f.name.clone(),
            format!("{}{sep}{}({params});", f.ret.spelling, f.name),
        );
        let callee = format!("{}({})", f.name, args.join(", "));
        let ret = &f.ret.spelling;
        let sep = if ret.ends_with('*') { "" } else { " " };
        body.push_str("    {\n");
        match f.ret.kind {
            Kind::Void => writeln!(body, "        {callee};\n        puts(\"ok\");").unwrap(),
            Kind::Int => writeln!(
                body,
                "        {ret}{sep}r = {callee};\n        printf(\"ok %lld\\n\", (long long)r);"
            )
            .unwrap(),
            Kind::Bool => writeln!(
                body,
                "        {ret}{sep}r = {callee};\n        puts(r ? \"ok true\" : \"ok false\");"
            )
            .unwrap(),
            Kind::Str => {
                quote = true;
                writeln!(
                    body,
                    "        {ret}{sep}r = {callee};\n        fputs(\"ok \", stdout);\n        if (codespec_run_quote(r) != 0) {{\n            return 2;\n        }}\n        putchar('\\n');"
                )
                .unwrap()
            }
            Kind::Other => {
                return Err(at(format!("a result of type `{ret}` cannot be printed")));
            }
        }
        body.push_str("    }\n");
    }
    let mut s = String::from(
        "/* Code generated by codespec-run-c from calls.json. DO NOT EDIT. */\n#include <stdbool.h>\n#include <stdint.h>\n#include <stdio.h>\n\n",
    );
    for t in &src.typedefs {
        writeln!(s, "{t}").unwrap();
    }
    for p in protos.values() {
        writeln!(s, "{p}").unwrap();
    }
    s.push('\n');
    if quote {
        s.push_str(QUOTE);
        s.push('\n');
    }
    s.push_str("int main(void) {\n");
    s.push_str(&body);
    s.push_str("    return 0;\n}\n");
    Ok(s)
}

fn lower_first(name: &str) -> String {
    let mut c = name.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_with_external_linkage_are_read_by_their_exported_name() {
        let s = Source::read(&[
            "typedef enum { Red = 0, Green = 1 } Color;\nstruct Pair {\n    long a;\n};\nstatic long twice(long n) {\nlong addTo(long a, long b) {\nconst char *quote(const char *s) {\nColor pick(long n) {\nlong zero(void) {\nlong add(long a, long b);\n    long inner(long n) {\n"
                .to_string(),
        ]);
        assert_eq!(
            s.funcs.keys().collect::<Vec<_>>(),
            ["AddTo", "Pick", "Quote", "Zero"]
        );
        assert_eq!(s.funcs["Quote"].ret.kind, Kind::Str);
        assert_eq!(s.funcs["Quote"].params[0].spelling, "const char *");
        assert_eq!(s.funcs["Pick"].ret.kind, Kind::Int);
        assert!(s.funcs["Zero"].params.is_empty());
        assert_eq!(s.typedefs, ["typedef enum { Red = 0, Green = 1 } Color;"]);
    }

    #[test]
    fn a_scalar_typedef_is_its_type_and_pointer_array_and_function_parameters_are_read() {
        let s = Source::read(&[
            "typedef long Celsius;\ntypedef Celsius Kelvin;\ntypedef struct Box Box;\nKelvin warm(Celsius c) {\nlong run(long (*f)(long, long), long xs[3], double *d) {\n"
                .to_string(),
        ]);
        assert_eq!(
            s.typedefs,
            ["typedef long Celsius;", "typedef Celsius Kelvin;"]
        );
        assert_eq!(s.funcs["Warm"].ret.kind, Kind::Int);
        assert_eq!(s.funcs["Warm"].params[0].kind, Kind::Int);
        let spellings: Vec<(&str, Kind)> = s.funcs["Run"]
            .params
            .iter()
            .map(|p| (p.spelling.as_str(), p.kind))
            .collect();
        assert_eq!(
            spellings,
            [
                ("long (*)(long, long)", Kind::Other),
                ("long [3]", Kind::Other),
                ("double *", Kind::Other)
            ]
        );
    }
}
