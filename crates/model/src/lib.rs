//! The `cir/1` code intermediate representation: one typed, structured document per module.
//!
//! A reader lifts a source language into a [`Module`], passes rewrite a `Module` into a `Module`,
//! and a writer emits a source language from one. The JSON encoding is canonical: struct fields in
//! declaration order, sequences in source order, no maps. [`Module::from_json`] refuses unknown
//! fields and every format other than [`FORMAT`], so a document is never read past content its
//! reader does not know.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod features;
pub use features::{
    FEATURE_FAMILIES, FEATURES_FORMAT, Feature, FeatureDocument, FeaturesReadError,
    features_from_json, features_json, vocabulary,
};
pub mod manifest;
pub mod walk;
pub use manifest::{
    ADAPTER_FORMAT, ADAPTER_ROLES, AdapterManifest, ManifestReadError, manifest_from_json,
};

/// The format this crate reads and writes.
pub const FORMAT: &str = "cir/1";

/// How deep a document's arrays and objects may nest for [`Module::from_json`]: past it the
/// document is refused before it is parsed, so reading never overflows the stack. A chain of
/// `+` over a long table nests about one level per operand.
pub const MAX_NESTING: usize = 512;

/// The stack of the thread [`Module::from_json`] reads on: room for [`MAX_NESTING`] levels.
const READ_STACK: usize = 64 << 20;

/// One source module: a Go package, a Rust crate module, a Java compilation unit set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Module {
    /// Always `cir/1`.
    pub format: String,
    pub name: String,
    /// Which reader produced this document.
    pub reader: Reader,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    pub types: Vec<TypeDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consts: Vec<ConstDecl>,
    /// Sentinel errors: module-level error values built from a fixed message.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorVar>,
    /// Module-level variables, in source order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vars: Vec<VarDecl>,
    pub funcs: Vec<Func>,
    /// Every function and method outside the module that the module calls, sorted by `path`, then
    /// receiver type name (`""` for a function), then `name`, each once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub externs: Vec<Extern>,
    /// Source declaration order, `type:<name>` (a type or one of the module's own interfaces),
    /// `const:<name>`, `error:<name>`, `var:<name>`,
    /// `func:<name>` or `func:<receiver type>.<name>`, for writers that keep it.
    pub layout: Vec<String>,
    /// Whole-module analysis results, written by passes and read by writers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<Facts>,
    /// Every construct the reader kept as `foreign` or could not read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusals: Vec<Refusal>,
    /// The module's import path in the source language's spelling (`net/http`), when the reader
    /// knows it: the name an `extern` of another module uses for it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Every function the module declares that is not in `funcs`: a declaration the reader
    /// refused, or one without a body in the source language. Each keeps its name, receiver,
    /// position, reason and the calls it makes, so the call graph has no hole where it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opaque: Vec<Opaque>,
    /// The method set of each named type the module declares, interface types excepted, for
    /// dispatching a call through an interface over a set of modules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<MethodSet>,
    /// Every function of the module linked by name to a function of another module (Go
    /// `//go:linkname`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linknames: Vec<Linkname>,
    /// Every call the initializers of the module's variables make, and every function they use
    /// as a value (Go `var hook = syscall.Connect`): run when the module is loaded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inits: Vec<CallRef>,
    /// Interface types: the module's own, in source order, then every interface of another
    /// module that an `iface_call` goes through, sorted by `path`, then `name`, each once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<Interface>,
    /// The functions a framework starts, in the reader's order: an HTTP resource method, a
    /// scheduled method, a message consumer. Each names a function of `funcs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<Entry>,
    /// The types of the module a dependency-injection container manages, in source order. Each
    /// names a type of `types`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beans: Vec<Bean>,
    /// Every injection point of the module, in source order, with what the container resolves it
    /// to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub injects: Vec<Inject>,
    /// The keys (`F` or `T.m`) of the functions of `funcs` that run in a transaction: the
    /// container opens one around each call. Source order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transactional: Vec<String>,
}

/// The kinds of [`Entry`] a document may hold.
pub const ENTRY_KINDS: &[&str] = &["http", "scheduled", "incoming"];

/// A function a framework starts, not the program: a reader recognised it from the framework's
/// annotations (Java Quarkus: `@GET` with `@Path`, `@Scheduled`, `@Incoming`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// One of [`ENTRY_KINDS`]: `http` (a resource method), `scheduled` (a method run on a timer)
    /// or `incoming` (a message consumer).
    pub kind: String,
    /// The key of the function in `funcs`: `F` or `T.m`.
    pub func: String,
    /// The HTTP method, `GET`, `POST`, `PUT`, `DELETE` or `PATCH`; `http` only.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub method: String,
    /// `http`: the path, the resource's and the method's joined, starting with `/`. `scheduled`:
    /// `every=<interval>` or `cron=<expression>`, as written. `incoming`: the channel name.
    pub on: String,
    /// `http`: the media types the method accepts, as written; empty when it declares none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<String>,
    /// `http`: the media types the method produces, as written; empty when it declares none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub produces: Vec<String>,
}

/// A type of the module that a dependency-injection container creates and holds: its scope
/// annotation without the package (`ApplicationScoped`, `Singleton`, `RequestScoped`,
/// `Dependent`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bean {
    /// The name of a type in `types`.
    pub name: String,
    pub scope: String,
    pub pos: Pos,
}

/// A point the container fills in: a field, or a constructor parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inject {
    /// `Type.field`, or `Type.<new>.param` for a constructor parameter.
    pub at: String,
    /// The type the point is declared with.
    pub t: Type,
    /// The qualified name of the one bean of the read packages that implements the type; empty
    /// when there is none or several, and the value comes from outside the read code (`t` then
    /// is an `extern` type).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bean: String,
}

/// An interface type: the methods a value of it can be called with. A value of the module's own
/// interface `name` has the type `named`; with `path`, the interface of another module, a value
/// of it has the type `extern`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    /// The module declaring it, in the source language's spelling (`io`); empty: this module.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    pub name: String,
    pub exported: bool,
    /// Every method, those of embedded interfaces included, sorted by name, each once.
    pub methods: Vec<IfaceMethod>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    /// Where the module declares it; empty for an interface of another module.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pos: Pos,
}

impl Interface {
    /// `name`, or `path.name` for an interface of another module.
    pub fn key(&self) -> String {
        if self.path.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.path, self.name)
        }
    }
}

/// One method of an interface: its name and its parameter and result types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IfaceMethod {
    pub name: String,
    pub params: Vec<Type>,
    pub results: Vec<Type>,
}

/// A function the module declares whose body `cir/1` does not hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Opaque {
    pub name: String,
    /// The name of the receiver's declared type, for a method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recv: Option<String>,
    pub pos: Pos,
    pub reason: String,
    /// Declared without a body in the source language: implemented in assembly, or by the
    /// function a [`Linkname`] names.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_body: bool,
    /// Every call its body makes, in source order; empty for a function without a body.
    pub calls: Vec<CallRef>,
}

/// The deepest nesting of arrays and objects in the JSON text `bytes`, brackets inside strings
/// not counted. Malformed text is left to the parser.
fn nesting(bytes: &[u8]) -> usize {
    let (mut depth, mut max, mut in_string, mut escaped) = (0usize, 0usize, false, false);
    for &b in bytes {
        if in_string {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                max = max.max(depth);
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Whether `name` is a name strict reading admits for a module, a declaration, a field, a
/// parameter or a local: one or more letters of any script, digits, `_` and `$` (Java), after an
/// optional `r#` (a Rust raw identifier, as `codespec-read-rust` keeps it). A writer puts names
/// into source text and file names, where any other character could end the name and begin code
/// or a path; each writer applies its own language's stricter rule on top.
pub fn is_name(name: &str) -> bool {
    let name = name.strip_prefix("r#").unwrap_or(name);
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// Whether `path` is a module path strict reading admits: empty, or `/`-separated segments, none
/// empty, `.` or `..`, each of letters, digits, `_`, `-`, `.` and `~` (`github.com/x/go-y`).
pub fn is_module_path(path: &str) -> bool {
    path.is_empty()
        || path.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '~'))
        })
}

type NameFn<'f, 'a> = &'f mut dyn FnMut(&'static str, &'a str);

fn tparam_names<'a>(tparams: &'a [TypeParam], f: NameFn<'_, 'a>) {
    for p in tparams {
        f("type parameter", &p.name);
        type_names(&p.constraint, f);
    }
}

fn field_names<'a>(fields: &'a [Field], f: NameFn<'_, 'a>) {
    for fl in fields {
        f("field", &fl.name);
        type_names(&fl.t, f);
    }
}

fn param_names<'a>(params: &'a [Param], f: NameFn<'_, 'a>) {
    for p in params {
        f("parameter", &p.name);
        type_names(&p.t, f);
    }
}

