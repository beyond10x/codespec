//! The fact base: every relation of [`RELATIONS`] computed from a set of `cir/1` documents, the
//! places it cannot see ([`Site`]) with the facts each could still hold ([`FactBase::wild`]), and
//! its cache encoding.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use codespec_model::{CallRef, ChanDir, Expr, FloatBits, IntWidth, Module, Stmt, Type, TypeDecl};

use crate::{Site, Value};

/// The type of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Str,
    Int,
    Bool,
    /// A node id, `<module path>.<function key>#<index path>`, or a function id.
    Node,
    /// A type id, its spelling.
    Type,
    /// A list of nodes: a path's witness.
    List,
}

impl Ty {
    pub fn name(self) -> &'static str {
        match self {
            Ty::Str => "string",
            Ty::Int => "int",
            Ty::Bool => "bool",
            Ty::Node => "node",
            Ty::Type => "type",
            Ty::List => "list",
        }
    }

    /// Whether `v` is a value of this type.
    pub fn holds(self, v: &Value) -> bool {
        match self {
            Ty::Str | Ty::Node | Ty::Type => matches!(v, Value::Str(_)),
            Ty::Int => matches!(v, Value::Int(_)),
            Ty::Bool => matches!(v, Value::Bool(_)),
            Ty::List => matches!(v, Value::List(_)),
        }
    }
}

/// Where a relation comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Documents,
    Bodies,
    Graph,
}

/// One relation of the fact base.
pub struct Relation {
    pub name: &'static str,
    pub fields: &'static [(&'static str, Ty)],
    pub about: &'static str,
    pub group: Group,
}

impl Relation {
    pub fn field(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|(f, _)| *f == name)
    }
}

use Ty::{Bool as B, Int as I, List as L, Node as N, Str as S, Type as T};

/// Every relation, in the order `codespec query --schema` prints them.
pub const RELATIONS: &[Relation] = &[
    Relation {
        name: "module",
        fields: &[
            ("path", S),
            ("name", S),
            ("lang", S),
            ("reader", S),
            ("version", S),
        ],
        about: "one per document; path is its import path, or its name when it has none",
        group: Group::Documents,
    },
    Relation {
        name: "func",
        fields: &[
            ("id", N),
            ("module", S),
            ("name", S),
            ("recv", S),
            ("exported", B),
            ("kind", S),
            ("pos", S),
            ("line", I),
        ],
        about: "every function: kind func, method, init, lambda (id the literal's node) or opaque (no body read)",
        group: Group::Documents,
    },
    Relation {
        name: "param",
        fields: &[("func", N), ("index", I), ("name", S), ("t", T)],
        about: "every parameter; the receiver of a method at index -1",
        group: Group::Documents,
    },
    Relation {
        name: "result",
        fields: &[("func", N), ("index", I), ("name", S), ("t", T)],
        about: "every result type",
        group: Group::Documents,
    },
    Relation {
        name: "raises",
        fields: &[("func", N), ("t", T)],
        about: "every extern type a function lists in raises: what its raise statements and the functions it calls raise, outside its results",
        group: Group::Documents,
    },
    Relation {
        name: "stmt",
        fields: &[
            ("id", N),
            ("func", N),
            ("kind", S),
            ("op", S),
            ("pos", S),
            ("line", I),
            ("parent", N),
        ],
        about: "every statement; func the declared function holding it, parent the node it sits in, op an assign's operator",
        group: Group::Documents,
    },
    Relation {
        name: "expr",
        fields: &[
            ("id", N),
            ("func", N),
            ("kind", S),
            ("t", T),
            ("stmt", N),
            ("parent", N),
            ("slot", S),
            ("index", I),
            ("op", S),
            ("name", S),
            ("value", S),
            ("x", N),
            ("y", N),
        ],
        about: "every expression: slot and index its place in parent (index -1 outside a list); x and y its operands",
        group: Group::Documents,
    },
    Relation {
        name: "let_name",
        fields: &[("stmt", N), ("index", I), ("name", S), ("last", B)],
        about: "every name a let binds",
        group: Group::Documents,
    },
    Relation {
        name: "field_init",
        fields: &[("expr", N), ("index", I), ("name", S), ("value", N)],
        about: "every field of a struct literal",
        group: Group::Documents,
    },
    Relation {
        name: "call",
        fields: &[
            ("site", N),
            ("func", N),
            ("kind", S),
            ("callee", S),
            ("path", S),
            ("type", S),
            ("name", S),
            ("recv", N),
            ("args", I),
            ("pos", S),
            ("line", I),
            ("listed", B),
        ],
        about: "every call, read or listed (foreign statements, opaque functions): kind func, method, iface, value or ref (a function used as a value), and for a listed call a reader names without resolving it named (a method name or a function's name alone, type empty) or macro (a macro invocation, path as written); callee <path>.<name> or <path>.<type>.<name>",
        group: Group::Documents,
    },
    Relation {
        name: "call_by",
        fields: &[("site", N), ("by", S)],
        about: "every listed call whose target a convention gave, not the source language's type information: by the convention the reader names (Groovy: grails-gorm, grails-service-field); calls links such a call as any other",
        group: Group::Documents,
    },
    Relation {
        name: "type",
        fields: &[
            ("id", T),
            ("kind", S),
            ("name", S),
            ("path", S),
            ("bits", I),
            ("signed", B),
            ("elem", T),
            ("key", T),
            ("fields", I),
            ("under", T),
            ("decl", S),
        ],
        about: "every type the documents name or declare; bits and signed of an integer, a named integer resolved; decl the kind of a declared type",
        group: Group::Documents,
    },
    Relation {
        name: "type_elem",
        fields: &[("type", T), ("index", I), ("t", T), ("last", B)],
        about: "every element of a tuple type",
        group: Group::Documents,
    },
    Relation {
        name: "type_field",
        fields: &[("type", T), ("index", I), ("name", S), ("t", T)],
        about: "every field of a declared struct",
        group: Group::Documents,
    },
    Relation {
        name: "fact",
        fields: &[("kind", S), ("module", S), ("item", S)],
        about: "the facts of codespec pass: fallible, comma_ok, shared, messages, read_only, displays, unordered_iteration",
        group: Group::Documents,
    },
    Relation {
        name: "refusal",
        fields: &[("module", S), ("pos", S), ("what", S), ("reason", S)],
        about: "every refusal a reader recorded",
        group: Group::Documents,
    },
    Relation {
        name: "foreign",
        fields: &[("stmt", N), ("lang", S), ("reason", S), ("calls_known", B)],
        about: "every foreign statement; calls_known when the reader listed its calls",
        group: Group::Documents,
    },
    Relation {
        name: "unknown",
        fields: &[("func", N), ("site", N), ("pos", S), ("reason", S)],
        about: "every place the fact base cannot see: an opaque function, a foreign statement without a call list, a call of a function value or through an interface",
        group: Group::Documents,
    },
    Relation {
        name: "reader_has",
        fields: &[("lang", S), ("part", S)],
        about: "what the readers of a language fill: bodies, calls, externs, dispatch",
        group: Group::Documents,
    },
    Relation {
        name: "method_set",
        fields: &[("t", T), ("name", S), ("sig", S), ("target", S)],
        about: "the method sets the reader listed, the target as a callee",
        group: Group::Documents,
    },
    Relation {
        name: "contains",
        fields: &[("outer", N), ("inner", N)],
        about: "inner lies inside outer, at any depth",
        group: Group::Bodies,
    },
    Relation {
        name: "cfg_next",
        fields: &[("from", N), ("to", N)],
        about: "control may pass from one statement to the next",
        group: Group::Bodies,
    },
    Relation {
        name: "entry",
        fields: &[("func", N), ("node", N)],
        about: "the first statement of a body (a function or a function literal)",
        group: Group::Bodies,
    },
    Relation {
        name: "exit",
        fields: &[("func", N), ("node", N)],
        about: "every return or raise of a body, every statement calling a function that lists raises, and every statement that falls off its end",
        group: Group::Bodies,
    },
    Relation {
        name: "dominates",
        fields: &[("a", N), ("b", N)],
        about: "every path from the entry to b passes a; a dominates itself",
        group: Group::Bodies,
    },
    Relation {
        name: "postdominates",
        fields: &[("a", N), ("b", N)],
        about: "every path from b to an exit passes a; a postdominates itself",
        group: Group::Bodies,
    },
    Relation {
        name: "guards",
        fields: &[("cond", N), ("node", N), ("branch", B)],
        about: "node runs only when cond, the condition of an if or a for, is branch",
        group: Group::Bodies,
    },
    Relation {
        name: "def",
        fields: &[("var", S), ("node", N)],
        about: "node sets the local var (<body>$<name>): let, var, assign, incr, decr, for_range, lookup_else; a parameter at its body",
        group: Group::Bodies,
    },
    Relation {
        name: "use",
        fields: &[("var", S), ("node", N)],
        about: "the expression node reads the local var",
        group: Group::Bodies,
    },
    Relation {
        name: "flows",
        fields: &[("from", N), ("to", N)],
        about: "the definition at from reaches the use at to",
        group: Group::Bodies,
    },
    Relation {
        name: "value",
        fields: &[("expr", N), ("var", S)],
        about: "the expression is the local var",
        group: Group::Bodies,
    },
    Relation {
        name: "calls",
        fields: &[("site", N), ("target", N)],
        about: "a call linked to the function of the set it names: a function, a method of a declared type, a function used as a value",
        group: Group::Graph,
    },
    Relation {
        name: "reaches",
        fields: &[("from", N), ("to", N)],
        about: "to is reached from from by one or more calls",
        group: Group::Graph,
    },
    Relation {
        name: "reaches_path",
        fields: &[("from", N), ("to", N), ("path", L)],
        about: "reaches with its witness: the functions from from to to, the least of the shortest call paths by function id",
        group: Group::Graph,
    },
    Relation {
        name: "reached",
        fields: &[("func", N)],
        about: "codespec reach (rta) reaches func from every main.main and init",
        group: Group::Graph,
    },
    Relation {
        name: "scc",
        fields: &[("func", N), ("comp", N)],
        about: "the strongly connected components of the functions over calls; comp the least function id of func's component",
        group: Group::Graph,
    },
    Relation {
        name: "effect",
        fields: &[("func", N), ("effect", S), ("pos", S)],
        about: "codespec effects: func may have the effect through the call at pos (its own, or one leading there), from the summaries and the barrier given",
        group: Group::Graph,
    },
    Relation {
        name: "syscall",
        fields: &[("func", N), ("name", S), ("pos", S)],
        about: "codespec effects: func may make the syscall through the call at pos, from the barrier given",
        group: Group::Graph,
    },
    Relation {
        name: "makes",
        fields: &[("func", N), ("t", T)],
        about: "func's body makes a value of the named type t: a struct literal, a conversion, a zero value, a literal or constant, a var, an error wrapped, a listed new; a method makes its receiver's type; making a struct makes the named types its fields hold by value",
        group: Group::Graph,
    },
    Relation {
        name: "implements",
        fields: &[("t", T), ("iface", S)],
        about: "the method set of t holds every method of the interface iface with an identical signature (the CHA test); iface a declared interface or one a call list names",
        group: Group::Graph,
    },
];

