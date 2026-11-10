//! cq's syntax: the lexer, the parser into [`Program`] and the printer `--explain` uses.
//!
//! ```text
//! module  = item*
//! item    = names | rule | fact | query | property
//! names   = "names" lname "=" "[" string ("," string)* ","? "]" "."
//! rule    = atom ":-" body "."         fact = atom "."         query = "?-" body "."
//! body    = lit ("," lit)*
//! lit     = atom | "!" atom | "!" "(" cond ")" | "(" body ("or" body)* ")" | cond
//!         | path | "!" path | every
//! atom    = ident "(" term ("," term)* ")" | ident "{" field ("," field)* "}"
//! field   = ident ":" (term | "~" string | "in" list)
//! cond    = term op term | term "~" string | term "in" list | Var "=" agg
//! agg     = ("count" | "min" | "max" | "sum" | "collect") "{" (term ":")? body "}"
//! term    = Var | "_" | string | int | "true" | "false" | Var "." ident | term ("+" | "-" | "*") term
//! list    = "@names." lname          lname = ident ("-" ident)*
//! op     = "==" | "!=" | "<" | "<=" | ">" | ">=" | "="
//! path    = "path" "(" term "->" term ("," popt)* ")"
//! every   = "every_path" "(" term "->" term ("," eopt)* ")"
//! popt    = "over" ":" ("cfg" | "calls") | "avoid" ":" avoid | "max" ":" int | "witness" ":" Var
//! eopt    = "over" ":" ("cfg" | "calls") | "through" ":" avoid | "max" ":" int
//! avoid   = Var "where" lit ("," lit)* | term
//! property = "property" ident "(" "scope" ":" ("body" | "calls") ")" "{" (trans | message)* "}"
//! trans   = ident "->" ident ":" event ("=>" ident ":=" Var ("," ident ":=" Var)*)?
//! event   = "exit" | "call" "{" field ("," field)* "}"
//! message = "message" string
//! ```
//!
//! A variable starts with an upper-case letter, a relation with a lower-case one; `//` starts a
//! comment. In a string `\"` is a quote and `\\` a backslash; any other backslash stays, so a
//! regular expression is written as it reads (`"^strconv\.Atoi$"`). A names list's name may hold
//! `-` between its words (`@names.never-fail`), as `codespec check` names its lists.
//!
//! `path(` is a path pattern when `->` follows its first term, else an atom of a relation named
//! `path`. The body after `where` ends at the `)` of the pattern or at the next `name:` option.
//! In a property's event a lower-case name in a field is a register; a property sets at most
//! [`MAX_REGISTERS`] of them, each before it is read on some transition, and names `start` and
//! `error`; its message names states and registers in braces (`{start}`, `{m}`). `defer
//! call{…}` waits for the `defer` statement of `cir/1` (§ 8 of `docs/query-language.md`) and is
//! refused, naming it, as is a `check` declaration (§ 5.4).
//!
//! Limits: parentheses, negated conditions, aggregates and the operators of a term nest at most
//! [`MAX_NESTING`] deep (`1 + 2 + 3` is two deep), and a body holds at most [`MAX_BODY`]
//! literals; a module past either is refused, naming the limit.

use std::fmt;

/// How deep parentheses, negated conditions, aggregates and the operators of a term may nest.
pub const MAX_NESTING: usize = 256;
/// The most literals one body holds.
pub const MAX_BODY: usize = 1000;
/// The most registers a property sets (§ 5.3: Topl's cost grows exponentially with them).
pub const MAX_REGISTERS: usize = 3;

/// A parsed cq module.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub rules: Vec<Rule>,
    /// The names lists it declares, in order.
    pub names: Vec<(String, Vec<String>)>,
    pub query: Option<Vec<Lit>>,
    /// Its properties, in order.
    pub properties: Vec<Property>,
}

/// The edges a path pattern follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// `cfg_next`, statement to statement.
    Cfg,
    /// The linked calls, function to function.
    Calls,
}

