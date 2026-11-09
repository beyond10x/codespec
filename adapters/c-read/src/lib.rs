//! The C reader: the C11 sources of one directory to one `cir/1` module, from the typed AST clang
//! prints as JSON (`clang -Xclang -ast-dump=json -fsyntax-only -std=c11`, one run per `.c` file,
//! `clang` from `PATH`). Every expression's type is clang's (`qualType`, through typedefs); the
//! reader resolves no name and infers no type of its own. Nothing is guessed: a statement outside
//! the subset is kept as a `foreign` statement in `c` (its source, the reason and the calls it
//! makes), a declaration outside it is left out or kept as an opaque function, each with a refusal
//! at its position. Positions are `<file>:<line>`, the file relative to the source directory.
//!
//! The module is named after the directory (`-` read as `_`). Every `.c` file of the directory is
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
//! | `typedef` of another type | nothing: the type it names stands for it |
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
//! | `struct T`, `enum E` the directory declares | `named` |
//! | `void` (a result) | no result; a call of it is typed `unit` |
//! | variadic extern parameters (`...`) | a last parameter `[]any`, the extern `variadic` |
//! | plain `char`, floating point, pointers, arrays, function pointers, `volatile`, `_Atomic` | refused |
//!
//! # Statements
//!
//! | C | cir/1 |
//! |---|---|
//! | `T x;` or `T x = 0;` (a zero initializer: `0`, `{0}`, `{}`) | `var` |
//! | `T x = e;` | `let`, `e` of type `T` |
//! | `x = e;`, `x op= e;` | `assign`; `x &= ~e` is `&^=` |
//! | `x++;`, `++x;`, `x--;`, `--x;` | `incr`, `decr` |
//! | `f(…);` | `expr` |
//! | `if`, `else` | `if` |
//! | `for (init; cond; inc)` | `for`: `init` a declaration (`let`) or an assignment, `inc` an assignment, `++` or `--` |
//! | `while (c)`, `for (;;)`, `while (1)` | `for` with or without a condition |
//! | `do { … } while (c);` | `for { …; if !c { break } }`; a `continue` in it is refused, as it would run `c` |
//! | `switch` on an integer, `case` values constant | `switch`; stacked labels are one case; a case's trailing `break` leaves the switch, which a `cir/1` case does by ending. A case that falls through, or a `break` elsewhere in a case, is refused |
//! | `break`, `continue` in a loop | `break`, `continue` |
//! | `return`, `return e` | `return` |
//! | a label | its statement: only the `goto` that jumps to it is refused |
//! | `goto`, inline assembly, a block with declarations of its own | refused |
//!
//! # Expressions
//!
//! Every expression's type is clang's type of it, but for comparisons and logic.
//!
//! | C | cir/1 |
//! |---|---|
//! | an integer literal; an integer constant converted to the type its context needs | `lit` of that type, under `-` when negative |
//! | `"…"` (printable ASCII, the escapes `\n \t \r \\ \"`) | `lit` of type `string`, spelled as Go spells it |
//! | a parameter or local, a file-scope variable | `var`, `global` |
//! | an enumerator where its enum is expected, an anonymous enum's enumerator | `const` |
//! | `+ - * / % & | ^ << >>` on one integer type (`<<` and `>>` with any count type) | `binop`; `x & ~y` is `&^` |
//! | unary `- + ~` | `unop` `- + ^` |
//! | `< > <= >= == !=` on one integer or enum type, `&& || !` | `binop`, `unop` of type `bool`, read where C converts them to `_Bool` or tests them; an integer tested is `x != 0` |
//! | `f(…)` | `call` of a function the module defines, else `extern` |
//! | `s.f` | `field` |
//! | `(struct T){…}`, `{…}` initializing a struct | `struct`, the fields it writes |
//!
//! An implicit or explicit conversion between two integer types, a `_Bool` used as an integer and a
//! comparison used as an `int` are refused: `cir/1` has no such conversion. So are pointer
//! arithmetic, `&`, `*`, `->`, `?:`, `sizeof`, the comma operator, an assignment or `++` inside an
//! expression, indexing, character and floating-point literals and a call through a function
//! pointer.
//!
//! # What the reader claims, and what it does not
//!
//! `codespec-read-c --describe` claims each feature whose conformance case has a hand-written C
//! source, `conformance/cases/<case>/c/case.c`. `tests/cases.rs` holds every claim: the source reads
//! without a refusal and with its subject, and passed and written as Go runs as the reference does.
//! Running the source as written is step 1 of `codespec conform`, with the C runner. The vocabulary
//! not claimed:
//!
//! | feature | why |
//! |---|---|
//! | `sem.int.wrap` | signed overflow is undefined in C, and `cir/1` integers wrap: a C source never relies on the wrap a reader would claim |
//! | `fact.*`, `stmt.foreign` | computed by `codespec pass`; what a reader writes for what it cannot read |
//! | `sem.switch.break` | a C `break` in a `switch` leaves the switch, never the loop around it |
//! | `op.eq` | its case compares strings, which `==` on `const char *` does not |
//! | `stmt.continue`, `stmt.expr`, `expr.extern`, `decl.var`, `type.named`, `intrinsic.*` and the rest | each case's subject needs a slice, a map, `println`, a string operation or a newtype, which C has not |