fn type_names<'a>(t: &'a Type, f: NameFn<'_, 'a>) {
    match t {
        Type::Int { .. }
        | Type::Float { .. }
        | Type::Bool
        | Type::String
        | Type::Error
        | Type::Any
        | Type::Unit => {}
        Type::Named { name, targs } => {
            f("type", name);
            targs.iter().for_each(|t| type_names(t, f));
        }
        Type::Ptr { elem }
        | Type::Slice { elem }
        | Type::Array { elem, .. }
        | Type::Chan { elem, .. } => type_names(elem, f),
        Type::Map { key, elem } => {
            type_names(key, f);
            type_names(elem, f);
        }
        Type::Tuple { elems } => elems.iter().for_each(|t| type_names(t, f)),
        Type::Extern { targs, .. } => targs.iter().for_each(|t| type_names(t, f)),
        Type::Func { params, results } => {
            params.iter().chain(results).for_each(|t| type_names(t, f))
        }
        Type::Param { name } => f("type parameter", name),
    }
}

fn stmt_names<'a>(body: &'a [Stmt], f: NameFn<'_, 'a>) {
    for s in body {
        match s {
            Stmt::Let { names, rhs, .. } => {
                names.iter().for_each(|n| f("local", n));
                rhs.iter().for_each(|e| expr_names(e, f));
            }
            Stmt::Var { name, t, .. } => {
                f("local", name);
                type_names(t, f);
            }
            Stmt::Assign { lhs, rhs, .. } => lhs.iter().chain(rhs).for_each(|e| expr_names(e, f)),
            Stmt::Incr { x, .. } | Stmt::Decr { x, .. } | Stmt::Expr { x, .. } => expr_names(x, f),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                expr_names(cond, f);
                stmt_names(then, f);
                stmt_names(otherwise, f);
            }
            Stmt::ForRange {
                key,
                val,
                over,
                body,
                ..
            } => {
                f("local", key);
                f("local", val);
                expr_names(over, f);
                stmt_names(body, f);
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                init.iter()
                    .for_each(|s| stmt_names(std::slice::from_ref(&**s), f));
                cond.iter().for_each(|e| expr_names(e, f));
                post.iter()
                    .for_each(|s| stmt_names(std::slice::from_ref(&**s), f));
                stmt_names(body, f);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Foreign { .. } => {}
            Stmt::Switch { tag, cases, .. } => {
                tag.iter().for_each(|e| expr_names(e, f));
                for c in cases {
                    c.values.iter().for_each(|e| expr_names(e, f));
                    stmt_names(&c.body, f);
                }
            }
            Stmt::Return { results, .. } => results.iter().for_each(|e| expr_names(e, f)),
            Stmt::Send { chan, x, .. } => {
                expr_names(chan, f);
                expr_names(x, f);
            }
            Stmt::LookupElse {
                name,
                map,
                index,
                then,
                ..
            } => {
                f("local", name);
                expr_names(map, f);
                expr_names(index, f);
                stmt_names(then, f);
            }
        }
    }
}

fn expr_names<'a>(e: &'a Expr, f: NameFn<'_, 'a>) {
    type_names(e.t(), f);
    match e {
        Expr::Lit { .. }
        | Expr::Float { .. }
        | Expr::Nil { .. }
        | Expr::Make { .. }
        | Expr::Zero { .. } => {}
        Expr::Var { name, .. } => f("local", name),
        Expr::Const { name, .. } => f("constant", name),
        Expr::Errvar { name, .. } => f("error", name),
        Expr::Global { name, .. } => f("variable", name),
        Expr::Field { x, name, .. } => {
            expr_names(x, f);
            f("field", name);
        }
        Expr::Index { x, index, .. } => {
            expr_names(x, f);
            expr_names(index, f);
        }
        Expr::Call {
            name, targs, args, ..
        } => {
            f("function", name);
            targs.iter().for_each(|t| type_names(t, f));
            args.iter().for_each(|a| expr_names(a, f));
        }
        Expr::Method { x, name, args, .. } => {
            expr_names(x, f);
            f("method", name);
            args.iter().for_each(|a| expr_names(a, f));
        }
        Expr::Extern { targs, args, .. } => {
            targs.iter().for_each(|t| type_names(t, f));
            args.iter().for_each(|a| expr_names(a, f));
        }
        Expr::ExternMethod {
            targs, recv, args, ..
        } => {
            targs.iter().for_each(|t| type_names(t, f));
            expr_names(recv, f);
            args.iter().for_each(|a| expr_names(a, f));
        }
        Expr::Binop { x, y, .. } => {
            expr_names(x, f);
            expr_names(y, f);
        }
        Expr::Unop { x, .. }
        | Expr::ToError { x, .. }
        | Expr::Spread { x, .. }
        | Expr::ToIface { x, .. } => expr_names(x, f),
        Expr::Struct { name, fields, .. } => {
            f("type", name);
            for fi in fields {
                f("field", &fi.name);
                expr_names(&fi.value, f);
            }
        }
        Expr::Conv { name, x, .. } => {
            f("type", name);
            expr_names(x, f);
        }
        Expr::Intrinsic { args, .. } => args.iter().for_each(|a| expr_names(a, f)),
        Expr::FuncRef { name, recv, .. } => {
            f("function", name);
            recv.iter().for_each(|r| expr_names(r, f));
        }
        Expr::Lambda {
            params,
            results,
            captures,
            body,
            ..
        } => {
            param_names(params, f);
            results.iter().for_each(|t| type_names(t, f));
            captures.iter().for_each(|c| f("local", c));
            stmt_names(body, f);
        }
        Expr::CallValue {
            f: callee, args, ..
        } => {
            expr_names(callee, f);
            args.iter().for_each(|a| expr_names(a, f));
        }
        Expr::IfaceCall {
            path,
            iface,
            name,
            recv,
            args,
            ..
        } => {
            if path.is_empty() {
                f("interface", iface);
                f("method", name);
            }
            expr_names(recv, f);
            args.iter().for_each(|a| expr_names(a, f));
        }
    }
}

impl Opaque {
    /// `name`, or `<receiver type>.<name>` for a method, as [`Func::key`] gives it.
    pub fn key(&self) -> String {
        match &self.recv {
            Some(r) => format!("{r}.{}", self.name),
            None => self.name.clone(),
        }
    }
}

/// A call a construct makes, named from the source language's own type information where the
/// construct is outside `cir/1` (a `foreign` statement, an [`Opaque`] function). A signature `sig`
/// is the source language's spelling of a function type without parameter names, every named
/// type qualified by its full import path: `func([]byte) (int, error)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum CallRef {
    /// A call of the function `name` of the module `path`; `path` empty: this module.
    Func {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        path: String,
        name: String,
        /// The arguments that are integer constants, with their values.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        consts: Vec<ConstArg>,
        pos: Pos,
    },
    /// A call of the method `name` declared on the named type `recv` of the module `path`
    /// (empty: this module), the receiver's type being known.
    Method {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        path: String,
        recv: String,
        name: String,
        pos: Pos,
    },
    /// A call of the method `name` through a value of the interface `iface` (its spelling, for
    /// reading), whose whole method set is `methods`.
    Iface {
        iface: String,
        name: String,
        methods: Vec<MethodSig>,
        pos: Pos,
    },
    /// A call of a function value of the signature `sig`.
    Value { sig: String, pos: Pos },
    /// The function `name` of `path` (empty: this module), or with `recv` the method, used as a
    /// value of the signature `sig`: it may run wherever a value of `sig` is called.
    Ref {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        path: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        recv: String,
        name: String,
        sig: String,
        pos: Pos,
    },
    /// A function literal of the signature `sig`. Its calls belong to the function that holds
    /// it, and it may run wherever a value of `sig` is called.
    Lambda {
        sig: String,
        calls: Vec<CallRef>,
        pos: Pos,
    },
    /// Not a call: a value of the named type `name` of the module `path` (empty: this module) is
    /// made here (a composite literal, `new`, a conversion, a constant or a zero value of that
    /// type, or a struct holding one in a field), so a call through an interface may reach its
    /// methods.
    New {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        path: String,
        name: String,
        pos: Pos,
    },
}

impl CallRef {
    pub fn pos(&self) -> &Pos {
        match self {
            CallRef::Func { pos, .. }
            | CallRef::Method { pos, .. }
            | CallRef::Iface { pos, .. }
            | CallRef::Value { pos, .. }
            | CallRef::Ref { pos, .. }
            | CallRef::Lambda { pos, .. }
            | CallRef::New { pos, .. } => pos,
        }
    }
}

/// The argument at `index` (from 0) of a call is the integer constant `value`, in decimal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstArg {
    pub index: usize,
    pub value: String,
}

/// A method's name and signature, as an interface or a method set lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MethodSig {
    pub name: String,
    pub sig: String,
}

/// The method set of the named type `t` (with its pointer type's methods), sorted by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MethodSet {
    pub t: String,
    pub methods: Vec<MethodEntry>,
}

/// One method of a method set: its name and signature, and the call that runs when it is called,
/// a `method` or, for a method promoted from an embedded interface, an `iface` call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MethodEntry {
    pub name: String,
    pub sig: String,
    pub target: CallRef,
}

/// The function `name` (a [`Func::key`] of this module) is linked to `target`, the function
/// `<path>.<name>` of another module: the one without a body is the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Linkname {
    pub name: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reader {
    /// The source language: `go`, `rust`, …
    pub language: String,
    pub name: String,
    pub version: String,
}

/// A source position, `<file>:<line>`.
pub type Pos = String;

/// Integer width. `platform` is the source language's machine-word integer (Go `int`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum IntWidth {
    #[serde(rename = "platform")]
    Platform,
    #[serde(rename = "8")]
    W8,
    #[serde(rename = "16")]
    W16,
    #[serde(rename = "32")]
    W32,
    #[serde(rename = "64")]
    W64,
}

