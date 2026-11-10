//! cq's syntax: the lexer, the parser into [`Program`] and the printer `--explain` uses.
//!
//! ```text
//! module  = item*
//! item    = names | rule | fact | query
//! names   = "names" ident "=" "[" string ("," string)* ","? "]" "."
//! rule    = atom ":-" body "."         fact = atom "."         query = "?-" body "."
//! body    = lit ("," lit)*
//! lit     = atom | "!" atom | "!" "(" cond ")" | "(" body ("or" body)* ")" | cond
//! atom    = ident "(" term ("," term)* ")" | ident "{" field ("," field)* "}"
//! field   = ident ":" (term | "~" string | "in" list)
//! cond    = term op term | term "~" string | term "in" list | Var "=" agg
//! agg     = ("count" | "min" | "max" | "sum" | "collect") "{" (term ":")? body "}"
//! term    = Var | "_" | string | int | "true" | "false" | Var "." ident | term ("+" | "-" | "*") term
//! list    = "@names." ident
//! op      = "==" | "!=" | "<" | "<=" | ">" | ">=" | "="
//! ```
//!
//! A variable starts with an upper-case letter, a relation with a lower-case one; `//` starts a
//! comment. In a string `\"` is a quote and `\\` a backslash; any other backslash stays, so a
//! regular expression is written as it reads (`"^strconv\.Atoi$"`).

use std::fmt;

/// A parsed cq module.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub rules: Vec<Rule>,
    /// The names lists it declares, in order.
    pub names: Vec<(String, Vec<String>)>,
    pub query: Option<Vec<Lit>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub head: Atom,
    pub body: Vec<Lit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    pub rel: String,
    pub args: Args,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Args {
    Positional(Vec<Term>),
    Named(Vec<(String, Field)>),
}

