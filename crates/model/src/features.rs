//! The `cir/1` feature vocabulary: every thing an adapter can support, named by a stable id.
//!
//! # Where the ids come from
//!
//! Statement, expression, type and declaration ids are the `k` tags of the model's own variants,
//! read from [`schema_json`](crate::schema_json), and fact ids are the fields of
//! [`Facts`](crate::Facts), so a variant or fact added to the model adds its id and the
//! drift-checked `schemas/cir-1.features.json` no longer matches until it is regenerated.
//! Operators, compound assignments, intrinsics, semantic differences and module declarations other
//! than types (`decl.var`) are not variants; they are the constant lists below, each id with a
//! one-line description. The operator and intrinsic lists are checked against the names the
//! model documents on `Expr::Binop`, `Expr::Unop`, `Stmt::Assign` and `Expr::Intrinsic`.
//!
//! # The format `codespec-features/1`
//!
//! One JSON object, two-space indentation, a final newline:
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-features/1` |
//! | `cir` | string | the `cir` format the vocabulary describes, `cir/1` |
//! | `features` | array of feature | every feature, sorted by `id`, ids unique |
//!
//! A feature:
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `id` | string | stable id, `<prefix>.<name>`: `stmt.`, `expr.`, `type.`, `decl.`, `fact.`, `op.`, `unop.`, `assign.`, `intrinsic.`, `sem.` |
//! | `family` | string | `statement`, `expression`, `operator`, `intrinsic`, `type`, `fact` or `semantic` |
//! | `op` | string, operator ids only | the `op` string cir/1 carries for it: a `binop`'s or `unop`'s `op`, or an `assign`'s compound `op` such as `+=` |
//! | `description` | string | one line |
//!
//! [`features_from_json`] reads it strictly: unknown fields and any other format are refused.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The format of the vocabulary document.
pub const FEATURES_FORMAT: &str = "codespec-features/1";

/// The families [`vocabulary`] generates, and the only ones [`features_from_json`] admits.
pub const FEATURE_FAMILIES: &[&str] = &[
    "statement",
    "expression",
    "operator",
    "intrinsic",
    "type",
    "fact",
    "semantic",
];

/// One feature: a stable id, its family, for an operator its `op` string, and a one-line
/// description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub id: String,
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    pub description: String,
}

/// A `codespec-features/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureDocument {
    pub format: String,
    pub cir: String,
    pub features: Vec<Feature>,
}

/// The `op` strings of `Expr::Binop`, as the Go reader writes them (Go's binary operators).
const BINOPS: &[(&str, &str, &str)] = &[
    ("add", "+", "addition, or string concatenation"),
    ("sub", "-", "subtraction"),
    ("mul", "*", "multiplication"),
    ("div", "/", "division"),
    ("rem", "%", "remainder"),
    ("and", "&", "bitwise and"),
    ("or", "|", "bitwise or"),
    ("xor", "^", "bitwise exclusive or"),
    ("and_not", "&^", "bit clear: x and not y"),
    ("shl", "<<", "left shift"),
    ("shr", ">>", "right shift"),
    ("land", "&&", "logical and, short-circuit"),
    ("lor", "||", "logical or, short-circuit"),
    ("eq", "==", "equality"),
    ("ne", "!=", "inequality"),
    ("lt", "<", "less than"),
    ("le", "<=", "less than or equal"),
    ("gt", ">", "greater than"),
    ("ge", ">=", "greater than or equal"),
];

/// The binary operators with a compound assignment (`Stmt::Assign`'s `op`, `<binop>=`); the id
/// is `assign.<name>` with the binary operator's name.
const COMPOUND: &[&str] = &[
    "add", "sub", "mul", "div", "rem", "and", "or", "xor", "and_not", "shl", "shr",
];

/// The `op` strings `Expr::Unop` documents.
const UNOPS: &[(&str, &str, &str)] = &[
    ("not", "!", "logical not"),
    ("neg", "-", "negation"),
    ("plus", "+", "unary plus"),
    ("complement", "^", "bitwise complement"),
    ("recv", "<-", "channel receive"),
    ("deref", "*", "dereference"),
    ("ref", "&", "reference"),
];

/// The names `Expr::Intrinsic` documents.
const INTRINSICS: &[(&str, &str)] = &[
    ("sprintf", "format values into a string"),
    ("errorf", "format values into a new error"),
    ("println", "print values and a newline on standard output"),
    ("append", "append values to a slice"),
    ("len", "length of a string, slice or map"),
    (
        "sorted_keys",
        "the keys of a map as a slice, ascending: integers numerically, strings by UTF-16 code unit",
    ),
];