/// What a path pattern's path may not pass (`avoid:`, `through:` of `every_path`).
#[derive(Debug, Clone, PartialEq)]
pub enum Avoid {
    /// One node.
    Node(Term),
    /// `N where body`: every `N` the body gives.
    Where(String, Vec<Lit>),
}

/// How a path pattern is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathForm {
    Path,
    /// `!path(…)`
    NotPath,
    /// `every_path(…, through: …)`, which is `!path(…, avoid: …)`.
    Every,
}

/// A path pattern (§ 5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct PathPat {
    pub form: PathForm,
    pub from: Term,
    pub to: Term,
    pub over: Over,
    pub avoid: Option<Avoid>,
    pub max: Option<i64>,
    pub witness: Option<String>,
}

/// What a property follows (§ 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The calls of one body along its control flow.
    Body,
    /// The calls along a call path from an entry of the call graph.
    Calls,
}

/// What moves a property from one state to another.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The body ends: a `return` or a statement falling off its end.
    Exit,
    /// A call matching the record pattern of `call`; a lower-case variable is a register.
    Call(Atom),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub event: Event,
    /// `=> register := Var`, in order.
    pub assign: Vec<(String, String)>,
}

/// A property: a small automaton over calls (§ 5.3).
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub scope: Scope,
    pub transitions: Vec<Transition>,
    pub message: Option<String>,
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
    Path(Box<PathPat>),
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
            Lit::Path(p) => write!(f, "{p}"),
        }
    }
}

impl fmt::Display for PathPat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (lead, word) = match self.form {
            PathForm::Path => ("path", "avoid"),
            PathForm::NotPath => ("!path", "avoid"),
            PathForm::Every => ("every_path", "through"),
        };
        let over = match self.over {
            Over::Cfg => "cfg",
            Over::Calls => "calls",
        };
        write!(f, "{lead}({} -> {}, over: {over}", self.from, self.to)?;
        match &self.avoid {
            Some(Avoid::Node(t)) => write!(f, ", {word}: {t}")?,
            Some(Avoid::Where(n, body)) => write!(f, ", {word}: {n} where {}", body_text(body))?,
            None => {}
        }
        if let Some(m) = self.max {
            write!(f, ", max: {m}")?;
        }
        if let Some(w) = &self.witness {
            write!(f, ", witness: {w}")?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = match self.scope {
            Scope::Body => "body",
            Scope::Calls => "calls",
        };
        writeln!(f, "property {}(scope: {scope}) {{", self.name)?;
        for t in &self.transitions {
            let event = match &t.event {
                Event::Exit => "exit".to_string(),
                Event::Call(a) => a.to_string(),
            };
            write!(f, "    {} -> {}: {event}", t.from, t.to)?;
            if !t.assign.is_empty() {
                let a: Vec<String> = t
                    .assign
                    .iter()
                    .map(|(r, v)| format!("{r} := {v}"))
                    .collect();
                write!(f, " => {}", a.join(", "))?;
            }
            writeln!(f)?;
        }
        if let Some(m) = &self.message {
            writeln!(f, "    message {}", quote(m))?;
        }
        write!(f, "  }}")
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
    ":-", "?-", "->", "==", "!=", "<=", ">=", ":=", "=>", "(", ")", "{", "}", "[", "]", ",", ".",
    ":", "!", "~", "@", "=", "<", ">", "+", "-", "*",
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
    depth: usize,
    /// Inside a property's event: a lower-case name in a term is a register.
    registers: bool,
}

const DESIGN: &str = "docs/query-language.md";

impl Parser {
    /// One level deeper; refused past [`MAX_NESTING`].
    fn enter(&mut self) -> Result<(), String> {
        if self.depth == MAX_NESTING {
            return Err(format!(
                "{}: nested deeper than {MAX_NESTING} (parentheses, negations, aggregates and operators together), the limit of cq",
                self.here()
            ));
        }
        self.depth += 1;
        Ok(())
    }