mod body;
mod json;
mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use codespec_model::{
    ConstDecl, Expr, Extern, Field, ForeignInit, Func, IntWidth, Module, Opaque, Param, Reader,
    Refusal, TypeDecl, VarDecl, Variant,
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
    !f.is_empty() && !f.starts_with('/') && !f.starts_with('<') && !f.contains("..")
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
        .and_then(|d| d.file_name().map(|n| n.to_string_lossy().replace('-', "_")))
        .unwrap_or_else(|| "c".into());
    Ok(Assembly::new(&src).run(&asts, module))
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
        }
    }

    fn refuse(&mut self, n: &J, what: String, reason: String) {
        self.refusals.push(Refusal {
            pos: pos_of(self.src, n),
            what,
            reason,
        });
    }

    fn run(mut self, asts: &[J], name: String) -> Module {
        let mut units = Vec::new();
        let mut defs = Vec::new();
        for ast in asts {
            let (unit, funcs) = self.declarations(ast);
            units.push(unit);
            defs.push(funcs);
        }
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
            errors: Vec::new(),
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
                    unit.typedefs.insert(
                        n.s("name").into(),
                        Typedef {
                            target: target.into(),
                            named,
                        },
                    );
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

    fn record(&mut self, unit: &Unit, n: &J) {
        let tag = n.s("tagUsed");
        let Some(name) = Self::tag_name(unit, n) else {
            self.refuse(n, tag.to_string(), format!("an unnamed {tag}"));
            return;
        };
        if self.tables.structs.contains_key(&name) || self.tables.refused.contains_key(&name) {
            return;
        }
        let mut why = None;
        let mut fields = Vec::new();
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
        for p in n.inner().iter().filter(|p| p.kind() == "ParmVarDecl") {
            if why.is_some() {
                break;
            }
            match self.tables.ty(unit, p.get("type")) {
                Ok(t) => params.push((p.s("id").to_string(), p.s("name").to_string(), t)),
                Err(e) => why = Some(format!("parameter {} is {e}", p.s("name"))),
            }
        }
        let result = match why {
            None => match self.tables.result(unit, n.get("type").s("qualType")) {
                Ok(t) => Some(t),
                Err(e) => {
                    why = Some(format!("its result is {e}"));
                    None
                }
            },
            Some(_) => None,
        };
        let mut b = Body::new(&self.tables, unit, self.src, decls, params.clone());
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
        let results = match result {
            Some(codespec_model::Type::Unit) | None => Vec::new(),
            Some(t) => vec![t],
        };
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
