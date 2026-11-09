//! C types to `cir/1` types, from the spelling clang gives a type (`qualType`, and
//! `desugaredQualType` when a typedef stands in for it).

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::{IntWidth, Type};

use crate::json::J;

/// What the module declares, over every file of the source directory.
#[derive(Default)]
pub struct Tables {
    /// Each struct the module declares, by name, with its field names in order.
    pub structs: BTreeMap<String, Vec<String>>,
    /// Each enum type the module declares, by name.
    pub enums: BTreeSet<String>,
    /// Each struct, union or enum the module declares but could not read, with why.
    pub refused: BTreeMap<String, String>,
    /// Each function the module defines, by name.
    pub funcs: BTreeSet<String>,
    /// Each file-scope variable the module defines, by name, with its type.
    pub globals: BTreeMap<String, Type>,
    /// The constants of anonymous enums, by name, typed `int`.
    pub consts: BTreeSet<String>,
}

/// One typedef of a file: the type it names, spelled, and the module type it names when that
/// type has no tag of its own (`typedef struct { … } P;`).
pub struct Typedef {
    pub target: String,
    pub named: Option<String>,
}

/// What one translation unit adds to resolve names and types: its typedefs, the typedef name of
/// each tag declared without one, each enum constant's enum, and each declaration by id.
#[derive(Default)]
pub struct Unit {
    pub typedefs: BTreeMap<String, Typedef>,
    /// Record or enum declaration id to the typedef name that names it.
    pub anon: BTreeMap<String, String>,
    /// Enum constant declaration id to its enum type's name; `None` for an anonymous enum's.
    pub enum_of: BTreeMap<String, Option<String>>,
    /// File-scope variable declaration id to its name.
    pub globals: BTreeMap<String, String>,
}

pub fn int(width: IntWidth, signed: bool) -> Type {
    Type::Int { width, signed }
}

/// `s` without nullability and `restrict` qualifiers, spaces collapsed.
fn clean(s: &str) -> String {
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| {
            !matches!(
                *w,
                "_Nonnull" | "_Nullable" | "_Null_unspecified" | "restrict" | "__restrict"
            )
        })
        .collect();
    words
        .join(" ")
        .replace("*restrict", "*")
        .replace("*__restrict", "*")
}

/// The builtin integer and boolean types, by clang's spelling; LP64 widths.
fn builtin(s: &str) -> Option<Type> {
    use IntWidth::*;
    Some(match s {
        "_Bool" | "bool" => Type::Bool,
        "signed char" => int(W8, true),
        "unsigned char" => int(W8, false),
        "short" | "short int" | "signed short" | "signed short int" => int(W16, true),
        "unsigned short" | "unsigned short int" => int(W16, false),
        "int" | "signed int" | "signed" => int(W32, true),
        "unsigned int" | "unsigned" => int(W32, false),
        "long" | "long int" | "signed long" | "long long" | "long long int"
        | "signed long long" => int(W64, true),
        "unsigned long" | "unsigned long int" | "unsigned long long" | "unsigned long long int" => {
            int(W64, false)
        }
        _ => return None,
    })
}

impl Tables {
    /// The `cir/1` type of a clang type object `{qualType, desugaredQualType}`.
    pub fn ty(&self, unit: &Unit, t: &J) -> Result<Type, String> {
        let q = clean(t.s("qualType"));
        let bare = q.strip_prefix("const ").unwrap_or(&q);
        if let Some(Typedef { named: Some(n), .. }) = unit.typedefs.get(bare) {
            return self.named(n);
        }
        let d = t.s("desugaredQualType");
        self.spelled(unit, if d.is_empty() { &q } else { d })
    }

    /// The `cir/1` type of the C type spelled `s`.
    pub fn spelled(&self, unit: &Unit, s: &str) -> Result<Type, String> {
        let s = clean(s);
        if s == "const char *" || s == "char const *" {
            return Ok(Type::String);
        }
        if s.contains("(*") || s.contains("(^") {
            return Err("a function pointer".into());
        }
        if s.ends_with('*') {
            return Err(format!("a pointer type ({s})"));
        }
        if s.contains('[') {
            return Err(format!("an array type ({s})"));
        }
        if s.split_whitespace()
            .any(|w| w == "volatile" || w == "_Atomic")
        {
            return Err(format!("a volatile or atomic type ({s})"));
        }
        let s = s.strip_prefix("const ").unwrap_or(&s);
        let s = s.strip_suffix(" const").unwrap_or(s);
        if s == "void" {
            return Ok(Type::Unit);
        }
        if let Some(t) = builtin(s) {
            return Ok(t);
        }
        if matches!(s, "char" | "float" | "double" | "long double") {
            return Err(match s {
                "char" => "plain char, whose signedness is the platform's".to_string(),
                _ => format!("the floating-point type {s}"),
            });
        }
        for tag in ["struct ", "enum ", "union "] {
            if let Some(name) = s.strip_prefix(tag) {
                if let Some(why) = self.refused.get(name) {
                    return Err(format!("{s}, which is not read: {why}"));
                }
                if tag == "union " {
                    return Err(format!("a union ({s})"));
                }
                let known = if tag == "struct " {
                    self.structs.contains_key(name)
                } else {
                    self.enums.contains(name)
                };
                if !known {
                    return Err(format!("{s}, which the source directory does not declare"));
                }
                return Ok(Type::Named {
                    name: name.into(),
                    targs: Vec::new(),
                });
            }
        }
        if let Some(td) = unit.typedefs.get(s) {
            if let Some(n) = &td.named {
                return self.named(n);
            }
            return self.spelled(unit, &td.target.clone());
        }
        Err(format!("the type {s}"))
    }

    /// The module type `n`, a struct or enum the module declares and reads.
    fn named(&self, n: &str) -> Result<Type, String> {
        if let Some(why) = self.refused.get(n) {
            return Err(format!("{n}, which is not read: {why}"));
        }
        if !self.structs.contains_key(n) && !self.enums.contains(n) {
            return Err(format!("{n}, which the source directory does not declare"));
        }
        Ok(Type::Named {
            name: n.into(),
            targs: Vec::new(),
        })
    }

    /// The result type of a function type spelled `s` (`long (long, long)`).
    pub fn result(&self, unit: &Unit, s: &str) -> Result<Type, String> {
        let s = s.trim();
        let Some(body) = s.strip_suffix(')') else {
            return Err(format!("the function type {s}"));
        };
        let mut depth = 1;
        let mut open = None;
        for (i, c) in body.char_indices().rev() {
            match c {
                ')' => depth += 1,
                '(' => {
                    depth -= 1;
                    if depth == 0 {
                        open = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(open) = open else {
            return Err(format!("the function type {s}"));
        };
        let result = s[..open].trim();
        if result.contains('(') {
            return Err("a function returning a function pointer".into());
        }
        self.spelled(unit, result)
    }
}
