//! The interpreter: a program compiled into join plans, stratified, and evaluated bottom-up,
//! semi-naive, stratum by stratum.
//!
//! Every fact carries the unknown sites it rests on; none is definite. A negated atom that no
//! fact satisfies is definite unless a fact the unknown sites could add would satisfy it: then it
//! holds with those sites. To know which facts the unknown sites could add, each relation that
//! appears under a negation or an aggregate is evaluated a second time over the fact base plus
//! its wild facts (a field of a wild fact matches anything, `Top`), the upper bound of what the
//! documents could hold.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use crate::facts::{FactBase, ON_DEMAND, Ty, relation};
use crate::regex::Regex;
use crate::syntax::{
    AggFn, Args, Atom, Avoid, Event, Field, Lit, MAX_BODY, Op, Over, PathForm, PathPat, Program,
    Property, Rule, Scope, Term, placeholders, query_text, registers_read,
};
use crate::{Answer, CHECK_LISTS, Glob, Names, Row, Value};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum V {
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    List(Vec<V>),
    /// Anything: a field of a wild fact.
    Top,
}

impl V {
    fn of(v: &Value) -> V {
        match v {
            Value::Bool(b) => V::Bool(*b),
            Value::Int(n) => V::Int(*n),
            Value::Str(s) => V::Str(Rc::from(s.as_str())),
            Value::List(l) => V::List(l.iter().map(V::of).collect()),
        }
    }

    fn value(&self) -> Value {
        match self {
            V::Bool(b) => Value::Bool(*b),
            V::Int(n) => Value::Int(*n),
            V::Str(s) => Value::Str(s.to_string()),
            V::List(l) => Value::List(l.iter().map(V::value).collect()),
            V::Top => unreachable!("an answer holds no wild field"),
        }
    }

    fn has_top(&self) -> bool {
        match self {
            V::Top => true,
            V::List(l) => l.iter().any(V::has_top),
            _ => false,
        }
    }
}

/// The unknown sites a fact rests on, sorted; empty: definite.
type Ann = Vec<u32>;

fn union(a: &mut Ann, b: &[u32]) {
    if b.is_empty() {
        return;
    }
    a.extend_from_slice(b);
    a.sort_unstable();
    a.dedup();
}

