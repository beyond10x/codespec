//! C types to `cir/1` types, from the spelling clang gives a type (`qualType`, and
//! `desugaredQualType` when a typedef stands in for it).

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::{FloatBits, IntWidth, Type};

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
    /// Each newtype the module declares, a typedef in the source directory of an integer, `_Bool`
    /// or floating-point type, by name, with the type it is defined over.
    pub newtypes: BTreeMap<String, Type>,
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

/// The parameter list of a function type spelled `(…)`, split at its top-level commas.
fn params_of(list: &str) -> Option<Vec<String>> {
    let inner = list.strip_prefix('(')?.strip_suffix(')')?;
    let (mut out, mut depth, mut cur) = (Vec::new(), 0usize, String::new());
    for c in inner.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.checked_sub(1)?,
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
    Some(out)
}

impl Tables {
    /// The `cir/1` type of a clang type object `{qualType, desugaredQualType}`. The spelling
    /// with its typedefs is read when it names a newtype of the module, else the desugared one.
    pub fn ty(&self, unit: &Unit, t: &J) -> Result<Type, String> {
        let q = clean(t.s("qualType"));
        let bare = q.strip_prefix("const ").unwrap_or(&q);
        if let Some(Typedef { named: Some(n), .. }) = unit.typedefs.get(bare) {
            return self.named(n);
        }
        let d = t.s("desugaredQualType");
        let newtype = q
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|w| self.newtypes.contains_key(w));
        self.spelled(unit, if d.is_empty() || newtype { &q } else { d })
    }

    /// The `cir/1` type of the C type spelled `s`.
    pub fn spelled(&self, unit: &Unit, s: &str) -> Result<Type, String> {
        let s = clean(s);
        if s == "const char *" || s == "char const *" {
            return Ok(Type::String);
        }
        if s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|w| w == "volatile" || w == "_Atomic")
        {
            return Err(format!("a volatile or atomic type ({s})"));
        }
        if let Some((result, list)) = s.split_once("(*)") {
            return self.func(unit, result.trim(), list.trim());
        }
        if s.contains("(*") || s.contains("(^") {
            return Err(format!("a pointer to a function pointer or an array ({s})"));
        }
        let s = match s.strip_suffix("*const") {
            Some(x) => format!("{x}*"),
            None => s.strip_suffix(" const").unwrap_or(&s).to_string(),
        };
        if let Some(elem) = s.strip_suffix('*') {
            return match self.spelled(unit, elem.trim())? {
                Type::Unit => Err(format!("a pointer to void ({s})")),
                Type::String => Err(format!("a pointer to a string ({s})")),
                elem => Ok(Type::Ptr {
                    elem: Box::new(elem),
                }),
            };
        }
        if let Some(open) = s.find('[') {
            let close = s[open..].find(']').map(|c| open + c);
            let len = close.and_then(|c| s[open + 1..c].trim().parse::<u64>().ok());
            let (Some(close), Some(len)) = (close, len) else {
                return Err(format!("an array of no constant length ({s})"));
            };
            let elem = format!("{}{}", &s[..open], &s[close + 1..]);
            return match self.spelled(unit, elem.trim())? {
                Type::Unit | Type::String => Err(format!("an array of strings ({s})")),
                elem => Ok(Type::Array {
                    len,
                    elem: Box::new(elem),
                }),
            };
        }
        let s = s.strip_prefix("const ").unwrap_or(&s);
        if s == "void" {
            return Ok(Type::Unit);
        }
        if let Some(t) = builtin(s) {
            return Ok(t);
        }
        match s {
            "float" => {
                return Ok(Type::Float {
                    bits: FloatBits::W32,
                });
            }
            "double" => {
                return Ok(Type::Float {
                    bits: FloatBits::W64,
                });
            }
            "char" => return Err("plain char, whose signedness is the platform's".to_string()),
            "long double" => {
                return Err(
                    "long double, whose width is the platform's and cir/1 does not have".into(),
                );
            }
            _ => {}
        }
        if self.newtypes.contains_key(s) {
            return self.named(s);
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

    /// A function pointer `result (*)(list)`: a `func` type, without results for `void`.
    fn func(&self, unit: &Unit, result: &str, list: &str) -> Result<Type, String> {
        let s = format!("{result} (*){list}");
        if result.contains('(') {
            return Err(format!(
                "a function pointer returning a function pointer ({s})"
            ));
        }
        let Some(list) = params_of(list) else {
            return Err(format!("the function pointer type {s}"));
        };
        let mut params = Vec::new();
        match list.as_slice() {
            [] => {
                return Err(format!(
                    "a function pointer without a prototype ({s}), whose parameters C does not say"
                ));
            }
            [v] if v == "void" => {}
            _ => {
                for p in &list {
                    if p == "..." {
                        return Err(format!("a pointer to a variadic function ({s})"));
                    }
                    match self.spelled(unit, p)? {
                        Type::Unit => return Err(format!("the function pointer type {s}")),
                        t => params.push(t),
                    }
                }
            }
        }
        let results = match self.spelled(unit, result)? {
            Type::Unit => Vec::new(),
            t => vec![t],
        };
        Ok(Type::Func { params, results })
    }

    /// The module type `n`, a struct, enum or newtype the module declares and reads.
    fn named(&self, n: &str) -> Result<Type, String> {
        if let Some(why) = self.refused.get(n) {
            return Err(format!("{n}, which is not read: {why}"));
        }
        if !self.structs.contains_key(n)
            && !self.enums.contains(n)
            && !self.newtypes.contains_key(n)
        {
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
