//! Writes an Odin package of free scalar procedures. Validate and render the
//! complete module before creating the output directory.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use codespec_model::{
    ADAPTER_FORMAT, AdapterManifest, Expr, FORMAT, Func, IntWidth, Module, Stmt, Type,
};

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;
type Scope<'a> = BTreeMap<&'a str, &'a Type>;

const FEATURES: &str = "type.int type.bool type.string expr.lit expr.var expr.binop expr.unop expr.call \
    stmt.return stmt.let stmt.assign stmt.if stmt.var stmt.incr stmt.decr op.add op.sub op.mul op.div \
    op.rem op.eq op.ne op.lt op.le op.gt op.ge op.land op.lor unop.neg unop.plus unop.not \
    sem.int.wrap sem.rem.sign assign.add assign.sub assign.mul assign.div assign.rem";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [arg] = args.as_slice() else {
        eprintln!("usage: codespec-write-odin <out> | --describe");
        return ExitCode::from(2);
    };
    if arg == "--describe" {
        print!(
            "{}",
            AdapterManifest {
                format: ADAPTER_FORMAT.into(),
                role: "writer".into(),
                lang: "odin".into(),
                cir: FORMAT.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                not: vec![],
                features: FEATURES.split_whitespace().map(str::to_owned).collect()
            }
            .to_json()
        );
        return ExitCode::SUCCESS;
    }
    match write(Path::new(arg)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec-write-odin: {e}");
            ExitCode::FAILURE
        }
    }
}

fn write(dir: &Path) -> Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let module = Module::from_json(&bytes).map_err(|e| e.to_string())?;
    let source = render(&module)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("module.odin"), source).map_err(|e| e.to_string())
}

fn ident(s: &str) -> Result<&str> {
    const RESERVED: &str = "asm auto_cast bit_field bit_set break case cast context continue defer \
        distinct do dynamic else enum fallthrough for foreign if import in map matrix not_in \
        or_break or_continue or_else or_return package proc return struct switch transmute typeid \
        union using when where true false nil int i8 i16 i32 i64 bool string min max panic main \
        runtime builtin intrinsics";
    if s.is_empty()
        || !s.as_bytes()[0].is_ascii_alphabetic()
        || !s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        || RESERVED.split_whitespace().any(|word| word == s)
    {
        return Err(format!("unsupported Odin identifier {s:?}"));
    }
    Ok(s)
}

fn declaration_name(s: &str) -> Result<&str> {
    if s.starts_with("codespec_") {
        return Err(format!("reserved Odin identifier {s:?}"));
    }
    ident(s)
}

fn ty(t: &Type) -> Result<&'static str> {
    match t {
        Type::Int {
            width,
            signed: true,
        } => Ok(match width {
            IntWidth::Platform => "int",
            IntWidth::W8 => "i8",
            IntWidth::W16 => "i16",
            IntWidth::W32 => "i32",
            IntWidth::W64 => "i64",
        }),
        Type::String => Ok("string"),
        Type::Bool => Ok("bool"),
        _ => Err(format!("unsupported Odin scalar type {t:?}")),
    }
}

