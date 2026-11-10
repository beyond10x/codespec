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
use crate::syntax::{AggFn, Args, Atom, Field, Lit, Op, Program, Rule, Term, query_text};
use crate::{Answer, Names, Row, Value};

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

/// The facts of one relation, with an index per set of bound columns, built when first asked.
#[derive(Default)]
struct Store {
    tuples: Vec<Vec<V>>,
    anns: Vec<Ann>,
    at: HashMap<Vec<V>, usize>,
    /// The tuples with a wild field; never in an index.
    tops: Vec<usize>,
    indexes: RefCell<HashMap<u64, Index>>,
}

impl Store {
    /// Adds a fact or what it rests on; whether anything changed.
    fn insert(&mut self, t: Vec<V>, ann: Ann) -> Option<usize> {
        if let Some(&k) = self.at.get(&t) {
            let old = &mut self.anns[k];
            if old.is_empty() {
                return None;
            }
            if ann.is_empty() {
                old.clear();
                return Some(k);
            }
            let before = old.len();
            union(old, &ann);
            return (old.len() != before).then_some(k);
        }
        let k = self.tuples.len();
        if t.iter().any(V::has_top) {
            self.tops.push(k);
        } else {
            for (mask, index) in self.indexes.get_mut().iter_mut() {
                index.entry(key_of(*mask, &t)).or_default().push(k);
            }
        }
        self.at.insert(t.clone(), k);
        self.tuples.push(t);
        self.anns.push(ann);
        Some(k)
    }

    /// The tuples whose columns in `mask` hold `key`, with every tuple holding a wild field.
    fn candidates(&self, mask: u64, key: &[V]) -> Vec<usize> {
        if mask == 0 || key.iter().any(V::has_top) {
            return (0..self.tuples.len()).collect();
        }
        let mut indexes = self.indexes.borrow_mut();
        let index = indexes.entry(mask).or_insert_with(|| {
            let mut ix: HashMap<Vec<V>, Vec<usize>> = HashMap::new();
            for (k, t) in self.tuples.iter().enumerate() {
                if !t.iter().any(V::has_top) {
                    ix.entry(key_of(mask, t)).or_default().push(k);
                }
            }
            ix
        });
        let mut out = index.get(key).cloned().unwrap_or_default();
        out.extend(&self.tops);
        out
    }
}