/// The relations computed when a query names them, not stored or cached.
pub const ON_DEMAND: &[&str] = &["reaches", "reaches_path"];

pub fn relation(name: &str) -> Option<&'static Relation> {
    RELATIONS.iter().find(|r| r.name == name)
}

/// A relation of `docs/query-language.md` § 4.2 the fact base does not build yet.
pub struct Waiting {
    pub name: &'static str,
    pub fields: &'static str,
    /// Why, completing "<name> …".
    pub why: &'static str,
}

/// The relations of § 4.2 that wait for a construct cq does not run yet: none since path
/// patterns (§ 5.2) gave `reaches_path` its witness.
pub const WAITING: &[Waiting] = &[];

/// Whether a site holds code nothing read: an opaque function, a foreign statement without a
/// call list (not a call the fact base cannot link, which `codespec reach` resolves itself).
pub(crate) fn hides_code(site: &crate::Site) -> bool {
    site.reason.starts_with(OPAQUE) || site.reason.starts_with(FOREIGN)
}

/// Whether a site is an opaque function: a body nothing read.
pub(crate) fn is_opaque(site: &crate::Site) -> bool {
    site.reason.starts_with(OPAQUE)
}

const OPAQUE: &str = "opaque function: ";
const FOREIGN: &str = "foreign statement without a call list: ";

pub fn waiting(name: &str) -> Option<&'static Waiting> {
    WAITING.iter().find(|w| w.name == name)
}

/// The text `codespec query --schema` prints.
pub fn schema_text() -> String {
    let mut out = String::from(
        "// The fact base of codespec query: every relation, its fields in order and their types.\n\
         // A node is <module path>.<function key>#<index path>; a position is <module path>/<file>:<line>.\n",
    );
    let mut group = None;
    for r in RELATIONS {
        if group != Some(r.group) {
            group = Some(r.group);
            out += match r.group {
                Group::Documents => "\n// From the documents, every reader.\n\n",
                Group::Bodies => "\n// Derived per read body.\n\n",
                Group::Graph => "\n// From codespec's graph.\n\n",
            };
        }
        let fields: Vec<String> = r
            .fields
            .iter()
            .map(|(f, t)| format!("{f}: {}", t.name()))
            .collect();
        out += &format!("{}{{{}}}\n    {}\n", r.name, fields.join(", "), r.about);
    }
    if !WAITING.is_empty() {
        out += "\n// Not built yet: a query naming one is refused.\n\n";
    }
    for w in WAITING {
        out += &format!("{}({})\n    {} {}\n", w.name, w.fields, w.name, w.why);
    }
    out
}

/// The format of the cache file.
pub const FACTS_FORMAT: &str = "codespec-query-facts/1";

/// A fact an unknown site could add (`None` a field that could hold anything), and the site.
pub(crate) type WildRow = (Vec<Option<Value>>, usize);

/// The facts of every relation but those computed on demand, sorted, each once; the facts the
/// unknown sites could add (`None` a field that could hold anything); the unknown sites.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FactBase {
    pub(crate) rels: BTreeMap<&'static str, Vec<Vec<Value>>>,
    pub(crate) wild: BTreeMap<&'static str, Vec<WildRow>>,
    pub(crate) sites: Vec<Site>,
}

fn s(x: impl Into<String>) -> Value {
    Value::Str(x.into())
}

fn i(n: impl TryInto<i64>) -> Value {
    Value::Int(n.try_into().unwrap_or(i64::MAX))
}

/// A module's path, or its name when it has none.
fn label(m: &Module) -> &str {
    if m.path.is_empty() { &m.name } else { &m.path }
}

/// A position of a module's source: `<path>/<file>:<line>`.
fn at(m: &Module, pos: &str) -> String {
    if m.path.is_empty() {
        pos.to_string()
    } else {
        format!("{}/{pos}", m.path)
    }
}

fn line(pos: &str) -> i64 {
    pos.rsplit_once(':')
        .and_then(|(_, l)| l.parse().ok())
        .unwrap_or(0)
}

/// `path.name`, `path.Recv.name`, the empty parts left out.
fn key(path: &str, recv: &str, name: &str) -> String {
    [path, recv, name]
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(".")
}

fn stmt_kind(s: &Stmt) -> &'static str {
    match s {
        Stmt::Let { .. } => "let",
        Stmt::Var { .. } => "var",
        Stmt::Assign { .. } => "assign",
        Stmt::Incr { .. } => "incr",
        Stmt::Decr { .. } => "decr",
        Stmt::If { .. } => "if",
        Stmt::ForRange { .. } => "for_range",
        Stmt::For { .. } => "for",
        Stmt::Break { .. } => "break",
        Stmt::Continue { .. } => "continue",
        Stmt::Switch { .. } => "switch",
        Stmt::Return { .. } => "return",
        Stmt::Raise { .. } => "raise",
        Stmt::Expr { .. } => "expr",
        Stmt::Send { .. } => "send",
        Stmt::LookupElse { .. } => "lookup_else",
        Stmt::Foreign { .. } => "foreign",
    }
}

fn stmt_pos(s: &Stmt) -> &str {
    match s {
        Stmt::Let { pos, .. }
        | Stmt::Var { pos, .. }
        | Stmt::Assign { pos, .. }
        | Stmt::Incr { pos, .. }
        | Stmt::Decr { pos, .. }
        | Stmt::If { pos, .. }
        | Stmt::ForRange { pos, .. }
        | Stmt::For { pos, .. }
        | Stmt::Break { pos }
        | Stmt::Continue { pos }
        | Stmt::Switch { pos, .. }
        | Stmt::Return { pos, .. }
        | Stmt::Raise { pos, .. }
        | Stmt::Expr { pos, .. }
        | Stmt::Send { pos, .. }
        | Stmt::LookupElse { pos, .. }
        | Stmt::Foreign { pos, .. } => pos,
    }
}

fn expr_kind(e: &Expr) -> &'static str {
    match e {
        Expr::Lit { .. } => "lit",
        Expr::Float { .. } => "float",
        Expr::Nil { .. } => "nil",
        Expr::Var { .. } => "var",
        Expr::Const { .. } => "const",
        Expr::Errvar { .. } => "errvar",
        Expr::Global { .. } => "global",
        Expr::Field { .. } => "field",
        Expr::Index { .. } => "index",
        Expr::Call { .. } => "call",
        Expr::Method { .. } => "method",
        Expr::Extern { .. } => "extern",
        Expr::ExternMethod { .. } => "extern_method",
        Expr::Binop { .. } => "binop",
        Expr::Unop { .. } => "unop",
        Expr::Struct { .. } => "struct",
        Expr::Conv { .. } => "conv",
        Expr::Make { .. } => "make",
        Expr::Zero { .. } => "zero",
        Expr::Intrinsic { .. } => "intrinsic",
        Expr::ToError { .. } => "to_error",
        Expr::FuncRef { .. } => "func_ref",
        Expr::Lambda { .. } => "lambda",
        Expr::CallValue { .. } => "call_value",
        Expr::Spread { .. } => "spread",
        Expr::IfaceCall { .. } => "iface_call",
        Expr::ToIface { .. } => "to_iface",
    }
}

fn width_bits(w: IntWidth) -> i64 {
    match w {
        IntWidth::W8 => 8,
        IntWidth::W16 => 16,
        IntWidth::W32 => 32,
        IntWidth::W64 | IntWidth::Platform => 64,
    }
}