/// A word split as `codespec check` splits names: at `_` and at case changes, lower case, a
/// final `s` dropped.
pub fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (k, &c) in chars.iter().enumerate() {
        if c == '_' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if c.is_uppercase() && !cur.is_empty() {
            let prev = chars[k - 1];
            let next_lower = chars.get(k + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower) {
                out.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.into_iter()
        .map(|w| {
            let w = w.to_lowercase();
            match w.strip_suffix('s') {
                Some(stem) if stem.len() >= 2 => stem.to_string(),
                _ => w,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Stores

/// The tuples of a relation by the values of some of their columns.
type Index = HashMap<Vec<V>, Vec<usize>>;

/// The facts of one relation, with an index per set of bound columns (the columns, ascending),
/// built when first asked.
#[derive(Default)]
struct Store {
    tuples: Vec<Vec<V>>,
    anns: Vec<Ann>,
    /// How often each tuple's annotation was replaced by a smaller one.
    shrunk: Vec<u8>,
    at: HashMap<Vec<V>, usize>,
    /// The tuples with a wild field; never in an index.
    tops: Vec<usize>,
    indexes: RefCell<HashMap<Vec<usize>, Index>>,
}

/// How often a fact's annotation is replaced by a smaller one; past that it only grows, so a
/// fixed point is always reached.
const MAX_SHRINKS: u8 = 16;

/// Whether the sorted `a` lies inside the sorted `b`.
fn subset(a: &[u32], b: &[u32]) -> bool {
    a.iter().all(|x| b.binary_search(x).is_ok())
}

impl Store {
    /// Adds a fact or what it rests on; whether anything changed. A fact derived twice rests on
    /// the sites of either derivation: a definite derivation makes it definite, one resting on
    /// fewer sites replaces the annotation (the fact holds whenever those sites allow it, the
    /// other derivation adds nothing), and one resting on other sites adds them.
    fn insert(&mut self, t: Vec<V>, ann: Ann) -> Option<usize> {
        if let Some(&k) = self.at.get(&t) {
            let old = &mut self.anns[k];
            if old.is_empty() || subset(old, &ann) {
                return None;
            }
            if subset(&ann, old) {
                if self.shrunk[k] == MAX_SHRINKS && !ann.is_empty() {
                    return None;
                }
                self.shrunk[k] = self.shrunk[k].saturating_add(1);
                *old = ann;
                return Some(k);
            }
            union(old, &ann);
            return Some(k);
        }
        let k = self.tuples.len();
        if t.iter().any(V::has_top) {
            self.tops.push(k);
        } else {
            for (cols, index) in self.indexes.get_mut().iter_mut() {
                index.entry(key_of(cols, &t)).or_default().push(k);
            }
        }
        self.at.insert(t.clone(), k);
        self.tuples.push(t);
        self.anns.push(ann);
        self.shrunk.push(0);
        Some(k)
    }

    /// The tuples whose columns in `mask` hold `key`, with every tuple holding a wild field.
    fn candidates(&self, cols: &[usize], key: &[V]) -> Vec<usize> {
        if cols.is_empty() || key.iter().any(V::has_top) {
            return (0..self.tuples.len()).collect();
        }
        let mut indexes = self.indexes.borrow_mut();
        let index = indexes.entry(cols.to_vec()).or_insert_with(|| {
            let mut ix: HashMap<Vec<V>, Vec<usize>> = HashMap::new();
            for (k, t) in self.tuples.iter().enumerate() {
                if !t.iter().any(V::has_top) {
                    ix.entry(key_of(cols, t)).or_default().push(k);
                }
            }
            ix
        });
        let mut out = index.get(key).cloned().unwrap_or_default();
        out.extend(&self.tops);
        out
    }
}

fn key_of(cols: &[usize], t: &[V]) -> Vec<V> {
    cols.iter().map(|&c| t[c].clone()).collect()
}

// ---------------------------------------------------------------------------------------------
// Compiled programs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Rel {
    Base(&'static str),
    Derived(usize),
}

#[derive(Debug, Clone, PartialEq)]
enum Arg {
    Var(usize),
    Val(V),
    Any,
    Arith(Box<Arg>, char, Box<Arg>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Builtin {
    Word,
    Int,
    Replace,
    Concat,
}

impl Builtin {
    fn of(name: &str) -> Option<(Builtin, usize, usize)> {
        // (builtin, arity, inputs: the first n arguments)
        Some(match name {
            "word" => (Builtin::Word, 3, 1),
            "int" => (Builtin::Int, 2, 1),
            "replace" => (Builtin::Replace, 4, 3),
            "concat" => (Builtin::Concat, 3, 2),
            _ => return None,
        })
    }
}

/// A names list, its entries read as `codespec check` reads the list of that name ([`Glob`]).
#[derive(Debug)]
struct NameList {
    entries: Vec<String>,
    exact: BTreeSet<String>,
    glob: Glob,
}

impl NameList {
    fn new(entries: &[String], glob: Glob) -> NameList {
        NameList {
            entries: entries.to_vec(),
            exact: entries.iter().cloned().collect(),
            glob,
        }
    }

    fn holds(&self, s: &str) -> bool {
        if self.exact.contains(s) {
            return true;
        }
        match self.glob {
            Glob::Exact => false,
            Glob::Prefix => self
                .entries
                .iter()
                .any(|e| e.strip_suffix('*').is_some_and(|p| s.starts_with(p))),
            Glob::File => self.entries.iter().any(|e| {
                if let Some(suffix) = e.strip_prefix('*') {
                    s.ends_with(suffix)
                } else {
                    e.strip_suffix('*').is_some_and(|p| s.starts_with(p))
                }
            }),
        }
    }
}

#[derive(Debug, Clone)]
enum Step {
    Scan(Rel, Vec<Arg>),
    Neg(Rel, Vec<Arg>),
    Cmp(Arg, Op, Arg),
    Match(Arg, Regex, bool),
    /// Membership in a names list; with the variable unbound, each entry as written.
    In(Arg, Rc<NameList>, bool),
    Builtin(Builtin, Vec<Arg>),
    Agg {
        var: usize,
        func: AggFn,
        rel: Rel,
        /// (column of the aggregated relation, the outer variable it is)
        outer: Vec<(usize, usize)>,
        target: Option<usize>,
    },
    Path(Box<PathStep>),
}

/// What a path may not pass.
#[derive(Debug, Clone)]
enum AvoidStep {
    None,
    Node(Arg),
    /// The nodes in column 0 of a relation the compiler made for `N where body`, its other
    /// columns the variables the body shares with the rule.
    Rel(Rel, Vec<Arg>),
}

/// A path pattern compiled: `neg` for `!path` and `every_path`.
#[derive(Debug, Clone)]
struct PathStep {
    over: Over,
    from: Arg,
    to: Arg,
    avoid: AvoidStep,
    max: Option<usize>,
    witness: Option<usize>,
    neg: bool,
}

/// A path pattern lowered; variables still by name.
#[derive(Debug, Clone)]
struct LowPath {
    over: Over,
    from: Term,
    to: Term,
    avoid: LowAvoid,
    max: Option<usize>,
    witness: Option<String>,
    neg: bool,
}

#[derive(Debug, Clone)]
enum LowAvoid {
    None,
    Node(Term),
    Rel(Rel, Vec<Term>),
}

/// A property compiled: its states (`start` first), its transitions, the registers and the
/// message.
struct CProp {
    head: usize,
    scope: Scope,
    states: Vec<String>,
    error: usize,
    trans: Vec<CTrans>,
    regs: usize,
    message: Vec<Piece>,
    /// The states whose leaving node the message prints.
    marked: Vec<bool>,
}

struct CTrans {
    from: usize,
    to: usize,
    event: CEvent,
}

enum CEvent {
    Exit,
    /// The call sites the event's relation holds in column 0, each with the values of its
    /// variables: (register, column) pairs the register must equal, and the registers set.
    /// `fields` names the `call` field each column holding a variable comes from, for the
    /// fields a listed call does not read (`recv`, `args`).
    Call {
        rel: usize,
        checks: Vec<(usize, usize)>,
        assigns: Vec<(usize, usize)>,
        fields: Vec<(usize, &'static str)>,
    },
}

/// A part of a property's message.
enum Piece {
    Text(String),
    State(usize),
    Reg(usize),
}

struct CRule {
    head: usize,
    args: Vec<Arg>,
    steps: Vec<Step>,
    vars: usize,
}

/// A relation the program derives: a user's, or one the compiler made for a negation or an
/// aggregate (named `__`…).
struct Derived {
    name: String,
    arity: usize,
}

pub struct Compiled {
    derived: Vec<Derived>,
    by_name: HashMap<String, usize>,
    rules: Vec<CRule>,
    /// The rules of each derived relation.
    by_head: Vec<Vec<usize>>,
    query: usize,
    vars: Vec<String>,
    /// Each derived relation's stratum, from 1.
    strata: Vec<usize>,
    props: Vec<CProp>,
    /// The property each derived relation is, if one.
    prop_of: Vec<Option<usize>>,
}

/// A body literal once record patterns, attributes, disjunctions and nested negations are
/// lowered; variables still by name.
#[derive(Debug, Clone)]
enum Low {
    Scan(Rel, Vec<Term>),
    Neg(Rel, Vec<Term>),
    Cmp(Term, Op, Term),
    Match(Term, String, bool),
    In(Term, String, bool),
    Builtin(Builtin, Vec<Term>),
    Agg {
        var: String,
        func: AggFn,
        rel: Rel,
        outer: Vec<String>,
        target: Option<String>,
    },
    Path(Box<LowPath>),
}

fn term_vars(t: &Term, out: &mut Vec<String>) {
    match t {
        Term::Var(v) => out.push(v.clone()),
        Term::Attr(v, _) => out.push(v.clone()),
        Term::Arith(a, _, b) => {
            term_vars(a, out);
            term_vars(b, out);
        }
        _ => {}
    }
}

fn atom_vars(a: &Atom, out: &mut Vec<String>) {
    match &a.args {
        Args::Positional(ts) => ts.iter().for_each(|t| term_vars(t, out)),
        Args::Named(fs) => fs.iter().for_each(|(_, f)| {
            if let Field::Term(t) = f {
                term_vars(t, out);
            }
        }),
    }
}

fn lit_vars(l: &Lit, out: &mut Vec<String>) {
    match l {
        Lit::Atom(a) | Lit::Not(a) => atom_vars(a, out),
        Lit::NotCond(c) => lit_vars(c, out),
        Lit::Cmp(a, _, b) => {
            term_vars(a, out);
            term_vars(b, out);
        }
        Lit::Match(t, _) | Lit::In(t, _) => term_vars(t, out),
        Lit::Agg {
            var, target, body, ..
        } => {
            out.push(var.clone());
            if let Some(t) = target {
                term_vars(t, out);
            }
            body.iter().for_each(|l| lit_vars(l, out));
        }
        Lit::Or(alts) => alts.iter().flatten().for_each(|l| lit_vars(l, out)),
        Lit::Path(p) => path_vars(p, out),
    }
}

/// The variables of a path pattern: its ends, its witness, the node it avoids and those of the
/// body after `where` but the node that body names.
fn path_vars(p: &PathPat, out: &mut Vec<String>) {
    term_vars(&p.from, out);
    term_vars(&p.to, out);
    if let Some(w) = &p.witness {
        out.push(w.clone());
    }
    match &p.avoid {
        Some(Avoid::Node(t)) => term_vars(t, out),
        Some(Avoid::Where(n, body)) => {
            let mut vs = Vec::new();
            body.iter().for_each(|l| lit_vars(l, &mut vs));
            out.extend(vs.into_iter().filter(|v| v != n));
        }
        None => {}
    }
}

/// The most bodies a program compiles into: the bodies the disjunctions of its rules and its
/// query expand into, with those of the negations and aggregates inside them, once for every
/// body they sit in.
pub const MAX_BODIES: usize = 1024;

/// How many literals a body holds once its parenthesised groups are flattened, the longest
/// alternative of each counted.
fn flat_len(body: &[Lit]) -> usize {
    body.iter().fold(0usize, |n, l| match l {
        Lit::Or(alts) => n.saturating_add(alts.iter().map(|a| flat_len(a)).max().unwrap_or(0)),
        _ => n.saturating_add(1),
    })
}

/// The variables of `l` outside the bodies of the aggregates in it.
fn visible_vars(l: &Lit, out: &mut Vec<String>) {
    match l {
        Lit::Agg { var, .. } => out.push(var.clone()),
        Lit::Or(alts) => alts.iter().flatten().for_each(|l| visible_vars(l, out)),
        l => lit_vars(l, out),
    }
}

/// How many bodies `body` expands into, counted without expanding, saturating.
fn bodies(body: &[Lit]) -> usize {
    body.iter().fold(1usize, |n, l| match l {
        Lit::Or(alts) => n.saturating_mul(
            alts.iter()
                .fold(0usize, |m, alt| m.saturating_add(bodies(alt))),
        ),
        _ => n,
    })
}

/// The bodies a body with disjunctions stands for.
fn expand(body: &[Lit]) -> Vec<Vec<Lit>> {
    let mut out: Vec<Vec<Lit>> = vec![Vec::new()];
    for l in body {
        match l {
            Lit::Or(alts) => {
                let mut next = Vec::new();
                for prefix in &out {
                    for alt in alts {
                        for tail in expand(alt) {
                            let mut b = prefix.clone();
                            b.extend(tail);
                            next.push(b);
                        }
                    }
                }
                out = next;
            }
            l => out.iter_mut().for_each(|b| b.push(l.clone())),
        }
    }
    out
}

struct Compiler<'a> {
    derived: Vec<Derived>,
    by_name: HashMap<String, usize>,
    rules: Vec<CRule>,
    names: &'a BTreeMap<String, Vec<String>>,
    lists: HashMap<String, Rc<NameList>>,
    fresh: usize,
    /// The bodies compiled so far.
    bodies: usize,
}

fn negate(op: Op) -> Option<Op> {
    Some(match op {
        Op::Eq => Op::Ne,
        Op::Ne => Op::Eq,
        Op::Lt => Op::Ge,
        Op::Le => Op::Gt,
        Op::Gt => Op::Le,
        Op::Ge => Op::Lt,
        Op::Bind => return None,
    })
}

impl Compiler<'_> {
    fn fresh(&mut self) -> String {
        self.fresh += 1;
        format!("_{}", self.fresh)
    }

    fn derived(&mut self, name: &str, arity: usize) -> Result<usize, String> {
        if let Some(&d) = self.by_name.get(name) {
            if self.derived[d].arity != arity {
                return Err(format!(
                    "{name} has {} fields in one place and {arity} in another",
                    self.derived[d].arity
                ));
            }
            return Ok(d);
        }
        self.derived.push(Derived {
            name: name.to_string(),
            arity,
        });
        self.by_name
            .insert(name.to_string(), self.derived.len() - 1);
        Ok(self.derived.len() - 1)
    }

    fn list(&mut self, name: &str) -> Result<Rc<NameList>, String> {
        if let Some(l) = self.lists.get(name) {
            return Ok(l.clone());
        }
        let Some(items) = self.names.get(name) else {
            return Err(format!(
                "no names list {name}: declare it with names {name} = [\"…\"]."
            ));
        };
        let glob = CHECK_LISTS
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(Glob::Prefix, |(_, g)| *g);
        let l = Rc::new(NameList::new(items, glob));
        self.lists.insert(name.to_string(), l.clone());
        Ok(l)
    }

    /// An atom as a relation and positional terms, with the conditions its fields add.
    fn atom(&mut self, a: &Atom, out_conds: &mut Vec<Low>) -> Result<(Rel, Vec<Term>), String> {
        if let Some(r) = relation(&a.rel) {
            let mut terms = vec![Term::Any; r.fields.len()];
            match &a.args {
                Args::Positional(ts) => {
                    if ts.len() != r.fields.len() {
                        return Err(format!(
                            "{} has {} fields, not {}: {}",
                            r.name,
                            r.fields.len(),
                            ts.len(),
                            a
                        ));
                    }
                    terms = ts.clone();
                }
                Args::Named(fs) => {
                    for (f, v) in fs {
                        let Some(c) = r.field(f) else {
                            let known: Vec<&str> = r.fields.iter().map(|(n, _)| *n).collect();
                            return Err(format!(
                                "{} has no field {f}; its fields are {}",
                                r.name,
                                known.join(", ")
                            ));
                        };
                        terms[c] = match v {
                            Field::Term(t) => t.clone(),
                            Field::Regex(re) => {
                                let x = self.fresh();
                                out_conds.push(Low::Match(Term::Var(x.clone()), re.clone(), false));
                                Term::Var(x)
                            }
                            Field::In(l) => {
                                let x = self.fresh();
                                out_conds.push(Low::In(Term::Var(x.clone()), l.clone(), false));
                                Term::Var(x)
                            }
                        };
                    }
                }
            }
            for (t, (f, ty)) in terms.iter().zip(r.fields) {
                let ok = match t {
                    Term::Str(_) => matches!(ty, Ty::Str | Ty::Node | Ty::Type),
                    Term::Int(_) => *ty == Ty::Int,
                    Term::Bool(_) => *ty == Ty::Bool,
                    _ => true,
                };
                if !ok {
                    return Err(format!("{}.{f} is a {}, not {t}: {a}", r.name, ty.name()));
                }
            }
            return Ok((Rel::Base(r.name), terms));
        }
        if !self.by_name.contains_key(&a.rel)
            && let Some(w) = crate::facts::waiting(&a.rel)
        {
            return Err(format!("{a}: {} {}", a.rel, w.why));
        }
        let Args::Positional(ts) = &a.args else {
            return Err(format!(
                "{} is not a relation of the fact base; write its fields in order: {}(…)",
                a.rel, a.rel
            ));
        };
        let Some(&d) = self.by_name.get(&a.rel) else {
            return Err(format!(
                "no relation {}: it is neither in the fact base (codespec query --schema) nor the head of a rule",
                a.rel
            ));
        };
        let d = self.derived(&a.rel.clone(), ts.len()).map(|_| d)?;
        Ok((Rel::Derived(d), ts.clone()))
    }

    /// Replaces each `X.field` in `t` with a fresh variable, a scan of `type` binding it before.
    fn attrs(&mut self, t: &Term, pre: &mut Vec<Low>) -> Result<Term, String> {
        Ok(match t {
            Term::Attr(v, f) => {
                let r = relation("type").expect("type is a relation");
                let Some(c) = r.field(f) else {
                    let known: Vec<&str> = r.fields.iter().map(|(n, _)| *n).collect();
                    return Err(format!(
                        "{v}.{f}: a type has no field {f}; its fields are {}",
                        known.join(", ")
                    ));
                };
                let x = self.fresh();
                let mut terms = vec![Term::Any; r.fields.len()];
                terms[0] = Term::Var(v.clone());
                terms[c] = Term::Var(x.clone());
                pre.push(Low::Scan(Rel::Base("type"), terms));
                Term::Var(x)
            }
            Term::Arith(a, op, b) => Term::Arith(
                Box::new(self.attrs(a, pre)?),
                *op,
                Box::new(self.attrs(b, pre)?),
            ),
            t => t.clone(),
        })
    }

    /// One literal lowered; `elsewhere` counts each variable's uses in the rest of the rule.
    fn lower(
        &mut self,
        l: &Lit,
        elsewhere: &dyn Fn(&str) -> bool,
        out: &mut Vec<Low>,
    ) -> Result<(), String> {
        match l {
            Lit::Atom(a) => {
                let mut conds = Vec::new();
                if let Some((b, n, _)) = Builtin::of(&a.rel) {
                    let Args::Positional(ts) = &a.args else {
                        return Err(format!("{} takes its arguments in order", a.rel));
                    };
                    if ts.len() != n {
                        return Err(format!("{} takes {n} arguments: {a}", a.rel));
                    }
                    let mut pre = Vec::new();
                    let ts = ts
                        .iter()
                        .map(|t| self.attrs(t, &mut pre))
                        .collect::<Result<Vec<_>, _>>()?;
                    out.extend(pre);
                    out.push(Low::Builtin(b, ts));
                    return Ok(());
                }
                let (rel, ts) = self.atom(a, &mut conds)?;
                let mut pre = Vec::new();
                let ts = ts
                    .iter()
                    .map(|t| self.attrs(t, &mut pre))
                    .collect::<Result<Vec<_>, _>>()?;
                out.push(Low::Scan(rel, ts));
                out.extend(pre);
                out.extend(conds);
            }
            Lit::Not(a) => {
                if Builtin::of(&a.rel).is_some() {
                    return Err(format!("{} cannot be negated", a.rel));
                }
                let mut conds = Vec::new();
                let (rel, ts) = self.atom(a, &mut conds)?;
                let complex = !conds.is_empty()
                    || ts
                        .iter()
                        .any(|t| matches!(t, Term::Attr(..) | Term::Arith(..)));
                if !complex {
                    let ts = ts
                        .into_iter()
                        .map(|t| match t {
                            Term::Var(v) if !elsewhere(&v) => Term::Any,
                            t => t,
                        })
                        .collect();
                    out.push(Low::Neg(rel, ts));
                    return Ok(());
                }
                let mut vs = Vec::new();
                atom_vars(a, &mut vs);
                let mut shared: Vec<String> = Vec::new();
                for v in vs {
                    if elsewhere(&v) && !shared.contains(&v) {
                        shared.push(v);
                    }
                }
                let aux = format!("__not{}", self.derived.len());
                let d = self.derived(&aux, shared.len())?;
                let head = Atom {
                    rel: aux,
                    args: Args::Positional(shared.iter().cloned().map(Term::Var).collect()),
                };
                self.rule(
                    &Rule {
                        head,
                        body: vec![Lit::Atom(a.clone())],
                    },
                    d,
                )?;
                out.push(Low::Neg(
                    Rel::Derived(d),
                    shared.into_iter().map(Term::Var).collect(),
                ));
            }
            Lit::NotCond(c) => match &**c {
                Lit::Cmp(a, op, b) => {
                    let Some(op) = negate(*op) else {
                        return Err(format!("!({c}): = binds, it cannot be negated"));
                    };
                    let mut pre = Vec::new();
                    let a = self.attrs(a, &mut pre)?;
                    let b = self.attrs(b, &mut pre)?;
                    out.extend(pre);
                    out.push(Low::Cmp(a, op, b));
                }
                Lit::Match(t, re) => out.push(Low::Match(t.clone(), re.clone(), true)),
                Lit::In(t, l) => out.push(Low::In(t.clone(), l.clone(), true)),
                other => return Err(format!("!({other}): only a condition can be negated so")),
            },
            Lit::Cmp(a, op, b) => {
                let mut pre = Vec::new();
                let a = self.attrs(a, &mut pre)?;
                let b = self.attrs(b, &mut pre)?;
                out.extend(pre);
                out.push(Low::Cmp(a, *op, b));
            }
            Lit::Match(t, re) => {
                let mut pre = Vec::new();
                let t = self.attrs(t, &mut pre)?;
                out.extend(pre);
                out.push(Low::Match(t, re.clone(), false));
            }
            Lit::In(t, l) => {
                let mut pre = Vec::new();
                let t = self.attrs(t, &mut pre)?;
                out.extend(pre);
                out.push(Low::In(t, l.clone(), false));
            }
            Lit::Agg {
                var,
                func,
                target,
                body,
            } => {
                let target = match target {
                    None => None,
                    Some(Term::Var(v)) => Some(v.clone()),
                    Some(t) => {
                        return Err(format!(
                            "{}{{{t} : …}}: the value aggregated is a variable",
                            match func {
                                AggFn::Count => "count",
                                AggFn::Min => "min",
                                AggFn::Max => "max",
                                AggFn::Sum => "sum",
                                AggFn::Collect => "collect",
                            }
                        ));
                    }
                };
                // Every `_` in the body is a variable of its own, so each binding counts.
                let body: Vec<Lit> = body.iter().map(|l| self.name_anys(l)).collect();
                let mut vs = Vec::new();
                body.iter().for_each(|l| lit_vars(l, &mut vs));
                // A variable only an inner aggregate's body holds, and nothing outside this
                // aggregate, is that inner aggregate's own: not a column here.
                let mut visible = Vec::new();
                body.iter().for_each(|l| visible_vars(l, &mut visible));
                vs.retain(|v| visible.contains(v) || elsewhere(v));
                let (mut outer, mut local) = (Vec::new(), Vec::new());
                for v in vs {
                    if outer.contains(&v) || local.contains(&v) {
                        continue;
                    }
                    if elsewhere(&v) && Some(&v) != target.as_ref() {
                        outer.push(v);
                    } else {
                        local.push(v);
                    }
                }
                if let Some(t) = &target
                    && !local.contains(t)
                    && !outer.contains(t)
                {
                    return Err(format!("the aggregated {t} does not occur in its body"));
                }
                let mut head_vars = outer.clone();
                head_vars.extend(local);
                let aux = format!("__agg{}", self.derived.len());
                let d = self.derived(&aux, head_vars.len())?;
                self.rule(
                    &Rule {
                        head: Atom {
                            rel: aux,
                            args: Args::Positional(
                                head_vars.iter().cloned().map(Term::Var).collect(),
                            ),
                        },
                        body,
                    },
                    d,
                )?;
                out.push(Low::Agg {
                    var: var.clone(),
                    func: *func,
                    rel: Rel::Derived(d),
                    outer,
                    target: target.map(|t| {
                        head_vars
                            .iter()
                            .position(|v| *v == t)
                            .expect("the target is a head variable")
                            .to_string()
                    }),
                });
            }
            Lit::Or(_) => unreachable!("disjunctions are expanded first"),
            Lit::Path(p) => {
                for t in [&p.from, &p.to] {
                    if matches!(t, Term::Attr(..) | Term::Arith(..)) {
                        return Err(format!("{p}: a path's ends are variables or nodes"));
                    }
                }
                let avoid = match &p.avoid {
                    None => LowAvoid::None,
                    Some(Avoid::Node(t)) => LowAvoid::Node(t.clone()),
                    Some(Avoid::Where(n, body)) => {
                        let mut vs = Vec::new();
                        body.iter().for_each(|l| lit_vars(l, &mut vs));
                        if !vs.contains(n) {
                            return Err(format!("{p}: the body after where does not name {n}"));
                        }
                        let mut ends = Vec::new();
                        term_vars(&p.from, &mut ends);
                        term_vars(&p.to, &mut ends);
                        ends.extend(p.witness.iter().cloned());
                        let mut shared: Vec<String> = Vec::new();
                        for v in vs {
                            if v != *n
                                && (elsewhere(&v) || ends.contains(&v))
                                && !shared.contains(&v)
                            {
                                shared.push(v);
                            }
                        }
                        let aux = format!("__avoid{}", self.derived.len());
                        let mut head = vec![Term::Var(n.clone())];
                        head.extend(shared.iter().cloned().map(Term::Var));
                        let d = self.derived(&aux, head.len())?;
                        self.rule(
                            &Rule {
                                head: Atom {
                                    rel: aux,
                                    args: Args::Positional(head),
                                },
                                body: body.clone(),
                            },
                            d,
                        )?;
                        LowAvoid::Rel(Rel::Derived(d), shared.into_iter().map(Term::Var).collect())
                    }
                };
                out.push(Low::Path(Box::new(LowPath {
                    over: p.over,
                    from: p.from.clone(),
                    to: p.to.clone(),
                    avoid,
                    max: p.max.map(|m| usize::try_from(m).unwrap_or(usize::MAX)),
                    witness: p.witness.clone(),
                    neg: p.form != PathForm::Path,
                })));
            }
        }
        Ok(())
    }

    /// An atom of an aggregate's body with a variable of its own for every `_` and every field
    /// a record pattern leaves out, so the aggregate counts the matching tuples themselves.
    fn name_anys(&mut self, l: &Lit) -> Lit {
        let t = |t: &Term, c: &mut Self| match t {
            Term::Any => Term::Var(c.fresh().replacen('_', "_any", 1)),
            t => t.clone(),
        };
        match l {
            Lit::Atom(a) => {
                let args = match &a.args {
                    Args::Positional(ts) => {
                        Args::Positional(ts.iter().map(|x| t(x, self)).collect())
                    }
                    Args::Named(fs) => {
                        let mut named: Vec<(String, Field)> = fs
                            .iter()
                            .map(|(n, f)| {
                                (
                                    n.clone(),
                                    match f {
                                        Field::Term(x) => Field::Term(t(x, self)),
                                        f => f.clone(),
                                    },
                                )
                            })
                            .collect();
                        if let Some(r) = relation(&a.rel) {
                            for (f, _) in r.fields {
                                if !fs.iter().any(|(n, _)| n == f) {
                                    named.push((f.to_string(), Field::Term(t(&Term::Any, self))));
                                }
                            }
                        }
                        Args::Named(named)
                    }
                };
                Lit::Atom(Atom {
                    rel: a.rel.clone(),
                    args,
                })
            }
            l => l.clone(),
        }
    }

    /// Compile `r` into the rules of the derived relation `head`.
    fn rule(&mut self, r: &Rule, head: usize) -> Result<(), String> {
        let Args::Positional(head_terms) = &r.head.args else {
            return Err(format!(
                "{}: a rule's head takes its fields in order",
                r.head
            ));
        };
        let what = if r.head.rel == "__query" {
            "the query".to_string()
        } else {
            r.head.to_string()
        };
        let n = bodies(&r.body);
        if n > MAX_BODIES {
            return Err(format!(
                "{what}: its disjunctions expand into {} bodies, more than {MAX_BODIES}, the limit of cq; split it into rules",
                if n == usize::MAX {
                    "too many".to_string()
                } else {
                    n.to_string()
                }
            ));
        }
        if self.bodies + n > MAX_BODIES {
            return Err(format!(
                "{what}: the program compiles into more than {MAX_BODIES} bodies (the disjunctions of its rules expanded, with the negations and aggregates inside them once for every body they sit in), the limit of cq; split the disjunctions into rules"
            ));
        }
        self.bodies += n;
        let len = flat_len(&r.body);
        if len > MAX_BODY {
            return Err(format!(
                "{what}: a body of {len} literals once its parenthesised groups are flattened, more than {MAX_BODY}, the limit of cq"
            ));
        }
        for body in expand(&r.body) {
            let mut lows = Vec::new();
            for (k, l) in body.iter().enumerate() {
                let mut others: Vec<String> = Vec::new();
                head_terms.iter().for_each(|t| term_vars(t, &mut others));
                for (j, m) in body.iter().enumerate() {
                    if j != k {
                        lit_vars(m, &mut others);
                    }
                }
                if let Lit::Agg { var, .. } = l {
                    others.push(var.clone());
                }
                let elsewhere = |v: &str| others.iter().any(|o| o == v);
                self.lower(l, &elsewhere, &mut lows)?;
            }
            let plan = self.plan(&r.head.to_string(), head_terms, lows)?;
            self.rules.push(CRule {
                head,
                args: plan.0,
                steps: plan.1,
                vars: plan.2,
            });
        }
        Ok(())
    }

    /// Orders the literals so each runs with what it needs bound, and numbers the variables.
    #[allow(clippy::type_complexity)]
    fn plan(
        &mut self,
        what: &str,
        head: &[Term],
        lows: Vec<Low>,
    ) -> Result<(Vec<Arg>, Vec<Step>, usize), String> {
        let mut slots: HashMap<String, usize> = HashMap::new();
        let mut bound: HashSet<String> = HashSet::new();
        let mut pending: Vec<Low> = lows;
        let mut steps = Vec::new();
        let vars_of = |t: &Term| {
            let mut v = Vec::new();
            term_vars(t, &mut v);
            v
        };
        let all_bound = |ts: &[&Term], bound: &HashSet<String>| {
            ts.iter()
                .all(|t| vars_of(t).iter().all(|v| bound.contains(v)))
        };
        let path_needs = |p: &LowPath| {
            let mut vs = Vec::new();
            match &p.avoid {
                LowAvoid::None => {}
                LowAvoid::Node(t) => term_vars(t, &mut vs),
                LowAvoid::Rel(_, ts) => ts.iter().for_each(|t| term_vars(t, &mut vs)),
            }
            if p.neg {
                term_vars(&p.from, &mut vs);
                term_vars(&p.to, &mut vs);
            }
            vs
        };
        while !pending.is_empty() {
            // A path waits for one of its ends to be bound while another literal can run; with
            // none left it walks from every node.
            let is_ready = |l: &Low, strict: bool| match l {
                Low::Path(p) => {
                    path_needs(p).iter().all(|v| bound.contains(v))
                        && (!strict
                            || p.neg
                            || all_bound(&[&p.from], &bound)
                            || all_bound(&[&p.to], &bound))
                }
                Low::Scan(..) => true,
                Low::Neg(_, ts) => all_bound(&ts.iter().collect::<Vec<_>>(), &bound),
                Low::Cmp(a, Op::Bind, b) => {
                    let free = |t: &Term| matches!(t, Term::Var(v) if !bound.contains(v));
                    (free(a) && all_bound(&[b], &bound))
                        || (free(b) && all_bound(&[a], &bound))
                        || all_bound(&[a, b], &bound)
                }
                Low::Cmp(a, _, b) => all_bound(&[a, b], &bound),
                Low::Match(t, ..) => all_bound(&[t], &bound),
                Low::In(t, _, neg) => {
                    all_bound(&[t], &bound)
                        || (!neg && matches!(t, Term::Var(v) if !bound.contains(v)))
                }
                Low::Builtin(b, ts) => {
                    let n = Builtin::of(match b {
                        Builtin::Word => "word",
                        Builtin::Int => "int",
                        Builtin::Replace => "replace",
                        Builtin::Concat => "concat",
                    })
                    .expect("a builtin")
                    .2;
                    all_bound(&ts[..n].iter().collect::<Vec<_>>(), &bound)
                }
                Low::Agg { outer, .. } => outer.iter().all(|v| bound.contains(v)),
            };
            let ready = pending
                .iter()
                .position(|l| is_ready(l, true))
                .or_else(|| pending.iter().position(|l| is_ready(l, false)));
            let Some(k) = ready else {
                let mut waiting = BTreeSet::new();
                for l in &pending {
                    let mut vs = Vec::new();
                    match l {
                        Low::Neg(_, ts) | Low::Builtin(_, ts) => {
                            ts.iter().for_each(|t| term_vars(t, &mut vs))
                        }
                        Low::Cmp(a, _, b) => {
                            term_vars(a, &mut vs);
                            term_vars(b, &mut vs);
                        }
                        Low::Match(t, ..) | Low::In(t, ..) => term_vars(t, &mut vs),
                        Low::Agg { outer, .. } => vs.extend(outer.iter().cloned()),
                        Low::Path(p) => vs.extend(path_needs(p)),
                        Low::Scan(..) => {}
                    }
                    waiting.extend(
                        vs.into_iter()
                            .filter(|v| !bound.contains(v) && !v.starts_with('_')),
                    );
                }
                let w: Vec<String> = waiting.into_iter().collect();
                return Err(format!(
                    "{what}: {} is used where nothing binds it; bind it with a positive atom",
                    if w.is_empty() {
                        "a variable".to_string()
                    } else {
                        w.join(", ")
                    }
                ));
            };
            let l = pending.remove(k);
            let slot = |v: &str, slots: &mut HashMap<String, usize>| {
                let n = slots.len();
                *slots.entry(v.to_string()).or_insert(n)
            };
            let arg = |t: &Term, slots: &mut HashMap<String, usize>| -> Arg {
                fn go(t: &Term, f: &mut dyn FnMut(&str) -> usize) -> Arg {
                    match t {
                        Term::Var(v) => Arg::Var(f(v)),
                        Term::Any => Arg::Any,
                        Term::Str(s) => Arg::Val(V::Str(Rc::from(s.as_str()))),
                        Term::Int(n) => Arg::Val(V::Int(*n)),
                        Term::Bool(b) => Arg::Val(V::Bool(*b)),
                        Term::Attr(..) => unreachable!("attributes are lowered"),
                        Term::Arith(a, op, b) => {
                            Arg::Arith(Box::new(go(a, f)), *op, Box::new(go(b, f)))
                        }
                    }
                }
                go(t, &mut |v| slot(v, slots))
            };
            let bind = |t: &Term, bound: &mut HashSet<String>| {
                for v in vars_of(t) {
                    bound.insert(v);
                }
            };
            let step = match l {
                Low::Scan(rel, ts) => {
                    let args = ts.iter().map(|t| arg(t, &mut slots)).collect();
                    ts.iter().for_each(|t| bind(t, &mut bound));
                    Step::Scan(rel, args)
                }
                Low::Neg(rel, ts) => {
                    Step::Neg(rel, ts.iter().map(|t| arg(t, &mut slots)).collect())
                }
                Low::Cmp(a, op, b) => {
                    let s = Step::Cmp(arg(&a, &mut slots), op, arg(&b, &mut slots));
                    if op == Op::Bind {
                        bind(&a, &mut bound);
                        bind(&b, &mut bound);
                    }
                    s
                }
                Low::Match(t, re, neg) => Step::Match(arg(&t, &mut slots), Regex::new(&re)?, neg),
                Low::In(t, l, neg) => {
                    let s = Step::In(arg(&t, &mut slots), self.list(&l)?, neg);
                    bind(&t, &mut bound);
                    s
                }
                Low::Builtin(b, ts) => {
                    let args = ts.iter().map(|t| arg(t, &mut slots)).collect();
                    ts.iter().for_each(|t| bind(t, &mut bound));
                    Step::Builtin(b, args)
                }
                Low::Agg {
                    var,
                    func,
                    rel,
                    outer,
                    target,
                } => {
                    let outer = outer
                        .iter()
                        .enumerate()
                        .map(|(c, v)| (c, slot(v, &mut slots)))
                        .collect();
                    let v = slot(&var, &mut slots);
                    bound.insert(var);
                    Step::Agg {
                        var: v,
                        func,
                        rel,
                        outer,
                        target: target.map(|t| t.parse().expect("a column")),
                    }
                }
                Low::Path(p) => {
                    let avoid = match &p.avoid {
                        LowAvoid::None => AvoidStep::None,
                        LowAvoid::Node(t) => AvoidStep::Node(arg(t, &mut slots)),
                        LowAvoid::Rel(rel, ts) => {
                            AvoidStep::Rel(*rel, ts.iter().map(|t| arg(t, &mut slots)).collect())
                        }
                    };
                    let s = PathStep {
                        over: p.over,
                        from: arg(&p.from, &mut slots),
                        to: arg(&p.to, &mut slots),
                        avoid,
                        max: p.max,
                        witness: p.witness.as_deref().map(|w| slot(w, &mut slots)),
                        neg: p.neg,
                    };
                    bind(&p.from, &mut bound);
                    bind(&p.to, &mut bound);
                    if let Some(w) = &p.witness {
                        bound.insert(w.clone());
                    }
                    Step::Path(Box::new(s))
                }
            };
            steps.push(step);
        }
        let mut args = Vec::new();
        for t in head {
            match t {
                Term::Var(v) if bound.contains(v) => args.push(Arg::Var(slots[v])),
                Term::Var(v) => {
                    return Err(format!("{what}: the head's {v} is not bound by the body"));
                }
                Term::Any => return Err(format!("{what}: a head holds no _")),
                Term::Attr(..) | Term::Arith(..) => {
                    return Err(format!("{what}: a head holds variables and values only"));
                }
                Term::Str(s) => args.push(Arg::Val(V::Str(Rc::from(s.as_str())))),
                Term::Int(n) => args.push(Arg::Val(V::Int(*n))),
                Term::Bool(b) => args.push(Arg::Val(V::Bool(*b))),
            }
        }
        Ok((args, steps, slots.len()))
    }
}

/// The variables a query prints: those of its literals outside aggregate bodies and negations,
/// in the order they first occur.
fn query_vars(body: &[Lit]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in body {
        let mut vs = Vec::new();
        match l {
            Lit::Atom(_) | Lit::Cmp(..) | Lit::Match(..) | Lit::In(..) => lit_vars(l, &mut vs),
            Lit::Agg { var, .. } => vs.push(var.clone()),
            Lit::Or(alts) => {
                for alt in alts {
                    vs.extend(query_vars(alt));
                }
            }
            Lit::Path(p) if p.form == PathForm::Path => {
                term_vars(&p.from, &mut vs);
                term_vars(&p.to, &mut vs);
                vs.extend(p.witness.iter().cloned());
            }
            Lit::Not(_) | Lit::NotCond(_) | Lit::Path(_) => {}
        }
        for v in vs {
            if !v.starts_with('_') && !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Compile a program with the names lists `names` gives over the program's own.
pub fn compile(p: &Program, names: &Names) -> Result<Compiled, String> {
    let Some(query) = &p.query else {
        return Err("the module has no query: add one, ?- …".into());
    };
    // A names file serves several modules and codespec check: a list this module does not
    // declare is left alone, one neither it nor codespec check has is refused, as codespec check
    // refuses it.
    let mut lists: BTreeMap<String, Vec<String>> = p.names.iter().cloned().collect();
    for name in names.lists.keys() {
        if !lists.contains_key(name) && !CHECK_LISTS.iter().any(|(n, _)| n == name) {
            let mut known: BTreeSet<&str> = CHECK_LISTS.iter().map(|(n, _)| *n).collect();
            known.extend(lists.keys().map(String::as_str));
            return Err(format!(
                "names: no list {name}; the lists are {}",
                known.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    for (name, items) in &names.lists {
        if let Some(l) = lists.get_mut(name) {
            *l = items.clone();
        }
    }
    let mut c = Compiler {
        derived: Vec::new(),
        by_name: HashMap::new(),
        rules: Vec::new(),
        names: &lists,
        lists: HashMap::new(),
        fresh: 0,
        bodies: 0,
    };
    for r in &p.rules {
        if relation(&r.head.rel).is_some() || Builtin::of(&r.head.rel).is_some() {
            return Err(format!(
                "{}: {} is a relation of the fact base; a rule cannot add to it",
                r.head, r.head.rel
            ));
        }
        if r.head.rel.starts_with("__") {
            return Err(format!("{}: a relation name starts with a letter", r.head));
        }
        let Args::Positional(ts) = &r.head.args else {
            return Err(format!(
                "{}: a rule's head takes its fields in order",
                r.head
            ));
        };
        c.derived(&r.head.rel, ts.len())?;
    }
    let mut heads = Vec::new();
    for prop in &p.properties {
        if relation(&prop.name).is_some()
            || Builtin::of(&prop.name).is_some()
            || c.by_name.contains_key(&prop.name)
        {
            return Err(format!(
                "property {}: a relation of that name exists; a property is a relation of its own",
                prop.name
            ));
        }
        heads.push(c.derived(&prop.name, 3)?);
    }
    for r in &p.rules {
        let d = c.by_name[&r.head.rel];
        c.rule(r, d)?;
    }
    let mut props = Vec::new();
    for (prop, head) in p.properties.iter().zip(heads) {
        props.push(c.property(prop, head)?);
    }
    let vars = query_vars(query);
    let q = c.derived("__query", vars.len())?;
    c.rule(
        &Rule {
            head: Atom {
                rel: "__query".into(),
                args: Args::Positional(vars.iter().cloned().map(Term::Var).collect()),
            },
            body: query.clone(),
        },
        q,
    )?;
    let strata = stratify(&c.derived, &c.rules, &props)?;
    let mut by_head: Vec<Vec<usize>> = vec![Vec::new(); c.derived.len()];
    for (k, r) in c.rules.iter().enumerate() {
        by_head[r.head].push(k);
    }
    let mut prop_of = vec![None; c.derived.len()];
    for (k, pr) in props.iter().enumerate() {
        prop_of[pr.head] = Some(k);
    }
    Ok(Compiled {
        by_name: c.by_name,
        by_head,
        derived: c.derived,
        rules: c.rules,
        query: q,
        vars,
        strata,
        props,
        prop_of,
    })
}

/// The relations a property's events read.
fn prop_deps(p: &CProp) -> impl Iterator<Item = usize> + '_ {
    p.trans.iter().filter_map(|t| match t.event {
        CEvent::Call { rel, .. } => Some(rel),
        CEvent::Exit => None,
    })
}

impl Compiler<'_> {
    /// A property: each call event a relation of its own, `(site, its variables…)`, from the
    /// record pattern of `call`; the states numbered from `start`.
    fn property(&mut self, p: &Property, head: usize) -> Result<CProp, String> {
        let mut states = vec!["start".to_string()];
        for t in &p.transitions {
            for s in [&t.from, &t.to] {
                if !states.contains(s) {
                    states.push(s.clone());
                }
            }
        }
        let state = |s: &str| states.iter().position(|x| x == s).expect("a state");
        let mut regs: Vec<String> = Vec::new();
        for t in &p.transitions {
            for (r, _) in &t.assign {
                if !regs.contains(r) {
                    regs.push(r.clone());
                }
            }
        }
        let reg = |r: &str| regs.iter().position(|x| x == r).expect("a register");
        let mut trans = Vec::new();
        for t in &p.transitions {
            let event = match &t.event {
                Event::Exit => CEvent::Exit,
                Event::Call(a) => {
                    let Args::Named(fields) = &a.args else {
                        unreachable!("the parser takes call{{…}} only")
                    };
                    let site = self.fresh();
                    let mut fields = fields.clone();
                    let mut body = Vec::new();
                    match fields.iter().position(|(n, _)| n == "site") {
                        Some(k) => {
                            let (_, f) = std::mem::replace(
                                &mut fields[k],
                                ("site".into(), Field::Term(Term::Var(site.clone()))),
                            );
                            let v = Term::Var(site.clone());
                            match f {
                                Field::Term(Term::Any) => {}
                                Field::Term(t) => body.push(Lit::Cmp(v, Op::Bind, t)),
                                Field::Regex(re) => body.push(Lit::Match(v, re)),
                                Field::In(l) => body.push(Lit::In(v, l)),
                            }
                        }
                        None => fields.push(("site".into(), Field::Term(Term::Var(site.clone())))),
                    }
                    let mut unread: Vec<(&'static str, String)> = Vec::new();
                    for (n, f) in &fields {
                        if let (Some(n), Field::Term(Term::Var(v))) =
                            (UNREAD.iter().find(|u| **u == n.as_str()), f)
                        {
                            unread.push((n, v.clone()));
                        }
                    }
                    let atom = Atom {
                        rel: "call".into(),
                        args: Args::Named(fields),
                    };
                    let mut vars = vec![site.clone()];
                    let mut vs = Vec::new();
                    atom_vars(&atom, &mut vs);
                    body.iter().for_each(|l| lit_vars(l, &mut vs));
                    for v in vs {
                        if !vars.contains(&v) {
                            vars.push(v);
                        }
                    }
                    body.insert(0, Lit::Atom(atom));
                    let aux = format!("__event{}", self.derived.len());
                    let d = self.derived(&aux, vars.len())?;
                    self.rule(
                        &Rule {
                            head: Atom {
                                rel: aux,
                                args: Args::Positional(
                                    vars.iter().cloned().map(Term::Var).collect(),
                                ),
                            },
                            body,
                        },
                        d,
                    )?;
                    let col = |v: &str| vars.iter().position(|x| x == v).expect("a column");
                    let unread = unread.iter().map(|(n, v)| (col(v), *n)).collect();
                    CEvent::Call {
                        rel: d,
                        checks: registers_read(t).iter().map(|r| (reg(r), col(r))).collect(),
                        assigns: t.assign.iter().map(|(r, v)| (reg(r), col(v))).collect(),
                        fields: unread,
                    }
                }
            };
            trans.push(CTrans {
                from: state(&t.from),
                to: state(&t.to),
                event,
            });
        }
        let mut message = Vec::new();
        let mut marked = vec![false; states.len()];
        let text = p.message.clone().unwrap_or_else(|| p.name.clone());
        let mut rest = text.as_str();
        let names = placeholders(&text);
        for name in &names {
            let at = rest.find(&format!("{{{name}}}")).expect("a placeholder");
            message.push(Piece::Text(rest[..at].to_string()));
            match states.iter().position(|s| s == name) {
                Some(s) => {
                    marked[s] = true;
                    message.push(Piece::State(s));
                }
                None => message.push(Piece::Reg(reg(name))),
            }
            rest = &rest[at + name.len() + 2..];
        }
        message.push(Piece::Text(rest.to_string()));
        Ok(CProp {
            head,
            scope: p.scope,
            error: state("error"),
            states,
            trans,
            regs: regs.len(),
            message,
            marked,
        })
    }
}

/// Each derived relation's stratum: 1 for one over the fact base only, one more than any it
/// negates or aggregates over. A negation or an aggregate inside a recursion is refused.
fn stratify(derived: &[Derived], rules: &[CRule], props: &[CProp]) -> Result<Vec<usize>, String> {
    let n = derived.len();
    let mut edges: Vec<BTreeSet<(usize, bool)>> = vec![BTreeSet::new(); n];
    for r in rules {
        for s in &r.steps {
            match s {
                Step::Scan(Rel::Derived(d), _) => {
                    edges[r.head].insert((*d, false));
                }
                Step::Neg(Rel::Derived(d), _)
                | Step::Agg {
                    rel: Rel::Derived(d),
                    ..
                } => {
                    edges[r.head].insert((*d, true));
                }
                // A path needs every node it avoids first, as a negation does.
                Step::Path(p) => {
                    if let AvoidStep::Rel(Rel::Derived(d), _) = &p.avoid {
                        edges[r.head].insert((*d, true));
                    }
                }
                _ => {}
            }
        }
    }
    // A property needs every call of its events first.
    for p in props {
        for d in prop_deps(p) {
            edges[p.head].insert((d, true));
        }
    }
    // Strongly connected components, dependencies first.
    let succ: Vec<Vec<usize>> = edges
        .iter()
        .map(|es| es.iter().map(|(w, _)| *w).collect())
        .collect();
    let comps = crate::graph::components(&succ);
    let mut comp_of = vec![0; n];
    for (k, comp) in comps.iter().enumerate() {
        for &v in comp {
            comp_of[v] = k;
        }
    }
    let mut strata = vec![1; n];
    for (k, comp) in comps.iter().enumerate() {
        let mut s = 1;
        for &v in comp {
            for &(w, neg) in &edges[v] {
                if comp_of[w] == k {
                    if neg {
                        let shown = |x: usize| derived[x].name.trim_start_matches('_').to_string();
                        return Err(format!(
                            "{} depends on {} through a negation or an aggregate inside a recursion: the rules are not stratifiable",
                            shown(v),
                            shown(w)
                        ));
                    }
                    continue;
                }
                s = s.max(strata[w] + usize::from(neg));
            }
        }
        for &v in comp {
            strata[v] = s;
        }
    }
    Ok(strata)
}

/// The user's rules under the stratum of their heads, then the query.
pub fn explain(p: &Program) -> Result<String, String> {
    let c = compile(p, &Names::default())?;
    let mut by: BTreeMap<usize, Vec<&Rule>> = BTreeMap::new();
    for r in &p.rules {
        by.entry(c.strata[c.by_name(&r.head.rel)])
            .or_default()
            .push(r);
    }
    let mut props: BTreeMap<usize, Vec<&Property>> = BTreeMap::new();
    for pr in &p.properties {
        props
            .entry(c.strata[c.by_name(&pr.name)])
            .or_default()
            .push(pr);
    }
    let mut out = String::new();
    let strata: BTreeSet<usize> = by.keys().chain(props.keys()).copied().collect();
    for s in strata {
        out += &format!("stratum {s}\n");
        for pr in props.get(&s).into_iter().flatten() {
            out += &format!("  {pr}\n");
        }
        for r in by.get(&s).into_iter().flatten() {
            out += &format!("  {r}\n");
        }
    }
    out += "query\n";
    out += &format!(
        "  {}\n",
        query_text(p.query.as_deref().expect("compile requires a query"))
    );
    Ok(out)
}

impl Compiled {
    fn by_name(&self, name: &str) -> usize {
        *self.by_name.get(name).expect("a compiled relation")
    }
}

// ---------------------------------------------------------------------------------------------
// Evaluation

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// What the documents hold: the answer.
    Lower,
    /// What they could hold, with the wild facts.
    Upper,
}

struct Eval<'a> {
    fb: &'a FactBase,
    c: &'a Compiled,
    base: RefCell<HashMap<(&'static str, bool), Rc<Store>>>,
    lower: Vec<Store>,
    upper: Vec<Store>,
    /// The relations evaluated in the upper mode too.
    in_upper: Vec<bool>,
    closure: RefCell<Option<Closure>>,
    graphs: RefCell<Option<Rc<Graphs>>>,
    /// Sites the evaluation names beyond the fact base's, numbered after them: the values of a
    /// listed call nothing read that a register met.
    extra: RefCell<Vec<crate::Site>>,
}

/// The graphs path patterns and properties walk, built from the fact base once.
#[derive(Default)]
struct Graphs {
    /// `cfg_next`: each node's successors, sorted.
    cfg: HashMap<Rc<str>, Vec<Rc<str>>>,
    /// Every node of the control flow: those of `cfg_next`, `entry` and `exit`.
    cfg_nodes: BTreeSet<Rc<str>>,
    /// The linked calls: each function's callees, sorted.
    calls: BTreeMap<Rc<str>, Vec<Rc<str>>>,
    funcs: BTreeSet<Rc<str>>,
    /// The functions a linked call from outside their own strongly connected component reaches.
    called: HashSet<Rc<str>>,
    /// Each function's own unknown sites: its wild calls and the calls nothing links.
    own: HashMap<Rc<str>, Ann>,
    /// The statement each expression sits in.
    stmt_of: HashMap<Rc<str>, Rc<str>>,
    /// The local each expression reads.
    value: HashMap<Rc<str>, Rc<str>>,
    /// Each expression's kind, operand `x`, name and operator.
    exprs: HashMap<Rc<str>, [Rc<str>; 4]>,
    /// (body, its first statement)
    entries: Vec<(Rc<str>, Rc<str>)>,
    exits: HashSet<Rc<str>>,
    /// The calls of each statement, in the order they run: a call's operands before it.
    events: HashMap<Rc<str>, Vec<Rc<str>>>,
    /// The calls each function makes, sorted.
    func_calls: HashMap<Rc<str>, Vec<Rc<str>>>,
    /// The functions each call links to.
    targets: HashMap<Rc<str>, Vec<Rc<str>>>,
    /// The statements that hide code (a foreign statement without a call list), with their site.
    hidden: HashMap<Rc<str>, u32>,
    /// The opaque functions, with their site.
    opaque: Vec<(Rc<str>, u32)>,
    /// The listed calls (of a foreign statement's call list): whether a receiver it has goes
    /// unread, and its position.
    listed: HashMap<Rc<str>, (bool, Rc<str>)>,
}

impl Graphs {
    /// The statement a node of a body is: itself, the statement an expression sits in, or the
    /// statement whose call list holds a listed call.
    fn stmt(&self, v: &str) -> Rc<str> {
        if let Some(n) = self.cfg_nodes.get(v) {
            return n.clone();
        }
        if let Some(s) = self.stmt_of.get(v) {
            return s.clone();
        }
        match v.find(".calls.") {
            Some(k) => Rc::from(&v[..k]),
            None => Rc::from(v),
        }
    }

    /// What a value of a register is: the local an expression reads, followed through `&`, `*`
    /// and fields (`<body>$mu.inner`), else the value itself.
    fn canon(&self, v: &V) -> V {
        let V::Str(s) = v else { return v.clone() };
        let mut fields: Vec<Rc<str>> = Vec::new();
        let mut cur = s.clone();
        loop {
            if let Some(var) = self.value.get(&cur) {
                let mut out = var.to_string();
                for f in fields.iter().rev() {
                    out.push('.');
                    out += f;
                }
                return V::Str(Rc::from(out));
            }
            match self.exprs.get(&cur) {
                Some([kind, x, name, _]) if &**kind == "field" && !x.is_empty() => {
                    fields.push(name.clone());
                    cur = x.clone();
                }
                Some([kind, x, _, op]) if &**kind == "unop" && matches!(&**op, "&" | "*") => {
                    cur = x.clone();
                }
                _ => return v.clone(),
            }
        }
    }

    fn succ(&self, over: Over, u: &str) -> &[Rc<str>] {
        match over {
            Over::Cfg => self.cfg.get(u).map_or(&[], Vec::as_slice),
            Over::Calls => self.calls.get(u).map_or(&[], Vec::as_slice),
        }
    }
}

/// The order two calls of one statement run in: a call's operands (a longer id under it)
/// before it, then by index path, numbers as numbers.
fn run_order(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut x, mut y) = (a.split('.'), b.split('.'));
    loop {
        match (x.next(), y.next()) {
            (Some(p), Some(q)) if p == q => {}
            (Some(p), Some(q)) => {
                return match (p.parse::<u64>(), q.parse::<u64>()) {
                    (Ok(m), Ok(n)) => m.cmp(&n),
                    _ => p.cmp(q),
                };
            }
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => return Ordering::Equal,
        }
    }
}

/// The least shortest paths from a root: each node reached, with its parent on its path.
struct Tree {
    root: Rc<str>,
    parent: HashMap<Rc<str>, Rc<str>>,
    /// In the order reached.
    reached: Vec<Rc<str>>,
    /// Whether the root is reached by a path of no edge.
    zero: bool,
}

impl Tree {
    /// The path from the root to `n`, both included.
    fn path(&self, n: &Rc<str>) -> Option<Vec<Rc<str>>> {
        let Some(mut p) = self.parent.get(n) else {
            return (self.zero && *n == self.root).then(|| vec![n.clone()]);
        };
        let mut out = vec![n.clone()];
        loop {
            out.push(p.clone());
            if *p == self.root {
                break;
            }
            p = &self.parent[p];
        }
        out.reverse();
        Some(out)
    }
}

/// Every node reached from `root` over `succ`, by paths of at most `max` edges that pass no
/// `blocked` node, and of one or more edges unless `zero`. Breadth first, each level's nodes
/// expanded in the order of their own paths and their successors in id order, so the parent
/// each node gets lies on the least of its shortest paths.
fn walk<'g>(
    succ: impl Fn(&str) -> &'g [Rc<str>],
    root: &Rc<str>,
    blocked: impl Fn(&str) -> bool,
    max: Option<usize>,
    zero: bool,
) -> Tree {
    let mut t = Tree {
        root: root.clone(),
        parent: HashMap::new(),
        reached: Vec::new(),
        zero: false,
    };
    if blocked(root) {
        return t;
    }
    let mut seen: HashSet<Rc<str>> = HashSet::new();
    let mut queue: std::collections::VecDeque<(Rc<str>, usize)> = Default::default();
    if zero {
        t.zero = true;
        seen.insert(root.clone());
        t.reached.push(root.clone());
    }
    queue.push_back((root.clone(), 0));
    while let Some((u, d)) = queue.pop_front() {
        if max.is_some_and(|m| d >= m) {
            continue;
        }
        for v in succ(&u) {
            if !blocked(v) && seen.insert(v.clone()) {
                t.parent.insert(v.clone(), u.clone());
                t.reached.push(v.clone());
                queue.push_back((v.clone(), d + 1));
            }
        }
    }
    t
}

/// One run of a property: its state, its registers and the node at which it last left each
/// state the message prints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Run {
    state: usize,
    regs: Vec<Option<V>>,
    /// For each register set from a value nothing read, the site naming it.
    unread: Vec<Option<u32>>,
    marks: Vec<Option<Rc<str>>>,
}

/// The fields of `call` a listed call does not read: its receiver (of a method, interface or
/// reference call) and its argument count. A register meeting one matches only possibly.
const UNREAD: &[&str] = &["recv", "args"];

/// Each function, the functions it reaches and the unknown sites it reaches ([`Eval::closure`]).
type Closure = Rc<Vec<(Rc<str>, Vec<Rc<str>>, Ann)>>;

/// The relations of the call graph whose upper bound [`Eval::graph_wild`] computes.
const GRAPH_WILD: &[&str] = &["effect", "syscall", "scc", "reached"];

fn eqv(a: &V, b: &V, mode: Mode) -> bool {
    a == b || (mode == Mode::Upper && (*a == V::Top || *b == V::Top))
}

fn arith(a: &V, op: char, b: &V) -> Option<V> {
    match (a, b) {
        (V::Top, _) | (_, V::Top) => Some(V::Top),
        (V::Int(x), V::Int(y)) => match op {
            '+' => x.checked_add(*y),
            '-' => x.checked_sub(*y),
            '*' => x.checked_mul(*y),
            _ => None,
        }
        .map(V::Int),
        _ => None,
    }
}

/// Whether a property's walk goes on with `key` resting on `a`: when it is new, or when it
/// was seen only resting on something and now holds definitely.
fn visit(seen: &mut HashMap<(Rc<str>, Run), Ann>, key: (Rc<str>, Run), a: &Ann) -> bool {
    match seen.get_mut(&key) {
        None => {
            seen.insert(key, a.clone());
            true
        }
        Some(old) if a.is_empty() && !old.is_empty() => {
            old.clear();
            true
        }
        Some(_) => false,
    }
}

/// Adds the run `x` resting on `a` to `runs`, a definite one replacing a possible one.
fn keep(runs: &mut Vec<(Run, Ann)>, x: Run, a: Ann) {
    match runs.iter_mut().find(|(r, _)| *r == x) {
        Some((_, b)) if a.is_empty() => b.clear(),
        Some(_) => {}
        None => runs.push((x, a)),
    }
}

fn value_of(a: &Arg, b: &[Option<V>]) -> Option<V> {
    match a {
        Arg::Var(x) => b[*x].clone(),
        Arg::Val(v) => Some(v.clone()),
        Arg::Any => None,
        Arg::Arith(x, op, y) => arith(&value_of(x, b)?, *op, &value_of(y, b)?),
    }
}

impl<'a> Eval<'a> {
    fn base_store(&self, name: &'static str, upper: bool) -> Rc<Store> {
        if let Some(s) = self.base.borrow().get(&(name, upper)) {
            return s.clone();
        }
        let mut st = Store::default();
        if name == "reaches_path" {
            let g = self.graphs();
            for f in g.calls.keys() {
                let t = walk(|u| g.succ(Over::Calls, u), f, |_| false, None, false);
                for n in &t.reached {
                    let w = t.path(n).expect("a node reached has a path");
                    st.insert(
                        vec![
                            V::Str(f.clone()),
                            V::Str(n.clone()),
                            V::List(w.into_iter().map(V::Str).collect()),
                        ],
                        Vec::new(),
                    );
                }
            }
            if upper {
                for (f, _, sites) in self.closure().iter() {
                    if !sites.is_empty() {
                        st.insert(vec![V::Str(f.clone()), V::Top, V::Top], sites.clone());
                    }
                }
            }
        } else if name == "reaches" {
            for (f, reached, sites) in self.closure().iter() {
                let f = V::Str(f.clone());
                for g in reached {
                    st.insert(vec![f.clone(), V::Str(g.clone())], Vec::new());
                }
                if upper && !sites.is_empty() {
                    st.insert(vec![f, V::Top], sites.clone());
                }
            }
        } else {
            for row in self.fb.rels.get(name).into_iter().flatten() {
                st.insert(row.iter().map(V::of).collect(), Vec::new());
            }
            if upper {
                for (row, site) in self.fb.wild.get(name).into_iter().flatten() {
                    let t = row
                        .iter()
                        .map(|v| v.as_ref().map_or(V::Top, V::of))
                        .collect();
                    st.insert(t, vec![*site as u32]);
                }
                self.graph_wild(name, &mut st);
            }
        }
        let st = Rc::new(st);
        self.base.borrow_mut().insert((name, upper), st.clone());
        st
    }

    /// The facts an unknown site could add to a relation of the call graph: a function from
    /// which some unknown site is reached may have any effect and make any syscall, and may sit in
    /// a component with any function, resting on those sites; any function may be reached when
    /// a reached function holds code nothing read (an opaque function, a foreign statement
    /// without a call list).
    fn graph_wild(&self, name: &str, st: &mut Store) {
        if !GRAPH_WILD.contains(&name) {
            return;
        }
        if name == "reached" {
            let reached: HashSet<&str> = self
                .fb
                .rels
                .get("reached")
                .into_iter()
                .flatten()
                .filter_map(|r| match &r[0] {
                    Value::Str(f) => Some(f.as_str()),
                    _ => None,
                })
                .collect();
            let mut sites = Vec::new();
            for row in self.fb.rels.get("unknown").into_iter().flatten() {
                if let (Value::Str(f), Some(k)) = (&row[0], self.site_of(row))
                    && reached.contains(f.as_str())
                    && crate::facts::hides_code(&self.fb.sites[k as usize])
                {
                    sites.push(k);
                }
            }
            sites.sort_unstable();
            sites.dedup();
            if !sites.is_empty() {
                st.insert(vec![V::Top], sites);
            }
            return;
        }
        let arity = relation(name).expect("a relation").fields.len();
        for (f, _, sites) in self.closure().iter() {
            if !sites.is_empty() {
                let mut t = vec![V::Top; arity];
                t[0] = V::Str(f.clone());
                st.insert(t, sites.clone());
            }
        }
    }

    /// The site of an `unknown` row.
    fn site_of(&self, row: &[Value]) -> Option<u32> {
        let (Value::Str(site), Value::Str(pos), Value::Str(reason)) = (&row[1], &row[2], &row[3])
        else {
            return None;
        };
        self.fb
            .sites
            .iter()
            .position(|s| s.site == *site && s.pos == *pos && s.reason == *reason)
            .map(|k| k as u32)
    }

    /// Each function the linked calls leave from or an unknown site sits in: the functions the
    /// calls reach from it, and the unknown sites reached (its own, its wild calls and the wild
    /// links of its sites, and those of every function it reaches). Computed once.
    fn closure(&self) -> Closure {
        if let Some(c) = self.closure.borrow().as_ref() {
            return c.clone();
        }
        let g = self.graphs();
        let mut froms: BTreeSet<&Rc<str>> = g.calls.keys().collect();
        froms.extend(g.own.keys());
        let mut out = Vec::new();
        for from in froms {
            let mut seen: BTreeSet<&Rc<str>> = BTreeSet::new();
            let mut stack: Vec<&Rc<str>> = g.calls.get(from).into_iter().flatten().collect();
            while let Some(f) = stack.pop() {
                if seen.insert(f) {
                    stack.extend(g.calls.get(f).into_iter().flatten());
                }
            }
            let mut sites: Ann = g.own.get(from).cloned().unwrap_or_default();
            for f in &seen {
                if let Some(s) = g.own.get(*f) {
                    union(&mut sites, s);
                }
            }
            out.push((from.clone(), seen.into_iter().cloned().collect(), sites));
        }
        let c = Rc::new(out);
        *self.closure.borrow_mut() = Some(c.clone());
        c
    }

    /// The graphs of path patterns and properties, built once.
    fn graphs(&self) -> Rc<Graphs> {
        if let Some(g) = self.graphs.borrow().as_ref() {
            return g.clone();
        }
        let fb = self.fb;
        let rows = |name: &str| fb.rels.get(name).into_iter().flatten();
        let rc = |v: &Value| -> Rc<str> {
            match v {
                Value::Str(s) => Rc::from(s.as_str()),
                _ => Rc::from(""),
            }
        };
        let mut g = Graphs::default();
        for r in rows("cfg_next") {
            let (a, b) = (rc(&r[0]), rc(&r[1]));
            g.cfg_nodes.insert(a.clone());
            g.cfg_nodes.insert(b.clone());
            g.cfg.entry(a).or_default().push(b);
        }
        for r in rows("entry") {
            let (f, n) = (rc(&r[0]), rc(&r[1]));
            g.cfg_nodes.insert(n.clone());
            g.entries.push((f, n));
        }
        for r in rows("exit") {
            let n = rc(&r[1]);
            g.cfg_nodes.insert(n.clone());
            g.exits.insert(n);
        }
        for r in rows("expr") {
            let id = rc(&r[0]);
            g.stmt_of.insert(id.clone(), rc(&r[4]));
            g.exprs
                .insert(id, [rc(&r[2]), rc(&r[11]), rc(&r[9]), rc(&r[8])]);
        }
        for r in rows("value") {
            g.value.insert(rc(&r[0]), rc(&r[1]));
        }
        for r in rows("func") {
            g.funcs.insert(rc(&r[0]));
        }
        let mut func_of: HashMap<Rc<str>, Rc<str>> = HashMap::new();
        for r in rows("call") {
            let (site, f) = (rc(&r[0]), rc(&r[1]));
            func_of.insert(site.clone(), f.clone());
            if r[11] == Value::Bool(true) {
                let recv = matches!(&*rc(&r[2]), "method" | "iface" | "ref");
                g.listed.insert(site.clone(), (recv, rc(&r[9])));
            }
            g.func_calls.entry(f).or_default().push(site.clone());
            let in_body = g.stmt_of.contains_key(&site) || site.contains(".calls.");
            if in_body {
                let st = g.stmt(&site);
                g.events.entry(st).or_default().push(site);
            }
        }
        let comp: HashMap<Rc<str>, Rc<str>> = rows("scc").map(|r| (rc(&r[0]), rc(&r[1]))).collect();
        for r in rows("calls") {
            let (site, t) = (rc(&r[0]), rc(&r[1]));
            g.targets.entry(site.clone()).or_default().push(t.clone());
            let caller = func_of.get(&site);
            if let Some(f) = caller {
                g.calls.entry(f.clone()).or_default().push(t.clone());
            }
            // A call within the callee's own strongly connected component (recursion) does not
            // make it a callee: it is still an entry if nothing outside calls it.
            let within =
                caller.is_some_and(|f| comp.get(f).is_some_and(|c| comp.get(&t) == Some(c)));
            if !within {
                g.called.insert(t);
            }
        }
        for (row, site) in fb.wild.get("call").into_iter().flatten() {
            if let Some(f) = &row[1] {
                union(g.own.entry(rc(f)).or_default(), &[*site as u32]);
            }
        }
        for (row, site) in fb.wild.get("calls").into_iter().flatten() {
            if let Some(Value::Str(s)) = &row[0] {
                let f = func_of.get(s.as_str()).cloned().or_else(|| {
                    fb.wild
                        .get("call")
                        .into_iter()
                        .flatten()
                        .find_map(|(w, _)| match (&w[0], &w[1]) {
                            (Some(Value::Str(ws)), Some(f)) if ws == s => Some(rc(f)),
                            _ => None,
                        })
                });
                if let Some(f) = f {
                    union(g.own.entry(f).or_default(), &[*site as u32]);
                }
            }
        }
        for row in rows("unknown") {
            let Some(k) = self.site_of(row) else { continue };
            let site = &fb.sites[k as usize];
            if crate::facts::is_opaque(site) {
                g.opaque.push((rc(&row[0]), k));
            } else if crate::facts::hides_code(site) {
                g.hidden.insert(rc(&row[1]), k);
            }
        }
        for v in g
            .cfg
            .values_mut()
            .chain(g.calls.values_mut())
            .chain(g.targets.values_mut())
        {
            v.sort();
            v.dedup();
        }
        for v in g.func_calls.values_mut() {
            v.sort();
            v.dedup();
        }
        for v in g.events.values_mut() {
            v.sort_by(|a, b| run_order(a, b));
            v.dedup();
        }
        let g = Rc::new(g);
        *self.graphs.borrow_mut() = Some(g.clone());
        g
    }

    /// The ways a path pattern goes on from `b`: each end it binds and its witness, resting on
    /// the unknown sites behind a node it might not pass.
    ///
    /// A path is walked three times at most, avoiding the nodes that are definitely avoided
    /// (`def`), those too that possibly are (`maybe`), and those an unknown site could add
    /// (`up`; a wild node avoids every node). A path passing no `maybe` node is definite but for
    /// the `up` nodes on it; one passing some rests on their sites. A negated path holds
    /// definitely when no path passes the `def` nodes alone, else possibly when the others
    /// would block every path. A node the walk reaches that holds an unknown site (a function
    /// with a wild call or a call nothing links, a foreign statement without a call list) could
    /// lead anywhere: a negated path rests on it, and the upper bound gives the wild end, or
    /// the bound end when no path reached it.
    fn path_ways(&self, p: &PathStep, b: &[Option<V>], ann: &Ann, mode: Mode) -> Ways<'_> {
        let g = self.graphs();
        let from = value_of(&p.from, b);
        let to = value_of(&p.to, b);
        let tops = from.as_ref().is_some_and(V::has_top) || to.as_ref().is_some_and(V::has_top);
        if tops {
            // The upper mode only: any path may join a node of code nothing read.
            let mut binds = Vec::new();
            for (a, v) in [(&p.from, &from), (&p.to, &to)] {
                if let (Arg::Var(x), None) = (a, v)
                    && !binds.iter().any(|(y, _)| y == x)
                {
                    binds.push((*x, V::Top));
                }
            }
            if let Some(w) = p.witness.filter(|w| b[*w].is_none()) {
                binds.push((w, V::Top));
            }
            return Ways::Binds(vec![(binds, ann.clone())]);
        }
        let node = |s: &str| -> Rc<str> {
            match p.over {
                Over::Cfg => g.stmt(s),
                Over::Calls => Rc::from(s),
            }
        };
        let mut def: HashSet<Rc<str>> = HashSet::new();
        let mut maybe: HashMap<Rc<str>, Ann> = HashMap::new();
        let mut up: HashMap<Rc<str>, Ann> = HashMap::new();
        let mut up_all: Ann = Vec::new();
        match &p.avoid {
            AvoidStep::None => {}
            AvoidStep::Node(a) => {
                if let Some(V::Str(s)) = value_of(a, b) {
                    def.insert(node(&s));
                }
            }
            AvoidStep::Rel(rel, args) => {
                let mut full = vec![Arg::Any];
                full.extend(args.iter().cloned());
                let low = self.store(*rel, Mode::Lower);
                for t in self.matches(&low, &full, b, None, Mode::Lower) {
                    if let V::Str(s) = &low.tuples[t][0] {
                        if low.anns[t].is_empty() {
                            def.insert(node(s));
                        } else {
                            union(maybe.entry(node(s)).or_default(), &low.anns[t]);
                        }
                    }
                }
                maybe.retain(|n, _| !def.contains(n));
                if self.has_upper(*rel) {
                    let upper = self.store(*rel, Mode::Upper);
                    for t in self.matches(&upper, &full, b, None, Mode::Upper) {
                        match &upper.tuples[t][0] {
                            V::Str(s) => union(up.entry(node(s)).or_default(), &upper.anns[t]),
                            _ => union(&mut up_all, &upper.anns[t]),
                        }
                    }
                }
                up.retain(|n, a| !def.contains(n) && !a.is_empty());
            }
        }
        let same_end = matches!((&p.from, &p.to), (Arg::Var(x), Arg::Var(y)) if x == y);
        let sources: Vec<(Rc<str>, bool)> = match &from {
            Some(V::Str(s)) => vec![(node(s), false)],
            Some(_) => Vec::new(),
            None => match p.over {
                Over::Cfg => g.cfg_nodes.iter().map(|n| (n.clone(), true)).collect(),
                Over::Calls => g.funcs.iter().map(|n| (n.clone(), true)).collect(),
            },
        };
        let succ = |u: &str| g.succ(p.over, u);
        let mut ways = Vec::new();
        for (src, bind_from) in sources {
            let tdef = walk(succ, &src, |n| def.contains(n), p.max, true);
            let tall = (!maybe.is_empty()).then(|| {
                walk(
                    succ,
                    &src,
                    |n| def.contains(n) || maybe.contains_key(n),
                    p.max,
                    true,
                )
            });
            let mut wild: Ann = Vec::new();
            for u in &tdef.reached {
                match p.over {
                    Over::Calls => {
                        if let Some(s) = g.own.get(u) {
                            union(&mut wild, s);
                        }
                    }
                    Over::Cfg => {
                        if let Some(&s) = g.hidden.get(u) {
                            union(&mut wild, &[s]);
                        }
                    }
                }
            }
            let to_node = match (&to, same_end && bind_from) {
                (_, true) => Some(src.clone()),
                (Some(V::Str(t)), _) => Some(node(t)),
                (Some(_), _) => continue,
                (None, _) => None,
            };
            if p.neg {
                let Some(t) = to_node else { continue };
                let all_maybe = || {
                    let mut a = Vec::new();
                    maybe.values().for_each(|x| union(&mut a, x));
                    a
                };
                let rests = if tdef.path(&t).is_none() {
                    Some(wild.clone())
                } else if tall.as_ref().is_some_and(|x| x.path(&t).is_none()) {
                    let mut a = all_maybe();
                    union(&mut a, &wild);
                    Some(a)
                } else if !up_all.is_empty()
                    || (!up.is_empty()
                        && walk(
                            succ,
                            &src,
                            |n| def.contains(n) || maybe.contains_key(n) || up.contains_key(n),
                            p.max,
                            true,
                        )
                        .path(&t)
                        .is_none())
                {
                    let mut a = all_maybe();
                    union(&mut a, &wild);
                    union(&mut a, &up_all);
                    up.values().for_each(|x| union(&mut a, x));
                    Some(a)
                } else {
                    None
                };
                if let Some(a) = rests {
                    let mut a2 = ann.clone();
                    union(&mut a2, &a);
                    ways.push((Vec::new(), a2));
                }
                continue;
            }
            let targets: Vec<Rc<str>> = match &to_node {
                Some(t) => vec![t.clone()],
                None => tdef.reached.clone(),
            };
            let mut ends = Vec::new();
            for t in targets {
                let (w, mut a) = match tall.as_ref().and_then(|x| x.path(&t)) {
                    Some(w) => (w, Vec::new()),
                    None => match tdef.path(&t) {
                        Some(w) => {
                            let mut a = Vec::new();
                            for n in &w {
                                if let Some(x) = maybe.get(n) {
                                    union(&mut a, x);
                                }
                            }
                            (w, a)
                        }
                        None => continue,
                    },
                };
                for n in &w {
                    if let Some(x) = up.get(n) {
                        union(&mut a, x);
                    }
                }
                union(&mut a, &up_all);
                ends.push((
                    Some(V::Str(t)),
                    Some(V::List(w.into_iter().map(V::Str).collect())),
                    a,
                ));
            }
            if mode == Mode::Upper && !wild.is_empty() {
                match &to_node {
                    None => ends.push((Some(V::Top), Some(V::Top), wild.clone())),
                    // A bound end no path reaches may still be reached through the unknown.
                    Some(t) if ends.is_empty() => {
                        ends.push((Some(V::Str(t.clone())), Some(V::Top), wild.clone()))
                    }
                    Some(_) => {}
                }
            }
            for (end, w, a) in ends {
                let mut binds = Vec::new();
                if let (Arg::Var(x), true) = (&p.from, bind_from) {
                    binds.push((*x, V::Str(src.clone())));
                }
                if let (Arg::Var(x), None, false) = (&p.to, &to, same_end) {
                    binds.push((*x, end.expect("an end")));
                }
                if let (Some(wi), Some(w)) = (p.witness, w) {
                    match &b[wi] {
                        Some(v) if *v != w && w != V::Top => continue,
                        Some(_) => {}
                        None => binds.push((wi, w)),
                    }
                }
                let mut a2 = ann.clone();
                union(&mut a2, &a);
                ways.push((binds, a2));
            }
        }
        Ways::Binds(ways)
    }

    /// The site naming the field `field` of the listed call `site`, which nothing read.
    fn unread_site(&self, fb_sites: usize, site: &str, pos: &str, field: &str) -> u32 {
        let x = crate::Site {
            site: site.to_string(),
            pos: pos.to_string(),
            reason: format!("the {field} of a listed call, not read"),
        };
        let mut extra = self.extra.borrow_mut();
        let k = match extra.iter().position(|s| *s == x) {
            Some(k) => k,
            None => {
                extra.push(x);
                extra.len() - 1
            }
        };
        (fb_sites + k) as u32
    }

    /// The runs `run` goes on to at the call `site`, each with what it rests on: every
    /// transition out of its state whose event matches, its registers equal and set; else it
    /// stays, as it always may in `start`. A register compared with a value nothing read (the
    /// receiver of a listed call, or a register set from one) matches only possibly: the run
    /// both moves and stays, each resting on the site naming that value.
    fn fire(&self, g: &Graphs, p: &CProp, run: &Run, site: &Rc<str>) -> Vec<(Run, Ann)> {
        let mut out: Vec<(Run, Ann)> = Vec::new();
        let mut moved = false;
        let mut stays: Ann = Vec::new();
        let key = [V::Str(site.clone())];
        let listed = g.listed.get(site);
        for t in p.trans.iter().filter(|t| t.from == run.state) {
            let CEvent::Call {
                rel,
                checks,
                assigns,
                fields,
            } = &t.event
            else {
                continue;
            };
            let unread = |c: usize| -> Option<u32> {
                let (recv, pos) = listed?;
                let &(_, field) = fields
                    .iter()
                    .find(|(x, f)| *x == c && (*f != "recv" || *recv))?;
                Some(self.unread_site(self.fb.sites.len(), site, pos, field))
            };
            let st = &self.lower[*rel];
            for k in st.candidates(&[0], &key) {
                let tuple = &st.tuples[k];
                if tuple[0] != key[0] {
                    continue;
                }
                let mut a: Ann = Vec::new();
                let mut holds = true;
                for &(r, c) in checks {
                    if let Some(s) = unread(c).or(run.unread[r]) {
                        union(&mut a, &[s]);
                    } else if run.regs[r].as_ref() != Some(&g.canon(&tuple[c])) {
                        holds = false;
                        break;
                    }
                }
                if !holds {
                    continue;
                }
                let mut next = run.clone();
                for &(r, c) in assigns {
                    next.regs[r] = Some(g.canon(&tuple[c]));
                    next.unread[r] = unread(c);
                }
                if p.marked[run.state] {
                    next.marks[run.state] = Some(site.clone());
                }
                next.state = t.to;
                if a.is_empty() {
                    moved = true;
                } else {
                    union(&mut stays, &a);
                }
                match out.iter_mut().find(|(x, _)| *x == next) {
                    Some((_, b)) if a.is_empty() => b.clear(),
                    Some(_) => {}
                    None => out.push((next, a)),
                }
            }
        }
        if run.state == 0 {
            stays.clear();
        }
        if run.state == 0 || !moved {
            match out.iter_mut().find(|(x, _)| x == run) {
                Some((_, b)) if stays.is_empty() => b.clear(),
                Some(_) => {}
                None => out.push((run.clone(), stays)),
            }
        }
        out
    }

    /// The rows of a property: (where it starts, where it reached error, its message), each
    /// resting on the code nothing read that its run passed; in the upper mode also a wild row
    /// for each place such code could make it fail.
    fn property_rows(&self, p: &CProp, mode: Mode) -> Vec<(Vec<V>, Ann)> {
        let g = self.graphs();
        let init = Run {
            state: 0,
            regs: vec![None; p.regs],
            unread: vec![None; p.regs],
            marks: vec![None; p.states.len()],
        };
        let mut out: Vec<(Vec<V>, Ann)> = Vec::new();
        let emit =
            |out: &mut Vec<(Vec<V>, Ann)>, start: &Rc<str>, at: &Rc<str>, run: &Run, a: &Ann| {
                let mut run = run.clone();
                run.marks[p.error] = Some(at.clone());
                let text: String = p
                    .message
                    .iter()
                    .map(|piece| match piece {
                        Piece::Text(t) => t.clone(),
                        Piece::State(s) => run.marks[*s].as_deref().unwrap_or("").to_string(),
                        Piece::Reg(r) => match &run.regs[*r] {
                            Some(V::Str(s)) => s.to_string(),
                            Some(V::Top) | None => String::new(),
                            Some(v) => v.value().to_string(),
                        },
                    })
                    .collect();
                out.push((
                    vec![
                        V::Str(start.clone()),
                        V::Str(at.clone()),
                        V::Str(Rc::from(text)),
                    ],
                    a.clone(),
                ));
            };
        let wild_row = |f: &Rc<str>| vec![V::Str(f.clone()), V::Top, V::Top];
        match p.scope {
            Scope::Body => {
                for (owner, entry) in &g.entries {
                    // First without the statements that hide code: what holds definitely; then
                    // through them, each run resting on those it passed.
                    let mut blocked = false;
                    for through in [false, true] {
                        if through && !blocked {
                            break;
                        }
                        let mut seen: HashMap<(Rc<str>, Run), Ann> = HashMap::new();
                        let mut queue = std::collections::VecDeque::new();
                        visit(&mut seen, (entry.clone(), init.clone()), &Ann::new());
                        queue.push_back((entry.clone(), init.clone(), Ann::new()));
                        let mut passed = Ann::new();
                        while let Some((n, run, mut a)) = queue.pop_front() {
                            if let Some(&site) = g.hidden.get(&n) {
                                if !through {
                                    blocked = true;
                                    continue;
                                }
                                union(&mut a, &[site]);
                                union(&mut passed, &[site]);
                            }
                            let mut runs = vec![(run, a)];
                            for site in g.events.get(&n).into_iter().flatten() {
                                let mut next: Vec<(Run, Ann)> = Vec::new();
                                for (r, ra) in &runs {
                                    for (x, mut xa) in self.fire(&g, p, r, site) {
                                        union(&mut xa, ra);
                                        if x.state == p.error {
                                            emit(&mut out, owner, site, &x, &xa);
                                        } else {
                                            keep(&mut next, x, xa);
                                        }
                                    }
                                }
                                runs = next;
                            }
                            if g.exits.contains(&n) {
                                for (r, ra) in &runs {
                                    let fails = p.trans.iter().any(|t| {
                                        t.from == r.state
                                            && t.to == p.error
                                            && matches!(t.event, CEvent::Exit)
                                    });
                                    if fails {
                                        let mut x = r.clone();
                                        if p.marked[r.state] {
                                            x.marks[r.state] = Some(n.clone());
                                        }
                                        emit(&mut out, owner, &n, &x, ra);
                                    }
                                }
                            }
                            for s in g.cfg.get(&n).into_iter().flatten() {
                                for (r, ra) in &runs {
                                    if visit(&mut seen, (s.clone(), r.clone()), ra) {
                                        queue.push_back((s.clone(), r.clone(), ra.clone()));
                                    }
                                }
                            }
                        }
                        if mode == Mode::Upper {
                            for &site in &passed {
                                out.push((wild_row(owner), vec![site]));
                            }
                        }
                    }
                }
                if mode == Mode::Upper {
                    for (f, site) in &g.opaque {
                        out.push((wild_row(f), vec![*site]));
                    }
                }
            }
            Scope::Calls => {
                let closure = if mode == Mode::Upper {
                    Some(self.closure())
                } else {
                    None
                };
                for e in g.funcs.iter().filter(|f| !g.called.contains(*f)) {
                    let mut seen: HashMap<(Rc<str>, Run), Ann> = HashMap::new();
                    let mut stack = vec![(e.clone(), init.clone(), Ann::new())];
                    visit(&mut seen, (e.clone(), init.clone()), &Ann::new());
                    while let Some((f, run, a)) = stack.pop() {
                        for site in g.func_calls.get(&f).into_iter().flatten() {
                            for (x, mut xa) in self.fire(&g, p, &run, site) {
                                union(&mut xa, &a);
                                if x.state == p.error {
                                    emit(&mut out, e, site, &x, &xa);
                                    continue;
                                }
                                for t in g.targets.get(site).into_iter().flatten() {
                                    if visit(&mut seen, (t.clone(), x.clone()), &xa) {
                                        stack.push((t.clone(), x.clone(), xa.clone()));
                                    }
                                }
                            }
                        }
                    }
                    if let Some(c) = &closure
                        && let Some((_, _, sites)) = c.iter().find(|(f, _, _)| f == e)
                        && !sites.is_empty()
                    {
                        out.push((wild_row(e), sites.clone()));
                    }
                }
            }
        }
        out
    }

    fn store(&self, rel: Rel, mode: Mode) -> StoreRef<'_> {
        match rel {
            Rel::Base(name) => StoreRef::Shared(self.base_store(name, mode == Mode::Upper)),
            Rel::Derived(d) => StoreRef::Own(match mode {
                Mode::Lower => &self.lower[d],
                Mode::Upper => &self.upper[d],
            }),
        }
    }

    /// Whether `rel` has an upper bound to consult, beyond its lower one.
    fn has_upper(&self, rel: Rel) -> bool {
        match rel {
            Rel::Base(name) => {
                self.fb.wild.contains_key(name)
                    || ON_DEMAND.contains(&name)
                    || GRAPH_WILD.contains(&name)
            }
            Rel::Derived(d) => self.in_upper[d],
        }
    }

    /// The tuples of `store` that agree with the bound arguments.
    fn matches(
        &self,
        store: &Store,
        args: &[Arg],
        b: &[Option<V>],
        only: Option<&[usize]>,
        mode: Mode,
    ) -> Vec<usize> {
        let mut cols = Vec::new();
        let mut key = Vec::new();
        for (c, a) in args.iter().enumerate() {
            let v = match a {
                Arg::Var(x) => b[*x].clone(),
                Arg::Val(v) => Some(v.clone()),
                Arg::Arith(..) => value_of(a, b),
                Arg::Any => None,
            };
            if let Some(v) = v {
                cols.push(c);
                key.push(v);
            }
        }
        let cands = match only {
            Some(d) => d.to_vec(),
            None => store.candidates(&cols, &key),
        };
        cands
            .into_iter()
            .filter(|&k| {
                let t = &store.tuples[k];
                cols.iter().zip(&key).all(|(&c, v)| eqv(v, &t[c], mode))
            })
            .collect()
    }

    /// Runs the steps of `r`, each solution's head tuple and what it rests on to `out`. The
    /// search keeps one frame per step on a stack of its own, so a body of any length runs
    /// without recursion; `delta` restricts the scan at its step to the given tuples.
    fn steps(
        &self,
        r: &CRule,
        delta: Option<(usize, &[usize])>,
        mode: Mode,
        out: &mut Vec<(Vec<V>, Ann)>,
    ) {
        let mut b: Vec<Option<V>> = vec![None; r.vars];
        let head =
            |b: &[Option<V>]| -> Option<Vec<V>> { r.args.iter().map(|a| value_of(a, b)).collect() };
        if r.steps.is_empty() {
            if let Some(t) = head(&b) {
                out.push((t, Vec::new()));
            }
            return;
        }
        let first = self.ways(r, 0, &mut b, &Vec::new(), delta, mode);
        let mut stack = vec![Frame {
            k: 0,
            ann: Vec::new(),
            ways: first,
            next: 0,
            set: Vec::new(),
        }];
        while let Some(f) = stack.last_mut() {
            for x in f.set.drain(..) {
                b[x] = None;
            }
            let Some(ann) = f.take(&mut b, mode) else {
                stack.pop();
                continue;
            };
            let k = f.k + 1;
            if k == r.steps.len() {
                if let Some(t) = head(&b) {
                    out.push((t, ann));
                }
                continue;
            }
            let ways = self.ways(r, k, &mut b, &ann, delta, mode);
            stack.push(Frame {
                k,
                ann,
                ways,
                next: 0,
                set: Vec::new(),
            });
        }
    }

    /// The ways step `k` of `r` goes on from the bindings `b`, the solution so far resting on
    /// `ann`. `b` is as it was when this returns.
    fn ways<'s>(
        &'s self,
        r: &'s CRule,
        k: usize,
        b: &mut [Option<V>],
        ann: &Ann,
        delta: Option<(usize, &[usize])>,
        mode: Mode,
    ) -> Ways<'s> {
        let one = |binds: Vec<(usize, V)>, a: Ann| Ways::Binds(vec![(binds, a)]);
        let none = || Ways::Binds(Vec::new());
        match &r.steps[k] {
            Step::Scan(rel, args) => {
                let st = self.store(*rel, mode);
                let only = delta.filter(|(j, _)| *j == k).map(|(_, d)| d);
                let ts = self.matches(&st, args, b, only, mode);
                Ways::Tuples { st, args, ts }
            }
            Step::Neg(rel, args) => {
                let low = self.store(*rel, Mode::Lower);
                let found = self.matches(&low, args, b, None, Mode::Lower);
                let mut a2 = ann.clone();
                match mode {
                    Mode::Lower => {
                        // A definite fact refutes the negation; one that only possibly holds
                        // leaves it possible, resting on the sites that fact rests on.
                        if found.iter().any(|&t| low.anns[t].is_empty()) {
                            return none();
                        }
                        for &t in &found {
                            union(&mut a2, &low.anns[t]);
                        }
                        if self.has_upper(*rel) {
                            let up = self.store(*rel, Mode::Upper);
                            for t in self.matches(&up, args, b, None, Mode::Upper) {
                                union(&mut a2, &up.anns[t]);
                            }
                        }
                    }
                    Mode::Upper => {
                        let wild = args
                            .iter()
                            .any(|a| value_of(a, b).is_some_and(|v| v.has_top()));
                        if !wild {
                            if found.iter().any(|&t| low.anns[t].is_empty()) {
                                return none();
                            }
                            for &t in &found {
                                union(&mut a2, &low.anns[t]);
                            }
                        }
                    }
                }
                one(Vec::new(), a2)
            }
            Step::Cmp(x, op, y) => {
                let (vx, vy) = (value_of(x, b), value_of(y, b));
                if *op == Op::Bind {
                    return match (x, y, vx, vy) {
                        (Arg::Var(s), _, None, Some(v)) | (_, Arg::Var(s), Some(v), None) => {
                            one(vec![(*s, v)], ann.clone())
                        }
                        (_, _, Some(p), Some(q)) if eqv(&p, &q, mode) => {
                            one(Vec::new(), ann.clone())
                        }
                        _ => none(),
                    };
                }
                let (Some(p), Some(q)) = (vx, vy) else {
                    return none();
                };
                let holds = if mode == Mode::Upper && (p.has_top() || q.has_top()) {
                    true
                } else {
                    match op {
                        Op::Eq => p == q,
                        Op::Ne => p != q,
                        _ => {
                            let ord = match (&p, &q) {
                                (V::Int(m), V::Int(n)) => Some(m.cmp(n)),
                                (V::Str(m), V::Str(n)) => Some(m.cmp(n)),
                                _ => None,
                            };
                            ord.is_some_and(|o| match op {
                                Op::Lt => o.is_lt(),
                                Op::Le => o.is_le(),
                                Op::Gt => o.is_gt(),
                                Op::Ge => o.is_ge(),
                                _ => unreachable!("handled above"),
                            })
                        }
                    }
                };
                if holds {
                    one(Vec::new(), ann.clone())
                } else {
                    none()
                }
            }
            Step::Match(x, re, neg) => {
                let holds = match value_of(x, b) {
                    Some(V::Str(s)) => re.is_match(&s) != *neg,
                    Some(V::Top) => true,
                    _ => *neg,
                };
                if holds {
                    one(Vec::new(), ann.clone())
                } else {
                    none()
                }
            }
            Step::In(x, list, neg) => {
                let v = value_of(x, b);
                if let (Arg::Var(s), None) = (x, &v) {
                    return Ways::Binds(
                        list.entries
                            .iter()
                            .map(|e| (vec![(*s, V::Str(Rc::from(e.as_str())))], ann.clone()))
                            .collect(),
                    );
                }
                let holds = match v {
                    Some(V::Str(s)) => list.holds(&s) != *neg,
                    Some(V::Top) => true,
                    _ => *neg,
                };
                if holds {
                    one(Vec::new(), ann.clone())
                } else {
                    none()
                }
            }
            Step::Builtin(which, args) => {
                let outputs = builtin(*which, args, b);
                let mut ways = Vec::new();
                for o in outputs {
                    let mut set = Vec::new();
                    let mut ok = true;
                    for (a, v) in args.iter().zip(&o) {
                        match a {
                            Arg::Var(x) if b[*x].is_none() => {
                                b[*x] = Some(v.clone());
                                set.push(*x);
                            }
                            Arg::Any => {}
                            a => {
                                if !value_of(a, b).is_some_and(|w| eqv(&w, v, mode)) {
                                    ok = false;
                                }
                            }
                        }
                    }
                    let binds: Vec<(usize, V)> = set
                        .into_iter()
                        .map(|x| (x, b[x].take().expect("set")))
                        .collect();
                    if ok {
                        ways.push((binds, ann.clone()));
                    }
                }
                Ways::Binds(ways)
            }
            Step::Agg {
                var,
                func,
                rel,
                outer,
                target,
            } => {
                let Rel::Derived(d) = rel else {
                    unreachable!("an aggregate runs over its own relation")
                };
                let arity = self.c.derived[*d].arity;
                let mut args = vec![Arg::Any; arity];
                for (c, x) in outer {
                    args[*c] = Arg::Var(*x);
                }
                let st = self.store(*rel, mode);
                let found = self.matches(&st, &args, b, None, mode);
                let mut a2 = ann.clone();
                let mut wild = false;
                for &t in &found {
                    union(&mut a2, &st.anns[t]);
                    wild |= !st.anns[t].is_empty() || st.tuples[t].iter().any(V::has_top);
                }
                if mode == Mode::Lower && self.in_upper[*d] {
                    let up = self.store(*rel, Mode::Upper);
                    for t in self.matches(&up, &args, b, None, Mode::Upper) {
                        union(&mut a2, &up.anns[t]);
                    }
                }
                let results: Vec<V> = if mode == Mode::Upper && wild {
                    vec![V::Top]
                } else {
                    let (sure, maybe): (Vec<usize>, Vec<usize>) =
                        found.iter().partition(|&&t| st.anns[t].is_empty());
                    let value = |t: usize| st.tuples[t][target.expect("a target")].clone();
                    aggregate(*func, &sure, &maybe, value)
                };
                let mut ways = Vec::new();
                for result in results {
                    match &b[*var] {
                        Some(v) => {
                            if eqv(v, &result, mode) {
                                ways.push((Vec::new(), a2.clone()));
                            }
                        }
                        None => ways.push((vec![(*var, result)], a2.clone())),
                    }
                }
                Ways::Binds(ways)
            }
            Step::Path(p) => self.path_ways(p, b, ann, mode),
        }
    }

    /// One stratum to its fixed point in `mode`.
    fn stratum(&mut self, heads: &BTreeSet<usize>, mode: Mode) {
        // A property's rows first: it depends on its events only, all in lower strata.
        let mut props = Vec::new();
        for &h in heads {
            if let Some(k) = self.c.prop_of[h] {
                props.push((h, self.property_rows(&self.c.props[k], mode)));
            }
        }
        for (h, rows) in props {
            let st = match mode {
                Mode::Lower => &mut self.lower[h],
                Mode::Upper => &mut self.upper[h],
            };
            for (t, a) in rows {
                st.insert(t, a);
            }
        }
        let rules: Vec<&CRule> = heads
            .iter()
            .flat_map(|&h| self.c.by_head[h].iter().map(|&k| &self.c.rules[k]))
            .collect();
        let recursive = |r: &CRule| -> Vec<usize> {
            r.steps
                .iter()
                .enumerate()
                .filter_map(|(k, s)| match s {
                    Step::Scan(Rel::Derived(d), _) if heads.contains(d) => Some(k),
                    _ => None,
                })
                .collect()
        };
        let mut delta: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut first = true;
        loop {
            let mut found: Vec<(usize, Vec<V>, Ann)> = Vec::new();
            for r in &rules {
                let mut out = Vec::new();
                if first {
                    self.steps(r, None, mode, &mut out);
                } else {
                    for k in recursive(r) {
                        let Step::Scan(Rel::Derived(d), _) = &r.steps[k] else {
                            unreachable!("a recursive scan")
                        };
                        let Some(dl) = delta.get(d) else { continue };
                        self.steps(r, Some((k, dl)), mode, &mut out);
                    }
                }
                found.extend(out.into_iter().map(|(t, a)| (r.head, t, a)));
            }
            first = false;
            let mut next: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for (h, t, a) in found {
                let st = match mode {
                    Mode::Lower => &mut self.lower[h],
                    Mode::Upper => &mut self.upper[h],
                };
                if let Some(k) = st.insert(t, a) {
                    next.entry(h).or_default().push(k);
                }
            }
            if next.is_empty() {
                return;
            }
            for v in next.values_mut() {
                v.sort_unstable();
                v.dedup();
            }
            delta = next;
        }
    }
}

/// The ways one step goes on, as [`Eval::ways`] gives them.
enum Ways<'s> {
    /// The tuples a scan matched, bound as its arguments say.
    Tuples {
        st: StoreRef<'s>,
        args: &'s [Arg],
        ts: Vec<usize>,
    },
    /// Bindings, each with what the solution then rests on.
    Binds(Vec<(Vec<(usize, V)>, Ann)>),
}

/// One step on the search stack: its ways, the next to try, the variables the current one bound.
struct Frame<'s> {
    k: usize,
    /// What the solution rests on before this step.
    ann: Ann,
    ways: Ways<'s>,
    next: usize,
    set: Vec<usize>,
}

impl Frame<'_> {
    /// Binds the next way that agrees with `b`; what the solution then rests on.
    fn take(&mut self, b: &mut [Option<V>], mode: Mode) -> Option<Ann> {
        loop {
            match &self.ways {
                Ways::Tuples { st, args, ts } => {
                    let &t = ts.get(self.next)?;
                    self.next += 1;
                    let tuple = &st.tuples[t];
                    let mut ok = true;
                    for (c, a) in args.iter().enumerate() {
                        if let Arg::Var(x) = a {
                            if b[*x].is_none() {
                                b[*x] = Some(tuple[c].clone());
                                self.set.push(*x);
                            } else if self.set.contains(x)
                                && !eqv(b[*x].as_ref().expect("set"), &tuple[c], mode)
                            {
                                ok = false;
                            }
                        }
                    }
                    if !ok {
                        for x in self.set.drain(..) {
                            b[x] = None;
                        }
                        continue;
                    }
                    let mut a = self.ann.clone();
                    union(&mut a, &st.anns[t]);
                    return Some(a);
                }
                Ways::Binds(list) => {
                    let (binds, a) = list.get(self.next)?;
                    self.next += 1;
                    for (x, v) in binds {
                        b[*x] = Some(v.clone());
                        self.set.push(*x);
                    }
                    return Some(a.clone());
                }
            }
        }
    }
}

/// The tuples a builtin gives for the arguments `b` binds: every argument, inputs included.
fn builtin(which: Builtin, args: &[Arg], b: &[Option<V>]) -> Vec<Vec<V>> {
    let input: Vec<Option<V>> = args.iter().map(|a| value_of(a, b)).collect();
    if input.iter().flatten().any(V::has_top) {
        return vec![vec![V::Top; args.len()]];
    }
    let str_of = |k: usize| match &input[k] {
        Some(V::Str(s)) => Some(s.clone()),
        _ => None,
    };
    match which {
        Builtin::Word => match str_of(0) {
            Some(s) => words(&s)
                .into_iter()
                .enumerate()
                .map(|(n, w)| vec![V::Str(s.clone()), V::Int(n as i64), V::Str(Rc::from(w))])
                .collect(),
            None => Vec::new(),
        },
        Builtin::Int => match str_of(0).and_then(|s| s.parse::<i64>().ok()) {
            Some(n) => vec![vec![input[0].clone().expect("bound"), V::Int(n)]],
            None => Vec::new(),
        },
        Builtin::Replace => match (str_of(0), str_of(1), str_of(2)) {
            (Some(s), Some(f), Some(t)) if !f.is_empty() => vec![vec![
                V::Str(s.clone()),
                V::Str(f.clone()),
                V::Str(t.clone()),
                V::Str(Rc::from(s.replace(&*f, &t))),
            ]],
            _ => Vec::new(),
        },
        Builtin::Concat => match (str_of(0), str_of(1)) {
            (Some(x), Some(y)) => vec![vec![
                V::Str(x.clone()),
                V::Str(y.clone()),
                V::Str(Rc::from(format!("{x}{y}"))),
            ]],
            _ => Vec::new(),
        },
    }
}

/// An aggregate over `n` tuples whose aggregated values are `vals`.
fn fold(func: AggFn, n: usize, mut vals: impl Iterator<Item = V>) -> Option<V> {
    match func {
        AggFn::Count => Some(V::Int(n as i64)),
        AggFn::Min => vals.min(),
        AggFn::Max => vals.max(),
        AggFn::Sum => vals.try_fold(V::Int(0), |acc, v| arith(&acc, '+', &v)),
        AggFn::Collect => {
            let set: BTreeSet<V> = vals.collect();
            Some(V::List(set.into_iter().collect()))
        }
    }
}

/// The values an aggregate takes when the tuples `sure` hold and each of `maybe` may or may not:
/// for `count`, `min` and `max` every value it takes over `sure` and some of `maybe`; for `sum`
/// and `collect` the value over `sure` and the value over all of them.
fn aggregate(func: AggFn, sure: &[usize], maybe: &[usize], value: impl Fn(usize) -> V) -> Vec<V> {
    let all = sure.iter().chain(maybe).map(|&t| value(t));
    let whole = fold(func, sure.len() + maybe.len(), all);
    if maybe.is_empty() {
        return whole.into_iter().collect();
    }
    let base = fold(func, sure.len(), sure.iter().map(|&t| value(t)));
    let mut out: BTreeSet<V> = BTreeSet::new();
    match func {
        AggFn::Count => {
            out.extend((sure.len()..=sure.len() + maybe.len()).map(|n| V::Int(n as i64)));
        }
        AggFn::Min | AggFn::Max => {
            for v in maybe.iter().map(|&t| value(t)) {
                let beyond = match &base {
                    None => true,
                    Some(m) if func == AggFn::Min => v < *m,
                    Some(m) => v > *m,
                };
                if beyond {
                    out.insert(v);
                }
            }
            out.extend(base);
        }
        AggFn::Sum | AggFn::Collect => {
            out.extend(base);
            out.extend(whole);
        }
    }
    out.into_iter().collect()
}

enum StoreRef<'a> {
    Shared(Rc<Store>),
    Own(&'a Store),
}

impl std::ops::Deref for StoreRef<'_> {
    type Target = Store;
    fn deref(&self) -> &Store {
        match self {
            StoreRef::Shared(s) => s,
            StoreRef::Own(s) => s,
        }
    }
}

/// Run a compiled program over a fact base.
pub fn evaluate(c: &Compiled, fb: &FactBase) -> Answer {
    let n = c.derived.len();
    // The relations the query needs, and those it needs an upper bound of.
    let mut need = vec![false; n];
    let mut stack = vec![c.query];
    while let Some(d) = stack.pop() {
        if std::mem::replace(&mut need[d], true) {
            continue;
        }
        if let Some(k) = c.prop_of[d] {
            stack.extend(prop_deps(&c.props[k]));
        }
        for r in c.by_head[d].iter().map(|&k| &c.rules[k]) {
            for s in &r.steps {
                match s {
                    Step::Scan(Rel::Derived(e), _)
                    | Step::Neg(Rel::Derived(e), _)
                    | Step::Agg {
                        rel: Rel::Derived(e),
                        ..
                    } => stack.push(*e),
                    Step::Path(p) => {
                        if let AvoidStep::Rel(Rel::Derived(e), _) = &p.avoid {
                            stack.push(*e);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let mut in_upper = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    for r in c.rules.iter().filter(|r| need[r.head]) {
        for s in &r.steps {
            match s {
                Step::Neg(Rel::Derived(e), _)
                | Step::Agg {
                    rel: Rel::Derived(e),
                    ..
                } => stack.push(*e),
                // What an unknown site could add to the nodes a path avoids.
                Step::Path(p) => {
                    if let AvoidStep::Rel(Rel::Derived(e), _) = &p.avoid {
                        stack.push(*e);
                    }
                }
                _ => {}
            }
        }
    }
    while let Some(d) = stack.pop() {
        if std::mem::replace(&mut in_upper[d], true) {
            continue;
        }
        for r in c.by_head[d].iter().map(|&k| &c.rules[k]) {
            for s in &r.steps {
                if let Step::Scan(Rel::Derived(e), _)
                | Step::Agg {
                    rel: Rel::Derived(e),
                    ..
                } = s
                {
                    stack.push(*e);
                }
            }
        }
    }
    let mut ev = Eval {
        fb,
        c,
        base: RefCell::new(HashMap::new()),
        lower: (0..n).map(|_| Store::default()).collect(),
        upper: (0..n).map(|_| Store::default()).collect(),
        in_upper,
        closure: RefCell::new(None),
        graphs: RefCell::new(None),
        extra: RefCell::new(Vec::new()),
    };
    let mut by_stratum: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for d in (0..n).filter(|&d| need[d]) {
        by_stratum.entry(c.strata[d]).or_default().insert(d);
    }
    for heads in by_stratum.values() {
        ev.stratum(heads, Mode::Lower);
        let up: BTreeSet<usize> = heads.iter().copied().filter(|&d| ev.in_upper[d]).collect();
        if !up.is_empty() {
            ev.stratum(&up, Mode::Upper);
        }
    }
    let q = &ev.lower[c.query];
    let mut rows: Vec<(Vec<V>, &Ann)> = q.tuples.iter().cloned().zip(&q.anns).collect();
    rows.sort();
    Answer {
        vars: c.vars.clone(),
        rows: rows
            .into_iter()
            .map(|(t, a)| {
                let extra = ev.extra.borrow();
                let mut unknown: Vec<crate::Site> = a
                    .iter()
                    .map(|&k| match fb.sites.get(k as usize) {
                        Some(s) => s.clone(),
                        None => extra[k as usize - fb.sites.len()].clone(),
                    })
                    .collect();
                unknown.sort_by(|x, y| {
                    (&x.site, &x.pos, &x.reason).cmp(&(&y.site, &y.pos, &y.reason))
                });
                Row {
                    values: t.iter().map(V::value).collect(),
                    unknown,
                }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::words;

    #[test]
    fn words_split_as_codespec_check_splits_them() {
        assert_eq!(words("NewSessionID"), ["new", "session", "id"]);
        assert_eq!(words("RemoveHeaders"), ["remove", "header"]);
        assert_eq!(words("parse_port"), ["parse", "port"]);
    }
}
