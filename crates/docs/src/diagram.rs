//! An SVG tree of one `cir/1` module: module, then its types, sentinel errors and functions, each
//! function's statements, and each statement's expressions, every node labelled with its kind and
//! its type. Nested expressions are collapsed into one line of source-like text, so the tree stays
//! five levels deep. The SVG is written by hand (no dependency); the same module gives the same
//! bytes.

use codespec_model::{Expr, Func, Module, Stmt, Type, TypeDecl};

/// Characters per label line before it wraps.
const WRAP: usize = 58;
/// How deep a collapsed expression is spelled out before it becomes `…`.
const SPELL_DEPTH: usize = 3;

const FONT: f64 = 12.0;
const CHAR: f64 = 7.3;
const LINE: f64 = 16.0;
const PAD_X: f64 = 10.0;
const PAD_Y: f64 = 6.0;
const INDENT: f64 = 30.0;
const GAP: f64 = 8.0;
const MARGIN: f64 = 16.0;

/// One box in the tree.
struct Node {
    depth: usize,
    /// The level, which picks the colour: `module`, `group`, `decl`, `stmt`, `expr`.
    level: &'static str,
    label: String,
}

/// `t` written as Go-like text: `int`, `*Greeter`, `map[string]int`, `(string, error)`.
pub fn type_text(t: &Type) -> String {
    match t {
        Type::Int { width, signed } => {
            let width = serde_json::to_value(width)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let base = if *signed { "int" } else { "uint" };
            if width == "platform" {
                base.to_owned()
            } else {
                format!("{base}{width}")
            }
        }
        Type::Bool => "bool".to_owned(),
        Type::String => "string".to_owned(),
        Type::Error => "error".to_owned(),
        Type::Any => "any".to_owned(),
        Type::Unit => "()".to_owned(),
        Type::Named { name } => name.clone(),
        Type::Ptr { elem } => format!("*{}", type_text(elem)),
        Type::Slice { elem } => format!("[]{}", type_text(elem)),
        Type::Map { key, elem } => format!("map[{}]{}", type_text(key), type_text(elem)),
        Type::Chan { elem, .. } => format!("chan {}", type_text(elem)),
        Type::Tuple { elems } => format!(
            "({})",
            elems.iter().map(type_text).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn kind(e: &Expr) -> &'static str {
    match e {
        Expr::Lit { .. } => "lit",
        Expr::Nil { .. } => "nil",
        Expr::Var { .. } => "var",
        Expr::Const { .. } => "const",
        Expr::Errvar { .. } => "errvar",
        Expr::Field { .. } => "field",
        Expr::Index { .. } => "index",
        Expr::Call { .. } => "call",
        Expr::Method { .. } => "method",
        Expr::Binop { .. } => "binop",
        Expr::Unop { .. } => "unop",
        Expr::Struct { .. } => "struct",
        Expr::Conv { .. } => "conv",
        Expr::Make { .. } => "make",
        Expr::Zero { .. } => "zero",
        Expr::Intrinsic { .. } => "intrinsic",
        Expr::ToError { .. } => "to_error",
    }
}

fn args(list: &[Expr], depth: usize) -> String {
    list.iter()
        .map(|a| spell(a, depth + 1))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `e` as one line of source-like text, nested expressions past [`SPELL_DEPTH`] as `…`.
fn spell(e: &Expr, depth: usize) -> String {
    if depth > SPELL_DEPTH {
        return "…".to_owned();
    }
    let d = depth + 1;
    match e {
        Expr::Lit { value, .. } => value.clone(),
        Expr::Nil { .. } => "nil".to_owned(),
        Expr::Var { name, .. } | Expr::Const { name, .. } | Expr::Errvar { name, .. } => {
            name.clone()
        }
        Expr::Field { x, name, .. } => format!("{}.{name}", spell(x, d)),
        Expr::Index { x, index, .. } => format!("{}[{}]", spell(x, d), spell(index, d)),
        Expr::Call { name, args: a, .. } | Expr::Intrinsic { name, args: a, .. } => {
            format!("{name}({})", args(a, depth))
        }
        Expr::Method {
            x, name, args: a, ..
        } => format!("{}.{name}({})", spell(x, d), args(a, depth)),
        Expr::Binop { op, x, y, .. } => format!("{} {op} {}", spell(x, d), spell(y, d)),
        Expr::Unop { op, x, .. } => format!("{op}{}", spell(x, d)),
        Expr::Struct { name, fields, .. } => format!(
            "{name}{{{}}}",
            fields
                .iter()
                .map(|f| format!("{}: {}", f.name, spell(&f.value, d)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Conv { name, x, .. } => format!("{name}({})", spell(x, d)),
        Expr::Make { t } => format!("make({})", type_text(t)),
        Expr::Zero { t } => format!("zero {}", type_text(t)),
        Expr::ToError { x, .. } => format!("to_error({})", spell(x, d)),
    }
}

fn expr_label(e: &Expr) -> String {
    format!("{} · {} : {}", kind(e), spell(e, 0), type_text(e.t()))
}

/// The statement's label and the expressions shown as its children.
fn stmt_parts(s: &Stmt) -> (String, Vec<&Expr>, Vec<&Stmt>) {
    match s {
        Stmt::Let { names, rhs, .. } => (
            format!("let {}", names.join(", ")),
            rhs.iter().collect(),
            vec![],
        ),
        Stmt::Var { name, t, .. } => (format!("var {name} : {}", type_text(t)), vec![], vec![]),
        Stmt::Assign { op, lhs, rhs, .. } => (
            format!(
                "assign {} {}",
                lhs.iter()
                    .map(|e| spell(e, 1))
                    .collect::<Vec<_>>()
                    .join(", "),
                if op.is_empty() { "=" } else { op }
            ),
            rhs.iter().collect(),
            vec![],
        ),
        Stmt::Incr { x, .. } => ("incr".to_owned(), vec![x], vec![]),
        Stmt::Decr { x, .. } => ("decr".to_owned(), vec![x], vec![]),
        Stmt::If {
            cond,
            then,
            otherwise,
            ..
        } => (
            "if".to_owned(),
            vec![cond],
            then.iter().chain(otherwise.iter()).collect(),
        ),
        Stmt::ForRange {
            key,
            val,
            over,
            body,
            ..
        } => (
            format!("for_range {key}, {val}"),
            vec![over],
            body.iter().collect(),
        ),
        Stmt::For {
            init,
            cond,
            post,
            body,
            ..
        } => (
            {
                let parts: Vec<&str> = [
                    (init.is_some(), "init"),
                    (cond.is_some(), "cond"),
                    (post.is_some(), "post"),
                ]
                .into_iter()
                .filter_map(|(on, name)| on.then_some(name))
                .collect();
                if parts.is_empty() {
                    "for".to_owned()
                } else {
                    format!("for ({})", parts.join("; "))
                }
            },
            cond.iter().collect(),
            init.iter()
                .map(|s| &**s)
                .chain(body.iter())
                .chain(post.iter().map(|s| &**s))
                .collect(),
        ),
        Stmt::Break { .. } => ("break".to_owned(), vec![], vec![]),
        Stmt::Continue { .. } => ("continue".to_owned(), vec![], vec![]),
        Stmt::Switch { tag, cases, .. } => (
            format!("switch, {} cases", cases.len()),
            tag.iter().collect(),
            cases.iter().flat_map(|c| c.body.iter()).collect(),
        ),
        Stmt::Return { results, .. } => ("return".to_owned(), results.iter().collect(), vec![]),
        Stmt::Expr { x, .. } => ("expr".to_owned(), vec![x], vec![]),
        Stmt::Send { chan, x, .. } => (format!("send on {}", spell(chan, 1)), vec![x], vec![]),
        Stmt::LookupElse {
            name,
            map,
            index,
            then,
            ..
        } => (
            format!(
                "lookup_else {name} = {}[{}], else:",
                spell(map, 1),
                spell(index, 1)
            ),
            vec![],
            then.iter().collect(),
        ),
        Stmt::Foreign { lang, reason, .. } => (format!("foreign {lang}: {reason}"), vec![], vec![]),
    }
}

fn push_stmt(nodes: &mut Vec<Node>, s: &Stmt, depth: usize) {
    let (label, exprs, children) = stmt_parts(s);
    nodes.push(Node {
        depth,
        level: "stmt",
        label,
    });
    for e in exprs {
        nodes.push(Node {
            depth: depth + 1,
            level: "expr",
            label: expr_label(e),
        });
    }
    for child in children {
        push_stmt(nodes, child, depth + 1);
    }
}

fn func_label(f: &Func) -> String {
    let params = f
        .params
        .iter()
        .map(|p| format!("{} {}", p.name, type_text(&p.t)))
        .collect::<Vec<_>>()
        .join(", ");
    let results = match f.results.as_slice() {
        [] => String::new(),
        [one] => format!(" → {}", type_text(one)),
        many => format!(
            " → ({})",
            many.iter().map(type_text).collect::<Vec<_>>().join(", ")
        ),
    };
    format!("func {}({params}){results}", f.key())
}

fn type_decl_label(t: &TypeDecl) -> String {
    match t {
        TypeDecl::Struct { name, fields, .. } => format!(
            "struct {name} {{ {} }}",
            fields
                .iter()
                .map(|f| format!("{} {}", f.name, type_text(&f.t)))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        TypeDecl::Newtype { name, under, .. } => format!("newtype {name} : {}", type_text(under)),
        TypeDecl::Enum {
            name,
            under,
            variants,
            ..
        } => format!(
            "enum {name} : {}, {} variants",
            type_text(under),
            variants.len()
        ),
        TypeDecl::Union { name, variants, .. } => {
            format!("union {name}, {} variants", variants.len())
        }
    }
}

fn nodes(m: &Module) -> Vec<Node> {
    let mut out = vec![Node {
        depth: 0,
        level: "module",
        label: format!(
            "module {} · {} · read by {} {}",
            m.name, m.format, m.reader.name, m.reader.version
        ),
    }];
    let group = |out: &mut Vec<Node>, label: String| {
        out.push(Node {
            depth: 1,
            level: "group",
            label,
        });
    };
    if !m.types.is_empty() {
        group(&mut out, format!("types ({})", m.types.len()));
        for t in &m.types {
            out.push(Node {
                depth: 2,
                level: "decl",
                label: type_decl_label(t),
            });
        }
    }
    if !m.errors.is_empty() {
        group(&mut out, format!("errors ({})", m.errors.len()));
        for e in &m.errors {
            out.push(Node {
                depth: 2,
                level: "decl",
                label: format!("{} = \"{}\"", e.name, e.message),
            });
        }
    }
    group(&mut out, format!("funcs ({})", m.funcs.len()));
    for f in &m.funcs {
        out.push(Node {
            depth: 2,
            level: "decl",
            label: func_label(f),
        });
        for s in &f.body {
            push_stmt(&mut out, s, 3);
        }
    }
    if let Some(facts) = &m.facts {
        let mut listed = Vec::new();
        for (name, items) in [
            ("shared", &facts.shared),
            ("messages", &facts.messages),
            ("comma_ok", &facts.comma_ok),
            ("fallible", &facts.fallible),
            ("read_only", &facts.read_only),
            ("displays", &facts.displays),
            ("unordered_iteration", &facts.unordered_iteration),
        ] {
            if !items.is_empty() {
                listed.push(format!("{name}: {}", items.join(", ")));
            }
        }
        group(&mut out, "facts".to_owned());
        for line in listed {
            out.push(Node {
                depth: 2,
                level: "decl",
                label: line,
            });
        }
    }
    if !m.refusals.is_empty() {
        group(&mut out, format!("refusals ({})", m.refusals.len()));
        for r in &m.refusals {
            out.push(Node {
                depth: 2,
                level: "decl",
                label: format!("{} {}: {}", r.pos, r.what, r.reason),
            });
        }
    }
    out
}

/// `label` split into lines of at most [`WRAP`] characters, at spaces where it can be.
fn wrap(label: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in label.split(' ') {
        let mut word = word.to_owned();
        while word.chars().count() > WRAP {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            let head: String = word.chars().take(WRAP).collect();
            word = word.chars().skip(WRAP).collect();
            lines.push(head);
        }
        let len = line.chars().count();
        if len > 0 && len + 1 + word.chars().count() > WRAP {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&word);
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn num(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}

/// The SVG for `m`, titled `title`.
pub fn svg(m: &Module, title: &str) -> String {
    let nodes = nodes(m);
    let mut boxes = Vec::new();
    let mut y = MARGIN;
    let mut width: f64 = 0.0;
    for node in &nodes {
        let lines = wrap(&node.label);
        let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let w = longest as f64 * CHAR + 2.0 * PAD_X;
        let h = lines.len() as f64 * LINE + 2.0 * PAD_Y;
        let x = MARGIN + node.depth as f64 * INDENT;
        width = width.max(x + w + MARGIN);
        boxes.push((x, y, w, h, lines));
        y += h + GAP;
    }
    let height = y - GAP + MARGIN;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-labelledby=\"title\">\n\
         <title id=\"title\">{}</title>\n\
         <style>\n\
         text {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: {font}px; fill: #1b1f24; }}\n\
         .edge {{ stroke: #8a929c; stroke-width: 1.2; fill: none; }}\n\
         .module {{ fill: #d7ecdf; stroke: #2f7d4f; }}\n\
         .group {{ fill: #eceff3; stroke: #6b7480; }}\n\
         .decl {{ fill: #dde8f7; stroke: #3a6db0; }}\n\
         .stmt {{ fill: #f6e7cf; stroke: #a8701f; }}\n\
         .expr {{ fill: #f1e2f3; stroke: #8a4a92; }}\n\
         rect {{ stroke-width: 1; }}\n\
         </style>\n",
        escape(title),
        w = num(width),
        h = num(height),
        font = num(FONT),
    );
    for (i, node) in nodes.iter().enumerate() {
        if node.depth == 0 {
            continue;
        }
        let Some(parent) = (0..i).rev().find(|&j| nodes[j].depth + 1 == node.depth) else {
            continue;
        };
        let (px, py, _, ph, _) = &boxes[parent];
        let (cx, cy, _, ch, _) = &boxes[i];
        let stem = px + 12.0;
        let mid = cy + ch / 2.0;
        out.push_str(&format!(
            "<path class=\"edge\" d=\"M{} {}V{}H{}\"/>\n",
            num(stem),
            num(py + ph),
            num(mid),
            num(*cx)
        ));
    }
    for (node, (x, y, w, h, lines)) in nodes.iter().zip(&boxes) {
        out.push_str(&format!(
            "<rect class=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"5\"/>\n",
            node.level,
            num(*x),
            num(*y),
            num(*w),
            num(*h)
        ));
        for (n, line) in lines.iter().enumerate() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{}</text>\n",
                num(x + PAD_X),
                num(y + PAD_Y + (n as f64 + 1.0) * LINE - 4.0),
                escape(line)
            ));
        }
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_labels_wrap_at_spaces_and_words_longer_than_a_line_are_cut() {
        assert_eq!(wrap("a b"), vec!["a b"]);
        let long = format!("{} {}", "x".repeat(40), "y".repeat(40));
        assert_eq!(wrap(&long), vec!["x".repeat(40), "y".repeat(40)]);
        let word = "z".repeat(WRAP + 3);
        assert_eq!(wrap(&word), vec!["z".repeat(WRAP), "zzz".to_owned()]);
    }

    #[test]
    fn nested_expressions_collapse_past_the_spelling_depth() {
        let var = |name: &str| Expr::Var {
            name: name.to_owned(),
            t: Type::String,
        };
        let mut e = var("a");
        for name in ["b", "c", "d", "e", "f"] {
            e = Expr::Field {
                x: Box::new(e),
                name: name.to_owned(),
                t: Type::String,
            };
        }
        assert_eq!(spell(&e, 0), "….c.d.e.f");
        assert_eq!(expr_label(&var("n")), "var · n : string");
        assert_eq!(
            type_text(&Type::Map {
                key: Box::new(Type::String),
                elem: Box::new(Type::Ptr {
                    elem: Box::new(Type::Named {
                        name: "Order".to_owned()
                    })
                })
            }),
            "map[string]*Order"
        );
    }
}