/// The spelling of a type, its id: `int`, `uint16`, `example.com/m.Port`, `*T`, `[]byte`,
/// `map[K]V`, `(int, error)`, `func(string) bool`.
fn spell(l: &str, t: &Type) -> String {
    let list = |ts: &[Type]| {
        ts.iter()
            .map(|t| spell(l, t))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let targs = |ts: &[Type]| {
        if ts.is_empty() {
            String::new()
        } else {
            format!("[{}]", list(ts))
        }
    };
    match t {
        Type::Int { width, signed } => {
            let base = if *signed { "int" } else { "uint" };
            match width {
                IntWidth::Platform => base.to_string(),
                w => format!("{base}{}", width_bits(*w)),
            }
        }
        Type::Float { bits } => match bits {
            FloatBits::W32 => "float32".into(),
            FloatBits::W64 => "float64".into(),
        },
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Error => "error".into(),
        Type::Any => "any".into(),
        Type::Unit => "unit".into(),
        Type::Named { name, targs: ta } => format!("{}{}", key(l, "", name), targs(ta)),
        Type::Ptr { elem } => format!("*{}", spell(l, elem)),
        Type::Slice { elem } => format!("[]{}", spell(l, elem)),
        Type::Array { len, elem } => format!("[{len}]{}", spell(l, elem)),
        Type::Map { key: k, elem } => format!("map[{}]{}", spell(l, k), spell(l, elem)),
        Type::Chan { elem, dir } => match dir {
            ChanDir::Send => format!("chan<- {}", spell(l, elem)),
            ChanDir::Recv => format!("<-chan {}", spell(l, elem)),
            ChanDir::Both => format!("chan {}", spell(l, elem)),
        },
        Type::Tuple { elems } => format!("({})", list(elems)),
        Type::Extern {
            path,
            name,
            targs: ta,
        } => format!("{}{}", key(path, "", name), targs(ta)),
        Type::Func { params, results } => match results.len() {
            0 => format!("func({})", list(params)),
            1 => format!("func({}) {}", list(params), spell(l, &results[0])),
            _ => format!("func({}) ({})", list(params), list(results)),
        },
        Type::Param { name } => name.clone(),
    }
}

/// The Go reader's spelling of a signature, as the method sets and the call lists hold it (and
/// as `codespec effects` spells a declared interface's methods for its CHA test): every named
/// type qualified by its module's path, `byte` and `rune` for the unsigned 8-bit and signed
/// 32-bit integers.
fn go_sig(m: &Module, t: &Type) -> String {
    let list = |ts: &[Type]| {
        ts.iter()
            .map(|t| go_sig(m, t))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let targs = |ts: &[Type]| {
        if ts.is_empty() {
            String::new()
        } else {
            format!("[{}]", list(ts))
        }
    };
    match t {
        Type::Int {
            width: IntWidth::W8,
            signed: false,
        } => "byte".into(),
        Type::Int {
            width: IntWidth::W32,
            signed: true,
        } => "rune".into(),
        Type::Int { width, signed } => {
            let w = match width {
                IntWidth::Platform => String::new(),
                w => width_bits(*w).to_string(),
            };
            format!("{}int{w}", if *signed { "" } else { "u" })
        }
        Type::Float { bits } => match bits {
            FloatBits::W32 => "float32".into(),
            FloatBits::W64 => "float64".into(),
        },
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Error => "error".into(),
        Type::Any => "any".into(),
        Type::Unit => String::new(),
        Type::Named { name, targs: ta } => format!("{}.{name}{}", label(m), targs(ta)),
        Type::Param { name } => name.clone(),
        Type::Ptr { elem } => format!("*{}", go_sig(m, elem)),
        Type::Slice { elem } => format!("[]{}", go_sig(m, elem)),
        Type::Array { len, elem } => format!("[{len}]{}", go_sig(m, elem)),
        Type::Map { key: k, elem } => format!("map[{}]{}", go_sig(m, k), go_sig(m, elem)),
        Type::Chan { elem, dir } => match dir {
            ChanDir::Both => format!("chan {}", go_sig(m, elem)),
            ChanDir::Send => format!("chan<- {}", go_sig(m, elem)),
            ChanDir::Recv => format!("<-chan {}", go_sig(m, elem)),
        },
        Type::Tuple { elems } => format!("({})", list(elems)),
        Type::Extern {
            path,
            name,
            targs: ta,
        } => format!("{path}.{name}{}", targs(ta)),
        Type::Func { params, results } => {
            let r = match results.as_slice() {
                [] => String::new(),
                [one] => format!(" {}", go_sig(m, one)),
                more => format!(" ({})", list(more)),
            };
            format!("func({}){r}", list(params))
        }
    }
}

/// The relations and wild facts as they are collected.
#[derive(Default)]
struct Builder {
    rels: BTreeMap<&'static str, BTreeSet<Vec<Value>>>,
    wild: BTreeMap<&'static str, BTreeSet<WildRow>>,
    sites: Vec<Site>,
    site_index: HashMap<(String, String, String), usize>,
    types: BTreeSet<String>,
    /// Each interface's methods as (name, signature), by its id.
    ifaces: BTreeMap<String, BTreeSet<(String, String)>>,
    /// Every function of the modules that lists `raises`, as (module path, receiver type name or
    /// `""`, name): a call of one leaves the caller there too.
    raising: Rc<BTreeSet<(String, String, String)>>,
}

impl Builder {
    fn add(&mut self, rel: &'static str, row: Vec<Value>) {
        debug_assert_eq!(
            relation(rel).map(|r| r.fields.len()),
            Some(row.len()),
            "{rel}"
        );
        self.rels.entry(rel).or_default().insert(row);
    }

    /// A fact the site could add: the named fields fixed, every other one anything.
    fn add_wild(&mut self, rel: &'static str, fixed: &[(&str, Value)], site: usize) {
        let r = relation(rel).expect("a relation of the schema");
        let mut row = vec![None; r.fields.len()];
        for (f, v) in fixed {
            row[r.field(f).expect("a field of the relation")] = Some(v.clone());
        }
        self.wild.entry(rel).or_default().insert((row, site));
    }

    /// A place the fact base cannot see; the same place twice (a document given twice) is one.
    fn unknown(&mut self, func: &str, site: &str, pos: String, reason: String) -> usize {
        self.add(
            "unknown",
            vec![s(func), s(site), s(pos.clone()), s(reason.clone())],
        );
        let x = Site {
            site: site.to_string(),
            pos,
            reason,
        };
        if let Some(&k) = self
            .site_index
            .get(&(x.site.clone(), x.pos.clone(), x.reason.clone()))
        {
            return k;
        }
        self.site_index.insert(
            (x.site.clone(), x.pos.clone(), x.reason.clone()),
            self.sites.len(),
        );
        self.sites.push(x);
        self.sites.len() - 1
    }

    /// `scc`: the components of the functions over the linked calls, each named by its least
    /// function id.
    fn scc(&mut self) {
        let funcs: Vec<String> = self
            .rels
            .get("func")
            .into_iter()
            .flatten()
            .filter_map(|r| match &r[0] {
                Value::Str(id) => Some(id.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let at: HashMap<&str, usize> = funcs
            .iter()
            .enumerate()
            .map(|(k, f)| (f.as_str(), k))
            .collect();
        let mut func_of: HashMap<&str, &str> = HashMap::new();
        for r in self.rels.get("call").into_iter().flatten() {
            if let (Value::Str(site), Value::Str(f)) = (&r[0], &r[1]) {
                func_of.insert(site, f);
            }
        }
        let mut edges: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); funcs.len()];
        for r in self.rels.get("calls").into_iter().flatten() {
            if let (Value::Str(site), Value::Str(t)) = (&r[0], &r[1])
                && let Some(f) = func_of.get(site.as_str())
                && let (Some(&a), Some(&b2)) = (at.get(f), at.get(t.as_str()))
            {
                edges[a].insert(b2);
            }
        }
        let edges: Vec<Vec<usize>> = edges.into_iter().map(|e| e.into_iter().collect()).collect();
        let mut rows = Vec::new();
        for comp in crate::graph::components(&edges) {
            let least = comp.iter().map(|&k| &funcs[k]).min().expect("a component");
            for &k in &comp {
                rows.push(vec![s(&funcs[k]), s(least)]);
            }
        }
        for r in rows {
            self.add("scc", r);
        }
    }

    /// A struct's value holds its fields' values: making one makes every named type its fields
    /// hold by value (an array's element too, not a pointer's), at any depth, as `codespec
    /// effects` counts them for RTA.
    fn fields_made(&mut self, modules: &[&Module]) {
        let mut fields_of: HashMap<String, Vec<String>> = HashMap::new();
        for m in modules {
            for d in &m.types {
                if let TypeDecl::Struct { name, fields, .. } = d {
                    let mut held = Vec::new();
                    for f in fields {
                        let mut t = &f.t;
                        while let Type::Array { elem, .. } = t {
                            t = elem;
                        }
                        if matches!(t, Type::Named { .. } | Type::Extern { .. }) {
                            held.push(self.ty(m, t));
                        }
                    }
                    fields_of.insert(key(label(m), "", name), held);
                }
            }
        }
        let made: Vec<Vec<Value>> = self
            .rels
            .get("makes")
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        for row in made {
            let (Value::Str(f), Value::Str(t)) = (&row[0], &row[1]) else {
                continue;
            };
            let mut seen: BTreeSet<String> = BTreeSet::new();
            let mut stack = vec![t.clone()];
            while let Some(t) = stack.pop() {
                // A generic type's id carries its arguments: `example.com/m.Box[int]`.
                let decl = t.split('[').next().unwrap_or(&t);
                for u in fields_of.get(decl).into_iter().flatten() {
                    if seen.insert(u.clone()) {
                        stack.push(u.clone());
                        self.add("makes", vec![s(f), s(u)]);
                    }
                }
            }
        }
    }

    /// `func` makes a value of `t` when `t` is a named type.
    fn makes(&mut self, m: &Module, func: &str, t: &Type) {
        if matches!(t, Type::Named { .. } | Type::Extern { .. }) {
            let id = self.ty(m, t);
            self.add("makes", vec![s(func), s(id)]);
        }
    }

    /// The id of `t`, with a `type` row for it and every type inside it.
    fn ty(&mut self, m: &Module, t: &Type) -> String {
        let l = label(m);
        let id = spell(l, t);
        if !self.types.insert(id.clone()) {
            return id;
        }
        let kind;
        let (mut name, mut path) = (String::new(), String::new());
        let (mut bits, mut signed, mut fields) = (0, false, 0usize);
        let (mut elem, mut keyt, mut under, mut decl) =
            (String::new(), String::new(), String::new(), "");
        match t {
            Type::Int { width, signed: sg } => {
                kind = "int";
                bits = width_bits(*width);
                signed = *sg;
            }
            Type::Float { bits: b } => {
                kind = "float";
                bits = match b {
                    FloatBits::W32 => 32,
                    FloatBits::W64 => 64,
                };
            }
            Type::Bool => kind = "bool",
            Type::String => kind = "string",
            Type::Error => kind = "error",
            Type::Any => kind = "any",
            Type::Unit => kind = "unit",
            Type::Named { name: n, targs } => {
                kind = "named";
                name = n.clone();
                path = l.to_string();
                for a in targs {
                    self.ty(m, a);
                }
                if let Some(d) = m.types.iter().find(|d| d.name() == n) {
                    match d {
                        TypeDecl::Struct { fields: fs, .. } => {
                            decl = "struct";
                            fields = fs.len();
                            for (k, f) in fs.iter().enumerate() {
                                let ft = self.ty(m, &f.t);
                                self.add("type_field", vec![s(&id), i(k), s(&f.name), s(ft)]);
                            }
                        }
                        TypeDecl::Newtype { under: u, .. } | TypeDecl::Enum { under: u, .. } => {
                            decl = if matches!(d, TypeDecl::Enum { .. }) {
                                "enum"
                            } else {
                                "newtype"
                            };
                            under = self.ty(m, u);
                            if let Some((b, sg)) = int_of(m, u, 0) {
                                bits = b;
                                signed = sg;
                            }
                        }
                        TypeDecl::Union { variants, .. } => {
                            decl = "union";
                            fields = variants.len();
                        }
                    }
                } else if m
                    .interfaces
                    .iter()
                    .any(|f| f.name == *n && (f.path.is_empty() || f.path == m.path))
                {
                    decl = "interface";
                }
            }
            Type::Ptr { elem: e } | Type::Slice { elem: e } | Type::Chan { elem: e, .. } => {
                kind = match t {
                    Type::Ptr { .. } => "ptr",
                    Type::Slice { .. } => "slice",
                    _ => "chan",
                };
                elem = self.ty(m, e);
            }
            Type::Array { len, elem: e } => {
                kind = "array";
                elem = self.ty(m, e);
                fields = usize::try_from(*len).unwrap_or(usize::MAX);
            }
            Type::Map { key: k, elem: e } => {
                kind = "map";
                keyt = self.ty(m, k);
                elem = self.ty(m, e);
            }
            Type::Tuple { elems } => {
                kind = "tuple";
                fields = elems.len();
                for (k, e) in elems.iter().enumerate() {
                    let et = self.ty(m, e);
                    self.add(
                        "type_elem",
                        vec![s(&id), i(k), s(et), Value::Bool(k + 1 == elems.len())],
                    );
                }
            }
            Type::Extern {
                path: p,
                name: n,
                targs,
            } => {
                kind = "extern";
                name = n.clone();
                path = p.clone();
                for a in targs {
                    self.ty(m, a);
                }
            }
            Type::Func { params, results } => {
                kind = "func";
                for x in params.iter().chain(results) {
                    self.ty(m, x);
                }
            }
            Type::Param { name: n } => {
                kind = "param";
                name = n.clone();
            }
        }
        self.add(
            "type",
            vec![
                s(&id),
                s(kind),
                s(name),
                s(path),
                Value::Int(bits),
                Value::Bool(signed),
                s(elem),
                s(keyt),
                i(fields),
                s(under),
                s(decl),
            ],
        );
        id
    }
}

/// The `effect` and `syscall` rows `codespec effects` gives the set: (function, relation, effect
/// or syscall, position of the call leading there).
fn effects(
    modules: &[Module],
    summaries: &[codespec_effects::Summary],
    barrier: Option<&codespec_effects::Barrier>,
) -> Vec<(String, &'static str, String, String)> {
    let opts = codespec_effects::Options {
        barrier,
        ..Default::default()
    };
    let report = codespec_effects::analyze_set(modules, summaries, &opts);
    let mut out = Vec::new();
    for fe in &report.funcs {
        let Some(m) = modules.iter().find(|m| {
            m.path == fe.path
                && (m
                    .funcs
                    .iter()
                    .any(|f| f.key() == fe.name && f.pos == fe.pos)
                    || m.opaque
                        .iter()
                        .any(|o| o.key() == fe.name && o.pos == fe.pos))
        }) else {
            continue;
        };
        let id = format!("{}.{}", label(m), fe.name);
        for (rel, sites) in [("effect", &fe.effect_sites), ("syscall", &fe.syscalls)] {
            for site in sites {
                for p in &site.at {
                    out.push((id.clone(), rel, site.name.clone(), at(m, p)));
                }
            }
        }
    }
    out
}

/// The integer type `t` is, a declared newtype or enum resolved: (bits, signed).
fn int_of(m: &Module, t: &Type, depth: usize) -> Option<(i64, bool)> {
    match t {
        Type::Int { width, signed } => Some((width_bits(*width), *signed)),
        Type::Named { name, .. } if depth < 8 => m.types.iter().find_map(|d| match d {
            TypeDecl::Newtype { name: n, under, .. } | TypeDecl::Enum { name: n, under, .. }
                if n == name =>
            {
                int_of(m, under, depth + 1)
            }
            _ => None,
        }),
        _ => None,
    }
}

/// A call to resolve against the functions of the set once all are known.
struct Link {
    site: String,
    path: String,
    recv: String,
    name: String,
}

/// One body's control flow, statement by statement.
#[derive(Default)]
struct Flow {
    ids: Vec<String>,
    index: HashMap<String, usize>,
    succ: Vec<BTreeSet<usize>>,
    exits: BTreeSet<usize>,
    /// The body's module path and the functions that raise (`Builder::raising`).
    own: String,
    raising: Rc<BTreeSet<(String, String, String)>>,
}

#[derive(Clone, Copy)]
enum Next {
    Node(usize),
    End,
}

impl Flow {
    fn node(&mut self, id: String) -> usize {
        if let Some(&n) = self.index.get(&id) {
            return n;
        }
        self.ids.push(id.clone());
        self.succ.push(BTreeSet::new());
        self.index.insert(id, self.ids.len() - 1);
        self.ids.len() - 1
    }

    fn edge(&mut self, a: usize, b: Next) {
        match b {
            Next::Node(b) => {
                self.succ[a].insert(b);
            }
            Next::End => {
                self.exits.insert(a);
            }
        }
    }

    /// The entry of `stmts` (at `base<i>`), each edge out of them added, control leaving them
    /// to `next`.
    fn seq(
        &mut self,
        func: &str,
        stmts: &[Stmt],
        base: &str,
        next: Next,
        brk: Option<Next>,
        cont: Option<Next>,
    ) -> Next {
        let mut next = next;
        for (k, st) in stmts.iter().enumerate().rev() {
            next = self.stmt(func, st, &format!("{base}{k}"), next, brk, cont);
        }
        next
    }

    fn stmt(
        &mut self,
        func: &str,
        st: &Stmt,
        path: &str,
        next: Next,
        brk: Option<Next>,
        cont: Option<Next>,
    ) -> Next {
        let me = self.node(format!("{func}#{path}"));
        // A call of a function that raises, outside a function literal, may leave the body at
        // this statement as a `raise` does (`codespec check`'s "may leave through").
        if calls_raising(st, &self.own, &self.raising) {
            self.edge(me, Next::End);
        }
        match st {
            Stmt::If {
                then, otherwise, ..
            } => {
                let t = self.seq(func, then, &format!("{path}.then."), next, brk, cont);
                let e = self.seq(func, otherwise, &format!("{path}.else."), next, brk, cont);
                self.edge(me, t);
                self.edge(me, e);
            }
            Stmt::ForRange { body, .. } => {
                let b = self.seq(
                    func,
                    body,
                    &format!("{path}.body."),
                    Next::Node(me),
                    Some(next),
                    Some(Next::Node(me)),
                );
                self.edge(me, b);
                self.edge(me, next);
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                let after = match post {
                    Some(_) => {
                        let p = self.node(format!("{func}#{path}.post"));
                        self.edge(p, Next::Node(me));
                        Next::Node(p)
                    }
                    None => Next::Node(me),
                };
                let b = self.seq(
                    func,
                    body,
                    &format!("{path}.body."),
                    after,
                    Some(next),
                    Some(after),
                );
                self.edge(me, b);
                if cond.is_some() {
                    self.edge(me, next);
                }
                if init.is_some() {
                    let n = self.node(format!("{func}#{path}.init"));
                    self.edge(n, Next::Node(me));
                    return Next::Node(n);
                }
            }
            Stmt::Switch { cases, .. } => {
                for (c, case) in cases.iter().enumerate() {
                    let b = self.seq(
                        func,
                        &case.body,
                        &format!("{path}.cases.{c}.body."),
                        next,
                        brk,
                        cont,
                    );
                    self.edge(me, b);
                }
                if !cases.iter().any(|c| c.values.is_empty()) {
                    self.edge(me, next);
                }
            }
            Stmt::Return { .. } | Stmt::Raise { .. } => self.edge(me, Next::End),
            Stmt::Break { .. } => self.edge(me, brk.unwrap_or(Next::End)),
            Stmt::Continue { .. } => self.edge(me, cont.unwrap_or(Next::End)),
            Stmt::LookupElse { then, .. } => {
                let t = self.seq(func, then, &format!("{path}.then."), next, brk, cont);
                self.edge(me, t);
                self.edge(me, next);
            }
            _ => self.edge(me, next),
        }
        Next::Node(me)
    }
}

/// Whether `st` itself (its nested blocks and function literals not included) calls, or uses
/// as a value, a function of `raising`; a `foreign` statement by its call list.
fn calls_raising(st: &Stmt, own: &str, raising: &BTreeSet<(String, String, String)>) -> bool {
    if raising.is_empty() {
        return false;
    }
    let path = |p: &str| {
        if p.is_empty() {
            own.to_string()
        } else {
            p.to_string()
        }
    };
    let has = |p: String, r: &str, n: &str| raising.contains(&(p, r.to_string(), n.to_string()));
    if let Stmt::Foreign {
        calls: Some(list), ..
    } = st
    {
        return list.iter().any(|c| match c {
            CallRef::Func { path: p, name, .. } => has(path(p), "", name),
            CallRef::Method {
                path: p,
                recv,
                name,
                ..
            }
            | CallRef::Ref {
                path: p,
                recv,
                name,
                ..
            } => has(path(p), recv, name),
            _ => false,
        });
    }
    fn each(e: &Expr, hit: &mut dyn FnMut(&Expr) -> bool, found: &mut bool) {
        if *found || matches!(e, Expr::Lambda { .. }) {
            return;
        }
        *found |= hit(e);
        codespec_model::walk::children(e, &mut |c| each(c, hit, found));
    }
    let rt = |e: &Expr| codespec_model::receiver_type_name(e.t()).to_string();
    let mut hit = |e: &Expr| match e {
        Expr::Call { name, .. } => has(own.to_string(), "", name),
        Expr::Method { x, name, .. } => has(own.to_string(), &rt(x), name),
        Expr::Extern { path: p, name, .. } => has(path(p), "", name),
        Expr::ExternMethod {
            path: p,
            type_name,
            name,
            ..
        } => has(path(p), type_name, name),
        Expr::FuncRef { name, recv, .. } => has(
            own.to_string(),
            &recv.as_ref().map(|r| rt(r)).unwrap_or_default(),
            name,
        ),
        _ => false,
    };
    let mut found = false;
    codespec_model::walk::stmt_exprs(st, &mut |e| each(e, &mut hit, &mut found));
    found
}

/// A set of small indices.
#[derive(Clone, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn empty(n: usize) -> Bits {
        Bits(vec![0; n.div_ceil(64)])
    }
    fn full(n: usize) -> Bits {
        let mut b = Bits::empty(n);
        for k in 0..n {
            b.set(k);
        }
        b
    }
    fn set(&mut self, k: usize) {
        self.0[k / 64] |= 1 << (k % 64);
    }
    fn unset(&mut self, k: usize) {
        self.0[k / 64] &= !(1 << (k % 64));
    }
    fn has(&self, k: usize) -> bool {
        self.0[k / 64] & (1 << (k % 64)) != 0
    }
    fn and(&mut self, o: &Bits) {
        for (a, b) in self.0.iter_mut().zip(&o.0) {
            *a &= b;
        }
    }
    fn or(&mut self, o: &Bits) {
        for (a, b) in self.0.iter_mut().zip(&o.0) {
            *a |= b;
        }
    }
    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.0.len() * 64).filter(|&k| self.has(k))
    }
}