/// Floating-point width: IEEE 754 binary32 or binary64 (Go `float32`, `float64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FloatBits {
    #[serde(rename = "32")]
    W32,
    #[serde(rename = "64")]
    W64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChanDir {
    Send,
    Recv,
    Both,
}

/// A type expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum Type {
    Int {
        width: IntWidth,
        signed: bool,
    },
    /// An IEEE 754 floating-point number of `bits` bits.
    Float {
        bits: FloatBits,
    },
    Bool,
    String,
    /// The source language's error type (Go `error`).
    Error,
    /// The top type (Go `any`).
    Any,
    Unit,
    /// A type the module declares or one of its own interfaces; with `targs`, the generic type
    /// `name` instantiated with them, as many as its declaration has type parameters.
    Named {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targs: Vec<Type>,
    },
    /// A reference to `elem`.
    Ptr {
        elem: Box<Type>,
    },
    Slice {
        elem: Box<Type>,
    },
    /// A fixed-size array of `len` values of `elem`; a value, copied on assignment.
    Array {
        len: u64,
        elem: Box<Type>,
    },
    Map {
        key: Box<Type>,
        elem: Box<Type>,
    },
    Chan {
        elem: Box<Type>,
        dir: ChanDir,
    },
    Tuple {
        elems: Vec<Type>,
    },
    /// An opaque type `name` declared outside the module, in the package, crate or class `path`
    /// (Go `sync.Mutex`); its fields and representation are not part of the module. With
    /// `targs`, the generic type `name` instantiated with them (Go `atomic.Pointer[T]`).
    Extern {
        path: String,
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targs: Vec<Type>,
    },
    /// A function value taking `params` and giving `results`.
    Func {
        params: Vec<Type>,
        results: Vec<Type>,
    },
    /// The type parameter `name` of the enclosing generic declaration: a function's or extern's
    /// `tparams`, a type declaration's `tparams`, or one a method's receiver type binds by naming
    /// it among its type arguments.
    Param {
        name: String,
    },
}

/// A type parameter of a generic function, type or extern: its name and its constraint, an
/// interface every type argument given for it implements: `any`, one of the module's own
/// interfaces (`named`, with type arguments if it is generic) or another module's (`extern`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypeParam {
    pub name: String,
    pub constraint: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub t: Type,
    /// An embedded field (Go): named after its type, a `named` or `extern` type or a `ptr` to
    /// one. The reader writes every promoted field access and method call as the explicit path
    /// through it, so no consumer needs promotion rules; the mark only keeps how the source
    /// declares the field. Only on a struct's fields.
    #[serde(default, skip_serializing_if = "is_false")]
    pub embedded: bool,
}