/// What a named field of a record pattern holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Term(Term),
    Regex(String),
    In(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Var(String),
    Any,
    Str(String),
    Int(i64),
    Bool(bool),
    /// `X.field`: a field of the type `X` names.
    Attr(String, String),
    Arith(Box<Term>, char, Box<Term>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `=`: binds an unbound side, else compares.
    Bind,
}

impl Op {
    fn text(self) -> &'static str {
        match self {
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Bind => "=",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFn {
    Count,
    Min,
    Max,
    Sum,
    Collect,
}

impl AggFn {
    fn text(self) -> &'static str {
        match self {
            AggFn::Count => "count",
            AggFn::Min => "min",
            AggFn::Max => "max",
            AggFn::Sum => "sum",
            AggFn::Collect => "collect",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Atom(Atom),
    Not(Atom),
    /// `!( cond )`
    NotCond(Box<Lit>),
    Cmp(Term, Op, Term),
    Match(Term, String),
    In(Term, String),
    Agg {
        var: String,
        func: AggFn,
        target: Option<Term>,
        body: Vec<Lit>,
    },
    Or(Vec<Vec<Lit>>),
}

// ---------------------------------------------------------------------------------------------
// Printing

fn quote(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::from("\"");
    for (k, &c) in chars.iter().enumerate() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' if matches!(chars.get(k + 1), None | Some('"') | Some('\\')) => {
                out.push_str("\\\\")
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(v) => write!(f, "{v}"),
            Term::Any => write!(f, "_"),
            Term::Str(s) => write!(f, "{}", quote(s)),
            Term::Int(n) => write!(f, "{n}"),
            Term::Bool(b) => write!(f, "{b}"),
            Term::Attr(v, a) => write!(f, "{v}.{a}"),
            Term::Arith(a, op, b) => write!(f, "{a} {op} {b}"),
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.args {
            Args::Positional(ts) => {
                let ts: Vec<String> = ts.iter().map(Term::to_string).collect();
                write!(f, "{}({})", self.rel, ts.join(", "))
            }
            Args::Named(fs) => {
                let fs: Vec<String> = fs
                    .iter()
                    .map(|(n, v)| match v {
                        Field::Term(t) => format!("{n}: {t}"),
                        Field::Regex(r) => format!("{n}: ~{}", quote(r)),
                        Field::In(l) => format!("{n}: in @names.{l}"),
                    })
                    .collect();
                write!(f, "{}{{{}}}", self.rel, fs.join(", "))
            }
        }
    }
}

fn body_text(body: &[Lit]) -> String {
    body.iter()
        .map(Lit::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

impl fmt::Display for Lit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lit::Atom(a) => write!(f, "{a}"),
            Lit::Not(a) => write!(f, "!{a}"),
            Lit::NotCond(c) => write!(f, "!({c})"),
            Lit::Cmp(a, op, b) => write!(f, "{a} {} {b}", op.text()),
            Lit::Match(t, r) => write!(f, "{t} ~ {}", quote(r)),
            Lit::In(t, l) => write!(f, "{t} in @names.{l}"),
            Lit::Agg {
                var,
                func,
                target,
                body,
            } => match target {
                Some(t) => write!(f, "{var} = {}{{{t} : {}}}", func.text(), body_text(body)),
                None => write!(f, "{var} = {}{{{}}}", func.text(), body_text(body)),
            },
            Lit::Or(alts) => {
                let alts: Vec<String> = alts.iter().map(|a| body_text(a)).collect();
                write!(f, "({})", alts.join(" or "))
            }
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.body.is_empty() {
            write!(f, "{}.", self.head)
        } else {
            write!(f, "{} :- {}.", self.head, body_text(&self.body))
        }
    }
}

/// The query as `--explain` prints it.
pub fn query_text(body: &[Lit]) -> String {
    format!("?- {}.", body_text(body))
}

// ---------------------------------------------------------------------------------------------
// Lexing

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Var(String),
    Str(String),
    Int(i64),
    Punct(&'static str),
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    line: usize,
    col: usize,
    /// Whether whitespace or a comment comes right before it.
    spaced: bool,
}

const PUNCT: &[&str] = &[
    ":-", "?-", "->", "==", "!=", "<=", ">=", "(", ")", "{", "}", "[", "]", ",", ".", ":", "!",
    "~", "@", "=", "<", ">", "+", "-", "*",
];

fn lex(src: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = src.chars().collect();
    let (mut k, mut line, mut col) = (0, 1, 1);
    let mut out = Vec::new();
    let mut spaced = true;
    while k < chars.len() {
        let c = chars[k];
        let advance = |k: &mut usize, line: &mut usize, col: &mut usize, n: usize| {
            for _ in 0..n {
                if chars[*k] == '\n' {
                    *line += 1;
                    *col = 1;
                } else {
                    *col += 1;
                }
                *k += 1;
            }
        };
        if c.is_whitespace() {
            advance(&mut k, &mut line, &mut col, 1);
            spaced = true;
            continue;
        }
        if c == '/' && chars.get(k + 1) == Some(&'/') {
            while k < chars.len() && chars[k] != '\n' {
                advance(&mut k, &mut line, &mut col, 1);
            }
            spaced = true;
            continue;
        }
        let (tl, tc) = (line, col);
        let tok = if c.is_ascii_alphabetic() || c == '_' {
            let start = k;
            while k < chars.len() && (chars[k].is_ascii_alphanumeric() || chars[k] == '_') {
                advance(&mut k, &mut line, &mut col, 1);
            }
            let word: String = chars[start..k].iter().collect();
            if word.starts_with(|c: char| c.is_ascii_uppercase()) || word.starts_with('_') {
                Tok::Var(word)
            } else {
                Tok::Ident(word)
            }
        } else if c.is_ascii_digit()
            || (c == '-'
                && chars.get(k + 1).is_some_and(|d| d.is_ascii_digit())
                && !matches!(
                    out.last(),
                    Some(Token {
                        tok: Tok::Var(_) | Tok::Int(_) | Tok::Str(_) | Tok::Punct(")"),
                        ..
                    })
                ))
        {
            let start = k;
            advance(&mut k, &mut line, &mut col, 1);
            while k < chars.len() && chars[k].is_ascii_digit() {
                advance(&mut k, &mut line, &mut col, 1);
            }
            let text: String = chars[start..k].iter().collect();
            Tok::Int(
                text.parse()
                    .map_err(|_| format!("{tl}:{tc}: integer {text} out of range"))?,
            )
        } else if c == '"' {
            advance(&mut k, &mut line, &mut col, 1);
            let mut s = String::new();
            loop {
                let Some(&c) = chars.get(k) else {
                    return Err(format!("{tl}:{tc}: unterminated string"));
                };
                if c == '"' {
                    advance(&mut k, &mut line, &mut col, 1);
                    break;
                }
                if c == '\\' && matches!(chars.get(k + 1), Some('"') | Some('\\')) {
                    s.push(chars[k + 1]);
                    advance(&mut k, &mut line, &mut col, 2);
                    continue;
                }
                s.push(c);
                advance(&mut k, &mut line, &mut col, 1);
            }
            Tok::Str(s)
        } else {
            let rest: String = chars[k..chars.len().min(k + 2)].iter().collect();
            let Some(p) = PUNCT.iter().find(|p| rest.starts_with(**p)) else {
                return Err(format!("{tl}:{tc}: unexpected {c:?}"));
            };
            advance(&mut k, &mut line, &mut col, p.len());
            Tok::Punct(p)
        };
        out.push(Token {
            tok,
            line: tl,
            col: tc,
            spaced,
        });
        spaced = false;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Parsing

struct Parser {
    toks: Vec<Token>,
    k: usize,
}

const DESIGN: &str = "docs/query-language.md";

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.k).map(|t| &t.tok)
    }

    fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.k + n).map(|t| &t.tok)
    }

    fn here(&self) -> String {
        match self.toks.get(self.k) {
            Some(t) => format!("{}:{}", t.line, t.col),
            None => match self.toks.last() {
                Some(t) => format!("{}:{}", t.line, t.col),
                None => "1:1".into(),
            },
        }
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        let found = match self.peek() {
            Some(Tok::Ident(s) | Tok::Var(s)) => s.clone(),
            Some(Tok::Str(s)) => quote(s),
            Some(Tok::Int(n)) => n.to_string(),
            Some(Tok::Punct(p)) => p.to_string(),
            None => "the end".into(),
        };
        Err(format!("{}: expected {what}, found {found}", self.here()))
    }

    fn is(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::Punct(q)) if *q == p)
    }

    fn eat(&mut self, p: &str) -> bool {
        if self.is(p) {
            self.k += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, p: &str) -> Result<(), String> {
        if self.eat(p) {
            Ok(())
        } else {
            self.err(&format!("`{p}`"))
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.k += 1;
                Ok(s)
            }
            _ => self.err("a name"),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Str(s)) => {
                let s = s.clone();
                self.k += 1;
                Ok(s)
            }
            _ => self.err("a string"),
        }
    }

    /// The source text from token `from` to the current one, for naming a refused construct.
    fn snippet(&self, from: usize) -> String {
        let mut out = String::new();
        for t in &self.toks[from..self.k.min(self.toks.len())] {
            if t.spaced && !out.is_empty() {
                out.push(' ');
            }
            out += &match &t.tok {
                Tok::Ident(s) | Tok::Var(s) => s.clone(),
                Tok::Str(s) => quote(s),
                Tok::Int(n) => n.to_string(),
                Tok::Punct(p) => p.to_string(),
            };
        }
        out
    }

    /// Skips to the bracket that closes the one just opened.
    fn skip_group(&mut self, open: &str, close: &str) {
        let mut depth = 1;
        while let Some(t) = self.peek() {
            if *t == Tok::Punct(open_static(open)) {
                depth += 1;
            } else if *t == Tok::Punct(open_static(close)) {
                depth -= 1;
                if depth == 0 {
                    self.k += 1;
                    return;
                }
            }
            self.k += 1;
        }
    }

    fn program(&mut self) -> Result<Program, String> {
        let mut p = Program {
            rules: Vec::new(),
            names: Vec::new(),
            query: None,
        };
        while self.peek().is_some() {
            if self.eat("?-") {
                let body = self.body()?;
                self.expect(".")?;
                if p.query.is_some() {
                    return Err(format!("{}: a module has one query (?- …)", self.here()));
                }
                p.query = Some(body);
                continue;
            }
            match (self.peek(), self.peek_at(1)) {
                (Some(Tok::Ident(w)), Some(Tok::Ident(name))) if w == "property" => {
                    return Err(format!(
                        "{}: the property {name} is a property (§ 5.3 of {DESIGN}), which cq does not run yet",
                        self.here()
                    ));
                }
                (Some(Tok::Ident(w)), Some(Tok::Ident(name))) if w == "check" => {
                    return Err(format!(
                        "{}: the check declaration {name} (§ 5.4 of {DESIGN}) is not run yet; write its query with ?-",
                        self.here()
                    ));
                }
                (Some(Tok::Ident(w)), Some(Tok::Ident(_))) if w == "names" => {
                    self.k += 1;
                    let name = self.ident()?;
                    self.expect("=")?;
                    self.expect("[")?;
                    let mut items = Vec::new();
                    while !self.is("]") {
                        items.push(self.string()?);
                        if !self.eat(",") {
                            break;
                        }
                    }
                    self.expect("]")?;
                    self.expect(".")?;
                    if p.names.iter().any(|(n, _)| *n == name) {
                        return Err(format!(
                            "{}: the names list {name} is declared twice",
                            self.here()
                        ));
                    }
                    p.names.push((name, items));
                    continue;
                }
                _ => {}
            }
            let head = self.atom()?;
            let body = if self.eat(":-") {
                self.body()?
            } else {
                Vec::new()
            };
            self.expect(".")?;
            p.rules.push(Rule { head, body });
        }
        Ok(p)
    }

    fn body(&mut self) -> Result<Vec<Lit>, String> {
        let mut lits = vec![self.lit()?];
        while self.eat(",") {
            lits.push(self.lit()?);
        }
        Ok(lits)
    }

    fn lit(&mut self) -> Result<Lit, String> {
        if self.eat("!") {
            if self.eat("(") {
                let c = self.cond()?;
                self.expect(")")?;
                return Ok(Lit::NotCond(Box::new(c)));
            }
            if matches!(self.peek(), Some(Tok::Ident(_))) {
                return Ok(Lit::Not(self.atom()?));
            }
            return self.err("a relation or a parenthesised condition after !");
        }
        if self.eat("(") {
            let mut alts = vec![self.body()?];
            while matches!(self.peek(), Some(Tok::Ident(w)) if w == "or") {
                self.k += 1;
                alts.push(self.body()?);
            }
            self.expect(")")?;
            return Ok(Lit::Or(alts));
        }
        if matches!(self.peek(), Some(Tok::Ident(w)) if w != "true" && w != "false") {
            return Ok(Lit::Atom(self.atom()?));
        }
        self.cond()
    }

    fn cond(&mut self) -> Result<Lit, String> {
        let left = self.term()?;
        if self.eat("~") {
            return Ok(Lit::Match(left, self.string()?));
        }
        if matches!(self.peek(), Some(Tok::Ident(w)) if w == "in") {
            self.k += 1;
            return Ok(Lit::In(left, self.list()?));
        }
        let op = match self.peek() {
            Some(Tok::Punct("==")) => Op::Eq,
            Some(Tok::Punct("!=")) => Op::Ne,
            Some(Tok::Punct("<")) => Op::Lt,
            Some(Tok::Punct("<=")) => Op::Le,
            Some(Tok::Punct(">")) => Op::Gt,
            Some(Tok::Punct(">=")) => Op::Ge,
            Some(Tok::Punct("=")) => Op::Bind,
            _ => return self.err("a comparison, ~ or in"),
        };
        self.k += 1;
        if op == Op::Bind
            && let Some(Tok::Ident(w)) = self.peek()
            && let Some(func) = agg_fn(w)
            && self.peek_at(1) == Some(&Tok::Punct("{"))
        {
            let Term::Var(var) = left else {
                return Err(format!("{}: an aggregate binds a variable", self.here()));
            };
            self.k += 2;
            let save = self.k;
            let mut target = None;
            if func != AggFn::Count {
                let t = self.term()?;
                if self.eat(":") {
                    target = Some(t);
                } else {
                    self.k = save;
                }
            }
            if func != AggFn::Count && target.is_none() {
                return Err(format!(
                    "{}: {} needs the value it aggregates: {}{{X : body}}",
                    self.here(),
                    func.text(),
                    func.text()
                ));
            }
            let body = self.body()?;
            self.expect("}")?;
            return Ok(Lit::Agg {
                var,
                func,
                target,
                body,
            });
        }
        let right = self.term()?;
        Ok(Lit::Cmp(left, op, right))
    }

    fn list(&mut self) -> Result<String, String> {
        self.expect("@")?;
        match self.peek() {
            Some(Tok::Ident(w)) if w == "names" => self.k += 1,
            _ => return self.err("@names.<list>"),
        }
        self.expect(".")?;
        self.ident()
    }

    fn term(&mut self) -> Result<Term, String> {
        let mut t = self.simple()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Punct("+")) => '+',
                Some(Tok::Punct("-")) => '-',
                Some(Tok::Punct("*")) => '*',
                _ => return Ok(t),
            };
            self.k += 1;
            let r = self.simple()?;
            t = Term::Arith(Box::new(t), op, Box::new(r));
        }
    }

    fn simple(&mut self) -> Result<Term, String> {
        let t = match self.peek().cloned() {
            Some(Tok::Var(v)) => {
                self.k += 1;
                if v == "_" {
                    return Ok(Term::Any);
                }
                if self.is(".")
                    && !self.toks[self.k].spaced
                    && matches!(self.peek_at(1), Some(Tok::Ident(_)))
                    && !self.toks[self.k + 1].spaced
                {
                    self.k += 1;
                    let a = self.ident()?;
                    return Ok(Term::Attr(v, a));
                }
                Term::Var(v)
            }
            Some(Tok::Str(s)) => {
                self.k += 1;
                Term::Str(s)
            }
            Some(Tok::Int(n)) => {
                self.k += 1;
                Term::Int(n)
            }
            Some(Tok::Ident(w)) if w == "true" || w == "false" => {
                self.k += 1;
                Term::Bool(w == "true")
            }
            _ => return self.err("a variable or a value"),
        };
        Ok(t)
    }

    fn atom(&mut self) -> Result<Atom, String> {
        let start = self.k;
        let rel = self.ident()?;
        if (rel == "path" || rel == "every_path") && self.is("(") {
            self.k += 1;
            self.skip_group("(", ")");
            return Err(format!(
                "{}: the path pattern {} is a path pattern (§ 5.2 of {DESIGN}), which cq does not run yet",
                self.toks[start].line.to_string() + ":" + &self.toks[start].col.to_string(),
                self.snippet(start)
            ));
        }
        if self.eat("(") {
            let mut terms = Vec::new();
            if !self.is(")") {
                terms.push(self.term()?);
                while self.eat(",") {
                    terms.push(self.term()?);
                }
            }
            self.expect(")")?;
            return Ok(Atom {
                rel,
                args: Args::Positional(terms),
            });
        }
        if self.eat("{") {
            let mut fields: Vec<(String, Field)> = Vec::new();
            if !self.is("}") {
                loop {
                    let name = self.ident()?;
                    self.expect(":")?;
                    let v = if self.eat("~") {
                        Field::Regex(self.string()?)
                    } else if matches!(self.peek(), Some(Tok::Ident(w)) if w == "in") {
                        self.k += 1;
                        Field::In(self.list()?)
                    } else {
                        Field::Term(self.term()?)
                    };
                    if fields.iter().any(|(n, _)| *n == name) {
                        return Err(format!(
                            "{}: {rel} names the field {name} twice",
                            self.here()
                        ));
                    }
                    fields.push((name, v));
                    if !self.eat(",") {
                        break;
                    }
                }
            }
            self.expect("}")?;
            return Ok(Atom {
                rel,
                args: Args::Named(fields),
            });
        }
        self.err("( or { after a relation name")
    }
}