/// The dominators of every node reachable from `start` over `preds`, as (dominator, node).
fn dominators(
    n: usize,
    start: &[usize],
    preds: &[Vec<usize>],
    succ: &[Vec<usize>],
) -> Vec<(usize, usize)> {
    let mut reach = vec![false; n];
    let mut stack: Vec<usize> = start.to_vec();
    while let Some(k) = stack.pop() {
        if !reach[k] {
            reach[k] = true;
            stack.extend(&succ[k]);
        }
    }
    let mut dom: Vec<Bits> = (0..n).map(|_| Bits::full(n)).collect();
    for &k in start {
        dom[k] = Bits::empty(n);
        dom[k].set(k);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for k in (0..n).filter(|&k| reach[k] && !start.contains(&k)) {
            let mut d = Bits::full(n);
            for &p in preds[k].iter().filter(|&&p| reach[p]) {
                d.and(&dom[p]);
            }
            d.set(k);
            if d != dom[k] {
                dom[k] = d;
                changed = true;
            }
        }
    }
    let mut out = Vec::new();
    for k in (0..n).filter(|&k| reach[k]) {
        for a in dom[k].iter().filter(|&a| a < n) {
            out.push((a, k));
        }
    }
    out
}

/// What one body defines and reads, for its reaching definitions.
#[derive(Default)]
struct Locals {
    /// (var, node): the definitions, parameters' at the body's own id.
    defs: Vec<(String, String)>,
    /// (var, use expr, statement holding it)
    uses: Vec<(String, String, String)>,
}