    /// A names list's name: words joined by `-` with no space around it.
    fn list_name(&mut self) -> Result<String, String> {
        let mut name = self.ident()?;
        while self.is("-")
            && !self.toks[self.k].spaced
            && matches!(self.peek_at(1), Some(Tok::Ident(_)))
            && !self.toks[self.k + 1].spaced
        {
            self.k += 1;
            name.push('-');
            name += &self.ident()?;
        }
        Ok(name)
    }

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

    fn program(&mut self) -> Result<Program, String> {
        let mut p = Program {
            rules: Vec::new(),
            names: Vec::new(),
            query: None,
            properties: Vec::new(),
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
                (Some(Tok::Ident(w)), Some(Tok::Ident(_))) if w == "property" => {
                    let prop = self.property()?;
                    if p.properties.iter().any(|q| q.name == prop.name) {
                        return Err(format!(
                            "{}: the property {} is declared twice",
                            self.here(),
                            prop.name
                        ));
                    }
                    p.properties.push(prop);
                    continue;
                }
                (Some(Tok::Ident(w)), Some(Tok::Ident(name))) if w == "check" => {
                    return Err(format!(
                        "{}: the check declaration {name} (§ 5.4 of {DESIGN}) is not run yet; write its query with ?-",
                        self.here()
                    ));
                }
                (Some(Tok::Ident(w)), Some(Tok::Ident(_))) if w == "names" => {
                    self.k += 1;
                    let name = self.list_name()?;
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
            if lits.len() == MAX_BODY {
                return Err(format!(
                    "{}: a body of more than {MAX_BODY} literals, the limit of cq",
                    self.here()
                ));
            }
            lits.push(self.lit()?);
        }
        Ok(lits)
    }

    fn lit(&mut self) -> Result<Lit, String> {
        if self.is("!") && self.path_ahead(1).is_some() {
            let at = self.here();
            self.k += 1;
            if self.path_ahead(0) == Some(true) {
                return Err(format!(
                    "{at}: !every_path(…, through: P) is path(…, avoid: P); write that"
                ));
            }
            return Ok(Lit::Path(Box::new(self.path(true)?)));
        }
        if self.path_ahead(0).is_some() {
            return Ok(Lit::Path(Box::new(self.path(false)?)));
        }
        if self.eat("!") {
            if self.eat("(") {
                self.enter()?;
                let c = self.cond()?;
                self.depth -= 1;
                self.expect(")")?;
                return Ok(Lit::NotCond(Box::new(c)));
            }
            if matches!(self.peek(), Some(Tok::Ident(_))) {
                return Ok(Lit::Not(self.atom()?));
            }
            return self.err("a relation or a parenthesised condition after !");
        }
        if self.eat("(") {
            self.enter()?;
            let mut alts = vec![self.body()?];
            while matches!(self.peek(), Some(Tok::Ident(w)) if w == "or") {
                self.k += 1;
                alts.push(self.body()?);
            }
            self.depth -= 1;
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
            self.enter()?;
            let body = self.body()?;
            self.depth -= 1;
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
        self.list_name()
    }

    /// A term; each operator nests it one level deeper (`1 + 2 + 3` is `(1 + 2) + 3`).
    fn term(&mut self) -> Result<Term, String> {
        let mut t = self.simple()?;
        let outer = self.depth;
        loop {
            let op = match self.peek() {
                Some(Tok::Punct("+")) => '+',
                Some(Tok::Punct("-")) => '-',
                Some(Tok::Punct("*")) => '*',
                _ => {
                    self.depth = outer;
                    return Ok(t);
                }
            };
            if let Err(e) = self.enter() {
                self.depth = outer;
                return Err(e);
            }
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
            // A register: a variable no user variable can be, as it starts in lower case.
            Some(Tok::Ident(w)) if self.registers => {
                self.k += 1;
                Term::Var(w)
            }
            _ => return self.err("a variable or a value"),
        };
        Ok(t)
    }

    fn atom(&mut self) -> Result<Atom, String> {
        let rel = self.ident()?;
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

    /// Whether a path pattern starts `n` tokens ahead: `path(` or `every_path(` and a term
    /// followed by `->`; `Some(true)` for `every_path`.
    fn path_ahead(&mut self, n: usize) -> Option<bool> {
        let every = match self.peek_at(n) {
            Some(Tok::Ident(w)) if w == "path" => false,
            Some(Tok::Ident(w)) if w == "every_path" => true,
            _ => return None,
        };
        if self.peek_at(n + 1) != Some(&Tok::Punct("(")) {
            return None;
        }
        let (k, depth) = (self.k, self.depth);
        self.k += n + 2;
        let arrow = self.term().is_ok() && self.is("->");
        (self.k, self.depth) = (k, depth);
        arrow.then_some(every)
    }

    /// A path pattern, `!` already read when `negated`.
    fn path(&mut self, negated: bool) -> Result<PathPat, String> {
        let at = self.here();
        let every = self.ident()? == "every_path";
        self.expect("(")?;
        let from = self.term()?;
        self.expect("->")?;
        let to = self.term()?;
        let mut p = PathPat {
            form: match (every, negated) {
                (true, _) => PathForm::Every,
                (false, true) => PathForm::NotPath,
                (false, false) => PathForm::Path,
            },
            from,
            to,
            over: Over::Cfg,
            avoid: None,
            max: None,
            witness: None,
        };
        let mut seen: Vec<String> = Vec::new();
        let word = if every { "through" } else { "avoid" };
        while self.eat(",") {
            let opt_at = self.here();
            let opt = self.ident()?;
            if seen.contains(&opt) {
                return Err(format!("{opt_at}: {opt} is given twice"));
            }
            self.expect(":")?;
            match opt.as_str() {
                "over" => {
                    let w = self.ident()?;
                    p.over = match w.as_str() {
                        "cfg" => Over::Cfg,
                        "calls" => Over::Calls,
                        _ => {
                            return Err(format!(
                                "{opt_at}: over: {w}: a path runs over cfg or calls"
                            ));
                        }
                    };
                }
                o if o == word => p.avoid = Some(self.avoid()?),
                "max" => match self.peek() {
                    Some(Tok::Int(n)) if *n >= 0 => {
                        p.max = Some(*n);
                        self.k += 1;
                    }
                    _ => return self.err("a number of edges, 0 or more"),
                },
                "witness" if !every => match self.peek().cloned() {
                    Some(Tok::Var(v)) if v != "_" => {
                        if negated {
                            return Err(format!(
                                "{opt_at}: a negated path has no path to be the witness of"
                            ));
                        }
                        self.k += 1;
                        p.witness = Some(v);
                    }
                    _ => return self.err("a variable for the witness"),
                },
                _ => {
                    return Err(format!(
                        "{opt_at}: {opt}: {} takes {}",
                        if every { "every_path" } else { "path" },
                        if every {
                            "over, through and max"
                        } else {
                            "over, avoid, max and witness"
                        }
                    ));
                }
            }
            seen.push(opt);
        }
        self.expect(")")
            .map_err(|e| format!("{e} (the path pattern at {at})"))?;
        Ok(p)
    }

    /// `N where body` or a term; the body ends where the pattern's next `name:` option starts.
    fn avoid(&mut self) -> Result<Avoid, String> {
        if let (Some(Tok::Var(v)), Some(Tok::Ident(w))) = (self.peek().cloned(), self.peek_at(1))
            && w == "where"
        {
            self.k += 2;
            self.enter()?;
            let mut lits = vec![self.lit()?];
            while self.is(",")
                && !(matches!(self.peek_at(1), Some(Tok::Ident(_)))
                    && self.peek_at(2) == Some(&Tok::Punct(":")))
            {
                self.k += 1;
                if lits.len() == MAX_BODY {
                    return Err(format!(
                        "{}: a body of more than {MAX_BODY} literals, the limit of cq",
                        self.here()
                    ));
                }
                lits.push(self.lit()?);
            }
            self.depth -= 1;
            return Ok(Avoid::Where(v, lits));
        }
        Ok(Avoid::Node(self.term()?))
    }

    /// `property name(scope: …) { … }`, checked: `start` and `error` named, at most
    /// [`MAX_REGISTERS`] registers, each set before it is read, a message naming states and
    /// registers only.
    fn property(&mut self) -> Result<Property, String> {
        let at = self.here();
        self.k += 1;
        let name = self.ident()?;
        self.expect("(")?;
        if self.ident()? != "scope" {
            return Err(format!(
                "{at}: property {name} takes (scope: body) or (scope: calls)"
            ));
        }
        self.expect(":")?;
        let scope = match self.ident()?.as_str() {
            "body" => Scope::Body,
            "calls" => Scope::Calls,
            other => {
                return Err(format!(
                    "{at}: property {name}: scope {other}; a property follows a body or calls"
                ));
            }
        };
        self.expect(")")?;
        self.expect("{")?;
        let mut transitions = Vec::new();
        let mut message = None;
        while !self.eat("}") {
            let t_at = self.here();
            let first = self.ident()?;
            if first == "message" {
                if message.is_some() {
                    return Err(format!("{t_at}: property {name} has one message"));
                }
                message = Some(self.string()?);
                continue;
            }
            self.expect("->")?;
            let to = self.ident()?;
            self.expect(":")?;
            let event = match self.peek() {
                Some(Tok::Ident(w)) if w == "exit" => {
                    self.k += 1;
                    if scope == Scope::Calls {
                        return Err(format!(
                            "{t_at}: exit is an event of scope: body; a call path has calls only"
                        ));
                    }
                    Event::Exit
                }
                Some(Tok::Ident(w)) if w == "defer" => {
                    return Err(format!(
                        "{t_at}: defer call{{…}} waits for the defer statement of cir/1 (§ 8 of {DESIGN}), which cq cannot see yet"
                    ));
                }
                _ => {
                    self.registers = true;
                    let a = self.atom();
                    self.registers = false;
                    let a = a?;
                    if a.rel != "call" || !matches!(a.args, Args::Named(_)) {
                        return Err(format!(
                            "{t_at}: an event is exit or a record pattern of call, call{{…}}; not {a}"
                        ));
                    }
                    Event::Call(a)
                }
            };
            let mut assign = Vec::new();
            if self.eat("=>") {
                loop {
                    let r = self.ident()?;
                    self.expect(":=")?;
                    let v = match self.peek().cloned() {
                        Some(Tok::Var(v)) if v != "_" => v,
                        _ => return self.err("the variable the register takes"),
                    };
                    self.k += 1;
                    assign.push((r, v));
                    if !self.eat(",") {
                        break;
                    }
                }
            }
            if first == "error" {
                return Err(format!("{t_at}: error ends a run; no transition leaves it"));
            }
            transitions.push(Transition {
                from: first,
                to,
                event,
                assign,
            });
        }
        let p = Property {
            name,
            scope,
            transitions,
            message,
        };
        check_property(&p).map_err(|e| format!("{at}: property {}: {e}", p.name))?;
        Ok(p)
    }
}

/// The registers `t`'s event reads: its lower-case variables.
pub fn registers_read(t: &Transition) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Event::Call(Atom {
        args: Args::Named(fs),
        ..
    }) = &t.event
    {
        for (_, f) in fs {
            if let Field::Term(Term::Var(v)) = f
                && v.starts_with(|c: char| c.is_ascii_lowercase())
                && !out.contains(v)
            {
                out.push(v.clone());
            }
        }
    }
    out
}

/// The states and registers of a property, in order of first mention, or why it is not one.
fn check_property(p: &Property) -> Result<(), String> {
    if !p.transitions.iter().any(|t| t.from == "start") {
        return Err("no transition leaves start".into());
    }
    if !p.transitions.iter().any(|t| t.to == "error") {
        return Err("no transition reaches error, so it reports nothing".into());
    }
    let mut regs: Vec<&str> = Vec::new();
    for t in &p.transitions {
        for (r, v) in &t.assign {
            if !r.starts_with(|c: char| c.is_ascii_lowercase()) {
                return Err(format!("{r} := {v}: a register's name is lower case"));
            }
            let mut vs = Vec::new();
            if let Event::Call(a) = &t.event {
                atom_vars_of(a, &mut vs);
            }
            if !vs.contains(v) {
                return Err(format!(
                    "{r} := {v}: {v} is not a field of the transition's call"
                ));
            }
            if !regs.contains(&r.as_str()) {
                regs.push(r);
            }
        }
    }
    if regs.len() > MAX_REGISTERS {
        return Err(format!(
            "{} registers ({}); a property sets at most {MAX_REGISTERS} registers",
            regs.len(),
            regs.join(", ")
        ));
    }
    for t in &p.transitions {
        for r in registers_read(t) {
            if !regs.contains(&r.as_str()) {
                return Err(format!(
                    "the register {r} is never set; set it with => {r} := X on an earlier transition"
                ));
            }
        }
    }
    if let Some(m) = &p.message {
        for name in placeholders(m) {
            let state = p.transitions.iter().any(|t| t.from == name || t.to == name);
            if !state && !regs.contains(&name.as_str()) {
                return Err(format!(
                    "message: {{{name}}} names neither a state nor a register"
                ));
            }
        }
    }
    Ok(())
}

/// The names a message holds in braces, `{start}`, in order.
pub fn placeholders(m: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = m;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        let name = &after[..close];
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            out.push(name.to_string());
        }
        rest = &after[close + 1..];
    }
    out
}