/// Behaviour two languages disagree on: one id per difference the harness has met.
const SEMANTICS: &[(&str, &str)] = &[
    (
        "sem.int.wrap",
        "integer arithmetic wraps on overflow, as Go's does",
    ),
    (
        "sem.rem.sign",
        "the remainder takes the sign of the dividend",
    ),
    (
        "sem.continue.post",
        "`continue` in a three-clause `for` runs its `post` statement first",
    ),
    (
        "sem.multi_assign.order",
        "a multi-value assignment reads every value before it writes any target",
    ),
    (
        "sem.map.order",
        "iteration over a map has no specified order",
    ),
    (
        "sem.switch.break",
        "a `break` inside a `switch` leaves the enclosing loop",
    ),
    (
        "sem.error.propagate",
        "a fallible call whose error the caller returns unchanged",
    ),
];

/// Module declarations that are not type declarations, so no `TypeDecl` variant: `decl.` ids in
/// the `type` family, beside the type declarations. `decl.variadic` is a property of a function
/// or extern declaration, `Param::variadic` and `Extern::variadic`.
const MODULE_DECLS: &[(&str, &str)] = &[
    (
        "decl.interface",
        "An interface type: the method signatures a value of it can be called with.",
    ),
    (
        "decl.var",
        "A module-level variable: a type and an optional initializer.",
    ),
    (
        "decl.variadic",
        "A function or extern whose last parameter, a slice, takes zero or more arguments.",
    ),
];

/// Marks on a declaration that are not declarations of their own, so no `TypeDecl` variant: `decl.`
/// ids in the `type` family, beside the type declarations.
const DECL_MARKS: &[(&str, &str)] = &[
    (
        "decl.embedded",
        "A struct field embedded in its struct (Go), named after its type; the reader writes every promoted field access and method call as the explicit path through it.",
    ),
    (
        "decl.type_params",
        "Type parameters of a generic function, type or extern, each constrained by an interface; a method of a generic type binds its type's parameters through its receiver.",
    ),
];

/// Marks on an expression or type use that are not variants of their own: `expr.` ids in the
/// `expression` family. `expr.instantiate` is a property of a call (`Expr::Call`, `Expr::Extern`,
/// `Expr::ExternMethod`) or a type use (`Type::Named`, `Type::Extern`) carrying `targs`.
const EXPR_MARKS: &[(&str, &str)] = &[(
    "expr.instantiate",
    "Type arguments written out on a call of a generic function or a use of a generic type, one per type parameter.",
)];

/// The variant-tagged enums of the model: schema definition, id prefix, family, noun.
const TAGGED: &[(&str, &str, &str, &str)] = &[
    ("Stmt", "stmt", "statement", "statement"),
    ("Expr", "expr", "expression", "expression"),
    ("Type", "type", "type", "type"),
    ("TypeDecl", "decl", "type", "type declaration"),
];

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn feature(id: String, family: &str, op: Option<String>, description: String) -> Feature {
    Feature {
        id,
        family: family.to_string(),
        op,
        description,
    }
}

fn operator(id: String, op: String, what: &str) -> Feature {
    let description = format!("`{op}`: {what}");
    feature(id, "operator", Some(op), description)
}

