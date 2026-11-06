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
pub use manifest::{
    ADAPTER_FORMAT, ADAPTER_ROLES, AdapterManifest, ManifestReadError, manifest_from_json,
};

/// The format this crate reads and writes.
pub const FORMAT: &str = "cir/1";

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
    pub funcs: Vec<Func>,
    /// Source declaration order, `type:<name>`, `const:<name>`, `error:<name>`, `func:<name>` or
    /// `func:<receiver type>.<name>`, for writers that keep it.
    pub layout: Vec<String>,
    /// Whole-module analysis results, written by passes and read by writers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<Facts>,
    /// Every construct the reader kept as `foreign` or could not read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusals: Vec<Refusal>,
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
    Bool,
    String,
    /// The source language's error type (Go `error`).
    Error,
    /// The top type (Go `any`).
    Any,
    Unit,
    /// A type the module declares.
    Named {
        name: String,
    },
    /// A reference to `elem`.
    Ptr {
        elem: Box<Type>,
    },
    Slice {
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub t: Type,
}

/// A named type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypeDecl {
    Struct {
        name: String,
        exported: bool,
        fields: Vec<Field>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        doc: String,
        pos: Pos,
    },
    /// A basic type given a name.
    Newtype {
        name: String,
        exported: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Param {
    pub name: String,
    pub t: Type,
}

/// A function or, with `recv`, a method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Func {
    pub name: String,
    pub exported: bool,
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
        Type::Named { name } => name,
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
    /// A call of a module function.
    Call {
        name: String,
        args: Vec<Expr>,
        t: Type,
    },
    Method {
        x: Box<Expr>,
        name: String,
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
}

impl Expr {
    pub fn t(&self) -> &Type {
        match self {
            Expr::Lit { t, .. }
            | Expr::Nil { t }
            | Expr::Var { t, .. }
            | Expr::Const { t, .. }
            | Expr::Errvar { t, .. }
            | Expr::Field { t, .. }
            | Expr::Index { t, .. }
            | Expr::Call { t, .. }
            | Expr::Method { t, .. }
            | Expr::Binop { t, .. }
            | Expr::Unop { t, .. }
            | Expr::Struct { t, .. }
            | Expr::Conv { t, .. }
            | Expr::Make { t }
            | Expr::Zero { t }
            | Expr::Intrinsic { t, .. }
            | Expr::ToError { t, .. } => t,
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
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Json(e) => write!(f, "not a cir/1 module: {e}"),
            ReadError::Format(found) => {
                write!(f, "format is {found:?}, this reader reads {FORMAT:?}")
            }
        }
    }
}

impl std::error::Error for ReadError {}

impl Module {
    /// Read a document, refusing unknown fields and any format other than `cir/1`.
    pub fn from_json(bytes: &[u8]) -> Result<Module, ReadError> {
        let probe: serde_json::Value = serde_json::from_slice(bytes).map_err(ReadError::Json)?;
        let format = probe.get("format").and_then(|f| f.as_str()).unwrap_or("");
        if format != FORMAT {
            return Err(ReadError::Format(format.to_string()));
        }
        serde_json::from_value(probe).map_err(ReadError::Json)
    }

    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Module always serializes");
        s.push('\n');
        s
    }
}

/// The JSON Schema of `cir/1`, as `schemas/cir-1.schema.json` holds it.
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Module);
    let mut s = serde_json::to_string_pretty(&schema).expect("a schema always serializes");
    s.push('\n');
    s
}