fn key_of(mask: u64, t: &[V]) -> Vec<V> {
    t.iter()
        .enumerate()
        .filter(|(c, _)| mask & (1 << c) != 0)
        .map(|(_, v)| v.clone())
        .collect()
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

#[derive(Debug, Clone)]
enum Step {
    Scan(Rel, Vec<Arg>),
    Neg(Rel, Vec<Arg>),
    Cmp(Arg, Op, Arg),
    Match(Arg, Regex, bool),
    In(Arg, Rc<BTreeSet<String>>, bool),
    Builtin(Builtin, Vec<Arg>),
    Agg {
        var: usize,
        func: AggFn,
        rel: Rel,
        /// (column of the aggregated relation, the outer variable it is)
        outer: Vec<(usize, usize)>,
        target: Option<usize>,
    },
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
    rules: Vec<CRule>,
    query: usize,
    vars: Vec<String>,
    /// Each derived relation's stratum, from 1.
    strata: Vec<usize>,
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
    }
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
    lists: HashMap<String, Rc<BTreeSet<String>>>,
    fresh: usize,
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

    fn list(&mut self, name: &str) -> Result<Rc<BTreeSet<String>>, String> {
        if let Some(l) = self.lists.get(name) {
            return Ok(l.clone());
        }
        let Some(items) = self.names.get(name) else {
            return Err(format!(
                "no names list {name}: declare it with names {name} = [\"…\"]."
            ));
        };
        let l = Rc::new(items.iter().cloned().collect::<BTreeSet<_>>());
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
        }
        Ok(())
    }

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
                    Args::Named(fs) => Args::Named(
                        fs.iter()
                            .map(|(n, f)| {
                                (
                                    n.clone(),
                                    match f {
                                        Field::Term(x) => Field::Term(t(x, self)),
                                        f => f.clone(),
                                    },
                                )
                            })
                            .collect(),
                    ),
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
        while !pending.is_empty() {
            let ready = pending.iter().position(|l| match l {
                Low::Scan(..) => true,
                Low::Neg(_, ts) => all_bound(&ts.iter().collect::<Vec<_>>(), &bound),
                Low::Cmp(a, Op::Bind, b) => {
                    let free = |t: &Term| matches!(t, Term::Var(v) if !bound.contains(v));
                    (free(a) && all_bound(&[b], &bound))
                        || (free(b) && all_bound(&[a], &bound))
                        || all_bound(&[a, b], &bound)
                }
                Low::Cmp(a, _, b) => all_bound(&[a, b], &bound),
                Low::Match(t, ..) | Low::In(t, ..) => all_bound(&[t], &bound),
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
            });
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
                Low::In(t, l, neg) => Step::In(arg(&t, &mut slots), self.list(&l)?, neg),
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
            Lit::Not(_) | Lit::NotCond(_) => {}
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
    let mut lists: BTreeMap<String, Vec<String>> = p.names.iter().cloned().collect();
    for (name, items) in &names.lists {
        if !lists.contains_key(name) {
            let declared: Vec<&str> = p.names.iter().map(|(n, _)| n.as_str()).collect();
            return Err(format!(
                "names: the module declares no list {name}; it declares {}",
                if declared.is_empty() {
                    "none".to_string()
                } else {
                    declared.join(", ")
                }
            ));
        }
        lists.insert(name.clone(), items.clone());
    }
    let mut c = Compiler {
        derived: Vec::new(),
        by_name: HashMap::new(),
        rules: Vec::new(),
        names: &lists,
        lists: HashMap::new(),
        fresh: 0,
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
    for r in &p.rules {
        let d = c.by_name[&r.head.rel];
        c.rule(r, d)?;
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
    let strata = stratify(&c.derived, &c.rules)?;
    Ok(Compiled {
        derived: c.derived,
        rules: c.rules,
        query: q,
        vars,
        strata,
    })
}

/// Each derived relation's stratum: 1 for one over the fact base only, one more than any it
/// negates or aggregates over. A negation or an aggregate inside a recursion is refused.
fn stratify(derived: &[Derived], rules: &[CRule]) -> Result<Vec<usize>, String> {
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
                _ => {}
            }
        }
    }
    // Tarjan's strongly connected components, dependencies first.
    struct T<'a> {
        edges: &'a [BTreeSet<(usize, bool)>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        comps: Vec<Vec<usize>>,
    }
    fn visit(t: &mut T, v: usize) {
        t.index[v] = Some(t.next);
        t.low[v] = t.next;
        t.next += 1;
        t.stack.push(v);
        t.on[v] = true;
        let es: Vec<usize> = t.edges[v].iter().map(|(w, _)| *w).collect();
        for w in es {
            match t.index[w] {
                None => {
                    visit(t, w);
                    t.low[v] = t.low[v].min(t.low[w]);
                }
                Some(i) if t.on[w] => t.low[v] = t.low[v].min(i),
                _ => {}
            }
        }
        if Some(t.low[v]) == t.index[v] {
            let mut comp = Vec::new();
            while let Some(w) = t.stack.pop() {
                t.on[w] = false;
                comp.push(w);
                if w == v {
                    break;
                }
            }
            t.comps.push(comp);
        }
    }
    let mut t = T {
        edges: &edges,
        index: vec![None; n],
        low: vec![0; n],
        on: vec![false; n],
        stack: Vec::new(),
        next: 0,
        comps: Vec::new(),
    };
    for v in 0..n {
        if t.index[v].is_none() {
            visit(&mut t, v);
        }
    }
    let mut comp_of = vec![0; n];
    for (k, comp) in t.comps.iter().enumerate() {
        for &v in comp {
            comp_of[v] = k;
        }
    }
    let mut strata = vec![1; n];
    for (k, comp) in t.comps.iter().enumerate() {
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
    let mut out = String::new();
    for (s, rules) in by {
        out += &format!("stratum {s}\n");
        for r in rules {
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
        self.derived
            .iter()
            .position(|d| d.name == name)
            .expect("a compiled relation")
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
}

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
        if name == "reaches" {
            self.reaches(&mut st, upper);
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
            }
        }
        let st = Rc::new(st);
        self.base.borrow_mut().insert((name, upper), st.clone());
        st
    }

    /// `reaches`: the closure of call → calls; in the upper mode, a function from which some
    /// unknown site is reached reaches anything, resting on those sites.
    fn reaches(&self, st: &mut Store, upper: bool) {
        let mut func_of: HashMap<&str, &str> = HashMap::new();
        for row in self.fb.rels.get("call").into_iter().flatten() {
            if let (Value::Str(site), Value::Str(f)) = (&row[0], &row[1]) {
                func_of.insert(site, f);
            }
        }
        let mut next: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for row in self.fb.rels.get("calls").into_iter().flatten() {
            if let (Value::Str(site), Value::Str(t)) = (&row[0], &row[1])
                && let Some(f) = func_of.get(site.as_str())
            {
                next.entry(f).or_default().insert(t);
            }
        }
        // The unknown sites of each function: its wild calls, and wild links of its sites.
        let mut own: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
        if upper {
            for (row, site) in self.fb.wild.get("call").into_iter().flatten() {
                if let Some(Value::Str(f)) = &row[1] {
                    own.entry(f).or_default().insert(*site as u32);
                }
            }
            for (row, site) in self.fb.wild.get("calls").into_iter().flatten() {
                if let Some(Value::Str(s)) = &row[0] {
                    let f =
                        func_of.get(s.as_str()).copied().or_else(|| {
                            self.fb.wild.get("call").into_iter().flatten().find_map(
                                |(w, _)| match (&w[0], &w[1]) {
                                    (Some(Value::Str(ws)), Some(Value::Str(wf))) if ws == s => {
                                        Some(wf.as_str())
                                    }
                                    _ => None,
                                },
                            )
                        });
                    if let Some(f) = f {
                        own.entry(f).or_default().insert(*site as u32);
                    }
                }
            }
        }
        let mut froms: BTreeSet<&str> = next.keys().copied().collect();
        froms.extend(own.keys().copied());
        for from in froms {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut stack: Vec<&str> = next.get(from).into_iter().flatten().copied().collect();
            while let Some(g) = stack.pop() {
                if seen.insert(g) {
                    stack.extend(next.get(g).into_iter().flatten().copied());
                }
            }
            let f = V::Str(Rc::from(from));
            let mut sites: BTreeSet<u32> = own.get(from).cloned().unwrap_or_default();
            for g in &seen {
                st.insert(vec![f.clone(), V::Str(Rc::from(*g))], Vec::new());
                if let Some(s) = own.get(g) {
                    sites.extend(s);
                }
            }
            if upper && !sites.is_empty() {
                st.insert(vec![f, V::Top], sites.into_iter().collect());
            }
        }
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
            Rel::Base(name) => self.fb.wild.contains_key(name) || ON_DEMAND.contains(&name),
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
        let mut mask = 0u64;
        let mut key = Vec::new();
        for (c, a) in args.iter().enumerate() {
            let v = match a {
                Arg::Var(x) => b[*x].clone(),
                Arg::Val(v) => Some(v.clone()),
                Arg::Arith(..) => value_of(a, b),
                Arg::Any => None,
            };
            if let Some(v) = v {
                mask |= 1 << c;
                key.push(v);
            }
        }
        let cands = match only {
            Some(d) => d.to_vec(),
            None => store.candidates(mask, &key),
        };
        cands
            .into_iter()
            .filter(|&k| {
                let t = &store.tuples[k];
                let mut j = 0;
                for (c, _) in args.iter().enumerate() {
                    if mask & (1 << c) != 0 {
                        if !eqv(&key[j], &t[c], mode) {
                            return false;
                        }
                        j += 1;
                    }
                }
                true
            })
            .collect()
    }

    /// Runs the steps of `r` from `k`, each solution's head tuple and what it rests on to `out`.
    #[allow(clippy::too_many_arguments)]
    fn steps(
        &self,
        r: &CRule,
        k: usize,
        b: &mut Vec<Option<V>>,
        ann: &Ann,
        delta: Option<(usize, &[usize])>,
        mode: Mode,
        out: &mut Vec<(Vec<V>, Ann)>,
    ) {
        let Some(step) = r.steps.get(k) else {
            let t: Option<Vec<V>> = r.args.iter().map(|a| value_of(a, b)).collect();
            if let Some(t) = t {
                out.push((t, ann.clone()));
            }
            return;
        };
        match step {
            Step::Scan(rel, args) => {
                let st = self.store(*rel, mode);
                let only = delta.filter(|(j, _)| *j == k).map(|(_, d)| d);
                for t in self.matches(&st, args, b, only, mode) {
                    let tuple = &st.tuples[t];
                    let mut set = Vec::new();
                    let mut ok = true;
                    for (c, a) in args.iter().enumerate() {
                        if let Arg::Var(x) = a
                            && b[*x].is_none()
                        {
                            b[*x] = Some(tuple[c].clone());
                            set.push(*x);
                        } else if let Arg::Var(x) = a
                            && set.contains(x)
                            && !eqv(b[*x].as_ref().expect("set"), &tuple[c], mode)
                        {
                            ok = false;
                        }
                    }
                    if ok {
                        let mut a2 = ann.clone();
                        union(&mut a2, &st.anns[t]);
                        self.steps(r, k + 1, b, &a2, delta, mode, out);
                    }
                    for x in set {
                        b[x] = None;
                    }
                }
            }
            Step::Neg(rel, args) => {
                let low = self.store(*rel, Mode::Lower);
                let found = self.matches(&low, args, b, None, Mode::Lower);
                let mut a2 = ann.clone();
                match mode {
                    Mode::Lower => {
                        if !found.is_empty() {
                            return;
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
                                return;
                            }
                            for &t in &found {
                                union(&mut a2, &low.anns[t]);
                            }
                        }
                    }
                }
                self.steps(r, k + 1, b, &a2, delta, mode, out);
            }
            Step::Cmp(x, op, y) => {
                if *op == Op::Bind {
                    let (vx, vy) = (value_of(x, b), value_of(y, b));
                    match (x, y, vx, vy) {
                        (Arg::Var(s), _, None, Some(v)) | (_, Arg::Var(s), Some(v), None) => {
                            b[*s] = Some(v);
                            self.steps(r, k + 1, b, ann, delta, mode, out);
                            b[*s] = None;
                        }
                        (_, _, Some(p), Some(q)) if eqv(&p, &q, mode) => {
                            self.steps(r, k + 1, b, ann, delta, mode, out);
                        }
                        _ => {}
                    }
                    return;
                }
                let (Some(p), Some(q)) = (value_of(x, b), value_of(y, b)) else {
                    return;
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
                    self.steps(r, k + 1, b, ann, delta, mode, out);
                }
            }
            Step::Match(x, re, neg) => {
                let holds = match value_of(x, b) {
                    Some(V::Str(s)) => re.is_match(&s) != *neg,
                    Some(V::Top) => true,
                    _ => *neg,
                };
                if holds {
                    self.steps(r, k + 1, b, ann, delta, mode, out);
                }
            }
            Step::In(x, set, neg) => {
                let holds = match value_of(x, b) {
                    Some(V::Str(s)) => set.contains(&*s) != *neg,
                    Some(V::Top) => true,
                    _ => *neg,
                };
                if holds {
                    self.steps(r, k + 1, b, ann, delta, mode, out);
                }
            }
            Step::Builtin(which, args) => {
                let input: Vec<Option<V>> = args.iter().map(|a| value_of(a, b)).collect();
                let outputs: Vec<Vec<V>> = if input.iter().flatten().any(V::has_top) {
                    vec![vec![V::Top; args.len()]]
                } else {
                    let str_of = |k: usize| match &input[k] {
                        Some(V::Str(s)) => Some(s.clone()),
                        _ => None,
                    };
                    match which {
                        Builtin::Word => match str_of(0) {
                            Some(s) => words(&s)
                                .into_iter()
                                .enumerate()
                                .map(|(n, w)| {
                                    vec![V::Str(s.clone()), V::Int(n as i64), V::Str(Rc::from(w))]
                                })
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
                };
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
                    if ok {
                        self.steps(r, k + 1, b, ann, delta, mode, out);
                    }
                    for x in set {
                        b[x] = None;
                    }
                }
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
                let vals = || {
                    found
                        .iter()
                        .map(|&t| st.tuples[t][target.expect("a target")].clone())
                };
                let result = if mode == Mode::Upper && wild {
                    Some(V::Top)
                } else {
                    match func {
                        AggFn::Count => Some(V::Int(found.len() as i64)),
                        AggFn::Min => vals().min(),
                        AggFn::Max => vals().max(),
                        AggFn::Sum => vals().try_fold(V::Int(0), |acc, v| arith(&acc, '+', &v)),
                        AggFn::Collect => {
                            let set: BTreeSet<V> = vals().collect();
                            Some(V::List(set.into_iter().collect()))
                        }
                    }
                };
                let Some(result) = result else { return };
                match &b[*var] {
                    Some(v) => {
                        if eqv(v, &result, mode) {
                            self.steps(r, k + 1, b, &a2, delta, mode, out);
                        }
                    }
                    None => {
                        b[*var] = Some(result);
                        self.steps(r, k + 1, b, &a2, delta, mode, out);
                        b[*var] = None;
                    }
                }
            }
        }
    }

    /// One stratum to its fixed point in `mode`.
    fn stratum(&mut self, heads: &BTreeSet<usize>, mode: Mode) {
        let rules: Vec<&CRule> = self
            .c
            .rules
            .iter()
            .filter(|r| heads.contains(&r.head))
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
                let mut b = vec![None; r.vars];
                if first {
                    self.steps(r, 0, &mut b, &Vec::new(), None, mode, &mut out);
                } else {
                    for k in recursive(r) {
                        let Step::Scan(Rel::Derived(d), _) = &r.steps[k] else {
                            unreachable!("a recursive scan")
                        };
                        let Some(dl) = delta.get(d) else { continue };
                        self.steps(r, 0, &mut b, &Vec::new(), Some((k, dl)), mode, &mut out);
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
        for r in c.rules.iter().filter(|r| r.head == d) {
            for s in &r.steps {
                if let Step::Scan(Rel::Derived(e), _)
                | Step::Neg(Rel::Derived(e), _)
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
    let mut in_upper = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    for r in c.rules.iter().filter(|r| need[r.head]) {
        for s in &r.steps {
            if let Step::Neg(Rel::Derived(e), _)
            | Step::Agg {
                rel: Rel::Derived(e),
                ..
            } = s
            {
                stack.push(*e);
            }
        }
    }
    while let Some(d) = stack.pop() {
        if std::mem::replace(&mut in_upper[d], true) {
            continue;
        }
        for r in c.rules.iter().filter(|r| r.head == d) {
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
                let mut unknown: Vec<crate::Site> =
                    a.iter().map(|&k| fb.sites[k as usize].clone()).collect();
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