fn render(m: &Module) -> Result<String> {
    if m.facts.is_none() {
        return Err("missing module facts: run codespec pass".into());
    }
    if !(m.types.is_empty()
        && m.vars.is_empty()
        && m.consts.is_empty()
        && m.errors.is_empty()
        && m.externs.is_empty()
        && m.opaque.is_empty()
        && m.refusals.is_empty()
        && m.linknames.is_empty()
        && m.inits.is_empty()
        && m.interfaces.is_empty()
        && m.entries.is_empty()
        && m.beans.is_empty()
        && m.injects.is_empty()
        && m.transactional.is_empty()
        && m.persistence.is_empty()
        && m.methods.is_empty())
    {
        return Err("the Odin writer supports modules containing only scalar functions".into());
    }
    let mut out = format!("package {}\n", declaration_name(&m.name)?);
    let functions: BTreeMap<_, _> = m.funcs.iter().map(|f| (f.name.as_str(), f)).collect();
    if functions.len() != m.funcs.len() {
        return Err("duplicate function name".into());
    }
    let mut helpers = BTreeSet::new();
    for f in &m.funcs {
        declaration_name(&f.name)?;
        if f.recv.is_some() || !f.tparams.is_empty() || !f.raises.is_empty() || f.body.is_empty() {
            return Err(format!(
                "{}: expected a free function with a scalar body",
                f.name
            ));
        }
        let mut scope = BTreeMap::new();
        let mut params = Vec::new();
        for p in &f.params {
            let n = ident(&p.name)?;
            if p.variadic || scope.insert(n, &p.t).is_some() {
                return Err(format!("{}: variadic or duplicate parameter", f.name));
            }
            params.push(format!("codespec_arg_{n}: {}", ty(&p.t)?));
        }
        let results = f.results.iter().map(ty).collect::<Result<Vec<_>>>()?;
        let ret = match results.as_slice() {
            [] => String::new(),
            [t] => format!(" -> {t}"),
            _ => format!(" -> ({})", results.join(", ")),
        };
        let _ = writeln!(
            out,
            "\n{}{} :: proc({}){ret} {{",
            if f.exported { "" } else { "@(private)\n" },
            f.name,
            params.join(", ")
        );
        for p in &f.params {
            let _ = writeln!(out, "\tcodespec_v_{} := codespec_arg_{}", p.name, p.name);
        }
        let mut w = Writer {
            out: &mut out,
            functions: &functions,
            results: &f.results,
            helpers: &mut helpers,
        };
        let returns = w
            .block(&f.body, &mut scope, 1)
            .map_err(|e| format!("{}: {e}", f.name))?;
        if !f.results.is_empty() && !returns {
            return Err(format!(
                "{}: execution can reach the end without returning",
                f.name
            ));
        }
        out.push_str("}\n");
    }
    for (op, t) in helpers {
        let (symbol, edge) = if op == "div" { ("/", "a") } else { ("%", "0") };
        let _ = writeln!(
            out,
            "\n@(private)\ncodespec_{op}_{t} :: proc(a, b: {t}) -> {t} {{\n\tif b == 0 {{ panic(\"integer division by zero\") }}\n\tif a == min({t}) && b == -1 {{ return {edge} }}\n\treturn a {symbol} b\n}}"
        );
    }
    Ok(out)
}

struct Writer<'a, 'b> {
    out: &'b mut String,
    functions: &'b BTreeMap<&'a str, &'a Func>,
    results: &'a [Type],
    helpers: &'b mut BTreeSet<(&'static str, &'static str)>,
}