impl Field {
    /// The name Go gives a field embedding `t`: the type's name, through one pointer; `None` for
    /// a type that cannot be embedded.
    pub fn embedded_name(t: &Type) -> Option<&str> {
        let t = match t {
            Type::Ptr { elem } => elem,
            t => t,
        };
        match t {
            Type::Named { name, .. } | Type::Extern { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// A named type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypeDecl {
    Struct {
        name: String,
        exported: bool,
        /// The type parameters of a generic struct, which its fields may name as `param` types.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tparams: Vec<TypeParam>,
        fields: Vec<Field>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        doc: String,
        pos: Pos,
    },
    /// A type given a name: a basic, function, map or slice type.
    Newtype {
        name: String,
        exported: bool,
        /// The type parameters of a generic newtype, which `under` may name as `param` types.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tparams: Vec<TypeParam>,
        under: Type,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        doc: String,
        pos: Pos,
    },
    /// A newtype whose values are exactly the declared variants.
    Enum {
        name: String,
        exported: bool,
        under: Type,
        variants: Vec<Variant>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        doc: String,
        pos: Pos,
    },
    /// Tagged variants, each carrying its own fields.
    Union {
        name: String,
        exported: bool,
        variants: Vec<UnionVariant>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        doc: String,
        pos: Pos,
    },
}

impl TypeDecl {
    pub fn name(&self) -> &str {
        match self {
            TypeDecl::Struct { name, .. }
            | TypeDecl::Newtype { name, .. }
            | TypeDecl::Enum { name, .. }
            | TypeDecl::Union { name, .. } => name,
        }
    }

    /// The type parameters; empty for a type that is not generic.
    pub fn tparams(&self) -> &[TypeParam] {
        match self {
            TypeDecl::Struct { tparams, .. } | TypeDecl::Newtype { tparams, .. } => tparams,
            TypeDecl::Enum { .. } | TypeDecl::Union { .. } => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnionVariant {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstDecl {
    pub name: String,
    pub t: Type,
    pub value: Expr,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorVar {
    pub name: String,
    pub message: String,
    pub pos: Pos,
}

/// A module-level variable of type `t`: the zero value of `t`, or the value of `init`, set before
/// any function of the module runs. An initializer outside cir/1 is kept as `foreign` source
/// instead, never both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VarDecl {
    pub name: String,
    pub t: Type,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init: Option<Expr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreign: Option<ForeignInit>,
    pub pos: Pos,
}

/// An initializer the reader kept verbatim, in `lang`, because cir/1 cannot express it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForeignInit {
    pub lang: String,
    pub src: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Param {
    pub name: String,
    pub t: Type,
    /// The last parameter of a function takes zero or more arguments, or one `spread` slice; `t`
    /// is the slice it is inside the body (Go `...T`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub variadic: bool,
}

/// A function outside the module: `name` in the package, crate or class `path`, with its declared
/// parameter and result types; with `recv`, a method of the extern type `recv` names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Extern {
    /// The import path in the source language's spelling: `os`, `math/rand`, `std::env`.
    pub path: String,
    pub name: String,
    /// The receiver of a method: an `extern` type of `path`, or a `ptr` to one. A method of a
    /// generic type binds that type's parameters by naming them as `param` type arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recv: Option<Type>,
    /// The function's own type parameters; a call gives as many type arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tparams: Vec<TypeParam>,
    pub params: Vec<Type>,
    pub results: Vec<Type>,
    /// The last of `params`, a slice, is variadic: a call passes zero or more arguments for it,
    /// or one `spread` slice.
    #[serde(default, skip_serializing_if = "is_false")]
    pub variadic: bool,
}

impl Extern {
    /// The name of the extern type a method's receiver belongs to; `""` for a function.
    pub fn recv_type_name(&self) -> &str {
        self.recv.as_ref().map_or("", extern_type_name)
    }

    /// `path.name`, or `path.Type.name` for a method.
    pub fn key(&self) -> String {
        match self.recv_type_name() {
            "" => format!("{}.{}", self.path, self.name),
            t => format!("{}.{t}.{}", self.path, self.name),
        }
    }
}

/// The name of the extern type `t` is, or points to; `""` for any other type.
pub fn extern_type_name(t: &Type) -> &str {
    extern_type(t).map_or("", |(_, name)| name)
}

/// The `(path, name)` of the extern type `t` is, or points to.
pub fn extern_type(t: &Type) -> Option<(&str, &str)> {
    match t {
        Type::Ptr { elem } => extern_type(elem),
        Type::Extern { path, name, .. } => Some((path, name)),
        _ => None,
    }
}

/// A function or, with `recv`, a method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Func {
    pub name: String,
    pub exported: bool,
    /// The function's own type parameters; a call gives as many type arguments. A method of a
    /// generic type has none of its own: its receiver's type binds the type's parameters by
    /// naming them as `param` type arguments (Go `func (p Pair[T]) First() T`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tparams: Vec<TypeParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recv: Option<Param>,
    pub params: Vec<Param>,
    pub results: Vec<Type>,
    pub body: Vec<Stmt>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub doc: String,
    pub pos: Pos,
}

impl Func {
    /// `name`, or `<receiver type>.<name>` for a method.
    pub fn key(&self) -> String {
        match &self.recv {
            Some(r) => format!("{}.{}", receiver_type_name(&r.t), self.name),
            None => self.name.clone(),
        }
    }
}

/// The declared type a receiver of type `t` belongs to.
pub fn receiver_type_name(t: &Type) -> &str {
    match t {
        Type::Ptr { elem } => receiver_type_name(elem),
        Type::Named { name, .. } => name,
        _ => "",
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// The values this case matches; empty for the default case.
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
}

/// A statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum Stmt {
    /// `names := rhs`; a name `_` discards.
    Let {
        names: Vec<String>,
        rhs: Vec<Expr>,
        pos: Pos,
    },
    /// `var name t`, the zero value.
    Var {
        name: String,
        t: Type,
        pos: Pos,
    },
    /// `lhs = rhs`, or `lhs <op> rhs` for a compound operator: `+=`, `-=`, `*=`, `/=`, `%=`, `&=`,
    /// `|=`, `^=`, `&^=`, `<<=` or `>>=`.
    Assign {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        op: String,
        lhs: Vec<Expr>,
        rhs: Vec<Expr>,
        pos: Pos,
    },
    Incr {
        x: Expr,
        pos: Pos,
    },
    Decr {
        x: Expr,
        pos: Pos,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        #[serde(rename = "else", default, skip_serializing_if = "Vec::is_empty")]
        otherwise: Vec<Stmt>,
        pos: Pos,
    },
    /// `for key, val := range over { body }`; `_` for an unused name.
    ForRange {
        key: String,
        val: String,
        over: Expr,
        body: Vec<Stmt>,
        pos: Pos,
    },
    /// `for init; cond; post { body }`: `init` runs once, `cond` is checked before each iteration
    /// (absent: always true), `post` runs after each iteration, including one a `continue` ends.
    /// `init` is a `let` or an `assign`; `post` an `assign`, `incr` or `decr`.
    For {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        init: Option<Box<Stmt>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cond: Option<Expr>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        post: Option<Box<Stmt>>,
        body: Vec<Stmt>,
        pos: Pos,
    },
    /// Leaves the innermost enclosing `for` or `for_range`.
    Break {
        pos: Pos,
    },
    /// Ends the current iteration of the innermost enclosing `for` or `for_range`.
    Continue {
        pos: Pos,
    },
    /// A `switch` on `tag`, or on `true` when `tag` is absent.
    Switch {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tag: Option<Expr>,
        cases: Vec<Case>,
        pos: Pos,
    },
    Return {
        results: Vec<Expr>,
        pos: Pos,
    },
    Expr {
        x: Expr,
        pos: Pos,
    },
    /// `chan <- x`.
    Send {
        chan: Expr,
        x: Expr,
        pos: Pos,
    },
    /// `name := map[index]`, running `then` (which ends in a return) when the key is absent.
    /// `name` is not bound inside `then`.
    LookupElse {
        name: String,
        map: Expr,
        index: Expr,
        then: Vec<Stmt>,
        pos: Pos,
    },
    /// Source the reader kept verbatim, in `lang`, because cir/1 cannot express it.
    Foreign {
        lang: String,
        src: String,
        reason: String,
        pos: Pos,
        /// Every call the source makes, in source order, when the reader lists them; absent when
        /// it does not, and then the calls are unknown.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        calls: Option<Vec<CallRef>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
}

/// An expression. `t` is the type the reader computed for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expr {
    /// A literal in the source language's spelling.
    Lit {
        value: String,
        t: Type,
    },
    /// A floating-point literal: `value` is the shortest decimal that reads back to the same
    /// number of `t`'s width, laid out as [`float_spelling`] gives it.
    Float {
        value: String,
        t: Type,
    },
    Nil {
        t: Type,
    },
    /// A local, parameter or receiver.
    Var {
        name: String,
        t: Type,
    },
    /// A declared constant or enum variant.
    Const {
        name: String,
        t: Type,
    },
    /// A sentinel error.
    Errvar {
        name: String,
        t: Type,
    },
    /// A module-level variable, declared in the module's `vars`; read, or assigned to on the left
    /// of an `assign`.
    Global {
        name: String,
        t: Type,
    },
    Field {
        x: Box<Expr>,
        name: String,
        t: Type,
    },
    Index {
        x: Box<Expr>,
        index: Box<Expr>,
        t: Type,
    },
    /// A call of a module function; of a generic one with `targs`, its type arguments written
    /// out, as many as it has type parameters.
    Call {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targs: Vec<Type>,
        args: Vec<Expr>,
        t: Type,
    },
    Method {
        x: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        t: Type,
    },
    /// A call of the function `name` of `path`, declared in the module's `externs`; with `targs`,
    /// the type arguments of a generic one, as many as its declaration has type parameters.
    Extern {
        path: String,
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targs: Vec<Type>,
        args: Vec<Expr>,
        t: Type,
    },
    /// A call of the method `name` of the extern type `type` of `path` on `recv`, declared in the
    /// module's `externs` with that receiver type; with `targs`, the type arguments of the
    /// method's own type parameters (those of a generic receiver type are `recv`'s type's).
    ExternMethod {
        path: String,
        #[serde(rename = "type")]
        type_name: String,
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        targs: Vec<Type>,
        recv: Box<Expr>,
        args: Vec<Expr>,
        t: Type,
    },
    /// `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `&^` (bit clear), `<<`, `>>`, `&&`, `||`, `==`, `!=`,
    /// `<`, `<=`, `>` or `>=`.
    Binop {
        op: String,
        x: Box<Expr>,
        y: Box<Expr>,
        t: Type,
    },
    /// `!`, `-`, `+`, `^` (bitwise complement), `<-` (channel receive), `*` (dereference) or `&`
    /// (reference).
    Unop {
        op: String,
        x: Box<Expr>,
        t: Type,
    },
    Struct {
        name: String,
        fields: Vec<FieldInit>,
        t: Type,
    },
    /// A conversion to the named type `name`.
    Conv {
        name: String,
        x: Box<Expr>,
        t: Type,
    },
    /// An empty map or slice.
    Make {
        t: Type,
    },
    Zero {
        t: Type,
    },
    /// `sprintf`, `errorf`, `println`, `append`, `len` or `sorted_keys`. `sorted_keys` takes a map and
    /// gives its keys as a slice in ascending order: integers numerically, strings by UTF-16 code
    /// unit, as Java's String.compareTo orders them, which byte order does not always match.
    Intrinsic {
        name: String,
        args: Vec<Expr>,
        t: Type,
    },
    /// A value of an error struct used where an error is expected.
    ToError {
        x: Box<Expr>,
        t: Type,
    },
    /// The module function `name` used as a value or, with `recv`, the method `name` bound to
    /// the receiver value `recv`.
    FuncRef {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recv: Option<Box<Expr>>,
        t: Type,
    },
    /// A function literal. `captures` names every local, parameter or receiver of the enclosing
    /// function it reads or writes, each in scope where the literal stands; inside `body` only
    /// `captures` and `params` are in scope.
    Lambda {
        params: Vec<Param>,
        results: Vec<Type>,
        captures: Vec<String>,
        body: Vec<Stmt>,
        t: Type,
    },
    /// A call of the function value `f`.
    CallValue {
        f: Box<Expr>,
        args: Vec<Expr>,
        t: Type,
    },
    /// The slice `x` passed as the variadic last parameter itself (Go `xs...`): only the last
    /// argument of a `call`, `method`, `extern` or `extern_method` of a variadic function, or of
    /// the `append` intrinsic.
    Spread {
        x: Box<Expr>,
        t: Type,
    },
    /// A call of the method `name` through `recv`, a value of the interface `iface` of `path`
    /// (empty: this module), declared in the module's `interfaces`, or a value of a type
    /// parameter that interface constrains: the method of whatever value `recv` holds runs.
    IfaceCall {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        path: String,
        iface: String,
        name: String,
        recv: Box<Expr>,
        args: Vec<Expr>,
        t: Type,
    },
    /// The value `x`, of a concrete type or of another interface, used where a value of the
    /// interface `t` is expected.
    ToIface {
        x: Box<Expr>,
        t: Type,
    },
}

impl Expr {
    pub fn t(&self) -> &Type {
        match self {
            Expr::Lit { t, .. }
            | Expr::Float { t, .. }
            | Expr::Nil { t }
            | Expr::Var { t, .. }
            | Expr::Const { t, .. }
            | Expr::Errvar { t, .. }
            | Expr::Global { t, .. }
            | Expr::Field { t, .. }
            | Expr::Index { t, .. }
            | Expr::Call { t, .. }
            | Expr::Method { t, .. }
            | Expr::Extern { t, .. }
            | Expr::ExternMethod { t, .. }
            | Expr::Binop { t, .. }
            | Expr::Unop { t, .. }
            | Expr::Struct { t, .. }
            | Expr::Conv { t, .. }
            | Expr::Make { t }
            | Expr::Zero { t }
            | Expr::Intrinsic { t, .. }
            | Expr::ToError { t, .. }
            | Expr::FuncRef { t, .. }
            | Expr::Lambda { t, .. }
            | Expr::CallValue { t, .. }
            | Expr::Spread { t, .. }
            | Expr::IfaceCall { t, .. }
            | Expr::ToIface { t, .. } => t,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    pub pos: Pos,
    pub what: String,
    pub reason: String,
}

/// Whole-module facts. Writers read these instead of deciding anything themselves.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    /// Struct types a reference to which is stored in a container or a struct field.
    pub shared: Vec<String>,
    /// For channels of `any`: every type sent on one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
    /// Functions whose `(T, bool)` result is always `(zero, false)` or `(x, true)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comma_ok: Vec<String>,
    /// Functions whose last result is an error, every error path returning zero values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallible: Vec<String>,
    /// Methods that write nothing rooted at their receiver.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_only: Vec<String>,
    /// Types with a text rendering: an `Error()`/`String()` method, or a string enum or newtype.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub displays: Vec<String>,
    /// Positions of loops over a map, whose order the source language leaves unspecified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unordered_iteration: Vec<Pos>,
}

/// Why a document is not a `cir/1` module.
#[derive(Debug)]
pub enum ReadError {
    Json(serde_json::Error),
    Format(String),
    /// The document parses, but breaks a rule the types cannot state.
    Invalid(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Json(e) => write!(f, "not a cir/1 module: {e}"),
            ReadError::Format(found) => {
                write!(f, "format is {found:?}, this reader reads {FORMAT:?}")
            }
            ReadError::Invalid(why) => write!(f, "not a cir/1 module: {why}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl Module {
    /// Read a document, refusing unknown fields and any format other than `cir/1`, and one nested
    /// deeper than [`MAX_NESTING`].
    pub fn from_json(bytes: &[u8]) -> Result<Module, ReadError> {
        let depth = nesting(bytes);
        if depth > MAX_NESTING {
            return Err(ReadError::Invalid(format!(
                "nested {depth} deep, past the {MAX_NESTING} cir/1 reading admits"
            )));
        }
        // Parsing and deserializing recurse once per level, several KiB each in a debug build:
        // they run on a thread whose stack holds MAX_NESTING levels whatever the caller's holds.
        std::thread::scope(|s| {
            std::thread::Builder::new()
                .name("cir1-read".into())
                .stack_size(READ_STACK)
                .spawn_scoped(s, || Self::read_bounded(bytes))
                .expect("a thread to read the document on")
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        })
    }

    /// [`Module::from_json`] once the nesting is known to be within [`MAX_NESTING`].
    fn read_bounded(bytes: &[u8]) -> Result<Module, ReadError> {
        // serde_json's own limit is 128; the bound checked by the caller stands in for it.
        let mut de = serde_json::Deserializer::from_slice(bytes);
        de.disable_recursion_limit();
        let probe = serde_json::Value::deserialize(&mut de).map_err(ReadError::Json)?;
        de.end().map_err(ReadError::Json)?;
        let format = probe.get("format").and_then(|f| f.as_str()).unwrap_or("");
        if format != FORMAT {
            return Err(ReadError::Format(format.to_string()));
        }
        let m: Module = serde_json::from_value(probe).map_err(ReadError::Json)?;
        m.check_names().map_err(ReadError::Invalid)?;
        m.check_variadic().map_err(ReadError::Invalid)?;
        m.check_externs().map_err(ReadError::Invalid)?;
        m.check_vars().map_err(ReadError::Invalid)?;
        m.check_captures().map_err(ReadError::Invalid)?;
        m.check_opaque().map_err(ReadError::Invalid)?;
        m.check_embedded().map_err(ReadError::Invalid)?;
        m.check_interfaces().map_err(ReadError::Invalid)?;
        m.check_generics().map_err(ReadError::Invalid)?;
        m.check_framework().map_err(ReadError::Invalid)?;
        Ok(m)
    }

    /// Every name the module gives one of its own declarations, where it declares it and where
    /// it uses it, with what it names: `module`, `type`, `type parameter`, `field`, `variant`,
    /// `interface`, `method`, `constant`, `error`, `variable`, `function`, `receiver`,
    /// `parameter` or `local`. Writers put these into source text. Names another module gives
    /// (externs, another module's interfaces, the calls listed for `foreign` and `opaque` code),
    /// the declarations `opaque` lists and the module `path` are not visited.
    pub fn each_name<'a>(&'a self, f: &mut dyn FnMut(&'static str, &'a str)) {
        f("module", &self.name);
        for d in &self.types {
            f("type", d.name());
            tparam_names(d.tparams(), f);
            match d {
                TypeDecl::Struct { fields, .. } => field_names(fields, f),
                TypeDecl::Newtype { under, .. } => type_names(under, f),
                TypeDecl::Enum {
                    under, variants, ..
                } => {
                    type_names(under, f);
                    for v in variants {
                        f("variant", &v.name);
                        expr_names(&v.value, f);
                    }
                }
                TypeDecl::Union { variants, .. } => {
                    for v in variants {
                        f("variant", &v.name);
                        field_names(&v.fields, f);
                    }
                }
            }
        }
        for i in self.interfaces.iter().filter(|i| i.path.is_empty()) {
            f("interface", &i.name);
            for m in &i.methods {
                f("method", &m.name);
                m.params
                    .iter()
                    .chain(&m.results)
                    .for_each(|t| type_names(t, f));
            }
        }
        for c in &self.consts {
            f("constant", &c.name);
            type_names(&c.t, f);
            expr_names(&c.value, f);
        }
        for e in &self.errors {
            f("error", &e.name);
        }
        for v in &self.vars {
            f("variable", &v.name);
            type_names(&v.t, f);
            v.init.iter().for_each(|e| expr_names(e, f));
        }
        for func in &self.funcs {
            f("function", &func.name);
            tparam_names(&func.tparams, f);
            if let Some(r) = &func.recv {
                f("receiver", &r.name);
                type_names(&r.t, f);
            }
            param_names(&func.params, f);
            func.results.iter().for_each(|t| type_names(t, f));
            stmt_names(&func.body, f);
        }
    }

    /// Every name of [`Module::each_name`] is a [`is_name`], and the module `path` an
    /// [`is_module_path`].
    fn check_names(&self) -> Result<(), String> {
        if !is_module_path(&self.path) {
            return Err(format!(
                "the module path {:?} is not segments of letters, digits, `_`, `-`, `.` and `~` joined by `/`, none empty, `.` or `..`",
                self.path
            ));
        }
        let mut bad = None;
        self.each_name(&mut |what, name| {
            if bad.is_none() && !is_name(name) {
                bad = Some(format!(
                    "the {what} name {name:?} is empty or holds a character other than a letter, a digit, `_` or `$` (after an optional `r#`)"
                ));
            }
        });
        bad.map_or(Ok(()), Err)
    }

    /// Every expression of the module at any depth, lambda bodies included: enum variant
    /// values, constants, variable initializers and function bodies.
    fn each_expr<'a>(&'a self, f: &mut dyn FnMut(&'a Expr)) {
        for d in &self.types {
            if let TypeDecl::Enum { variants, .. } = d {
                for v in variants {
                    walk::walk_expr(&v.value, f);
                }
            }
        }
        for c in &self.consts {
            walk::walk_expr(&c.value, f);
        }
        for init in self.vars.iter().filter_map(|v| v.init.as_ref()) {
            walk::walk_expr(init, f);
        }
        for func in &self.funcs {
            walk::body_exprs(&func.body, f);
        }
    }

    /// For a call that can take a `spread`: what it calls, its arguments, and its declared
    /// parameter count and whether the last is variadic, where the module declares it (a call of
    /// a function in `opaque` has no declared signature). `None` for any other expression.
    fn spread_callee<'e>(&self, e: &'e Expr) -> Option<SpreadCallee<'e>> {
        let func = |f: &Func| (f.params.len(), f.params.last().is_some_and(|p| p.variadic));
        let ext = |d: &Extern| (d.params.len(), d.variadic);
        Some(match e {
            Expr::Call { name, args, .. } => (
                format!("call {name}"),
                args,
                self.funcs
                    .iter()
                    .find(|f| f.recv.is_none() && f.name == *name)
                    .map(func),
            ),
            Expr::Method { x, name, args, .. } => {
                let t = receiver_type_name(x.t());
                (
                    format!("method call {t}.{name}"),
                    args,
                    self.funcs
                        .iter()
                        .find(|f| {
                            f.name == *name
                                && f.recv
                                    .as_ref()
                                    .is_some_and(|r| receiver_type_name(&r.t) == t)
                        })
                        .map(func),
                )
            }
            Expr::Extern {
                path, name, args, ..
            } => (
                format!("extern call {path}.{name}"),
                args,
                self.extern_decl(path, name).map(ext),
            ),
            Expr::ExternMethod {
                path,
                type_name,
                name,
                args,
                ..
            } => (
                format!("extern method call {path}.{type_name}.{name}"),
                args,
                self.extern_method_decl(path, type_name, name).map(ext),
            ),
            Expr::Intrinsic { name, args, .. } => (
                format!("intrinsic {name}"),
                args,
                Some(if name == "append" {
                    (2, true)
                } else {
                    (args.len(), false)
                }),
            ),
            Expr::CallValue { args, .. } => {
                ("call of a function value".into(), args, Some((0, false)))
            }
            _ => return None,
        })
    }

    /// A variadic parameter is a function's last, and a slice; a variadic extern's last parameter
    /// is a slice; no lambda parameter or receiver is variadic. A `spread` is only the last
    /// argument of a call whose last parameter is variadic, as many arguments as the callee has
    /// parameters (a callee the module does not declare is not checked).
    fn check_variadic(&self) -> Result<(), String> {
        for f in &self.funcs {
            let key = f.key();
            if f.recv.as_ref().is_some_and(|r| r.variadic) {
                return Err(format!("{key}: the receiver is variadic"));
            }
            for (i, p) in f.params.iter().enumerate() {
                if p.variadic && i + 1 != f.params.len() {
                    return Err(format!(
                        "{key}: parameter {} is variadic but not the last",
                        p.name
                    ));
                }
                if p.variadic && !matches!(p.t, Type::Slice { .. }) {
                    return Err(format!(
                        "{key}: variadic parameter {} is not a slice",
                        p.name
                    ));
                }
            }
        }
        if let Some(x) = self
            .externs
            .iter()
            .find(|x| x.variadic && !matches!(x.params.last(), Some(Type::Slice { .. })))
        {
            return Err(format!(
                "extern {} is variadic, but its last parameter is not a slice",
                x.key()
            ));
        }
        let (mut spreads, mut placed) = (0usize, 0usize);
        let mut bad = None;
        self.each_expr(&mut |e| {
            if bad.is_some() {
                return;
            }
            match e {
                Expr::Spread { .. } => spreads += 1,
                Expr::Lambda { params, .. } if params.iter().any(|p| p.variadic) => {
                    bad = Some("a lambda parameter is variadic".to_string());
                }
                _ => {}
            }
            let Some((what, args, decl)) = self.spread_callee(e) else {
                return;
            };
            for (i, a) in args.iter().enumerate() {
                if !matches!(a, Expr::Spread { .. }) {
                    continue;
                }
                if i + 1 != args.len() {
                    bad = Some(format!("{what} passes a spread argument that is not the last"));
                    return;
                }
                placed += 1;
                match decl {
                    Some((_, false)) => {
                        bad = Some(format!(
                            "{what} passes a spread argument, but its last parameter is not variadic"
                        ));
                    }
                    Some((n, true)) if n != args.len() => {
                        bad = Some(format!(
                            "{what} has {} arguments with a spread, its declaration {n} parameters",
                            args.len()
                        ));
                    }
                    _ => {}
                }
            }
        });
        if let Some(b) = bad {
            return Err(b);
        }
        if spreads != placed {
            return Err(
                "a spread outside the arguments of a call of a variadic function".to_string(),
            );
        }
        Ok(())
    }

    /// The declaration of the interface `name` of `path` (empty: this module), if the module has
    /// one.
    pub fn interface_decl(&self, path: &str, name: &str) -> Option<&Interface> {
        self.interfaces
            .iter()
            .find(|i| i.path == path && i.name == name)
    }

    /// The module's own interfaces first, named once and by no type declaration; those of other
    /// modules after them, sorted by `(path, name)` with no repeat; each interface's methods
    /// sorted by name with no repeat. Every interface call names a declared interface and one of
    /// its methods, with as many arguments as that method has parameters, and every `to_iface`
    /// converts to an interface: the module's own, declared, or an `extern` type.
    fn check_interfaces(&self) -> Result<(), String> {
        let mut own = std::collections::BTreeSet::new();
        let mut last_extern: Option<(&str, &str)> = None;
        for i in &self.interfaces {
            if i.path.is_empty() {
                if last_extern.is_some() {
                    return Err(format!(
                        "interface {} of this module after one of another module",
                        i.name
                    ));
                }
                if !own.insert(i.name.as_str()) {
                    return Err(format!("interface {} is declared twice", i.name));
                }
                if self.types.iter().any(|d| d.name() == i.name) {
                    return Err(format!("interface {} is also declared as a type", i.name));
                }
            } else {
                let key = (i.path.as_str(), i.name.as_str());
                if last_extern.is_some_and(|l| l >= key) {
                    return Err(format!(
                        "interfaces out of order or repeated at {}",
                        i.key()
                    ));
                }
                last_extern = Some(key);
            }
            for w in i.methods.windows(2) {
                if w[0].name >= w[1].name {
                    return Err(format!(
                        "interface {}: methods out of order or repeated at {}",
                        i.key(),
                        w[1].name
                    ));
                }
            }
        }
        let mut bad = None;
        let mut check = |e: &Expr| {
            if bad.is_some() {
                return;
            }
            match e {
                Expr::IfaceCall {
                    path,
                    iface,
                    name,
                    args,
                    ..
                } => {
                    let what = if path.is_empty() {
                        format!("interface call {iface}.{name}")
                    } else {
                        format!("interface call {path}.{iface}.{name}")
                    };
                    bad = match self.interface_decl(path, iface) {
                        None => Some(format!("{what} names an undeclared interface")),
                        Some(d) => match d.methods.iter().find(|m| m.name == *name) {
                            None => Some(format!("{what} names no method of {}", d.key())),
                            Some(m) if m.params.len() != args.len() => Some(format!(
                                "{what} has {} arguments, its method {} parameters",
                                args.len(),
                                m.params.len()
                            )),
                            Some(_) => None,
                        },
                    }
                }
                Expr::ToIface { t, .. } => {
                    bad = match t {
                        Type::Extern { .. } => None,
                        Type::Named { name, .. } if own.contains(name.as_str()) => None,
                        Type::Named { name, .. } => Some(format!(
                            "to_iface to {name}, which is not a declared interface"
                        )),
                        other => Some(format!(
                            "to_iface to a {} type, which is not an interface",
                            serde_json::to_value(other)
                                .ok()
                                .and_then(|v| v["k"].as_str().map(str::to_owned))
                                .unwrap_or_default()
                        )),
                    }
                }
                _ => {}
            }
        };
        for c in &self.consts {
            walk::walk_expr(&c.value, &mut check);
        }
        for init in self.vars.iter().filter_map(|v| v.init.as_ref()) {
            walk::walk_expr(init, &mut check);
        }
        for func in &self.funcs {
            walk::body_exprs(&func.body, &mut check);
        }
        bad.map_or(Ok(()), Err)
    }

    /// How many type parameters the type `name` declares: its type declaration's, none for one
    /// of the module's own interfaces or a name the module does not declare.
    fn tparam_count(&self, name: &str) -> usize {
        self.types
            .iter()
            .find(|d| d.name() == name)
            .map_or(0, |d| d.tparams().len())
    }

    /// `t` names only type parameters in `scope`, and every `named` type in it has as many type
    /// arguments as its declaration type parameters.
    fn check_type(&self, t: &Type, scope: &[&str], what: &str) -> Result<(), String> {
        let each = |ts: &[Type]| ts.iter().try_for_each(|t| self.check_type(t, scope, what));
        match t {
            Type::Param { name } if !scope.contains(&name.as_str()) => {
                Err(format!("{what}: type parameter {name} is not in scope"))
            }
            Type::Named { name, targs } => {
                let want = self.tparam_count(name);
                if targs.len() != want {
                    return Err(format!(
                        "{what}: type {name} has {} type arguments, its declaration {want} type parameters",
                        targs.len()
                    ));
                }
                each(targs)
            }
            Type::Extern { targs, .. } => each(targs),
            Type::Ptr { elem }
            | Type::Slice { elem }
            | Type::Array { elem, .. }
            | Type::Chan { elem, .. } => self.check_type(elem, scope, what),
            Type::Map { key, elem } => {
                self.check_type(key, scope, what)?;
                self.check_type(elem, scope, what)
            }
            Type::Tuple { elems } => each(elems),
            Type::Func { params, results } => {
                each(params)?;
                each(results)
            }
            Type::Int { .. }
            | Type::Float { .. }
            | Type::Bool
            | Type::String
            | Type::Error
            | Type::Any
            | Type::Unit
            | Type::Param { .. } => Ok(()),
        }
    }

    /// Type parameters named once each, every constraint an interface (`any`, one of the
    /// module's own, or an `extern` type) in `scope`; gives the scope with them added.
    fn tparam_scope<'a>(
        &self,
        tparams: &'a [TypeParam],
        mut scope: Vec<&'a str>,
        what: &str,
    ) -> Result<Vec<&'a str>, String> {
        for p in tparams {
            if scope.contains(&p.name.as_str()) {
                return Err(format!(
                    "{what}: type parameter {} is declared twice",
                    p.name
                ));
            }
            scope.push(&p.name);
        }
        for p in tparams {
            let interface = match &p.constraint {
                Type::Any | Type::Extern { .. } => true,
                Type::Named { name, .. } => self.interface_decl("", name).is_some(),
                _ => false,
            };
            if !interface {
                return Err(format!(
                    "{what}: the constraint of type parameter {} is not an interface",
                    p.name
                ));
            }
            self.check_type(&p.constraint, &scope, what)?;
        }
        Ok(scope)
    }

    /// The type parameters a method's receiver type binds: its type arguments, each a `param`
    /// named once.
    fn receiver_scope<'a>(recv: &'a Type, what: &str) -> Result<Vec<&'a str>, String> {
        let base = match recv {
            Type::Ptr { elem } => elem,
            t => t,
        };
        let targs = match base {
            Type::Named { targs, .. } | Type::Extern { targs, .. } => targs.as_slice(),
            _ => &[],
        };
        let mut scope = Vec::new();
        for t in targs {
            match t {
                Type::Param { name } if !scope.contains(&name.as_str()) => scope.push(name),
                _ => {
                    return Err(format!(
                        "{what}: the receiver's type arguments are not type parameters, each once"
                    ));
                }
            }
        }
        Ok(scope)
    }

    /// Every type of the module names only type parameters in scope where it stands (a generic
    /// declaration's own, those a method's receiver binds), every constraint is an interface,
    /// and every instantiation gives as many type arguments as its declaration has type
    /// parameters: a generic type's use, a call of a generic function the module declares, and a
    /// call of a generic extern.
    fn check_generics(&self) -> Result<(), String> {
        for d in &self.types {
            let what = format!("type {}", d.name());
            let scope = self.tparam_scope(d.tparams(), Vec::new(), &what)?;
            match d {
                TypeDecl::Struct { fields, .. } => {
                    for f in fields {
                        self.check_type(&f.t, &scope, &what)?;
                    }
                }
                TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. } => {
                    self.check_type(under, &scope, &what)?
                }
                TypeDecl::Union { variants, .. } => {
                    for f in variants.iter().flat_map(|v| &v.fields) {
                        self.check_type(&f.t, &scope, &what)?;
                    }
                }
            }
        }
        for i in self.interfaces.iter() {
            let what = format!("interface {}", i.key());
            for m in &i.methods {
                for t in m.params.iter().chain(&m.results) {
                    self.check_type(t, &[], &what)?;
                }
            }
        }
        for x in &self.externs {
            let what = format!("extern {}", x.key());
            let bound = match &x.recv {
                Some(r) => Self::receiver_scope(r, &what)?,
                None => Vec::new(),
            };
            let scope = self.tparam_scope(&x.tparams, bound, &what)?;
            for t in x.recv.iter().chain(&x.params).chain(&x.results) {
                self.check_type(t, &scope, &what)?;
            }
        }
        for c in &self.consts {
            let what = format!("const {}", c.name);
            self.check_type(&c.t, &[], &what)?;
            self.check_body_types(None, std::slice::from_ref(&c.value), &[], &what)?;
        }
        for v in &self.vars {
            let what = format!("var {}", v.name);
            self.check_type(&v.t, &[], &what)?;
            self.check_body_types(None, v.init.as_slice(), &[], &what)?;
        }
        for f in &self.funcs {
            let what = f.key();
            let bound = match &f.recv {
                Some(r) => Self::receiver_scope(&r.t, &what)?,
                None => Vec::new(),
            };
            let scope = self.tparam_scope(&f.tparams, bound, &what)?;
            let sig = f.recv.iter().chain(&f.params).map(|p| &p.t);
            for t in sig.chain(&f.results) {
                self.check_type(t, &scope, &what)?;
            }
            self.check_body_types(Some(&f.body), &[], &scope, &what)?;
        }
        Ok(())
    }

