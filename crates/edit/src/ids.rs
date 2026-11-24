//! The two-way mapping between the `at` paths of `codespec-spans/1` and the ids cq gives
//! (`crates/query`), for every kind an edit names:
//!
//! | what | cq id | `at` |
//! |---|---|---|
//! | a function or method | `<module>.<key>` | `funcs.<i>` |
//! | a function without a read body | `<module>.<key>` | `opaque.<i>` |
//! | a declared type | `<module>.<name>` | `types.<i>` |
//! | a field of a declared struct | its type's id and the field's name | `types.<i>.fields.<j>` |
//! | a statement or expression | `<module>.<key>#<index path>` | `funcs.<i>.body.<index path>` |
//! | the name of field `j` of a struct literal | the literal's node id and `j` | `funcs.<i>.body.<index path>.fields.<j>.name` |
//!
//! `<module>` is the module's `path`, or its `name` when it has none; `<key>` is the function's
//! `name`, or `<receiver type>.<name>` for a method. Below `funcs.<i>.body` the path is cq's index
//! path, which names a struct literal's field `j` by its value (`fields.<j>`): the token naming
//! the field is `fields.<j>.name`, so a value that is itself a call keeps a `name` span of its own.
//!
//! [`source_names`] gives the `source_name(node, text)` records of the fact base: the `text` of
//! each `name` span under [`Id::key`].

use std::collections::{BTreeMap, HashMap};

use codespec_model::{Module, TypeDecl};
use codespec_query::{FactBase, Value};

use crate::spans::{Role, SpanDoc};

/// The expression kinds whose name a wave-1 edit renames: each has a `name` span.
pub const NAMED_EXPRS: &[&str] = &[
    "call",
    "method",
    "extern",
    "extern_method",
    "func_ref",
    "field",
];

/// What an edit names, in cq's terms.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Id {
    /// A function or method, read or opaque: `<module>.<key>`.
    Func(String),
    /// A declared type: `<module>.<name>`.
    Type(String),
    /// A field of a declared struct: its type's id and the field's `cir/1` name.
    Field { ty: String, name: String },
    /// A statement or expression: `<module>.<key>#<index path>`.
    Node(String),
    /// The token naming field `index` of the struct literal `node`.
    LiteralField { node: String, index: usize },
}

impl Id {
    /// The text a `source_name` record keys the id by: a function, type or node id as cq gives
    /// it; a struct field `<type id>#<field name>` (a method of the same name is
    /// `<type id>.<name>`); a struct literal's field `<node id>.fields.<j>.name`.
    pub fn key(&self) -> String {
        match self {
            Id::Func(id) | Id::Type(id) | Id::Node(id) => id.clone(),
            Id::Field { ty, name } => format!("{ty}#{name}"),
            Id::LiteralField { node, index } => format!("{node}.fields.{index}.name"),
        }
    }
}

/// What an `at` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// `funcs.<i>`.
    Func,
    /// `opaque.<i>`.
    Opaque,
    /// `types.<i>`.
    Type,
    /// `types.<i>.fields.<j>` of a struct.
    Field,
    /// A statement, with cq's kind (`stmt.kind`).
    Stmt(String),
    /// An expression, with cq's kind (`expr.kind`).
    Expr(String),
    /// The name of a struct literal's field.
    LiteralField,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Func => write!(f, "function"),
            Kind::Opaque => write!(f, "opaque function"),
            Kind::Type => write!(f, "type"),
            Kind::Field => write!(f, "struct field"),
            Kind::Stmt(k) => write!(f, "{k} statement"),
            Kind::Expr(k) => write!(f, "{k} expression"),
            Kind::LiteralField => write!(f, "struct literal field"),
        }
    }
}

/// One `source_name(node, text)` record: the source spelling `text` of the thing `node` names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceName {
    pub node: String,
    pub text: String,
}

/// A statement or expression of a function body.
#[derive(Debug, Clone)]
struct Node {
    kind: Kind,
    /// A struct literal's number of fields.
    fields: usize,
}

/// The mapping for one `cir/1` module.
pub struct Ids<'m> {
    module: &'m Module,
    label: String,
    /// Function id to its index in `funcs`, and the same for `opaque`.
    funcs: HashMap<String, usize>,
    opaque: HashMap<String, usize>,
    /// Type name to its index in `types`.
    types: HashMap<String, usize>,
    /// (index in `funcs`, index path) to the node.
    nodes: BTreeMap<(usize, String), Node>,
}

/// The name the fact base gives a module: its path, or its name when it has none.
pub fn label(m: &Module) -> &str {
    if m.path.is_empty() { &m.name } else { &m.path }
}