impl<'a> Writer<'a, '_> {
    fn block(&mut self, ss: &'a [Stmt], scope: &mut Scope<'a>, depth: usize) -> Result<bool> {
        let pad = "\t".repeat(depth);
        let mut returned = false;
        for s in ss {
            if returned {
                return Err("unreachable statement".into());
            }
            let line = match s {
                Stmt::Return { results, .. } => {
                    if !results.iter().map(Expr::t).eq(self.results) {
                        return Err("return types differ from signature".into());
                    }
                    returned = true;
                    format!("return {}", self.exprs(results, scope)?.join(", "))
                        .trim_end()
                        .to_owned()
                }
                Stmt::Let { names, rhs, .. } if names.len() == 1 && rhs.len() == 1 => {
                    let n = ident(&names[0])?;
                    let value = self.expr(&rhs[0], scope)?;
                    if scope.insert(n, rhs[0].t()).is_some() {
                        return Err(format!("duplicate binding {n}"));
                    }
                    format!("codespec_v_{n} := {value}")
                }
                Stmt::Var { name, t, .. } => {
                    let n = ident(name)?;
                    if scope.insert(n, t).is_some() {
                        return Err(format!("duplicate binding {n}"));
                    }
                    format!("codespec_v_{n}: {}", ty(t)?)
                }
                Stmt::Assign { lhs, rhs, op, .. } if lhs.len() == 1 && rhs.len() == 1 => {
                    let x = &lhs[0];
                    let y = &rhs[0];
                    if !matches!(x, Expr::Var { .. }) || x.t() != y.t() {
                        return Err("assignment needs one local of the same type".into());
                    }
                    let id = self.expr(x, scope)?;
                    let v = self.expr(y, scope)?;
                    let v = match op.as_str() {
                        "" => v,
                        "+=" | "-=" | "*=" | "/=" | "%=" => {
                            self.binary(&op[..1], &id, &v, x.t(), x.t())?
                        }
                        _ => return Err(format!("unsupported assignment {op}")),
                    };
                    format!("{id} = {v}")
                }
                Stmt::Incr { x, .. } | Stmt::Decr { x, .. }
                    if matches!(
                        x,
                        Expr::Var {
                            t: Type::Int { .. },
                            ..
                        }
                    ) =>
                {
                    let v = self.expr(x, scope)?;
                    format!(
                        "{v} {} 1",
                        if matches!(s, Stmt::Incr { .. }) {
                            "+="
                        } else {
                            "-="
                        }
                    )
                }
                Stmt::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } if cond.t() == &Type::Bool => {
                    let test = self.expr(cond, scope)?;
                    let _ = writeln!(self.out, "{pad}if {test} {{");
                    let yes = self.block(then, &mut scope.clone(), depth + 1)?;
                    if !otherwise.is_empty() {
                        let _ = writeln!(self.out, "{pad}}} else {{");
                        returned = self.block(otherwise, &mut scope.clone(), depth + 1)? && yes;
                    }
                    "}".into()
                }
                _ => {
                    let node = serde_json::to_value(s).map_err(|e| e.to_string())?;
                    return Err(format!(
                        "unsupported {} statement at {}",
                        node["k"], node["pos"]
                    ));
                }
            };
            let _ = writeln!(self.out, "{pad}{line}");
        }
        Ok(returned)
    }

    fn exprs(&mut self, xs: &[Expr], s: &Scope) -> Result<Vec<String>> {
        xs.iter().map(|x| self.expr(x, s)).collect()
    }

    fn expr(&mut self, x: &Expr, s: &Scope) -> Result<String> {
        let t = ty(x.t())?;
        match x {
            Expr::Var { name, t } if s.get(name.as_str()).copied() == Some(t) => {
                Ok(format!("codespec_v_{name}"))
            }
            Expr::Lit {
                value,
                t: Type::String,
            } => {
                let text: String =
                    serde_json::from_str(value).map_err(|_| "expected a quoted UTF-8 string")?;
                Ok(odin_string(&text))
            }
            Expr::Lit {
                value,
                t: Type::Bool,
            } if matches!(value.as_str(), "true" | "false") => Ok(value.clone()),
            Expr::Lit {
                value,
                t: Type::Int { width, .. },
            } => {
                codespec_conformance::check_argument(value, codespec_conformance::ArgKind::Int)
                    .map_err(|_| {
                        format!("expected a canonical signed decimal integer, got {value:?}")
                    })?;
                let n = value.parse::<i64>().map_err(|e| e.to_string())?;
                let fits = match width {
                    IntWidth::W8 => i8::try_from(n).is_ok(),
                    IntWidth::W16 => i16::try_from(n).is_ok(),
                    IntWidth::W32 => i32::try_from(n).is_ok(),
                    IntWidth::W64 | IntWidth::Platform => true,
                };
                if !fits {
                    return Err(format!("{value} is outside the range of {t}"));
                }
                Ok(format!("{t}({value})"))
            }
            Expr::Zero { t: Type::String } => Ok("\"\"".into()),
            Expr::Zero { t: Type::Bool } => Ok("false".into()),
            Expr::Zero { .. } => Ok(format!("{t}(0)")),
            Expr::Call {
                name,
                targs,
                args,
                t,
            } if targs.is_empty() => {
                let f = self
                    .functions
                    .get(name.as_str())
                    .ok_or_else(|| format!("undeclared function {name}"))?;
                if f.results.as_slice() != std::slice::from_ref(t)
                    || !f.params.iter().map(|p| &p.t).eq(args.iter().map(Expr::t))
                {
                    return Err(format!("call of {name} differs from its signature"));
                }
                Ok(format!("{name}({})", self.exprs(args, s)?.join(", ")))
            }
            Expr::Unop { op, x: inner, t } if inner.t() == t => {
                let op = match (op.as_str(), t) {
                    ("!", Type::Bool) => "!",
                    ("+" | "-", Type::Int { .. }) => op.as_str(),
                    ("^", Type::Int { .. }) => "~",
                    _ => return Err("unsupported unary operator".into()),
                };
                Ok(format!("({op}{})", self.expr(inner, s)?))
            }
            Expr::Binop { op, x, y, t } if x.t() == y.t() => {
                let left = self.expr(x, s)?;
                let right = self.expr(y, s)?;
                self.binary(op, &left, &right, x.t(), t)
            }
            _ => Err("unsupported scalar expression or unresolved local".into()),
        }
    }

    fn binary(
        &mut self,
        op: &str,
        x: &str,
        y: &str,
        operand: &Type,
        result: &Type,
    ) -> Result<String> {
        match (op, operand, result) {
            ("/" | "%", Type::Int { .. }, t) if operand == t => {
                let name = if op == "/" { "div" } else { "rem" };
                let ty = ty(t)?;
                self.helpers.insert((name, ty));
                Ok(format!("codespec_{name}_{ty}({x}, {y})"))
            }
            ("+" | "-" | "*", Type::Int { .. }, t) if operand == t => Ok(format!("({x} {op} {y})")),
            ("&&" | "||", Type::Bool, Type::Bool)
            | ("==" | "!=", _, Type::Bool)
            | ("<" | ">" | "<=" | ">=", Type::Int { .. }, Type::Bool) => {
                Ok(format!("({x} {op} {y})"))
            }
            _ => Err(format!("unsupported binary operator {op} for these types")),
        }
    }
}

fn odin_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