/// Every feature of `cir/1`, sorted by id.
pub fn vocabulary() -> Vec<Feature> {
    let schema: Value =
        serde_json::from_str(&crate::schema_json()).expect("the schema is valid JSON");
    let defs = &schema["$defs"];
    let mut out = Vec::new();
    for (def, prefix, family, noun) in TAGGED {
        let variants = defs[def]["oneOf"]
            .as_array()
            .unwrap_or_else(|| panic!("schema $defs.{def} has no oneOf"));
        for v in variants {
            let k = v["properties"]["k"]["const"]
                .as_str()
                .unwrap_or_else(|| panic!("a variant of {def} has no `k` const"));
            let description = match v["description"].as_str() {
                Some(d) => one_line(d),
                None => format!("the `{k}` {noun}"),
            };
            out.push(feature(format!("{prefix}.{k}"), family, None, description));
        }
    }
    let facts = defs["Facts"]["properties"]
        .as_object()
        .expect("schema $defs.Facts has properties");
    for (name, p) in facts {
        let description = match p["description"].as_str() {
            Some(d) => one_line(d),
            None => format!("the `{name}` fact"),
        };
        out.push(feature(format!("fact.{name}"), "fact", None, description));
    }
    for (name, op, what) in BINOPS {
        out.push(operator(format!("op.{name}"), op.to_string(), what));
    }
    for name in COMPOUND {
        let (_, op, what) = BINOPS
            .iter()
            .find(|(n, _, _)| n == name)
            .expect("every compound assignment names a binary operator");
        out.push(operator(
            format!("assign.{name}"),
            format!("{op}="),
            &format!("{what}, assigned to the left-hand side"),
        ));
    }
    for (name, op, what) in UNOPS {
        out.push(operator(format!("unop.{name}"), op.to_string(), what));
    }
    for (name, what) in INTRINSICS {
        out.push(feature(
            format!("intrinsic.{name}"),
            "intrinsic",
            None,
            (*what).to_string(),
        ));
    }
    for (id, what) in MODULE_DECLS {
        out.push(feature(
            (*id).to_string(),
            "type",
            None,
            (*what).to_string(),
        ));
    }
    for (id, what) in DECL_MARKS {
        out.push(feature(
            (*id).to_string(),
            "type",
            None,
            (*what).to_string(),
        ));
    }
    for (id, what) in EXPR_MARKS {
        out.push(feature(
            (*id).to_string(),
            "expression",
            None,
            (*what).to_string(),
        ));
    }
    for (id, what) in SEMANTICS {
        out.push(feature(
            (*id).to_string(),
            "semantic",
            None,
            (*what).to_string(),
        ));
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The `codespec-features/1` document, as `schemas/cir-1.features.json` holds it: two-space
/// indentation, a final newline.
pub fn features_json() -> String {
    let doc = FeatureDocument {
        format: FEATURES_FORMAT.to_string(),
        cir: crate::FORMAT.to_string(),
        features: vocabulary(),
    };
    let mut s = serde_json::to_string_pretty(&doc).expect("the vocabulary always serializes");
    s.push('\n');
    s
}

/// Why a document is not a `codespec-features/1` vocabulary.
#[derive(Debug)]
pub enum FeaturesReadError {
    Json(serde_json::Error),
    Format(String),
    /// `cir` is not `cir/1`; holds what it is.
    Cir(String),
    /// The feature with this id does not follow the previous one in strict order: the ids are
    /// unsorted or repeated.
    Order {
        id: String,
    },
    /// The feature with this id carries `op` without being an operator, or is an operator
    /// without `op`.
    Op {
        id: String,
    },
    /// The feature with this id names a family the vocabulary does not have.
    Family {
        id: String,
        family: String,
    },
}

impl std::fmt::Display for FeaturesReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeaturesReadError::Json(e) => write!(f, "not a {FEATURES_FORMAT} document: {e}"),
            FeaturesReadError::Format(found) => {
                write!(
                    f,
                    "format is {found:?}, this reader reads {FEATURES_FORMAT:?}"
                )
            }
            FeaturesReadError::Cir(found) => {
                write!(
                    f,
                    "cir is {found:?}, this vocabulary is of {:?}",
                    crate::FORMAT
                )
            }
            FeaturesReadError::Order { id } => write!(
                f,
                "feature {id:?} is not after the previous one: ids are sorted and unique"
            ),
            FeaturesReadError::Op { id } => write!(
                f,
                "feature {id:?}: `op` is present exactly on operator features"
            ),
            FeaturesReadError::Family { id, family } => write!(
                f,
                "feature {id:?}: unknown family {family:?}, one of {FEATURE_FAMILIES:?}"
            ),
        }
    }
}

impl std::error::Error for FeaturesReadError {}

/// Read a `codespec-features/1` document as the module doc defines it, refusing unknown fields,
/// any other format, a `cir` other than `cir/1`, ids not strictly sorted, `op` anywhere but on
/// exactly the operator features, and an unknown family.
pub fn features_from_json(bytes: &[u8]) -> Result<FeatureDocument, FeaturesReadError> {
    let probe: Value = serde_json::from_slice(bytes).map_err(FeaturesReadError::Json)?;
    let format = probe.get("format").and_then(Value::as_str).unwrap_or("");
    if format != FEATURES_FORMAT {
        return Err(FeaturesReadError::Format(format.to_string()));
    }
    let doc: FeatureDocument = serde_json::from_value(probe).map_err(FeaturesReadError::Json)?;
    if doc.cir != crate::FORMAT {
        return Err(FeaturesReadError::Cir(doc.cir));
    }
    let mut previous: Option<&str> = None;
    for f in &doc.features {
        if previous.is_some_and(|p| p >= f.id.as_str()) {
            return Err(FeaturesReadError::Order { id: f.id.clone() });
        }
        previous = Some(&f.id);
        if !FEATURE_FAMILIES.contains(&f.family.as_str()) {
            return Err(FeaturesReadError::Family {
                id: f.id.clone(),
                family: f.family.clone(),
            });
        }
        if f.op.is_some() != (f.family == "operator") {
            return Err(FeaturesReadError::Op { id: f.id.clone() });
        }
    }
    Ok(doc)
}