impl<'m> Ids<'m> {
    /// The mapping of `module`, its body nodes from a fact base built over it alone.
    pub fn new(module: &'m Module) -> Ids<'m> {
        let facts = FactBase::build(std::slice::from_ref(module));
        Ids::from_facts(module, &facts)
    }

    /// The mapping of `module`, its body nodes from `facts`, a fact base over a set holding it.
    pub fn from_facts(module: &'m Module, facts: &FactBase) -> Ids<'m> {
        let label = label(module).to_string();
        let mut funcs = HashMap::new();
        for (i, f) in module.funcs.iter().enumerate() {
            funcs.entry(format!("{label}.{}", f.key())).or_insert(i);
        }
        let mut opaque = HashMap::new();
        for (i, o) in module.opaque.iter().enumerate() {
            opaque.entry(format!("{label}.{}", o.key())).or_insert(i);
        }
        let mut types = HashMap::new();
        for (i, t) in module.types.iter().enumerate() {
            types.entry(t.name().to_string()).or_insert(i);
        }
        let mut nodes = BTreeMap::new();
        let place = |id: &str| -> Option<(usize, String)> {
            let (f, path) = id.split_once('#')?;
            Some((*funcs.get(f)?, path.to_string()))
        };
        for (rel, stmt) in [("stmt", true), ("expr", false)] {
            for row in facts.rows(rel) {
                let (Some(Value::Str(id)), Some(Value::Str(kind))) = (row.first(), row.get(2))
                else {
                    continue;
                };
                if let Some(at) = place(id) {
                    let kind = if stmt {
                        Kind::Stmt(kind.clone())
                    } else {
                        Kind::Expr(kind.clone())
                    };
                    nodes.insert(at, Node { kind, fields: 0 });
                }
            }
        }
        for row in facts.rows("field_init") {
            let (Some(Value::Str(id)), Some(Value::Int(j))) = (row.first(), row.get(1)) else {
                continue;
            };
            if let Some(n) = place(id).and_then(|at| nodes.get_mut(&at)) {
                n.fields = n.fields.max(*j as usize + 1);
            }
        }
        Ids {
            module,
            label,
            funcs,
            opaque,
            types,
            nodes,
        }
    }

    /// The module the mapping is of.
    pub fn module(&self) -> &'m Module {
        self.module
    }

    /// The module's name in ids: its path, or its name when it has none.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The `at` of `id`, when it names something of the module.
    pub fn at(&self, id: &Id) -> Option<String> {
        match id {
            Id::Func(f) => {
                if let Some(i) = self.funcs.get(f) {
                    Some(format!("funcs.{i}"))
                } else {
                    self.opaque.get(f).map(|i| format!("opaque.{i}"))
                }
            }
            Id::Type(t) => {
                let i = self.type_index(t)?;
                Some(format!("types.{i}"))
            }
            Id::Field { ty, name } => {
                let i = self.type_index(ty)?;
                let TypeDecl::Struct { fields, .. } = &self.module.types[i] else {
                    return None;
                };
                let j = fields.iter().position(|f| &f.name == name)?;
                Some(format!("types.{i}.fields.{j}"))
            }
            Id::Node(n) => {
                let (i, path) = self.node_place(n)?;
                Some(format!("funcs.{i}.body.{path}"))
            }
            Id::LiteralField { node, index } => {
                let (i, path) = self.node_place(node)?;
                let n = &self.nodes[&(i, path.clone())];
                (n.kind == Kind::Expr("struct".into()) && *index < n.fields)
                    .then(|| format!("funcs.{i}.body.{path}.fields.{index}.name"))
            }
        }
    }

    /// The id `at` names, when it names something of the module.
    pub fn id(&self, at: &str) -> Option<Id> {
        let (place, _) = self.resolve(at)?;
        Some(match place {
            Place::Func(i) => Id::Func(format!("{}.{}", self.label, self.module.funcs[i].key())),
            Place::Opaque(i) => Id::Func(format!("{}.{}", self.label, self.module.opaque[i].key())),
            Place::Type(i) => Id::Type(self.type_id(i)),
            Place::Field(i, j) => {
                let TypeDecl::Struct { fields, .. } = &self.module.types[i] else {
                    unreachable!("a field place is a struct's")
                };
                Id::Field {
                    ty: self.type_id(i),
                    name: fields[j].name.clone(),
                }
            }
            Place::Node(i, path) => Id::Node(self.node_id(i, &path)),
            Place::LiteralField(i, path, j) => Id::LiteralField {
                node: self.node_id(i, &path),
                index: j,
            },
        })
    }

    /// What `at` names, when it names something of the module.
    pub fn kind(&self, at: &str) -> Option<Kind> {
        self.resolve(at).map(|(_, k)| k)
    }

    /// Every `at` the module has, with what it names, in the order of [`crate::spans::at_order`].
    pub fn ats(&self) -> Vec<(String, Kind)> {
        let m = self.module;
        let mut out = Vec::new();
        for i in 0..m.funcs.len() {
            out.push((format!("funcs.{i}"), Kind::Func));
        }
        for i in 0..m.opaque.len() {
            out.push((format!("opaque.{i}"), Kind::Opaque));
        }
        for (i, t) in m.types.iter().enumerate() {
            out.push((format!("types.{i}"), Kind::Type));
            if let TypeDecl::Struct { fields, .. } = t {
                for j in 0..fields.len() {
                    out.push((format!("types.{i}.fields.{j}"), Kind::Field));
                }
            }
        }
        for ((i, path), n) in &self.nodes {
            out.push((format!("funcs.{i}.body.{path}"), n.kind.clone()));
            for j in 0..n.fields {
                out.push((
                    format!("funcs.{i}.body.{path}.fields.{j}.name"),
                    Kind::LiteralField,
                ));
            }
        }
        out.sort_by(|a, b| crate::spans::at_order(&a.0, &b.0));
        out
    }

    fn type_index(&self, id: &str) -> Option<usize> {
        let name = id.strip_prefix(&self.label)?.strip_prefix('.')?;
        self.types.get(name).copied()
    }

    fn type_id(&self, i: usize) -> String {
        format!("{}.{}", self.label, self.module.types[i].name())
    }

    fn node_id(&self, i: usize, path: &str) -> String {
        format!("{}.{}#{path}", self.label, self.module.funcs[i].key())
    }

    fn node_place(&self, id: &str) -> Option<(usize, String)> {
        let (f, path) = id.split_once('#')?;
        let i = *self.funcs.get(f)?;
        let key = (i, path.to_string());
        self.nodes.contains_key(&key).then_some(key)
    }

    fn resolve(&self, at: &str) -> Option<(Place, Kind)> {
        let m = self.module;
        let mut seg = at.split('.');
        let list = seg.next()?;
        let i = index(seg.next()?)?;
        let rest: Vec<&str> = seg.collect();
        match (list, rest.as_slice()) {
            ("funcs", []) if i < m.funcs.len() => Some((Place::Func(i), Kind::Func)),
            ("opaque", []) if i < m.opaque.len() => Some((Place::Opaque(i), Kind::Opaque)),
            ("types", []) if i < m.types.len() => Some((Place::Type(i), Kind::Type)),
            ("types", ["fields", j]) if i < m.types.len() => {
                let j = index(j)?;
                match &m.types[i] {
                    TypeDecl::Struct { fields, .. } if j < fields.len() => {
                        Some((Place::Field(i, j), Kind::Field))
                    }
                    _ => None,
                }
            }
            ("funcs", ["body", path @ ..]) if i < m.funcs.len() && !path.is_empty() => {
                if let [node @ .., "fields", j, "name"] = path {
                    let j = index(j)?;
                    let node = node.join(".");
                    let n = self.nodes.get(&(i, node.clone()))?;
                    return (n.kind == Kind::Expr("struct".into()) && j < n.fields)
                        .then_some((Place::LiteralField(i, node, j), Kind::LiteralField));
                }
                let path = path.join(".");
                let n = self.nodes.get(&(i, path.clone()))?;
                Some((Place::Node(i, path), n.kind.clone()))
            }
            _ => None,
        }
    }
}

enum Place {
    Func(usize),
    Opaque(usize),
    Type(usize),
    Field(usize, usize),
    Node(usize, String),
    LiteralField(usize, String, usize),
}

/// A list index as a path writes it: decimal, no leading zero.
fn index(s: &str) -> Option<usize> {
    let n: usize = s.parse().ok()?;
    (n.to_string() == s).then_some(n)
}

/// The `source_name(node, text)` records of a span document: one per `name` span, the span's
/// `text` under the [`Id::key`] of what it names, in the document's order.
pub fn source_names(doc: &SpanDoc, ids: &Ids) -> Vec<SourceName> {
    doc.spans
        .iter()
        .filter(|s| s.role == Role::Name)
        .filter_map(|s| {
            Some(SourceName {
                node: ids.id(&s.at)?.key(),
                text: s.text.clone()?,
            })
        })
        .collect()
}