fn atom_vars_of(a: &Atom, out: &mut Vec<String>) {
    let mut push = |t: &Term| {
        if let Term::Var(v) = t {
            out.push(v.clone());
        }
    };
    match &a.args {
        Args::Positional(ts) => ts.iter().for_each(&mut push),
        Args::Named(fs) => fs.iter().for_each(|(_, f)| {
            if let Field::Term(t) = f {
                push(t);
            }
        }),
    }
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

/// Parse a cq module. A `check` declaration (§ 5.4) or a `defer` event (§ 8) is refused, naming
/// it.
pub fn parse(src: &str) -> Result<Program, String> {
    let toks = lex(src)?;
    Parser {
        toks,
        k: 0,
        depth: 0,
        registers: false,
    }
    .program()
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

    #[test]
    fn a_list_name_holds_hyphens_and_prints_back() {
        let p = parse("names never-fail = [\"a\"].\nr(X) :- func{id: X, name: in @names.never-fail}, X in @names.fold-words.").unwrap();
        assert_eq!(p.names[0].0, "never-fail");
        assert_eq!(
            p.rules[0].to_string(),
            "r(X) :- func{id: X, name: in @names.never-fail}, X in @names.fold-words."
        );
        let p = parse("r(X) :- s(X), Y = X - Z.").unwrap();
        assert_eq!(p.rules[0].to_string(), "r(X) :- s(X), Y = X - Z.");
    }

    #[test]
    fn nesting_past_the_limit_is_refused_naming_it() {
        let ok = format!(
            "?- {}a(X){}.",
            "(".repeat(MAX_NESTING),
            ")".repeat(MAX_NESTING)
        );
        assert!(parse(&ok).is_ok());
        let deep = format!(
            "?- {}a(X){}.",
            "(".repeat(MAX_NESTING + 1),
            ")".repeat(MAX_NESTING + 1)
        );
        let e = parse(&deep).unwrap_err();
        assert!(
            e.contains(&format!("nested deeper than {MAX_NESTING}")),
            "{e}"
        );
        let sum = format!("?- a(X), N = 1{}.", " + 1".repeat(MAX_NESTING));
        assert!(parse(&sum).is_ok());
        let sum = format!("?- a(X), N = 1{}.", " + 1".repeat(MAX_NESTING + 1));
        assert!(parse(&sum).unwrap_err().contains("the limit of cq"));
        let long = format!("?- a(X){}.", ", a(X)".repeat(MAX_BODY - 1));
        assert!(parse(&long).is_ok());
        let long = format!("?- a(X){}.", ", a(X)".repeat(MAX_BODY));
        assert!(
            parse(&long)
                .unwrap_err()
                .contains(&format!("more than {MAX_BODY} literals"))
        );
    }
}
