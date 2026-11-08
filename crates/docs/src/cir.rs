//! The `cir/1` reference page, from `schemas/cir-1.schema.json` (itself generated from
//! `crates/model/src/lib.rs`): the module object, then every definition, a tagged union as one row
//! per kind, a plain object as one row per field.

use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::{Result, SOURCE, cell, front_matter};

/// Where the `cir/1` reference lands, relative to the repository root.
pub const PAGE: &str = "website/docs/reference/cir-1.md";

/// The schema the page is generated from, relative to the repository root.
pub const SCHEMA: &str = "schemas/cir-1.schema.json";

/// The anchor Docusaurus gives the heading `## <name>`.
fn anchor(name: &str) -> String {
    name.to_lowercase()
}

/// A schema node written as a type: `string`, `[Stmt]`, `Expr?`, `"let"`.
fn type_of(node: &Value) -> Result<String> {
    if let Some(reference) = node["$ref"].as_str() {
        let name = reference
            .strip_prefix("#/$defs/")
            .ok_or_else(|| format!("unexpected $ref {reference}"))?;
        return Ok(format!("[`{name}`](#{})", anchor(name)));
    }
    if let Some(value) = node.get("const") {
        return Ok(format!("`{value}`"));
    }
    if let Some(any) = node["anyOf"].as_array() {
        let non_null: Vec<&Value> = any.iter().filter(|n| n["type"] != "null").collect();
        if non_null.len() == 1 && any.len() == 2 {
            return Ok(format!("{} or null", type_of(non_null[0])?));
        }
        return Err(format!("unsupported anyOf {node}"));
    }
    if let Some(types) = node["type"].as_array() {
        let non_null: Vec<&Value> = types.iter().filter(|t| *t != "null").collect();
        if let ([only], 2) = (non_null.as_slice(), types.len()) {
            let mut inner = node.clone();
            inner["type"] = (*only).clone();
            return Ok(format!("{} or null", type_of(&inner)?));
        }
        return Err(format!("unsupported schema node {node}"));
    }
    match node["type"].as_str() {
        Some("array") => Ok(format!("list of {}", type_of(&node["items"])?)),
        Some(simple @ ("string" | "boolean" | "integer" | "number")) => Ok(format!("`{simple}`")),
        _ => Err(format!("unsupported schema node {node}")),
    }
}

fn description(node: &Value) -> String {
    cell(node["description"].as_str().unwrap_or_default())
}

/// One row per field of the object `node`: the field, its type, whether it is required, its
/// description. The `k` tag of a tagged union's member is left out.
fn fields(node: &Value) -> Result<String> {
    let empty = Map::new();
    let properties = node["properties"].as_object().unwrap_or(&empty);
    let required: Vec<&str> = node["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut out = String::from("| Field | Type | Required | Meaning |\n|---|---|---|---|\n");
    for (name, property) in properties {
        out.push_str(&format!(
            "| `{name}` | {} | {} | {} |\n",
            type_of(property)?,
            if required.contains(&name.as_str()) {
                "yes"
            } else {
                "no"
            },
            description(property),
        ));
    }
    Ok(out)
}

/// The fields of one tagged-union member, on one line: `names*: list of string`, required ones
/// marked with a star.
fn inline_fields(node: &Value) -> Result<String> {
    let empty = Map::new();
    let properties = node["properties"].as_object().unwrap_or(&empty);
    let required: Vec<&str> = node["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut parts = Vec::new();
    for (name, property) in properties {
        if name == "k" {
            continue;
        }
        let star = if required.contains(&name.as_str()) {
            "\\*"
        } else {
            ""
        };
        parts.push(format!("`{name}`{star}: {}", type_of(property)?));
    }
    Ok(if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join("; ")
    })
}

/// The section for the definition `name`.
fn definition(name: &str, node: &Value) -> Result<String> {
    let mut out = format!("\n## {name}\n\n");
    let about = description(node);
    if !about.is_empty() {
        out.push_str(&format!("{about}\n\n"));
    }
    if let Some(members) = node["oneOf"].as_array() {
        out.push_str(&format!(
            "A tagged union: the field `k` names the kind. {} kinds.\n\n\
             | `k` | Fields (\\* required) | Meaning |\n|---|---|---|\n",
            members.len()
        ));
        for member in members {
            let kind = member["properties"]["k"]["const"]
                .as_str()
                .ok_or_else(|| format!("{name}: a member has no `k` const"))?;
            out.push_str(&format!(
                "| `{kind}` | {} | {} |\n",
                inline_fields(member)?,
                description(member)
            ));
        }
    } else if let Some(values) = node["enum"].as_array() {
        let values: Vec<String> = values.iter().map(|v| format!("`{v}`")).collect();
        out.push_str(&format!("A string, one of {}.\n", values.join(", ")));
    } else if node["type"] == "object" {
        out.push_str(&fields(node)?);
    } else {
        return Err(format!("{name}: unsupported definition {node}"));
    }
    Ok(out)
}

/// The `cir/1` reference page for the schema `schema`.
pub fn page_for(schema: &Value) -> Result<String> {
    let defs = schema["$defs"]
        .as_object()
        .ok_or("the schema has no $defs")?;
    let mut out = front_matter(
        "cir/1 reference",
        "cir/1",
        "Every object, kind and field of the cir/1 format, generated from its JSON Schema.",
        2,
    );
    out.push_str(&format!(
        "# `cir/1`\n\nGenerated by `codespec-docs` from [`{SCHEMA}`]({SOURCE}/{SCHEMA}), which \
         `codespec schema` prints from the model in \
         [`crates/model/src/lib.rs`]({SOURCE}/crates/model/src/lib.rs). The model is the \
         definition; the schema and this page follow it. Every object refuses fields it does not \
         list. Fields are listed in alphabetical order, as the schema lists them; the canonical \
         encoding writes them in the model's declaration order. Why the format looks like this: \
         [The cir/1 model](/docs/concepts/cir-1).\n\n## Module\n\n{}\n\n{}",
        description(schema),
        fields(schema)?,
    ));
    out.push_str(&format!(
        "\n## Definitions\n\n{} definitions: {}.\n",
        defs.len(),
        defs.keys()
            .map(|name| format!("[`{name}`](#{})", anchor(name)))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for (name, node) in defs {
        out.push_str(&definition(name, node)?);
    }
    Ok(out)
}

/// The `cir/1` reference page for the repository at `root`.
pub fn page(root: &Path) -> Result<String> {
    let text =
        fs::read_to_string(root.join(SCHEMA)).map_err(|e| format!("reading {SCHEMA}: {e}"))?;
    let schema: Value = serde_json::from_str(&text).map_err(|e| format!("{SCHEMA}: {e}"))?;
    page_for(&schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_statement_and_expression_kind_is_on_the_page() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let page = page(&root).unwrap();
        assert!(page.contains("# generated by codespec-docs, do not edit"));
        for kind in [
            "let",
            "var",
            "assign",
            "if",
            "for_range",
            "switch",
            "return",
            "send",
            "lookup_else",
            "foreign",
            "lit",
            "binop",
            "to_error",
            "union",
            "newtype",
        ] {
            assert!(page.contains(&format!("| `{kind}` |")), "{kind}");
        }
        assert!(page.contains("\n## Stmt\n"));
        assert!(page.contains("| `facts` | [`Facts`](#facts) or null | no |"));
        assert!(page.contains("A string, one of `\"platform\"`, `\"8\"`"));
    }

    #[test]
    fn an_unknown_schema_shape_is_refused() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"x": {"type": "object"}},
            "$defs": {}
        });
        assert!(page_for(&schema).is_err());
    }
}