    /// The types and instantiations of `body`'s statements and of `exprs`, at any depth, lambda
    /// bodies included, in `scope`.
    fn check_body_types(
        &self,
        body: Option<&[Stmt]>,
        exprs: &[Expr],
        scope: &[&str],
        what: &str,
    ) -> Result<(), String> {
        let mut bad: Option<String> = None;
        let var_types = |s: &Stmt, bad: &mut Option<String>| {
            if let Stmt::Var { t, .. } = s
                && bad.is_none()
            {
                *bad = self.check_type(t, scope, what).err();
            }
        };
        let arity = |what_call: String, have: usize, want: usize| {
            (have != want).then(|| {
                format!(
                    "{what}: {what_call} has {have} type arguments, its declaration {want} type parameters"
                )
            })
        };
        let check = |e: &Expr, bad: &mut Option<String>| {
            if bad.is_some() {
                return;
            }
            if let Err(err) = self.check_type(e.t(), scope, what) {
                *bad = Some(err);
                return;
            }
            let targs = match e {
                Expr::Call { name, targs, .. } => {
                    if let Some(f) = self
                        .funcs
                        .iter()
                        .find(|f| f.recv.is_none() && f.name == *name)
                    {
                        *bad = arity(format!("call {name}"), targs.len(), f.tparams.len());
                    }
                    targs
                }
                Expr::Extern {
                    path, name, targs, ..
                } => {
                    if let Some(d) = self.extern_decl(path, name) {
                        *bad = arity(
                            format!("extern call {path}.{name}"),
                            targs.len(),
                            d.tparams.len(),
                        );
                    }
                    targs
                }
                Expr::ExternMethod {
                    path,
                    type_name,
                    name,
                    targs,
                    ..
                } => {
                    if let Some(d) = self.extern_method_decl(path, type_name, name) {
                        *bad = arity(
                            format!("extern method call {path}.{type_name}.{name}"),
                            targs.len(),
                            d.tparams.len(),
                        );
                    }
                    targs
                }
                Expr::Lambda {
                    params,
                    results,
                    body,
                    ..
                } => {
                    for t in params.iter().map(|p| &p.t).chain(results) {
                        if bad.is_none() {
                            *bad = self.check_type(t, scope, what).err();
                        }
                    }
                    walk::walk_stmts(body, &mut |s| var_types(s, bad));
                    return;
                }
                _ => return,
            };
            for t in targs {
                if bad.is_none() {
                    *bad = self.check_type(t, scope, what).err();
                }
            }
        };
        if let Some(body) = body {
            walk::walk_stmts(body, &mut |s| var_types(s, &mut bad));
            walk::body_exprs(body, &mut |e| check(e, &mut bad));
        }
        for e in exprs {
            walk::walk_expr(e, &mut |e| check(e, &mut bad));
        }
        bad.map_or(Ok(()), Err)
    }

