//! The one module of the crate that starts processes and touches files; the others are pure.
//!
//! [`read_spanned`] runs a reader with `--spans` and checks both documents it gives.

use std::path::Path;
use std::process::Command;

use codespec_model::{MAX_NESTING, Module};
use serde_json::Value;

use crate::ids::{Ids, label};
use crate::spans::{self, FORMAT, SpanDoc, sha256_hex};

/// The fields of a reader's `--spans` output.
const OUTPUT_KEYS: &[&str] = &["cir", "files", "spans", "unspanned", "refs"];

/// The stack of the thread a reader's output is parsed on: room for a `cir/1` document of
/// [`MAX_NESTING`] levels, as `Module::from_json` reads it.
const PARSE_STACK: usize = 64 << 20;

/// A module read with spans: the `cir/1` document and its span document, both strictly read.
#[derive(Debug, Clone)]
pub struct Spanned {
    pub cir: Module,
    pub spans: SpanDoc,
}

impl Spanned {
    /// The canonical encoding of the `cir/1` document, the bytes `cir_sha256` is of.
    pub fn cir_json(&self) -> String {
        self.cir.to_json()
    }

    /// The canonical encoding of the span document.
    pub fn spans_json(&self) -> String {
        self.spans.to_json()
    }
}

/// The language a reader program reads: its file name after `codespec-read-`.
pub fn reader_lang(reader: &Path) -> String {
    let name = reader
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.strip_prefix("codespec-read-")
        .map(str::to_string)
        .unwrap_or(name)
}

/// Run `reader --spans dir`, take its object `{cir, files, spans, unspanned, refs}`, strictly
/// read the `cir/1` module, add `format`, `module` and `cir_sha256`, and strictly read the span
/// document against the module and the files under `dir`. A reader that fails on `--spans` or
/// prints a bare module is refused: "the <lang> reader gives no spans".
pub fn read_spanned(reader: &Path, dir: &Path) -> Result<Spanned, String> {
    let lang = reader_lang(reader);
    let no_spans = |why: String| format!("the {lang} reader gives no spans: {why}");
    let output = Command::new(reader)
        .arg("--spans")
        .arg(dir)
        .output()
        .map_err(|e| format!("{}: {e}", reader.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        return Err(no_spans(format!(
            "{} --spans exited with {}: {first}",
            reader.display(),
            output.status
        )));
    }
    let split = split(&output.stdout).map_err(|e| format!("{}: {e}", reader.display()))?;
    let Some((cir, rest)) = split else {
        return Err(no_spans(format!(
            "{} --spans printed a bare cir/1 module",
            reader.display()
        )));
    };
    if let Some(k) = rest.keys().find(|k| !OUTPUT_KEYS.contains(&k.as_str())) {
        return Err(format!(
            "{}: the --spans output has an unknown field {k:?}",
            reader.display()
        ));
    }
    let module = Module::from_json(&cir).map_err(|e| format!("{}: cir: {e}", reader.display()))?;
    let mut doc = serde_json::Map::new();
    doc.insert("format".into(), FORMAT.into());
    doc.insert("module".into(), label(&module).into());
    doc.insert(
        "cir_sha256".into(),
        sha256_hex(module.to_json().as_bytes()).into(),
    );
    for k in &OUTPUT_KEYS[1..] {
        if let Some(v) = rest.get(*k) {
            doc.insert(k.to_string(), v.clone());
        }
    }
    let ids = Ids::new(&module);
    let source = |path: &str| std::fs::read(dir.join(path)).map_err(|e| e.to_string());
    let spans = spans::read_value(&Value::Object(doc), &ids, &source)
        .map_err(|e| format!("{}: {e}", reader.display()))?;
    drop(ids);
    Ok(Spanned { cir: module, spans })
}

/// A reader's output split into the bytes of its `cir` member and its other members; `None` when
/// it is a `cir/1` module itself. The document may nest as deep as a `cir/1` document does, so it
/// is parsed, and its `cir` member written and dropped, on a thread with room for that.
type Split = Option<(Vec<u8>, serde_json::Map<String, Value>)>;

fn split(bytes: &[u8]) -> Result<Split, String> {
    let depth = nesting(bytes);
    if depth > MAX_NESTING + 1 {
        return Err(format!(
            "output nested {depth} deep, past the {MAX_NESTING} cir/1 reading admits"
        ));
    }
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .name("spans-read".into())
            .stack_size(PARSE_STACK)
            .spawn_scoped(s, || {
                let mut de = serde_json::Deserializer::from_slice(bytes);
                de.disable_recursion_limit();
                let v: Value = serde::Deserialize::deserialize(&mut de)
                    .map_err(|e| format!("output is not JSON: {e}"))?;
                de.end()
                    .map_err(|e| format!("output is not one JSON value: {e}"))?;
                let Value::Object(mut obj) = v else {
                    return Err("output is not a JSON object".to_string());
                };
                if obj.contains_key("format") {
                    return Ok(None);
                }
                let cir = obj
                    .remove("cir")
                    .ok_or("the --spans output has no cir member")?;
                let cir = serde_json::to_vec(&cir).expect("a JSON value always serializes");
                Ok(Some((cir, obj)))
            })
            .expect("a thread to parse the reader's output on")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// The deepest nesting of arrays and objects in the JSON text `bytes`, brackets inside strings not
/// counted.
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
            b'[' | b'{' => {
                depth += 1;
                max = max.max(depth);
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max
}