/// One scope of locals: a function body or a function literal's, with what the literal
/// captures from the scope around it.
struct Scope {
    owner: String,
    captures: Vec<String>,
}

struct Walker<'a> {
    b: &'a mut Builder,
    m: &'a Module,
    links: &'a mut Vec<Link>,
    /// The declared function holding the body.
    func: String,
    ancestors: Vec<String>,
    guards: Vec<(String, bool)>,
    scopes: Vec<Scope>,
    locals: BTreeMap<String, Locals>,
    /// The position of every statement walked so far, by id.
    positions: HashMap<String, String>,
}

impl Walker<'_> {
    fn label(&self) -> String {
        label(self.m).to_string()
    }

    fn own(&self, path: &str) -> String {
        if path.is_empty() {
            self.m.path.clone()
        } else {
            path.to_string()
        }
    }

    fn var(&self, name: &str) -> String {
        for sc in self.scopes.iter().rev() {
            if !sc.captures.iter().any(|c| c == name) {
                return format!("{}${name}", sc.owner);
            }
        }
        format!("{}${name}", self.func)
    }

    fn owner(&self) -> String {
        self.scopes
            .last()
            .map_or_else(|| self.func.clone(), |s| s.owner.clone())
    }

    fn def(&mut self, name: &str, node: &str) {
        if name == "_" || name.is_empty() {
            return;
        }
        let v = self.var(name);
        self.b.add("def", vec![s(&v), s(node)]);
        let owner = self.owner();
        self.locals
            .entry(owner)
            .or_default()
            .defs
            .push((v, node.to_string()));
    }

    /// Rows every node gets: what contains it, what guards it.
    fn placed(&mut self, id: &str) {
        for a in &self.ancestors {
            self.b.add("contains", vec![s(a), s(id)]);
        }
        for (c, branch) in &self.guards {
            self.b
                .add("guards", vec![s(c), s(id), Value::Bool(*branch)]);
        }
    }

    fn params(
        &mut self,
        owner: &str,
        recv: Option<&codespec_model::Param>,
        params: &[codespec_model::Param],
        results: &[Type],
    ) {
        if let Some(r) = recv {
            let t = self.b.ty(self.m, &r.t);
            self.b.add("param", vec![s(owner), i(-1), s(&r.name), s(t)]);
            self.def(&r.name, owner);
        }
        for (k, p) in params.iter().enumerate() {
            let t = self.b.ty(self.m, &p.t);
            self.b.add("param", vec![s(owner), i(k), s(&p.name), s(t)]);
            self.def(&p.name, owner);
        }
        for (k, r) in results.iter().enumerate() {
            let t = self.b.ty(self.m, r);
            self.b.add("result", vec![s(owner), i(k), s(""), s(t)]);
        }
    }

    /// A body: its statements, then its control flow and reaching definitions.
    fn body(&mut self, owner: &str, stmts: &[Stmt], base: &str, parent: &str) {
        for (k, st) in stmts.iter().enumerate() {
            self.stmt(st, &format!("{base}{k}"), parent);
        }
        let mut flow = Flow {
            own: label(self.m).to_string(),
            raising: self.b.raising.clone(),
            ..Flow::default()
        };
        let entry = flow.seq(&self.func.clone(), stmts, base, Next::End, None, None);
        let n = flow.ids.len();
        let succ: Vec<Vec<usize>> = flow
            .succ
            .iter()
            .map(|s| s.iter().copied().collect())
            .collect();
        let mut preds = vec![Vec::new(); n + 1];
        for (a, ss) in succ.iter().enumerate() {
            for &b2 in ss {
                self.b
                    .add("cfg_next", vec![s(&flow.ids[a]), s(&flow.ids[b2])]);
                preds[b2].push(a);
            }
        }
        for &x in &flow.exits {
            self.b.add("exit", vec![s(owner), s(&flow.ids[x])]);
        }
        let Next::Node(entry) = entry else { return };
        self.b.add("entry", vec![s(owner), s(&flow.ids[entry])]);
        for (a, b2) in dominators(n, &[entry], &preds[..n], &succ) {
            self.b
                .add("dominates", vec![s(&flow.ids[a]), s(&flow.ids[b2])]);
        }
        // Post-dominators: the dominators of the reversed graph from a node after every exit.
        let mut rsucc: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
        let mut rpreds: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
        for (a, ss) in succ.iter().enumerate() {
            for &b2 in ss {
                rsucc[b2].push(a);
                rpreds[a].push(b2);
            }
        }
        for &x in &flow.exits {
            rsucc[n].push(x);
            rpreds[x].push(n);
        }
        for (a, b2) in dominators(n + 1, &[n], &rpreds, &rsucc) {
            if a < n && b2 < n {
                self.b
                    .add("postdominates", vec![s(&flow.ids[a]), s(&flow.ids[b2])]);
            }
        }
        self.reaching(owner, &flow, entry, &preds[..n]);
    }

    /// `flows` rows from the reaching definitions of one body.
    fn reaching(&mut self, owner: &str, flow: &Flow, entry: usize, preds: &[Vec<usize>]) {
        let Some(locals) = self.locals.remove(owner) else {
            return;
        };
        let n = flow.ids.len();
        let d = locals.defs.len();
        if d == 0 {
            return;
        }
        let mut gen_ = vec![Bits::empty(d); n];
        let mut kill = vec![Bits::empty(d); n];
        let mut at_entry = Bits::empty(d);
        for (k, (v, node)) in locals.defs.iter().enumerate() {
            match flow.index.get(node) {
                Some(&x) => gen_[x].set(k),
                None => at_entry.set(k),
            }
            for (j, (w, _)) in locals.defs.iter().enumerate() {
                if w == v
                    && j != k
                    && let Some(&x) = flow.index.get(node)
                {
                    kill[x].set(j);
                }
            }
        }
        let mut input = vec![Bits::empty(d); n];
        let mut output = vec![Bits::empty(d); n];
        let mut changed = true;
        while changed {
            changed = false;
            for x in 0..n {
                let mut inx = if x == entry {
                    at_entry.clone()
                } else {
                    Bits::empty(d)
                };
                for &p in &preds[x] {
                    inx.or(&output[p]);
                }
                let mut out = inx.clone();
                for j in kill[x].iter().filter(|&j| j < d) {
                    out.unset(j);
                }
                out.or(&gen_[x]);
                if inx != input[x] || out != output[x] {
                    input[x] = inx;
                    output[x] = out;
                    changed = true;
                }
            }
        }
        for (v, e, st) in &locals.uses {
            let Some(&x) = flow.index.get(st) else {
                continue;
            };
            for k in input[x].iter().filter(|&k| k < d) {
                if locals.defs[k].0 == *v {
                    self.b.add("flows", vec![s(&locals.defs[k].1), s(e)]);
                }
            }
        }
    }

    fn stmt(&mut self, st: &Stmt, path: &str, parent: &str) {
        let id = format!("{}#{path}", self.func);
        let pos = at(self.m, stmt_pos(st));
        self.positions.insert(id.clone(), pos.clone());
        let op = match st {
            Stmt::Assign { op, .. } => op.as_str(),
            _ => "",
        };
        self.b.add(
            "stmt",
            vec![
                s(&id),
                s(&self.func),
                s(stmt_kind(st)),
                s(op),
                s(&pos),
                Value::Int(line(stmt_pos(st))),
                s(parent),
            ],
        );
        self.placed(&id);
        self.ancestors.push(id.clone());
        let e = |w: &mut Self, x: &Expr, slot: &str, k: Option<usize>| {
            let p = match k {
                Some(k) => format!("{path}.{slot}.{k}"),
                None => format!("{path}.{slot}"),
            };
            w.expr(x, &p, &id, &id, slot, k);
        };
        match st {
            Stmt::Let { names, rhs, .. } => {
                for (k, n) in names.iter().enumerate() {
                    self.b.add(
                        "let_name",
                        vec![s(&id), i(k), s(n), Value::Bool(k + 1 == names.len())],
                    );
                }
                for (k, x) in rhs.iter().enumerate() {
                    e(self, x, "rhs", Some(k));
                }
                for n in names {
                    self.def(n, &id);
                }
            }
            Stmt::Var { name, t, .. } => {
                self.b.ty(self.m, t);
                self.b.makes(self.m, &self.func, t);
                self.def(name, &id);
            }
            Stmt::Assign { op, lhs, rhs, .. } => {
                for (k, x) in lhs.iter().enumerate() {
                    let target = op.is_empty() && matches!(x, Expr::Var { .. });
                    let p = format!("{path}.lhs.{k}");
                    self.expr_as(x, &p, &id, &id, "lhs", Some(k), !target);
                }
                for (k, x) in rhs.iter().enumerate() {
                    e(self, x, "rhs", Some(k));
                }
                for x in lhs {
                    if let Expr::Var { name, .. } = x {
                        self.def(name, &id);
                    }
                }
            }
            Stmt::Incr { x, .. } | Stmt::Decr { x, .. } => {
                e(self, x, "x", None);
                if let Expr::Var { name, .. } = x {
                    self.def(name, &id);
                }
            }
            Stmt::Expr { x, .. } | Stmt::Raise { x, .. } => e(self, x, "x", None),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                e(self, cond, "cond", None);
                let c = format!("{id}.cond");
                self.guards.push((c.clone(), true));
                self.body_stmts(then, &format!("{path}.then."), &id);
                self.guards.pop();
                self.guards.push((c, false));
                self.body_stmts(otherwise, &format!("{path}.else."), &id);
                self.guards.pop();
            }
            Stmt::ForRange {
                key: k,
                val,
                over,
                body,
                ..
            } => {
                e(self, over, "over", None);
                self.def(k, &id);
                self.def(val, &id);
                self.body_stmts(body, &format!("{path}.body."), &id);
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                if let Some(init) = init {
                    self.stmt(init, &format!("{path}.init"), &id);
                }
                if let Some(c) = cond {
                    e(self, c, "cond", None);
                    self.guards.push((format!("{id}.cond"), true));
                }
                self.body_stmts(body, &format!("{path}.body."), &id);
                if let Some(post) = post {
                    self.stmt(post, &format!("{path}.post"), &id);
                }
                if cond.is_some() {
                    self.guards.pop();
                }
            }
            Stmt::Switch { tag, cases, .. } => {
                if let Some(t) = tag {
                    e(self, t, "tag", None);
                }
                for (c, case) in cases.iter().enumerate() {
                    for (k, v) in case.values.iter().enumerate() {
                        let p = format!("{path}.cases.{c}.values.{k}");
                        self.expr(v, &p, &id, &id, "values", Some(k));
                    }
                    self.body_stmts(&case.body, &format!("{path}.cases.{c}.body."), &id);
                }
            }
            Stmt::Return { results, .. } => {
                for (k, x) in results.iter().enumerate() {
                    e(self, x, "results", Some(k));
                }
            }
            Stmt::Send { chan, x, .. } => {
                e(self, chan, "chan", None);
                e(self, x, "x", None);
            }
            Stmt::LookupElse {
                name,
                map,
                index,
                then,
                ..
            } => {
                e(self, map, "map", None);
                e(self, index, "index", None);
                self.def(name, &id);
                self.body_stmts(then, &format!("{path}.then."), &id);
            }
            Stmt::Foreign {
                lang,
                reason,
                calls,
                ..
            } => {
                self.b.add(
                    "foreign",
                    vec![s(&id), s(lang), s(reason), Value::Bool(calls.is_some())],
                );
                match calls {
                    Some(list) => self.call_list(list, &format!("{id}.calls.")),
                    None => {
                        let site = self.b.unknown(
                            &self.func,
                            &id,
                            pos.clone(),
                            format!("{FOREIGN}{reason}"),
                        );
                        let f = s(&self.func);
                        self.b
                            .add_wild("call", &[("site", s(&id)), ("func", f.clone())], site);
                        self.b.add_wild("call_by", &[("site", s(&id))], site);
                        self.b.add_wild("calls", &[("site", s(&id))], site);
                        self.b
                            .add_wild("expr", &[("func", f.clone()), ("stmt", s(&id))], site);
                        self.b.add_wild("makes", &[("func", f)], site);
                    }
                }
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
        self.ancestors.pop();
    }

    fn body_stmts(&mut self, stmts: &[Stmt], base: &str, parent: &str) {
        for (k, st) in stmts.iter().enumerate() {
            self.stmt(st, &format!("{base}{k}"), parent);
        }
    }

    /// A call list's calls, a function literal's included, at `<base><k>`.
    fn call_list(&mut self, calls: &[CallRef], base: &str) {
        for (k, c) in calls.iter().enumerate() {
            let site = format!("{base}{k}");
            let (kind, path, recv, name) = match c {
                CallRef::Func { path, name, .. } => {
                    ("func", self.own(path), String::new(), name.clone())
                }
                CallRef::Method {
                    path,
                    recv,
                    name,
                    by,
                    ..
                } => {
                    if !by.is_empty() {
                        self.b.add("call_by", vec![s(&site), s(by)]);
                    }
                    ("method", self.own(path), recv.clone(), name.clone())
                }
                CallRef::Iface {
                    iface,
                    name,
                    methods,
                    ..
                } => {
                    self.b
                        .ifaces
                        .entry(iface.clone())
                        .or_default()
                        .extend(methods.iter().map(|x| (x.name.clone(), x.sig.clone())));
                    let (p, r) = iface.rsplit_once('.').unwrap_or(("", iface));
                    ("iface", self.own(p), r.to_string(), name.clone())
                }
                CallRef::Value { .. } => ("value", String::new(), String::new(), String::new()),
                CallRef::Ref {
                    path, recv, name, ..
                } => ("ref", self.own(path), recv.clone(), name.clone()),
                CallRef::Named {
                    form, path, name, ..
                } => {
                    let kind = if form == "macro" { "macro" } else { "named" };
                    (kind, path.clone(), String::new(), name.clone())
                }
                CallRef::Lambda { calls, .. } => {
                    self.call_list(calls, &format!("{site}.calls."));
                    continue;
                }
                CallRef::New { path, name, .. } => {
                    let p = if path.is_empty() {
                        label(self.m).to_string()
                    } else {
                        path.clone()
                    };
                    self.b
                        .add("makes", vec![s(&self.func), s(key(&p, "", name))]);
                    continue;
                }
            };
            let pos = at(self.m, c.pos());
            self.call_row(
                &site,
                kind,
                &path,
                &recv,
                &name,
                "",
                0,
                &pos,
                line(c.pos()),
                true,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn call_row(
        &mut self,
        site: &str,
        kind: &'static str,
        path: &str,
        recv: &str,
        name: &str,
        recv_expr: &str,
        args: usize,
        pos: &str,
        ln: i64,
        listed: bool,
    ) {
        let callee = if kind == "value" {
            String::new()
        } else {
            key(path, recv, name)
        };
        let ty = if recv.is_empty() {
            String::new()
        } else {
            key(path, recv, "")
        };
        self.b.add(
            "call",
            vec![
                s(site),
                s(&self.func),
                s(kind),
                s(callee),
                s(path),
                s(ty),
                s(name),
                s(recv_expr),
                i(args),
                s(pos),
                Value::Int(ln),
                Value::Bool(listed),
            ],
        );
        match kind {
            // A call of the module's own function links within the module: by its label, as
            // the index holds it, where the module has no path.
            "func" | "method" | "ref" => self.links.push(Link {
                site: site.to_string(),
                path: if path.is_empty() {
                    label(self.m).to_string()
                } else {
                    path.to_string()
                },
                recv: recv.to_string(),
                name: name.to_string(),
            }),
            // A macro invocation is no call of a function: the calls of its arguments are
            // listed as calls of their own.
            "macro" => {}
            _ => {
                let reason = if kind == "value" {
                    "call of a function value, not linked by calls".to_string()
                } else if kind == "named" {
                    format!(
                        "call of {name}, named as written and not resolved by the reader (a method on a value of a type it did not know, or a function by its name alone), not linked by calls"
                    )
                } else {
                    format!(
                        "call through the interface {}, not linked by calls",
                        key(path, recv, "")
                    )
                };
                let k = self.b.unknown(&self.func, site, pos.to_string(), reason);
                self.b.add_wild("calls", &[("site", s(site))], k);
            }
        }
    }

    fn expr(
        &mut self,
        x: &Expr,
        path: &str,
        stmt: &str,
        parent: &str,
        slot: &str,
        k: Option<usize>,
    ) {
        self.expr_as(x, path, stmt, parent, slot, k, true);
    }

    #[allow(clippy::too_many_arguments)]
    fn expr_as(
        &mut self,
        x: &Expr,
        path: &str,
        stmt: &str,
        parent: &str,
        slot: &str,
        k: Option<usize>,
        read: bool,
    ) {
        let id = format!("{}#{path}", self.func);
        let t = self.b.ty(self.m, x.t());
        match x {
            Expr::Struct { .. }
            | Expr::Conv { .. }
            | Expr::Zero { .. }
            | Expr::Lit { .. }
            | Expr::Const { .. } => self.b.makes(self.m, &self.func, x.t()),
            Expr::ToError { x: a, .. } => self.b.makes(self.m, &self.func, a.t()),
            _ => {}
        }
        let (mut op, mut name, mut value) = ("", String::new(), String::new());
        let (mut xid, mut yid) = (String::new(), String::new());
        let sub = |field: &str| format!("{path}.{field}");
        let child = |field: &str| format!("{}#{path}.{field}", self.func);
        // (operand, its path, slot, index)
        let mut kids: Vec<(&Expr, String, &str, Option<usize>)> = Vec::new();
        match x {
            Expr::Lit { value: v, .. } | Expr::Float { value: v, .. } => value = v.clone(),
            Expr::Var { name: n, .. }
            | Expr::Const { name: n, .. }
            | Expr::Errvar { name: n, .. }
            | Expr::Global { name: n, .. } => name = n.clone(),
            Expr::Field { x: a, name: n, .. } => {
                name = n.clone();
                xid = child("x");
                kids.push((a, sub("x"), "x", None));
            }
            Expr::Index { x: a, index, .. } => {
                xid = child("x");
                yid = child("index");
                kids.push((a, sub("x"), "x", None));
                kids.push((index, sub("index"), "index", None));
            }
            Expr::Call { name: n, args, .. }
            | Expr::Intrinsic { name: n, args, .. }
            | Expr::Extern { name: n, args, .. } => {
                name = n.clone();
                for (j, a) in args.iter().enumerate() {
                    kids.push((a, format!("{path}.args.{j}"), "args", Some(j)));
                }
            }
            Expr::Method {
                x: a,
                name: n,
                args,
                ..
            } => {
                name = n.clone();
                xid = child("x");
                kids.push((a, sub("x"), "x", None));
                for (j, a) in args.iter().enumerate() {
                    kids.push((a, format!("{path}.args.{j}"), "args", Some(j)));
                }
            }
            Expr::ExternMethod {
                name: n,
                recv,
                args,
                ..
            }
            | Expr::IfaceCall {
                name: n,
                recv,
                args,
                ..
            } => {
                name = n.clone();
                xid = child("recv");
                kids.push((recv, sub("recv"), "recv", None));
                for (j, a) in args.iter().enumerate() {
                    kids.push((a, format!("{path}.args.{j}"), "args", Some(j)));
                }
            }
            Expr::Binop { op: o, x: a, y, .. } => {
                op = o;
                xid = child("x");
                yid = child("y");
                kids.push((a, sub("x"), "x", None));
                kids.push((y, sub("y"), "y", None));
            }
            Expr::Unop { op: o, x: a, .. } => {
                op = o;
                xid = child("x");
                kids.push((a, sub("x"), "x", None));
            }
            Expr::Conv { name: n, x: a, .. } => {
                name = n.clone();
                xid = child("x");
                kids.push((a, sub("x"), "x", None));
            }
            Expr::ToError { x: a, .. } | Expr::Spread { x: a, .. } | Expr::ToIface { x: a, .. } => {
                xid = child("x");
                kids.push((a, sub("x"), "x", None));
            }
            Expr::Struct {
                name: n, fields, ..
            } => {
                name = n.clone();
                for (j, fi) in fields.iter().enumerate() {
                    let vid = child(&format!("fields.{j}"));
                    self.b
                        .add("field_init", vec![s(&id), i(j), s(&fi.name), s(vid)]);
                    kids.push((&fi.value, format!("{path}.fields.{j}"), "fields", Some(j)));
                }
            }
            Expr::FuncRef { name: n, recv, .. } => {
                name = n.clone();
                if let Some(r) = recv {
                    xid = child("recv");
                    kids.push((r, sub("recv"), "recv", None));
                }
            }
            Expr::CallValue { f, args, .. } => {
                xid = child("f");
                kids.push((f, sub("f"), "f", None));
                for (j, a) in args.iter().enumerate() {
                    kids.push((a, format!("{path}.args.{j}"), "args", Some(j)));
                }
            }
            Expr::Make { .. } | Expr::Zero { .. } | Expr::Nil { .. } | Expr::Lambda { .. } => {}
        }
        self.b.add(
            "expr",
            vec![
                s(&id),
                s(&self.func),
                s(expr_kind(x)),
                s(&t),
                s(stmt),
                s(parent),
                s(slot),
                Value::Int(k.map_or(-1, |k| k as i64)),
                s(op),
                s(&name),
                s(value),
                s(xid),
                s(yid),
            ],
        );
        self.placed(&id);
        if let Expr::Var { name: n, .. } = x {
            let v = self.var(n);
            self.b.add("value", vec![s(&id), s(&v)]);
            if read {
                self.b.add("use", vec![s(&v), s(&id)]);
                let owner = self.owner();
                self.locals
                    .entry(owner)
                    .or_default()
                    .uses
                    .push((v, id.clone(), stmt.to_string()));
            }
        }
        self.calls_of(x, &id, stmt);
        self.ancestors.push(id.clone());
        for (a, p, sl, j) in kids {
            self.expr(a, &p, stmt, &id, sl, j);
        }
        if let Expr::Lambda {
            params,
            results,
            captures,
            body,
            ..
        } = x
        {
            let st_pos = self.stmt_pos_of(stmt);
            self.b.add(
                "func",
                vec![
                    s(&id),
                    s(self.label()),
                    s(""),
                    s(""),
                    Value::Bool(false),
                    s("lambda"),
                    s(&st_pos),
                    Value::Int(line(&st_pos)),
                ],
            );
            self.scopes.push(Scope {
                owner: id.clone(),
                captures: captures.clone(),
            });
            self.params(&id, None, params, results);
            let guards = std::mem::take(&mut self.guards);
            self.body(&id, body, &format!("{path}.body."), &id);
            self.guards = guards;
            self.scopes.pop();
        }
        self.ancestors.pop();
    }

    fn stmt_pos_of(&self, stmt: &str) -> String {
        self.positions.get(stmt).cloned().unwrap_or_default()
    }

    /// The call row of a call expression.
    fn calls_of(&mut self, x: &Expr, id: &str, stmt: &str) {
        let l = self.m.path.clone();
        let rt = |e: &Expr| codespec_model::receiver_type_name(e.t()).to_string();
        let (kind, path, recv, name, recv_expr, args) = match x {
            Expr::Call { name, args, .. } => (
                "func",
                l,
                String::new(),
                name.clone(),
                String::new(),
                args.len(),
            ),
            Expr::Method {
                x: r, name, args, ..
            } => (
                "method",
                l,
                rt(r),
                name.clone(),
                format!("{id}.x"),
                args.len(),
            ),
            Expr::Extern {
                path, name, args, ..
            } => (
                "func",
                self.own(path),
                String::new(),
                name.clone(),
                String::new(),
                args.len(),
            ),
            Expr::ExternMethod {
                path,
                type_name,
                name,
                args,
                ..
            } => (
                "method",
                self.own(path),
                type_name.clone(),
                name.clone(),
                format!("{id}.recv"),
                args.len(),
            ),
            Expr::IfaceCall {
                path,
                iface,
                name,
                args,
                ..
            } => (
                "iface",
                self.own(path),
                iface.clone(),
                name.clone(),
                format!("{id}.recv"),
                args.len(),
            ),
            Expr::CallValue { args, .. } => (
                "value",
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                args.len(),
            ),
            Expr::FuncRef { name, recv, .. } => (
                "ref",
                l,
                recv.as_ref().map(|r| rt(r)).unwrap_or_default(),
                name.clone(),
                if recv.is_some() {
                    format!("{id}.recv")
                } else {
                    String::new()
                },
                0,
            ),
            _ => return,
        };
        let pos = self.stmt_pos_of(stmt);
        let ln = line(&pos);
        self.call_row(
            id, kind, &path, &recv, &name, &recv_expr, args, &pos, ln, false,
        );
    }
}

impl FactBase {
    /// The fact base of `modules`, read as one program, with no effect summaries and no barrier.
    /// The order of the modules does not matter.
    pub fn build(modules: &[Module]) -> FactBase {
        FactBase::build_with(modules, &[], None)
    }

    /// The fact base of `modules`, `effect` and `syscall` from `summaries` and `barrier` as
    /// `codespec effects` reads them.
    pub fn build_with(
        modules: &[Module],
        summaries: &[codespec_effects::Summary],
        barrier: Option<&codespec_effects::Barrier>,
    ) -> FactBase {
        let mut order: Vec<&Module> = modules.iter().collect();
        order.sort_by(|a, b| (label(a), &a.name).cmp(&(label(b), &b.name)));
        let mut b = Builder {
            raising: Rc::new(
                modules
                    .iter()
                    .flat_map(|m| {
                        m.funcs.iter().filter(|f| !f.raises.is_empty()).map(|f| {
                            let recv = f
                                .recv
                                .as_ref()
                                .map(|r| codespec_model::receiver_type_name(&r.t).to_string());
                            (
                                label(m).to_string(),
                                recv.unwrap_or_default(),
                                f.name.clone(),
                            )
                        })
                    })
                    .collect(),
            ),
            ..Builder::default()
        };
        let mut links = Vec::new();
        let mut index: HashMap<(String, String, String), String> = HashMap::new();
        let mut has: BTreeSet<(String, &str)> = BTreeSet::new();
        let mut method_sets: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
        for m in &order {
            let l = label(m).to_string();
            let lang = m.reader.language.clone();
            b.add(
                "module",
                vec![
                    s(&l),
                    s(&m.name),
                    s(&lang),
                    s(&m.reader.name),
                    s(&m.reader.version),
                ],
            );
            has.insert((lang.clone(), "bodies"));
            if !m.externs.is_empty() {
                has.insert((lang.clone(), "externs"));
            }
            if !m.methods.is_empty() {
                has.insert((lang.clone(), "dispatch"));
            }
            for d in &m.types {
                b.ty(
                    m,
                    &Type::Named {
                        name: d.name().to_string(),
                        targs: Vec::new(),
                    },
                );
            }
            for f in &m.interfaces {
                if f.path.is_empty() || f.path == m.path {
                    b.ty(
                        m,
                        &Type::Named {
                            name: f.name.clone(),
                            targs: Vec::new(),
                        },
                    );
                }
                let id = key(if f.path.is_empty() { &l } else { &f.path }, "", &f.name);
                let methods: Vec<(String, String)> = f
                    .methods
                    .iter()
                    .map(|x| {
                        let sig = go_sig(
                            m,
                            &Type::Func {
                                params: x.params.clone(),
                                results: x.results.clone(),
                            },
                        );
                        (x.name.clone(), sig)
                    })
                    .collect();
                b.ifaces.entry(id).or_default().extend(methods);
            }
            for f in &m.funcs {
                let id = format!("{l}.{}", f.key());
                let recv = f
                    .recv
                    .as_ref()
                    .map(|r| codespec_model::receiver_type_name(&r.t).to_string())
                    .unwrap_or_default();
                index
                    .entry((label(m).to_string(), recv.clone(), f.name.clone()))
                    .or_insert_with(|| id.clone());
                let kind = if f.recv.is_some() {
                    "method"
                } else if f.name == "init" {
                    "init"
                } else {
                    "func"
                };
                b.add(
                    "func",
                    vec![
                        s(&id),
                        s(&l),
                        s(&f.name),
                        s(&recv),
                        Value::Bool(f.exported),
                        s(kind),
                        s(at(m, &f.pos)),
                        Value::Int(line(&f.pos)),
                    ],
                );
                if !recv.is_empty() {
                    b.add("makes", vec![s(&id), s(key(&l, "", &recv))]);
                }
                let mut w = Walker {
                    b: &mut b,
                    m,
                    links: &mut links,
                    func: id.clone(),
                    ancestors: Vec::new(),
                    guards: Vec::new(),
                    scopes: Vec::new(),
                    locals: BTreeMap::new(),
                    positions: HashMap::new(),
                };
                w.params(&id, f.recv.as_ref(), &f.params, &f.results);
                for t in &f.raises {
                    let t = w.b.ty(m, t);
                    w.b.add("raises", vec![s(&id), s(t)]);
                }
                w.body(&id, &f.body, "", &id);
                if f.body
                    .iter()
                    .any(|st| matches!(st, Stmt::Foreign { calls: Some(_), .. }))
                {
                    has.insert((lang.clone(), "calls"));
                }
            }
            for o in &m.opaque {
                let id = format!("{l}.{}", o.key());
                let recv = o.recv.clone().unwrap_or_default();
                index
                    .entry((label(m).to_string(), recv.clone(), o.name.clone()))
                    .or_insert_with(|| id.clone());
                b.add(
                    "func",
                    vec![
                        s(&id),
                        s(&l),
                        s(&o.name),
                        s(&recv),
                        Value::Bool(o.name.starts_with(|c: char| c.is_uppercase())),
                        s("opaque"),
                        s(at(m, &o.pos)),
                        Value::Int(line(&o.pos)),
                    ],
                );
                if !recv.is_empty() {
                    b.add("makes", vec![s(&id), s(key(&l, "", &recv))]);
                }
                let site = b.unknown(&id, &id, at(m, &o.pos), format!("{OPAQUE}{}", o.reason));
                for rel in [
                    "stmt", "expr", "param", "result", "raises", "entry", "exit", "makes",
                ] {
                    b.add_wild(rel, &[("func", s(&id))], site);
                }
                b.add_wild("contains", &[("outer", s(&id))], site);
                b.add_wild("call", &[("site", s(&id)), ("func", s(&id))], site);
                b.add_wild("call_by", &[("site", s(&id))], site);
                b.add_wild("calls", &[("site", s(&id))], site);
                if !o.calls.is_empty() {
                    has.insert((lang.clone(), "calls"));
                }
                let mut w = Walker {
                    b: &mut b,
                    m,
                    links: &mut links,
                    func: id.clone(),
                    ancestors: Vec::new(),
                    guards: Vec::new(),
                    scopes: Vec::new(),
                    locals: BTreeMap::new(),
                    positions: HashMap::new(),
                };
                w.call_list(&o.calls, &format!("{id}#calls."));
            }
            for r in &m.refusals {
                b.add(
                    "refusal",
                    vec![s(&l), s(at(m, &r.pos)), s(&r.what), s(&r.reason)],
                );
            }
            let facts = match &m.facts {
                Some(f) => f.clone(),
                None => codespec_passes::facts(m),
            };
            for (kind, items) in [
                ("shared", &facts.shared),
                ("messages", &facts.messages),
                ("comma_ok", &facts.comma_ok),
                ("fallible", &facts.fallible),
                ("read_only", &facts.read_only),
                ("displays", &facts.displays),
            ] {
                for item in items {
                    b.add("fact", vec![s(kind), s(&l), s(key(&l, "", item))]);
                }
            }
            for p in &facts.unordered_iteration {
                b.add("fact", vec![s("unordered_iteration"), s(&l), s(at(m, p))]);
            }
            for ms in &m.methods {
                let t = key(&l, "", &ms.t);
                method_sets
                    .entry(t.clone())
                    .or_default()
                    .extend(ms.methods.iter().map(|x| (x.name.clone(), x.sig.clone())));
                for me in &ms.methods {
                    let target = match &me.target {
                        CallRef::Method {
                            path, recv, name, ..
                        } => key(if path.is_empty() { &m.path } else { path }, recv, name),
                        CallRef::Iface { iface, name, .. } => key(iface, "", name),
                        CallRef::Func { path, name, .. } => {
                            key(if path.is_empty() { &m.path } else { path }, "", name)
                        }
                        _ => String::new(),
                    };
                    b.add(
                        "method_set",
                        vec![s(&t), s(&me.name), s(&me.sig), s(target)],
                    );
                }
            }
        }
        for (lang, part) in has {
            b.add("reader_has", vec![s(lang), s(part)]);
        }
        for k in links {
            if let Some(target) = index.get(&(k.path, k.recv, k.name)) {
                b.add("calls", vec![s(k.site), s(target)]);
            }
        }
        // implements: the CHA test, every method of the interface in the type's method set.
        for (iface, methods) in &b.ifaces.clone() {
            if methods.is_empty() {
                continue;
            }
            for (t, set) in &method_sets {
                if methods.is_subset(set) {
                    b.add("implements", vec![s(t), s(iface)]);
                }
            }
        }
        b.fields_made(&order);
        b.scc();
        for (func, rel, name, pos) in effects(modules, summaries, barrier) {
            b.add(rel, vec![s(func), s(name), s(pos)]);
        }
        let opts = codespec_effects::Options {
            dispatch: codespec_effects::DispatchMode::Rta,
            ..Default::default()
        };
        if let Ok(report) = codespec_effects::reach(modules, &opts) {
            for mr in report.modules {
                let l = if mr.path.is_empty() { mr.name } else { mr.path };
                for k in mr.reached {
                    b.add("reached", vec![s(format!("{l}.{k}"))]);
                }
            }
        }
        FactBase {
            rels: b
                .rels
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            wild: b
                .wild
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            sites: b.sites,
        }
    }

    /// The facts of a stored relation, sorted.
    pub fn rows(&self, relation: &str) -> Vec<Vec<Value>> {
        self.rels.get(relation).cloned().unwrap_or_default()
    }

    /// The places the fact base cannot see, as the `unknown` relation numbers them.
    pub fn sites(&self) -> &[Site] {
        &self.sites
    }

    /// The cache encoding: `codespec-query-facts/1`, the same facts the same bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        use serde_json::{Map, Value as J, json};
        let v = |x: &Value| match x {
            Value::Str(x) => J::from(x.as_str()),
            Value::Int(n) => J::from(*n),
            Value::Bool(b) => J::from(*b),
            Value::List(_) => unreachable!("no stored relation holds a list"),
        };
        let mut rels = Map::new();
        for r in RELATIONS.iter().filter(|r| !ON_DEMAND.contains(&r.name)) {
            let rows: Vec<J> = self
                .rels
                .get(r.name)
                .into_iter()
                .flatten()
                .map(|row| J::Array(row.iter().map(v).collect()))
                .collect();
            rels.insert(r.name.to_string(), J::Array(rows));
        }
        let mut wild = Map::new();
        for (name, rows) in &self.wild {
            let rows: Vec<J> = rows
                .iter()
                .map(|(row, site)| {
                    json!([
                        J::Array(row.iter().map(|x| x.as_ref().map_or(J::Null, v)).collect()),
                        site
                    ])
                })
                .collect();
            wild.insert(name.to_string(), J::Array(rows));
        }
        let sites: Vec<J> = self
            .sites
            .iter()
            .map(|x| json!({"site": x.site, "pos": x.pos, "reason": x.reason}))
            .collect();
        let doc = json!({
            "format": FACTS_FORMAT,
            "version": env!("CARGO_PKG_VERSION"),
            "relations": rels,
            "wild": wild,
            "sites": sites,
        });
        let mut out = doc.to_string().into_bytes();
        out.push(b'\n');
        out
    }

    /// A cache file read back; a file of another format or version, or one whose facts do not
    /// fit the schema, is refused.
    pub fn from_bytes(bytes: &[u8]) -> Result<FactBase, String> {
        use serde_json::Value as J;
        let doc: J = serde_json::from_slice(bytes).map_err(|e| format!("facts: {e}"))?;
        if doc["format"] != FACTS_FORMAT {
            return Err(format!("facts: not {FACTS_FORMAT}"));
        }
        if doc["version"] != env!("CARGO_PKG_VERSION") {
            return Err("facts: written by another codespec version".into());
        }
        let value = |x: &J, t: Ty| -> Result<Value, String> {
            let v = match x {
                J::String(x) => Value::Str(x.clone()),
                J::Number(n) => Value::Int(n.as_i64().ok_or("facts: not an integer")?),
                J::Bool(b) => Value::Bool(*b),
                other => return Err(format!("facts: not a value: {other}")),
            };
            if t.holds(&v) {
                Ok(v)
            } else {
                Err(format!("facts: {x} is not a {}", t.name()))
            }
        };
        let row_of = |r: &Relation, x: &J| -> Result<Vec<J>, String> {
            match x {
                J::Array(a) if a.len() == r.fields.len() => Ok(a.clone()),
                _ => Err(format!("facts: a {} row of the wrong shape", r.name)),
            }
        };
        let mut fb = FactBase::default();
        let rels = doc["relations"].as_object().ok_or("facts: no relations")?;
        for (name, rows) in rels {
            let r = relation(name)
                .filter(|r| !ON_DEMAND.contains(&r.name))
                .ok_or_else(|| format!("facts: no relation {name}"))?;
            let mut set = BTreeSet::new();
            for x in rows.as_array().ok_or("facts: rows are not a list")? {
                let row = row_of(r, x)?
                    .iter()
                    .zip(r.fields)
                    .map(|(x, (_, t))| value(x, *t))
                    .collect::<Result<Vec<_>, _>>()?;
                set.insert(row);
            }
            fb.rels.insert(r.name, set.into_iter().collect());
        }
        let sites = doc["sites"].as_array().ok_or("facts: no sites")?;
        for x in sites {
            let field = |k: &str| {
                x[k].as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("facts: a site without {k}"))
            };
            fb.sites.push(Site {
                site: field("site")?,
                pos: field("pos")?,
                reason: field("reason")?,
            });
        }
        for (name, rows) in doc["wild"].as_object().ok_or("facts: no wild facts")? {
            let r = relation(name).ok_or_else(|| format!("facts: no relation {name}"))?;
            let mut list = Vec::new();
            for x in rows.as_array().ok_or("facts: rows are not a list")? {
                let site = x[1]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|&n| n < fb.sites.len())
                    .ok_or("facts: a wild fact without its site")?;
                let row = row_of(r, &x[0])?
                    .iter()
                    .zip(r.fields)
                    .map(|(x, (_, t))| {
                        if x.is_null() {
                            Ok(None)
                        } else {
                            value(x, *t).map(Some)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                list.push((row, site));
            }
            fb.wild.insert(r.name, list);
        }
        Ok(fb)
    }
}