    /// No var has both an initializer and a foreign one, and every global names a declared var.
    fn check_vars(&self) -> Result<(), String> {
        if let Some(v) = self
            .vars
            .iter()
            .find(|v| v.init.is_some() && v.foreign.is_some())
        {
            return Err(format!(
                "var {} has both an initializer and a foreign one",
                v.name
            ));
        }
        let mut bad = None;
        let mut check = |e: &Expr| {
            if let Expr::Global { name, .. } = e
                && bad.is_none()
                && !self.vars.iter().any(|v| v.name == *name)
            {
                bad = Some(format!("global {name} is not declared"));
            }
        };
        for d in &self.types {
            if let TypeDecl::Enum { variants, .. } = d {
                for v in variants {
                    walk::walk_expr(&v.value, &mut check);
                }
            }
        }
        for c in &self.consts {
            walk::walk_expr(&c.value, &mut check);
        }
        for v in &self.vars {
            if let Some(init) = &v.init {
                walk::walk_expr(init, &mut check);
            }
        }
        for func in &self.funcs {
            walk::body_exprs(&func.body, &mut check);
        }
        bad.map_or(Ok(()), Err)
    }

    /// Every embedded field is a struct's, named after its type, and named by no other field of
    /// its struct.
    fn check_embedded(&self) -> Result<(), String> {
        for d in &self.types {
            match d {
                TypeDecl::Struct { name, fields, .. } => {
                    for (i, f) in fields.iter().enumerate() {
                        if !f.embedded {
                            continue;
                        }
                        if Field::embedded_name(&f.t) != Some(f.name.as_str()) {
                            return Err(format!(
                                "embedded field {} of struct {name} is not named after its type",
                                f.name
                            ));
                        }
                        if fields
                            .iter()
                            .enumerate()
                            .any(|(j, g)| j != i && g.name == f.name)
                        {
                            return Err(format!(
                                "struct {name} declares the field {} twice",
                                f.name
                            ));
                        }
                    }
                }
                TypeDecl::Union { name, variants, .. } => {
                    if let Some(f) = variants.iter().flat_map(|v| &v.fields).find(|f| f.embedded) {
                        return Err(format!(
                            "union {name}: field {} is embedded; only a struct's field can be",
                            f.name
                        ));
                    }
                }
                TypeDecl::Newtype { .. } | TypeDecl::Enum { .. } => {}
            }
        }
        Ok(())
    }

