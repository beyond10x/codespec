//! The runtime form: C that includes the kit's `codespec_rt.h` (`conformance/c/codespec_rt.h`,
//! the header `codespec-write-c` writes) and spells what C lacks through it. The forms read, and
//! where their types come from, are the crate doc's section "C in runtime form".

use std::collections::BTreeMap;

use codespec_model::{ChanDir, IntWidth, Type};

use crate::types::{Tables, int};

/// The runtime header the reader knows: a source directory's `codespec_rt.h` must equal it.
pub const RT: &str = include_str!("../../../conformance/c/codespec_rt.h");

/// The file name of the runtime header.
pub const RT_FILE: &str = "codespec_rt.h";

/// One `/* codespec: func <name>(<params>) <results> */` line, in Go's spelling of the types.
#[derive(Clone, Debug)]
pub struct SigLine {
    pub params: Vec<String>,
    pub results: Vec<String>,
}

/// `s` split at the commas outside brackets and parentheses, each part trimmed.
fn split_top(s: &str) -> Vec<String> {
    let (mut out, mut depth, mut cur) = (Vec::new(), 0i32, String::new());
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

/// The `codespec: func` lines of `text`, by function name; a line that does not read is an
/// error naming it.
pub fn sig_lines(text: &str, out: &mut BTreeMap<String, SigLine>) -> Result<(), String> {
    for line in text.lines() {
        let Some(f) = line
            .trim()
            .strip_prefix("/* codespec: func ")
            .and_then(|r| r.strip_suffix("*/"))
        else {
            continue;
        };
        let bad = || format!("the line {line:?}, which does not read as a codespec: func line");
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
        let params = split_top(&rest[..end]);
        let res = rest[end + 1..].trim();
        let results = if res.is_empty() {
            Vec::new()
        } else if let Some(inner) = res.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            split_top(inner)
        } else {
            vec![res.to_string()]
        };
        let name = name.trim().to_string();
        if out
            .insert(name.clone(), SigLine { params, results })
            .is_some()
        {
            return Err(format!("a second codespec: func line of {name}"));
        }
    }
    Ok(())
}

/// The runtime's type of a C type spelled `s` (no `const`): `Some` for a name of the runtime,
/// an error for one no form the reader reads holds or whose element type the spelling lacks.
pub fn rt_type(s: &str) -> Option<Result<Type, String>> {
    let s = s.trim();
    Some(match s {
        "int64_t" => Ok(int(IntWidth::Platform, true)),
        "cs_string" | "struct cs_string" => Ok(Type::String),
        "cs_error" | "struct cs_error_s *" => Ok(Type::Error),
        "cs_any" | "struct cs_any" => Ok(Type::Any),
        _ => match container(s) {
            Some(c) => Err(format!(
                "a {c}, whose element type is the one a codespec: func line or a use gives it"
            )),
            None if s.starts_with("cs_") || s.starts_with("struct cs_") => Err(format!(
                "the runtime's {s}, which no form the reader reads holds"
            )),
            None => return None,
        },
    })
}

/// The runtime container a C type spelled `s` is: `cs_slice`, `cs_map *` or `cs_chan *`.
pub fn container(s: &str) -> Option<&'static str> {
    let s = s.trim();
    let s = s.strip_prefix("const ").unwrap_or(s);
    match s.replace("struct ", "").as_str() {
        "cs_slice" => Some("cs_slice"),
        "cs_map *" => Some("cs_map *"),
        "cs_chan *" => Some("cs_chan *"),
        _ => None,
    }
}

/// The C type of the runtime a value of `t` has, for the types the runtime holds.
pub fn c_form(t: &Type) -> Option<&'static str> {
    Some(match t {
        Type::String => "cs_string",
        Type::Error => "cs_error",
        Type::Any => "cs_any",
        Type::Slice { .. } => "cs_slice",
        Type::Map { .. } => "cs_map *",
        Type::Chan { .. } => "cs_chan *",
        _ => return None,
    })
}