fn open_static(p: &str) -> &'static str {
    PUNCT.iter().find(|q| **q == p).copied().unwrap_or("")
}

fn agg_fn(w: &str) -> Option<AggFn> {
    Some(match w {
        "count" => AggFn::Count,
        "min" => AggFn::Min,
        "max" => AggFn::Max,
        "sum" => AggFn::Sum,
        "collect" => AggFn::Collect,
        _ => return None,
    })
}

/// Parse a cq module. A path pattern (§ 5.2) or a property (§ 5.3) is refused, naming it.
pub fn parse(src: &str) -> Result<Program, String> {
    let toks = lex(src)?;
    Parser { toks, k: 0 }.program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_prints_back_as_it_parses() {
        let src = r#"narrow(E, From, To) :- expr{id: E, kind: "conv", t: To, x: X}, expr{id: X, t: From}, To.bits < From.bits, !(X ~ "^a\.b$"), N = count{call{func: F}}, M = max{L : stmt{line: L}}, J == N - 1, (a(X) or b(X)), !c(X), X in @names.receive."#;
        let p = parse(src).unwrap();
        let printed = p.rules[0].to_string();
        assert_eq!(printed, src);
        assert_eq!(parse(&printed).unwrap(), p);
    }

    #[test]
    fn a_negative_number_and_a_subtraction_are_told_apart() {
        let p = parse("r(X) :- let_name{index: -1, stmt: X}, Y = X - 1.").unwrap();
        assert_eq!(
            p.rules[0].to_string(),
            "r(X) :- let_name{index: -1, stmt: X}, Y = X - 1."
        );
    }
}