    /// Every entry point of a known kind naming a function of `funcs`, every bean naming a type of
    /// `types`.
    fn check_framework(&self) -> Result<(), String> {
        let funcs: std::collections::BTreeSet<String> = self.funcs.iter().map(Func::key).collect();
        for e in &self.entries {
            if !ENTRY_KINDS.contains(&e.kind.as_str()) {
                return Err(format!("entry point of unknown kind {}", e.kind));
            }
            if !funcs.contains(&e.func) {
                return Err(format!(
                    "entry point names {}, which is no function of the module",
                    e.func
                ));
            }
        }
        for t in &self.transactional {
            if !funcs.contains(t) {
                return Err(format!(
                    "transactional names {t}, which is no function of the module"
                ));
            }
        }
        for b in &self.beans {
            let declared = self.types.iter().any(|d| match d {
                TypeDecl::Struct { name, .. }
                | TypeDecl::Newtype { name, .. }
                | TypeDecl::Enum { name, .. }
                | TypeDecl::Union { name, .. } => *name == b.name,
            });
            if !declared {
                return Err(format!("bean {} is no type of the module", b.name));
            }
        }
        Ok(())
    }

    /// Every opaque function named once, and by no function of `funcs`.
    fn check_opaque(&self) -> Result<(), String> {
        let mut seen: std::collections::BTreeSet<String> =
            self.funcs.iter().map(Func::key).collect();
        for o in &self.opaque {
            let key = o.key();
            if !seen.insert(key.clone()) {
                return Err(format!(
                    "opaque function {key} is declared twice, or also as a function"
                ));
            }
        }
        Ok(())
    }

    /// The declaration for the extern `path`.`name`, if the module has one.
    pub fn extern_decl(&self, path: &str, name: &str) -> Option<&Extern> {
        self.externs
            .iter()
            .find(|x| x.path == path && x.recv.is_none() && x.name == name)
    }

    /// The declaration for the method `name` of the extern type `type_name` of `path`, if the
    /// module has one.
    pub fn extern_method_decl(&self, path: &str, type_name: &str, name: &str) -> Option<&Extern> {
        self.externs.iter().find(|x| {
            x.path == path && x.recv.is_some() && x.recv_type_name() == type_name && x.name == name
        })
    }