/// The runtime's `CS_KEY_*` of a map key type.
pub fn key_kind(t: &Type) -> Option<&'static str> {
    Some(match t {
        Type::Int { width, signed } => match (width, signed) {
            (IntWidth::W8, true) => "CS_KEY_I8",
            (IntWidth::W16, true) => "CS_KEY_I16",
            (IntWidth::W32, true) => "CS_KEY_I32",
            (_, true) => "CS_KEY_I64",
            (IntWidth::W8, false) => "CS_KEY_U8",
            (IntWidth::W16, false) => "CS_KEY_U16",
            (IntWidth::W32, false) => "CS_KEY_U32",
            (_, false) => "CS_KEY_U64",
        },
        Type::Bool => "CS_KEY_BOOL",
        Type::String => "CS_KEY_STRING",
        _ => return None,
    })
}

/// The runtime's `CS_K_*` of a type an `any` holds, for those a `cs_type` describes without an
/// element.
pub fn any_kind(t: &Type) -> Option<&'static str> {
    Some(match t {
        Type::Int { width, signed } => match (width, signed) {
            (IntWidth::W8, true) => "CS_K_INT8",
            (IntWidth::W16, true) => "CS_K_INT16",
            (IntWidth::W32, true) => "CS_K_INT32",
            (_, true) => "CS_K_INT64",
            (IntWidth::W8, false) => "CS_K_UINT8",
            (IntWidth::W16, false) => "CS_K_UINT16",
            (IntWidth::W32, false) => "CS_K_UINT32",
            (_, false) => "CS_K_UINT64",
        },
        Type::Bool => "CS_K_BOOL",
        Type::String => "CS_K_STRING",
        _ => return None,
    })
}

impl Tables {
    /// The `cir/1` type of a type of a `codespec:` line, in Go's spelling: `int` is the C
    /// `int64_t` the runtime form writes for it.
    pub fn go(&self, s: &str) -> Result<Type, String> {
        let s = s.trim();
        let ints = [
            ("int", IntWidth::Platform, true),
            ("int8", IntWidth::W8, true),
            ("int16", IntWidth::W16, true),
            ("int32", IntWidth::W32, true),
            ("int64", IntWidth::W64, true),
            ("uint", IntWidth::Platform, false),
            ("uint8", IntWidth::W8, false),
            ("uint16", IntWidth::W16, false),
            ("uint32", IntWidth::W32, false),
            ("uint64", IntWidth::W64, false),
        ];
        if let Some((_, w, signed)) = ints.iter().find(|(n, _, _)| *n == s) {
            return Ok(int(*w, *signed));
        }
        match s {
            "bool" => return Ok(Type::Bool),
            "string" => return Ok(Type::String),
            "error" => return Ok(Type::Error),
            "any" => return Ok(Type::Any),
            _ => {}
        }
        if let Some(e) = s.strip_prefix("[]") {
            return Ok(Type::Slice {
                elem: Box::new(self.go(e)?),
            });
        }
        if let Some(rest) = s.strip_prefix("map[") {
            let mut depth = 1;
            for (i, c) in rest.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            let key = self.go(&rest[..i])?;
                            if key_kind(&key).is_none() {
                                return Err(format!("a map keyed by {}", &rest[..i]));
                            }
                            return Ok(Type::Map {
                                key: Box::new(key),
                                elem: Box::new(self.go(&rest[i + 1..])?),
                            });
                        }
                    }
                    _ => {}
                }
            }
            return Err(format!("the type {s}"));
        }
        if let Some(e) = s.strip_prefix("chan<- ") {
            return Ok(Type::Chan {
                elem: Box::new(self.go(e)?),
                dir: ChanDir::Send,
            });
        }
        if let Some(e) = s.strip_prefix("chan ") {
            return Ok(Type::Chan {
                elem: Box::new(self.go(e)?),
                dir: ChanDir::Both,
            });
        }
        if let Some(e) = s.strip_prefix('*') {
            return Ok(Type::Ptr {
                elem: Box::new(self.go(e)?),
            });
        }
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return self.module_type(s);
        }
        Err(format!("the type {s} of a codespec: line"))
    }
}