    /// `externs` sorted by `(path, receiver type name, name)` with no repeat, each receiver an
    /// extern type of the declaration's own `path`, and every extern call and extern method call
    /// declared, with as many arguments as its declaration has parameters (a variadic last one
    /// counting as zero or more).
    fn check_externs(&self) -> Result<(), String> {
        for w in self.externs.windows(2) {
            let key = |x: &Extern| {
                (
                    x.path.clone(),
                    x.recv_type_name().to_string(),
                    x.name.clone(),
                )
            };
            if key(&w[0]) >= key(&w[1]) {
                return Err(format!(
                    "externs out of order or repeated at {}",
                    w[1].key()
                ));
            }
        }
        for x in &self.externs {
            if let Some(r) = &x.recv
                && !matches!(extern_type(r), Some((p, _)) if p == x.path)
            {
                return Err(format!(
                    "extern {} has a receiver that is not an extern type of {}",
                    x.key(),
                    x.path
                ));
            }
        }
        let mut bad = None;
        // A variadic last parameter takes zero or more arguments.
        let arity = |what: String, d: &Extern, args: &[Expr]| {
            let fits = if d.variadic {
                args.len() + 1 >= d.params.len()
            } else {
                d.params.len() == args.len()
            };
            (!fits).then(|| {
                format!(
                    "{what} has {} arguments, its declaration {} parameters",
                    args.len(),
                    d.params.len()
                )
            })
        };
        let mut check = |e: &Expr| {
            if bad.is_some() {
                return;
            }
            match e {
                Expr::Extern {
                    path, name, args, ..
                } => {
                    let what = format!("extern call {path}.{name}");
                    bad = match self.extern_decl(path, name) {
                        None => Some(format!("{what} is not declared")),
                        Some(d) => arity(what, d, args),
                    }
                }
                Expr::ExternMethod {
                    path,
                    type_name,
                    name,
                    args,
                    ..
                } => {
                    let what = format!("extern method call {path}.{type_name}.{name}");
                    bad = match self.extern_method_decl(path, type_name, name) {
                        None => Some(format!("{what} is not declared")),
                        Some(d) => arity(what, d, args),
                    }
                }
                Expr::Float { value, t } => bad = self.check_float(value, t).err(),
                _ => {}
            }
        };
        for d in &self.types {
            if let TypeDecl::Enum { variants, .. } = d {
                for v in variants {
                    walk::walk_expr(&v.value, &mut check);
                }
            }
        }
        for c in &self.consts {
            walk::walk_expr(&c.value, &mut check);
        }
        for init in self.vars.iter().filter_map(|v| v.init.as_ref()) {
            walk::walk_expr(init, &mut check);
        }
        for func in &self.funcs {
            walk::body_exprs(&func.body, &mut check);
        }
        bad.map_or(Ok(()), Err)
    }

    /// The width of a float type: `float`, or a declared newtype or enum over one.
    fn float_bits(&self, t: &Type) -> Option<FloatBits> {
        match t {
            Type::Float { bits } => Some(*bits),
            Type::Named { name, .. } => self.types.iter().find_map(|d| match d {
                TypeDecl::Newtype { name: n, under, .. }
                | TypeDecl::Enum { name: n, under, .. }
                    if n == name =>
                {
                    self.float_bits(under)
                }
                _ => None,
            }),
            _ => None,
        }
    }

    /// A float literal is of a float type, a finite number of that width, and spelled as
    /// [`float_spelling`] spells it.
    fn check_float(&self, value: &str, t: &Type) -> Result<(), String> {
        let Some(bits) = self.float_bits(t) else {
            return Err(format!("float literal {value:?} is not of a float type"));
        };
        let number = match bits {
            FloatBits::W32 => value
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
                .map(f64::from),
            FloatBits::W64 => value.parse::<f64>().ok().filter(|v| v.is_finite()),
        };
        let Some(number) = number else {
            return Err(format!("float literal {value:?} is not a finite number"));
        };
        let spelled = float_spelling(number, bits);
        if spelled != value {
            return Err(format!("float literal {value:?} is written {spelled:?}"));
        }
        Ok(())
    }

    /// Every lambda's `captures` in scope where it stands: the enclosing function's receiver or
    /// parameters, a name an earlier statement of an enclosing block binds (`let`, `var`,
    /// `lookup_else`, a loop's own names inside it), or a capture or parameter of an enclosing
    /// lambda. A `foreign` statement may bind any name, so after one in an enclosing block every
    /// capture is admitted.
    fn check_captures(&self) -> Result<(), String> {
        for func in &self.funcs {
            let mut scope: Vec<&str> = func
                .recv
                .iter()
                .chain(&func.params)
                .map(|p| p.name.as_str())
                .collect();
            scope_block(&func.body, &mut scope, false, &func.key())?;
        }
        Ok(())
    }

    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Module always serializes");
        s.push('\n');
        s
    }
}

/// The spelling of a float literal's `value`: the shortest decimal that reads back to `v` at
/// width `bits`, in plain notation (`1000`, `0.25`, `-0`) when its decimal exponent `e` is
/// `-7 < e < 21`, else as `<digits>e<e>` with no `+` and no leading zero (`1e21`, `1.5e-7`):
/// ECMAScript's number layout without the `+`.
pub fn float_spelling(v: f64, bits: FloatBits) -> String {
    let sci = match bits {
        FloatBits::W32 => format!("{:e}", v as f32),
        FloatBits::W64 => format!("{v:e}"),
    };
    let (mantissa, exp) = sci.split_once('e').expect("`{:e}` writes an exponent");
    let exp: i64 = exp.parse().expect("`{:e}` writes an integer exponent");
    if !(-7 < exp && exp < 21) {
        return sci;
    }
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let point = exp + 1;
    let len = digits.len() as i64;
    if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
    } else if len <= point {
        format!("{sign}{digits}{}", "0".repeat((point - len) as usize))
    } else {
        let (int, frac) = digits.split_at(point as usize);
        format!("{sign}{int}.{frac}")
    }
}

/// Checks the lambdas of `body` against `scope`, the names in scope where it starts; `open` after
/// a `foreign` statement. The names `body` binds leave scope at its end.
fn scope_block<'a>(
    body: &'a [Stmt],
    scope: &mut Vec<&'a str>,
    open: bool,
    func: &str,
) -> Result<(), String> {
    let mark = scope.len();
    let mut open = open;
    for s in body {
        open |= scope_stmt(s, scope, open, func)?;
    }
    scope.truncate(mark);
    Ok(())
}

/// Checks the lambdas of `s` and binds the names `s` binds in the block that holds it; true for a
/// `foreign` statement.
fn scope_stmt<'a>(
    s: &'a Stmt,
    scope: &mut Vec<&'a str>,
    open: bool,
    func: &str,
) -> Result<bool, String> {
    if !matches!(s, Stmt::For { .. }) {
        let mut r = Ok(());
        walk::stmt_exprs(s, &mut |e| {
            if r.is_ok() {
                r = scope_expr(e, scope, open, func);
            }
        });
        r?;
    }
    match s {
        Stmt::Let { names, .. } => scope.extend(names.iter().map(String::as_str)),
        Stmt::Var { name, .. } => scope.push(name),
        Stmt::If {
            then, otherwise, ..
        } => {
            scope_block(then, scope, open, func)?;
            scope_block(otherwise, scope, open, func)?;
        }
        Stmt::ForRange { key, val, body, .. } => {
            let mark = scope.len();
            scope.extend([key.as_str(), val.as_str()]);
            scope_block(body, scope, open, func)?;
            scope.truncate(mark);
        }
        Stmt::For {
            init,
            cond,
            post,
            body,
            ..
        } => {
            let mark = scope.len();
            let mut inner = open;
            if let Some(i) = init {
                inner |= scope_stmt(i, scope, open, func)?;
            }
            if let Some(c) = cond {
                scope_expr(c, scope, inner, func)?;
            }
            if let Some(p) = post {
                scope_stmt(p, scope, inner, func)?;
            }
            scope_block(body, scope, inner, func)?;
            scope.truncate(mark);
        }
        Stmt::Switch { cases, .. } => {
            for c in cases {
                scope_block(&c.body, scope, open, func)?;
            }
        }
        Stmt::LookupElse { name, then, .. } => {
            scope_block(then, scope, open, func)?;
            scope.push(name);
        }
        Stmt::Foreign { .. } => return Ok(true),
        _ => {}
    }
    Ok(false)
}

/// Checks every lambda in `e`: its captures against `scope`, its body against its own captures
/// and parameters.
fn scope_expr<'a>(e: &'a Expr, scope: &[&'a str], open: bool, func: &str) -> Result<(), String> {
    if let Expr::Lambda {
        params,
        captures,
        body,
        ..
    } = e
    {
        if let Some(c) = captures
            .iter()
            .find(|c| !open && !scope.contains(&c.as_str()))
        {
            return Err(format!(
                "lambda in {func} captures {c}, which is not in scope"
            ));
        }
        let mut inner: Vec<&str> = captures
            .iter()
            .chain(params.iter().map(|p| &p.name))
            .map(String::as_str)
            .collect();
        return scope_block(body, &mut inner, false, func);
    }
    let mut r = Ok(());
    walk::children(e, &mut |x| {
        if r.is_ok() {
            r = scope_expr(x, scope, open, func);
        }
    });
    r
}

/// What a call that can take a `spread` calls, its arguments, and the callee's declared parameter
/// count and whether its last parameter is variadic, where the module declares it.
type SpreadCallee<'e> = (String, &'e [Expr], Option<(usize, bool)>);

/// The JSON Schema of `cir/1`, as `schemas/cir-1.schema.json` holds it.
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Module);
    let mut s = serde_json::to_string_pretty(&schema).expect("a schema always serializes");
    s.push('\n');
    s
}
