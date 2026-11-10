//! Per-function differential runs (`docs/evaluator.md` § 5.4): every exported function of a
//! module gets seeded calls, made by the evaluator and by the native runners on what the writers
//! write, and the outputs must be byte-equal. `codespec-eval-diff` runs them through the writer and
//! runner pairs `codespec-write-<lang>` / `codespec-run-<lang>` for `cir`, `go` and `rust`; `task
//! eval-diff` runs it on `examples/series` and `examples/inventory`, 200 seeds per function.
//!
//! # Entries and probes
//!
//! A `codespec-calls/1` file calls exported top-level functions with integer, `bool` and `string`
//! arguments and prints their results ([`codespec_conformance`] § `calls.json`). [`plan`] gives
//! every exported function of the module (after `codespec pass`) one **entry**, the function the
//! calls name:
//!
//! - a top-level function whose parameters and results are already of those kinds is its own
//!   entry;
//! - a **constructor**, an exported top-level function without parameters whose one result is a
//!   struct `T` or `*T` the module declares, gets no entry of its own: it has no input to draw,
//!   and its result is not printable. It starts every probe of a method of `T`;
//! - any other function gets a **probe**, a generated exported top-level function
//!   `Probe<Type><Name>` (or `Probe<Name>`) added to the module, which builds what the calls file
//!   cannot pass and calls it.
//!
//! A probe's parameters are drawn values: an integer of the parameter's width, a `bool`, a
//! `string`, and for an enum the module declares an `int` that the generated helper
//! `pick<Enum>(i int) <Enum>` maps to its `i`-th variant (the last for any `i` past it). Its body:
//!
//! 1. the receiver of a method: for a struct with a constructor, `recv := New()` (or `recv :=
//!    &T{…}` when the constructor's whole body returns that literal, the form in which the Rust
//!    writer owns a pointer to a type `shared` does not list) followed by [`SLOTS`] setup slots,
//!    each an `if` on a drawn operation choosing one **mutator** of `T` or none (a mutator is an
//!    exported method of `T` with drawable parameters that the `read_only` fact does not list),
//!    its results dropped; for an enum, `pick<Enum>` of a drawn `int`; for a
//!    struct without a constructor whose fields are all drawable, a struct literal of drawn
//!    fields. A mutator listed `fallible` is called as `…, err := recv.M(…)` followed by `if err !=
//!    nil { return <zero values>, err }`, the one form in which the Rust writer writes a call of
//!    a fallible function, and the probe gets a last `error` result. The calls never choose a
//!    setup call that returns an error (§ Calls), so that return is not taken;
//! 2. the call, its results bound to `res0`, `res1`, …; a result listed `fallible` returns its
//!    error the same way;
//! 3. each result returned: a struct whose fields are all printable flattened into its fields in
//!    declaration order, a slice of integers or strings that the function fills from a loop over
//!    a map (a position the `unordered_iteration` fact lists inside its body) sorted ascending by
//!    insertion sort, since Go, Rust and the evaluator iterate a map in three different orders.
//!    A method that returns no value returns its receiver's state after the call instead: the
//!    receiver's fields when all are printable (a slice field copied element by element into a
//!    new slice, since the Rust writer refuses a copy sharing its array), else the results of its
//!    exported methods that
//!    `read_only` lists, take no parameters and return one printable value.
//!
//! A function none of this reaches (a parameter of another kind, a receiver with neither, a
//! method without a value whose receiver shows no state) is listed in [`Plan::skipped`] with the
//! reason; `codespec-eval-diff` fails when any is.
//!
//! For a probe with a fallible mutator, [`Plan::setup`] also holds its **setup function**
//! `SetupProbe<…>`: the same parameters, the receiver and the setup slots alone, returning the
//! first setup call's error or `nil`. Only the evaluator runs it.
//!
//! # Calls
//!
//! [`calls`] gives, per entry in order, one call per seed `1..=n`. Each seed's values come from
//! [`SplitMix64`] seeded with the seed and the entry's name, by integer arithmetic only, so a seed
//! gives the same calls on every platform. For the probed function's own parameters, a quarter of
//! integers are edge values of the width (`0`, `±1`, its extremes and one off them, and for 64
//! bits `2147483647`, `46337`, `2^32`), an eighth in `[-1000, 1000)`, the rest in `[-3, 20]`;
//! strings are one of [`STRINGS`]. A setup call draws integers in `[1, 20]` but for one in 16,
//! drawn as above. Each seed first draws a **key** from [`SETUP_STRINGS`]: a setup call's string
//! is the key three times in four, else drawn from [`SETUP_STRINGS`], and the probed function's
//! own string the key half the time, else drawn from [`STRINGS`], so that the calls name what
//! the setup calls added. Each seed fills its first `k` slots, `k` the larger of two draws from
//! `0..=SLOTS`, each with a drawn mutator, and every other seed puts them in the mutators'
//! declaration order (the mutators that add state are declared before those that need it in
//! both examples).
//!
//! A setup call that would return an error is skipped, not made: slot by slot, the evaluator runs
//! the setup function with the slots before as chosen and those after empty, and a slot whose call
//! returns an error is drawn again (mutator and arguments) up to [`REDRAWS`] times, then left
//! empty. So every call reaches the probed function, and a seed whose first draw failed gets a
//! setup call that succeeds on the state the earlier ones made.
//!
//! [`reach`] measures this on a run: per function, how many of its seeds reach it (the setup
//! function returns `nil` on their arguments) and how many of those print a result rather than an
//! error. `codespec-eval-diff` prints both sums per example and fails when a function's seeds
//! reach it fewer than half the time.
//!
//! # Comparison
//!
//! [`compare`] takes the first output as the reference and names, for the first that differs, the
//! line, both texts and the call: its function, entry and seed.

use std::collections::{BTreeMap, BTreeSet};

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_harness::SplitMix64;
use codespec_model::{Func, IntWidth, Module, Type, TypeDecl, receiver_type_name};
use serde_json::{Value, json};

use crate::{Options, run_calls};

/// Setup slots of a probe whose receiver comes from a constructor.
pub const SLOTS: usize = 4;

/// How many times a setup slot whose call returns an error is drawn again before it is left
/// empty.
pub const REDRAWS: usize = 3;

/// The strings a `string` parameter of the probed function draws from: few, so that keys repeat.
pub const STRINGS: &[&str] = &["a", "b", "", "A b"];

/// The strings a `string` parameter of a setup call draws from.
pub const SETUP_STRINGS: &[&str] = &["a", "b"];

/// What a parameter of an entry draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draw {
    /// A signed integer of this many bits.
    Int(u32),
    Bool,
    Str,
    /// An index into an enum's `n` variants.
    Pick(usize),
    /// A setup slot's operation: one of `n` mutators, or `n` for none.
    Op(usize),
    /// A setup call's integer of this many bits.
    SetupInt(u32),
    /// A setup call's string.
    SetupStr,
}

/// One function's entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The function's key in the module: `Steps`, `Inventory.Find`.
    pub func: String,
    /// The exported top-level function the calls name: the function itself or its probe.
    pub call: String,
    /// The constructor that makes the receiver of a probed method, when it has one.
    pub receiver: Option<String>,
    /// What each parameter of `call` draws, in order.
    pub params: Vec<Draw>,
    /// Each setup slot's parameters: its operation first, then every mutator's arguments.
    pub slots: Vec<std::ops::Range<usize>>,
    /// The function of [`Plan::setup`] that makes the probe's setup calls alone and returns their
    /// first error, when a mutator can return one.
    pub setup: Option<String>,
}

/// The module with its probes, the entries and what has none.
#[derive(Debug, Clone)]
pub struct Plan {
    /// The module given, with the probes and helpers added and `codespec pass` run over it.
    pub module: Module,
    /// [`Plan::module`] with each [`Entry::setup`] function added: the evaluator runs it to choose
    /// the setup calls (module doc § Calls). No writer is given it.
    pub setup: Module,
    pub entries: Vec<Entry>,
    /// The constructors, by key, in module order.
    pub constructors: Vec<String>,
    /// Exported functions without an entry, by key, and why.
    pub skipped: Vec<(String, String)>,
}

fn int_bits(w: IntWidth) -> u32 {
    match w {
        IntWidth::W8 => 8,
        IntWidth::W16 => 16,
        IntWidth::W32 => 32,
        IntWidth::W64 | IntWidth::Platform => 64,
    }
}

fn platform_int() -> Type {
    Type::Int {
        width: IntWidth::Platform,
        signed: true,
    }
}

/// What a probe knows of the module.
struct Scope<'a> {
    m: &'a Module,
    types: BTreeMap<&'a str, &'a TypeDecl>,
    read_only: BTreeSet<&'a str>,
    fallible: BTreeSet<&'a str>,
    unordered: BTreeSet<&'a str>,
}

impl<'a> Scope<'a> {
    fn new(m: &'a Module) -> Scope<'a> {
        let facts = m.facts.as_ref();
        let list = |f: fn(&codespec_model::Facts) -> &Vec<String>| -> BTreeSet<&'a str> {
            facts
                .map(|x| f(x).iter().map(String::as_str).collect())
                .unwrap_or_default()
        };
        Scope {
            m,
            types: m.types.iter().map(|t| (t.name(), t)).collect(),
            read_only: list(|f| &f.read_only),
            fallible: list(|f| &f.fallible),
            unordered: facts
                .map(|f| f.unordered_iteration.iter().map(|p| p.as_str()).collect())
                .unwrap_or_default(),
        }
    }

    fn enum_variants(&self, t: &Type) -> Option<(&'a str, Vec<&'a str>)> {
        let Type::Named { name, targs } = t else {
            return None;
        };
        if !targs.is_empty() {
            return None;
        }
        match self.types.get(name.as_str())? {
            TypeDecl::Enum { name, variants, .. } if !variants.is_empty() => Some((
                name.as_str(),
                variants.iter().map(|v| v.name.as_str()).collect(),
            )),
            _ => None,
        }
    }

    /// What a parameter of type `t` draws, if anything.
    fn draw(&self, t: &Type) -> Option<Draw> {
        match t {
            Type::Int {
                width,
                signed: true,
            } => Some(Draw::Int(int_bits(*width))),
            Type::Bool => Some(Draw::Bool),
            Type::String => Some(Draw::Str),
            _ => self.enum_variants(t).map(|(_, v)| Draw::Pick(v.len())),
        }
    }

    /// Whether the kit's drivers print a value of type `t`.
    fn printable(&self, t: &Type) -> bool {
        match t {
            Type::Int { signed: true, .. } | Type::Bool | Type::String => true,
            Type::Slice { elem } => self.printable(elem) && !matches!(**elem, Type::Slice { .. }),
            Type::Named { name, targs } if targs.is_empty() => {
                match self.types.get(name.as_str()) {
                    Some(TypeDecl::Enum { under, .. }) | Some(TypeDecl::Newtype { under, .. }) => {
                        self.printable(under)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// The fields of a struct `t` when every one is printable.
    fn flat_fields(&self, t: &Type) -> Option<Vec<(&'a str, Type)>> {
        let Type::Named { name, targs } = t else {
            return None;
        };
        if !targs.is_empty() {
            return None;
        }
        match self.types.get(name.as_str())? {
            TypeDecl::Struct {
                fields, tparams, ..
            } if tparams.is_empty() => fields
                .iter()
                .map(|f| self.printable(&f.t).then(|| (f.name.as_str(), f.t.clone())))
                .collect(),
            _ => None,
        }
    }

    fn struct_fields(&self, name: &str) -> Option<&'a [codespec_model::Field]> {
        match self.types.get(name)? {
            TypeDecl::Struct {
                fields, tparams, ..
            } if tparams.is_empty() => Some(fields),
            _ => None,
        }
    }

    fn is_fallible(&self, f: &Func) -> bool {
        f.results.last() == Some(&Type::Error) && self.fallible.contains(f.key().as_str())
    }

    fn mutators(&self, t: &str) -> Vec<&'a Func> {
        self.m
            .funcs
            .iter()
            .filter(|f| {
                f.exported
                    && f.tparams.is_empty()
                    && f.recv
                        .as_ref()
                        .is_some_and(|r| receiver_type_name(&r.t) == t)
                    && !self.read_only.contains(f.key().as_str())
                    && f.params.iter().all(|p| self.draw(&p.t).is_some())
            })
            .collect()
    }

    /// The exported methods of `t` that `read_only` lists, without parameters, whose one result
    /// is printable: what shows a receiver's state when its fields do not.
    fn readers(&self, t: &str) -> Vec<&'a Func> {
        self.m
            .funcs
            .iter()
            .filter(|f| {
                f.exported
                    && f.tparams.is_empty()
                    && f.params.is_empty()
                    && f.recv
                        .as_ref()
                        .is_some_and(|r| receiver_type_name(&r.t) == t)
                    && self.read_only.contains(f.key().as_str())
                    && matches!(f.results.as_slice(), [r] if self.printable(r))
            })
            .collect()
    }

    /// Whether `f` fills a result from a loop over a map.
    fn iterates_a_map(&self, f: &Func) -> bool {
        let body = serde_json::to_value(&f.body).expect("a body serializes");
        let mut found = false;
        positions(&body, &mut |p| found |= self.unordered.contains(p));
        found
    }
}

fn positions(v: &Value, f: &mut dyn FnMut(&str)) {
    match v {
        Value::Object(o) => {
            if let Some(p) = o.get("pos").and_then(Value::as_str) {
                f(p);
            }
            o.values().for_each(|x| positions(x, f));
        }
        Value::Array(a) => a.iter().for_each(|x| positions(x, f)),
        _ => {}
    }
}

fn is_constructor(f: &Func, s: &Scope) -> Option<String> {
    if f.recv.is_some() || !f.exported || !f.params.is_empty() || !f.tparams.is_empty() {
        return None;
    }
    let [t] = f.results.as_slice() else {
        return None;
    };
    let inner = match t {
        Type::Ptr { elem } => elem.as_ref(),
        other => other,
    };
    match inner {
        Type::Named { name, targs } if targs.is_empty() => {
            s.struct_fields(name).map(|_| name.clone())
        }
        _ => None,
    }
}

// ---- cir/1 JSON builders -------------------------------------------------------------------

fn ty(t: &Type) -> Value {
    serde_json::to_value(t).expect("a type serializes")
}

fn var(name: &str, t: &Type) -> Value {
    json!({"k": "var", "name": name, "t": ty(t)})
}

fn int_lit(v: i64) -> Value {
    json!({"k": "lit", "value": v.to_string(), "t": ty(&platform_int())})
}

fn binop(op: &str, x: Value, y: Value, t: &Type) -> Value {
    json!({"k": "binop", "op": op, "x": x, "y": y, "t": ty(t)})
}

fn result_type(results: &[Type]) -> Type {
    match results {
        [one] => one.clone(),
        many => Type::Tuple {
            elems: many.to_vec(),
        },
    }
}

/// Statements with positions `probe.go:<n>`, numbered as they are made.
struct Lines(u32);

impl Lines {
    fn pos(&mut self) -> String {
        self.0 += 1;
        format!("probe.go:{}", self.0)
    }
}

/// The probe being built: its parameters, each with its type and draw, and the enums it picks.
struct Probe<'s, 'a> {
    s: &'s Scope<'a>,
    params: Vec<(String, Type, Draw)>,
    lines: &'s mut Lines,
    picks: &'s mut BTreeSet<String>,
}

impl Probe<'_, '_> {
    /// A new parameter drawing for type `t`, and the expression giving the value of `t`.
    fn arg(&mut self, prefix: &str, t: &Type) -> Value {
        let draw = match self.s.draw(t).expect("only drawable types are passed") {
            Draw::Int(bits) if prefix == "a" => Draw::SetupInt(bits),
            Draw::Str if prefix == "a" => Draw::SetupStr,
            d => d,
        };
        let name = format!("{prefix}{}", self.params.len());
        match &draw {
            Draw::Pick(_) => {
                let (e, _) = self.s.enum_variants(t).expect("an enum");
                self.picks.insert(e.to_string());
                self.params
                    .push((name.clone(), platform_int(), draw.clone()));
                json!({"k": "call", "name": format!("pick{e}"), "args": [var(&name, &platform_int())], "t": ty(t)})
            }
            _ => {
                self.params.push((name.clone(), t.clone(), draw));
                var(&name, t)
            }
        }
    }

    fn op(&mut self, n: usize) -> Value {
        let name = format!("o{}", self.params.len());
        self.params
            .push((name.clone(), platform_int(), Draw::Op(n)));
        var(&name, &platform_int())
    }
}

fn zeros(types: &[Type]) -> Vec<Value> {
    types
        .iter()
        .map(|t| json!({"k": "zero", "t": ty(t)}))
        .collect()
}

/// `names := call` then, when the last name is `err`, `if err != nil { return <zeros>, err }`.
fn bind_checked(
    names: Vec<String>,
    call: Value,
    fallible: bool,
    returns: &[Type],
    lines: &mut Lines,
) -> Vec<Value> {
    let mut out = vec![json!({"k": "let", "names": names, "rhs": [call], "pos": lines.pos()})];
    if fallible {
        let mut rs = zeros(&returns[..returns.len() - 1]);
        rs.push(var("err", &Type::Error));
        out.push(json!({
            "k": "if",
            "cond": binop("!=", var("err", &Type::Error), json!({"k": "nil", "t": ty(&Type::Error)}), &Type::Bool),
            "then": [{"k": "return", "results": rs, "pos": lines.pos()}],
            "pos": lines.pos(),
        }));
    }
    out
}

/// An `if op == 0 { … } else if op == 1 { … } …` chain over `arms`, built from the last arm back.
fn chain(op: &Value, arms: Vec<(usize, Vec<Value>)>, lines: &mut Lines) -> Value {
    let mut chain: Option<Value> = None;
    for (j, then) in arms.into_iter().rev() {
        let mut stmt = json!({
            "k": "if",
            "cond": binop("==", op.clone(), int_lit(j as i64), &Type::Bool),
            "then": then,
            "pos": lines.pos(),
        });
        if let Some(rest) = chain.take() {
            stmt["else"] = json!([rest]);
        }
        chain = Some(stmt);
    }
    chain.expect("an arm")
}

/// Insertion sort of the slice variable `v` of element type `elem`, ascending.
fn insertion_sort(v: &str, elem: &Type, lines: &mut Lines) -> Value {
    let int = platform_int();
    let st = Type::Slice {
        elem: Box::new(elem.clone()),
    };
    let at = |i: Value| json!({"k": "index", "x": var(v, &st), "index": i, "t": ty(elem)});
    let si = || var("si", &int);
    let sj = || var("sj", &int);
    let sj1 = || binop("+", sj(), int_lit(1), &int);
    json!({
        "k": "for",
        "init": {"k": "let", "names": ["si"], "rhs": [int_lit(1)], "pos": lines.pos()},
        "cond": binop("<", si(), json!({"k": "intrinsic", "name": "len", "args": [var(v, &st)], "t": ty(&int)}), &Type::Bool),
        "post": {"k": "incr", "x": si(), "pos": lines.pos()},
        "body": [
            {"k": "let", "names": ["sx"], "rhs": [at(si())], "pos": lines.pos()},
            {"k": "let", "names": ["sj"], "rhs": [binop("-", si(), int_lit(1), &int)], "pos": lines.pos()},
            {
                "k": "for",
                "cond": binop("&&", binop(">=", sj(), int_lit(0), &Type::Bool), binop(">", at(sj()), var("sx", elem), &Type::Bool), &Type::Bool),
                "body": [
                    {"k": "assign", "lhs": [at(sj1())], "rhs": [at(sj())], "pos": lines.pos()},
                    {"k": "decr", "x": sj(), "pos": lines.pos()},
                ],
                "pos": lines.pos(),
            },
            {"k": "assign", "lhs": [at(sj1())], "rhs": [var("sx", elem)], "pos": lines.pos()},
        ],
        "pos": lines.pos(),
    })
}

/// How a probe gets the receiver of a method.
enum Recv<'a> {
    /// A top-level function: none.
    None,
    /// `recv := <ctor>()` of type `rt`, or `recv := &T{…}` when the constructor's whole body
    /// returns that literal (`inline`), then setup slots over `muts`.
    Ctor {
        ctor: String,
        rt: Type,
        inline: Option<Value>,
        muts: Vec<&'a Func>,
    },
    /// `pick<Enum>` of a drawn `int`.
    Enum(Type),
    /// A struct literal of drawn fields.
    Literal(String, &'a [codespec_model::Field]),
}

/// How one result of the probed function is returned.
enum Out<'a> {
    /// As it is; `true` when it is sorted first.
    Plain(Type, bool),
    /// Flattened into these fields.
    Fields(Type, Vec<(&'a str, Type)>),
}

/// The `&T{…}` a pointer constructor's whole body returns: a probe binds it as the constructor's
/// value, the form in which the Rust writer owns a pointer to a type `shared` does not list (a
/// local bound to a call returning one is refused).
fn returned_literal(ctor: &Func) -> Option<Value> {
    if !matches!(ctor.results.as_slice(), [Type::Ptr { .. }]) {
        return None;
    }
    let body = serde_json::to_value(&ctor.body).expect("a body serializes");
    let [ret] = body.as_array()?.as_slice() else {
        return None;
    };
    let [e] = ret.get("results")?.as_array()?.as_slice() else {
        return None;
    };
    (ret["k"] == "return" && e["k"] == "unop" && e["op"] == "&" && e["x"]["k"] == "struct")
        .then(|| e.clone())
}

/// A probe: the function JSON, its setup function's JSON when a setup call can return an error,
/// and the entry.
struct Built {
    func: Value,
    setup: Option<Value>,
    entry: Entry,
}

/// The probe of `f` named `name`, its setup function named `setup_name`, or why there is none.
fn probe<'a>(
    f: &'a Func,
    name: &str,
    setup_name: &str,
    s: &Scope<'a>,
    ctors: &BTreeMap<String, String>,
    lines: &mut Lines,
    picks: &mut BTreeSet<String>,
) -> Result<Built, String> {
    if !f.tparams.is_empty() {
        return Err("it has type parameters".into());
    }
    for p in &f.params {
        if s.draw(&p.t).is_none() {
            return Err(format!("parameter {} cannot be drawn", p.name));
        }
    }
    let recv = match &f.recv {
        None => Recv::None,
        Some(r) => {
            let t = receiver_type_name(&r.t);
            let named = Type::Named {
                name: t.to_string(),
                targs: Vec::new(),
            };
            if let Some(ctor) = ctors.get(t) {
                let ctor_f =
                    s.m.funcs
                        .iter()
                        .find(|g| g.recv.is_none() && &g.name == ctor)
                        .expect("the constructor is in the module");
                Recv::Ctor {
                    ctor: ctor.clone(),
                    rt: ctor_f.results[0].clone(),
                    inline: returned_literal(ctor_f),
                    muts: s.mutators(t),
                }
            } else if s.enum_variants(&named).is_some() {
                Recv::Enum(named)
            } else if let Some(fields) = s.struct_fields(t) {
                if let Some(fd) = fields.iter().find(|fd| s.draw(&fd.t).is_none()) {
                    return Err(format!(
                        "receiver {t} has no constructor and its field {} cannot be drawn",
                        fd.name
                    ));
                }
                Recv::Literal(t.to_string(), fields)
            } else {
                return Err(format!(
                    "receiver {t} has no constructor and is neither an enum nor a struct"
                ));
            }
        }
    };
    let setup_fallible = match &recv {
        Recv::Ctor { muts, .. } => muts.iter().any(|g| s.is_fallible(g)),
        _ => false,
    };
    let own_err = s.is_fallible(f);
    let values: &[Type] = if own_err {
        &f.results[..f.results.len() - 1]
    } else {
        &f.results
    };
    let mut outs = Vec::new();
    for (i, t) in values.iter().enumerate() {
        if s.printable(t) {
            let sort = matches!(t, Type::Slice { elem } if matches!(**elem, Type::Int { .. } | Type::String))
                && s.iterates_a_map(f);
            outs.push(Out::Plain(t.clone(), sort));
        } else if let Some(fields) = s.flat_fields(t) {
            outs.push(Out::Fields(t.clone(), fields));
        } else {
            return Err(format!(
                "result {} is neither printable nor a struct of printable fields",
                i + 1
            ));
        }
    }
    // A method without a value to return shows what it did through its receiver's state after
    // the call: the receiver's fields when all are printable, else its readers' results.
    let mut state: Vec<(Value, Type)> = Vec::new();
    let mut copies: Vec<(String, Value, Type)> = Vec::new();
    if values.is_empty()
        && let Some(r) = &f.recv
    {
        let Recv::Ctor { rt, .. } = &recv else {
            return Err(
                "it returns no value, and only a receiver its constructor makes is read after \
                 the call"
                    .into(),
            );
        };
        let t = receiver_type_name(&r.t);
        let named = Type::Named {
            name: t.to_string(),
            targs: Vec::new(),
        };
        if let Some(fields) = s.flat_fields(&named) {
            for (fname, ft) in fields {
                let field =
                    json!({"k": "field", "x": var("recv", rt), "name": fname, "t": ty(&ft)});
                if matches!(ft, Type::Slice { .. }) {
                    // Copied element by element: the Rust writer refuses a copy of a slice,
                    // which shares its array with the original in Go.
                    let v = format!("st{}", copies.len());
                    copies.push((v.clone(), field, ft.clone()));
                    state.push((var(&v, &ft), ft));
                } else {
                    state.push((field, ft));
                }
            }
        } else {
            for g in s.readers(t) {
                let rt1 = g.results[0].clone();
                state.push((
                    json!({"k": "method", "x": var("recv", rt), "name": g.name, "args": [], "t": ty(&rt1)}),
                    rt1,
                ));
            }
        }
        if state.is_empty() {
            return Err(format!(
                "it returns no value, and {t} has neither printable fields nor a method that \
                 reads them"
            ));
        }
    }
    let fallible = own_err || setup_fallible;
    let mut out_types: Vec<Type> = Vec::new();
    for o in &outs {
        match o {
            Out::Plain(t, _) => out_types.push(t.clone()),
            Out::Fields(_, fields) => out_types.extend(fields.iter().map(|(_, t)| t.clone())),
        }
    }
    out_types.extend(state.iter().map(|(_, t)| t.clone()));
    if fallible {
        out_types.push(Type::Error);
    }

    let mut pr = Probe {
        s,
        params: Vec::new(),
        lines,
        picks,
    };
    let mut body: Vec<Value> = Vec::new();
    let mut setup_body: Vec<Value> = Vec::new();
    let mut slots = Vec::new();
    let x = match &recv {
        Recv::None => None,
        Recv::Ctor {
            ctor,
            rt,
            inline,
            muts,
        } => {
            let value = inline
                .clone()
                .unwrap_or_else(|| json!({"k": "call", "name": ctor, "args": [], "t": ty(rt)}));
            let bind =
                json!({"k": "let", "names": ["recv"], "rhs": [value], "pos": pr.lines.pos()});
            body.push(bind.clone());
            setup_body.push(bind);
            for _ in 0..SLOTS {
                if muts.is_empty() {
                    break;
                }
                let start = pr.params.len();
                let o = pr.op(muts.len());
                let mut arms = Vec::new();
                let mut setup_arms = Vec::new();
                for (j, g) in muts.iter().enumerate() {
                    let args: Vec<Value> = g.params.iter().map(|p| pr.arg("a", &p.t)).collect();
                    let call = json!({"k": "method", "x": var("recv", rt), "name": g.name, "args": args, "t": ty(&result_type(&g.results))});
                    let (then, then_setup) = if s.is_fallible(g) {
                        let mut names: Vec<String> =
                            (1..g.results.len()).map(|_| "_".to_string()).collect();
                        names.push("err".into());
                        (
                            bind_checked(names.clone(), call.clone(), true, &out_types, pr.lines),
                            bind_checked(names, call, true, &[Type::Error], pr.lines),
                        )
                    } else {
                        let stmt = json!({"k": "expr", "x": call, "pos": pr.lines.pos()});
                        (vec![stmt.clone()], vec![stmt])
                    };
                    arms.push((j, then));
                    setup_arms.push((j, then_setup));
                }
                body.push(chain(&o, arms, pr.lines));
                setup_body.push(chain(&o, setup_arms, pr.lines));
                slots.push(start..pr.params.len());
            }
            Some(var("recv", rt))
        }
        Recv::Enum(t) => Some(pr.arg("r", t)),
        Recv::Literal(t, fields) => {
            let values: Vec<Value> = fields
                .iter()
                .map(|fd| json!({"name": fd.name, "value": pr.arg("r", &fd.t)}))
                .collect();
            Some(
                json!({"k": "struct", "name": t, "fields": values, "t": ty(&Type::Named { name: t.clone(), targs: Vec::new() })}),
            )
        }
    };
    let args: Vec<Value> = f.params.iter().map(|p| pr.arg("p", &p.t)).collect();
    let rt = ty(&result_type(&f.results));
    let call = match x {
        Some(x) => json!({"k": "method", "x": x, "name": f.name, "args": args, "t": rt}),
        None => json!({"k": "call", "name": f.name, "args": args, "t": rt}),
    };
    let mut names: Vec<String> = (0..values.len()).map(|i| format!("res{i}")).collect();
    if own_err {
        names.push("err".into());
    }
    if names.is_empty() {
        body.push(json!({"k": "expr", "x": call, "pos": pr.lines.pos()}));
    } else {
        body.extend(bind_checked(names, call, own_err, &out_types, pr.lines));
    }
    let mut results = Vec::new();
    for (i, o) in outs.iter().enumerate() {
        let v = format!("res{i}");
        match o {
            Out::Plain(t, sort) => {
                if *sort && let Type::Slice { elem } = t {
                    body.push(insertion_sort(&v, elem, pr.lines));
                }
                results.push(var(&v, t));
            }
            Out::Fields(t, fields) => {
                for (fname, ft) in fields {
                    results
                        .push(json!({"k": "field", "x": var(&v, t), "name": fname, "t": ty(ft)}));
                }
            }
        }
    }
    for (v, over, st) in copies {
        let Type::Slice { elem } = &st else {
            unreachable!("only a slice is copied")
        };
        body.push(json!({"k": "let", "names": [v], "rhs": [{"k": "make", "t": ty(&st)}], "pos": pr.lines.pos()}));
        body.push(json!({
            "k": "for_range",
            "key": "_",
            "val": "sv",
            "over": over,
            "body": [{
                "k": "assign",
                "lhs": [var(&v, &st)],
                "rhs": [{"k": "intrinsic", "name": "append", "args": [var(&v, &st), var("sv", elem)], "t": ty(&st)}],
                "pos": pr.lines.pos(),
            }],
            "pos": pr.lines.pos(),
        }));
    }
    results.extend(state.into_iter().map(|(e, _)| e));
    if fallible {
        results.push(json!({"k": "nil", "t": ty(&Type::Error)}));
    }
    body.push(json!({"k": "return", "results": results, "pos": pr.lines.pos()}));
    let params: Vec<Value> = pr
        .params
        .iter()
        .map(|(n, t, _)| json!({"name": n, "t": ty(t)}))
        .collect();
    let func = json!({
        "name": name,
        "exported": true,
        "params": params,
        "results": out_types.iter().map(ty).collect::<Vec<_>>(),
        "body": body,
        "doc": format!("{name} calls {} with drawn values.", f.key()),
        "pos": pr.lines.pos(),
    });
    let setup = setup_fallible.then(|| {
        setup_body.push(json!({"k": "return", "results": [{"k": "nil", "t": ty(&Type::Error)}], "pos": pr.lines.pos()}));
        json!({
            "name": setup_name,
            "exported": true,
            "params": params,
            "results": [ty(&Type::Error)],
            "body": setup_body,
            "doc": format!("{setup_name} makes the setup calls of {name} alone."),
            "pos": pr.lines.pos(),
        })
    });
    let entry = Entry {
        func: f.key(),
        call: name.to_string(),
        receiver: match &recv {
            Recv::Ctor { ctor, .. } => Some(ctor.clone()),
            _ => None,
        },
        params: pr.params.into_iter().map(|(_, _, d)| d).collect(),
        slots,
        setup: setup.as_ref().map(|_| setup_name.to_string()),
    };
    Ok(Built { func, setup, entry })
}

fn pick_func(e: &str, variants: &[&str], lines: &mut Lines) -> Value {
    let int = platform_int();
    let et = Type::Named {
        name: e.to_string(),
        targs: Vec::new(),
    };
    let konst = |v: &str| json!({"k": "const", "name": v, "t": ty(&et)});
    let (last, rest) = variants.split_last().expect("an enum has a variant");
    let mut body = Vec::new();
    for (i, v) in rest.iter().enumerate() {
        body.push(json!({
            "k": "if",
            "cond": binop("==", var("i", &int), int_lit(i as i64), &Type::Bool),
            "then": [{"k": "return", "results": [konst(v)], "pos": lines.pos()}],
            "pos": lines.pos(),
        }));
    }
    body.push(json!({"k": "return", "results": [konst(last)], "pos": lines.pos()}));
    json!({
        "name": format!("pick{e}"),
        "exported": false,
        "params": [{"name": "i", "t": ty(&int)}],
        "results": [ty(&et)],
        "body": body,
        "pos": lines.pos(),
    })
}

/// The entries of every exported function of `m` (module doc).
pub fn plan(m: &Module) -> Result<Plan, String> {
    let m = codespec_passes::run(m);
    let s = Scope::new(&m);
    let mut ctors: BTreeMap<String, String> = BTreeMap::new();
    let mut constructors = Vec::new();
    for f in &m.funcs {
        if let Some(t) = is_constructor(f, &s) {
            constructors.push(f.key());
            ctors.entry(t).or_insert_with(|| f.name.clone());
        }
    }
    let taken: BTreeSet<String> = m.funcs.iter().map(|f| f.name.clone()).collect();
    let mut lines = Lines(0);
    let mut picks = BTreeSet::new();
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    let mut added: Vec<Value> = Vec::new();
    let mut setups: Vec<Value> = Vec::new();
    for f in m.funcs.iter().filter(|f| f.exported) {
        if constructors.contains(&f.key()) {
            continue;
        }
        let plain = f.recv.is_none()
            && f.tparams.is_empty()
            && f.params
                .iter()
                .all(|p| matches!(s.draw(&p.t), Some(Draw::Int(_) | Draw::Bool | Draw::Str)))
            && f.results
                .iter()
                .enumerate()
                .all(|(i, t)| s.printable(t) || (*t == Type::Error && i + 1 == f.results.len()));
        if plain {
            entries.push(Entry {
                func: f.key(),
                call: f.name.clone(),
                receiver: None,
                params: f.params.iter().filter_map(|p| s.draw(&p.t)).collect(),
                slots: Vec::new(),
                setup: None,
            });
            continue;
        }
        let mut name = format!("Probe{}", f.key().replace('.', ""));
        while taken.contains(&name) {
            name.push('_');
        }
        let mut setup_name = format!("Setup{name}");
        while taken.contains(&setup_name) {
            setup_name.push('_');
        }
        match probe(f, &name, &setup_name, &s, &ctors, &mut lines, &mut picks) {
            Ok(b) => {
                added.push(b.func);
                setups.extend(b.setup);
                entries.push(b.entry);
            }
            Err(why) => skipped.push((f.key(), why)),
        }
    }
    let mut helpers = Vec::new();
    for e in &picks {
        let (_, variants) = s
            .enum_variants(&Type::Named {
                name: e.clone(),
                targs: Vec::new(),
            })
            .expect("an enum");
        let name = format!("pick{e}");
        if taken.contains(&name) {
            return Err(format!("the module already has a function {name}"));
        }
        helpers.push(pick_func(e, &variants, &mut lines));
    }
    let mut doc: Value = serde_json::from_str(&m.to_json()).expect("a module is JSON");
    let add = |doc: &mut Value, funcs: Vec<Value>| {
        for f in funcs {
            let key = format!("func:{}", f["name"].as_str().expect("a name"));
            doc["funcs"].as_array_mut().expect("funcs").push(f);
            doc["layout"]
                .as_array_mut()
                .expect("layout")
                .push(json!(key));
        }
    };
    add(&mut doc, helpers.into_iter().chain(added).collect());
    let probed = Module::from_json(doc.to_string().as_bytes())
        .map_err(|e| format!("the probe module does not read: {e}"))?;
    add(&mut doc, setups);
    let setup = Module::from_json(doc.to_string().as_bytes())
        .map_err(|e| format!("the setup module does not read: {e}"))?;
    Ok(Plan {
        module: codespec_passes::run(&probed),
        setup,
        entries,
        constructors,
        skipped,
    })
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn draw_int(r: &mut SplitMix64, bits: u32) -> i64 {
    let max = if bits == 64 {
        i64::MAX
    } else {
        (1i64 << (bits - 1)) - 1
    };
    let min = -max - 1;
    let mut edges = vec![0, 1, -1, max, min, max - 1, min + 1];
    if bits == 64 {
        edges.extend([2_147_483_647, 46_337, 1 << 32]);
    }
    match r.next_u64() % 8 {
        0 | 1 => edges[(r.next_u64() % edges.len() as u64) as usize],
        2 => ((r.next_u64() % 2000) as i64 - 1000).clamp(min, max),
        _ => ((r.next_u64() % 24) as i64 - 3).clamp(min, max),
    }
}

fn pick<'s>(r: &mut SplitMix64, xs: &[&'s str]) -> &'s str {
    xs[(r.next_u64() % xs.len() as u64) as usize]
}

/// One drawn value; `key` is the seed's key string (module doc § Calls).
fn draw(r: &mut SplitMix64, d: &Draw, key: &str) -> String {
    match d {
        Draw::Int(bits) => draw_int(r, *bits).to_string(),
        Draw::SetupInt(_) if !r.next_u64().is_multiple_of(16) => {
            (1 + r.next_u64() % 20).to_string()
        }
        Draw::SetupInt(bits) => draw_int(r, *bits).to_string(),
        Draw::Bool => r.next_u64().is_multiple_of(2).to_string(),
        Draw::Str if r.next_u64().is_multiple_of(2) => format!("\"{key}\""),
        Draw::Str => format!("\"{}\"", pick(r, STRINGS)),
        Draw::SetupStr if !r.next_u64().is_multiple_of(4) => format!("\"{key}\""),
        Draw::SetupStr => format!("\"{}\"", pick(r, SETUP_STRINGS)),
        Draw::Pick(n) | Draw::Op(n) => (r.next_u64() % *n as u64).to_string(),
    }
}

/// The number of mutators slot `s` of `e` chooses from: its "none" value.
fn none(e: &Entry, s: &std::ops::Range<usize>) -> u64 {
    match e.params[s.start] {
        Draw::Op(n) => n as u64,
        _ => unreachable!("a slot starts with its operation"),
    }
}

/// Whether each of `argss`, passed to `func` of `m`, returns without an error: one evaluator run
/// for them all, again past a call that stops.
fn ok_each(m: &Module, func: &str, argss: &[Vec<String>]) -> Vec<bool> {
    let mut ok: Vec<bool> = Vec::with_capacity(argss.len());
    while ok.len() < argss.len() {
        let calls = Calls {
            format: CALLS_FORMAT.into(),
            calls: argss[ok.len()..]
                .iter()
                .map(|a| Call {
                    func: func.to_string(),
                    args: a.clone(),
                })
                .collect(),
        };
        let run = run_calls(m, &calls, &Options::default()).expect("setup calls fit");
        let before = ok.len();
        ok.extend(run.stdout.lines().map(|l| l == "ok"));
        if run.stop.is_some() {
            ok.push(false);
        }
        assert!(ok.len() > before, "a run of setup calls printed nothing");
    }
    ok
}

/// `seeds` calls per entry, seeds `1..=seeds`, in entry order (module doc § Calls).
pub fn calls(plan: &Plan, seeds: u64) -> Calls {
    let mut calls = Vec::new();
    for e in &plan.entries {
        let mut rngs = Vec::new();
        let mut keys = Vec::new();
        let mut argss = Vec::new();
        for seed in 1..=seeds {
            let mut r = SplitMix64(fnv(&e.call) ^ seed);
            let key = pick(&mut r, SETUP_STRINGS);
            let mut args: Vec<String> = e.params.iter().map(|d| draw(&mut r, d, key)).collect();
            // The first `active` slots run a mutator, the rest none, `active` the larger of two
            // draws; every other seed runs them in the mutators' declaration order.
            let slots = e.slots.len() as u64 + 1;
            let active = (r.next_u64() % slots).max(r.next_u64() % slots) as usize;
            let sorted = r.next_u64().is_multiple_of(2);
            let mut ops: Vec<u64> = e.slots[..active]
                .iter()
                .map(|s| args[s.start].parse().expect("an op is a number"))
                .collect();
            if sorted {
                ops.sort_unstable();
            }
            for (k, s) in e.slots.iter().enumerate() {
                args[s.start] = ops.get(k).copied().unwrap_or(none(e, s)).to_string();
            }
            rngs.push(r);
            keys.push(key);
            argss.push(args);
        }
        if let Some(setup) = &e.setup {
            // Slot by slot, for every seed at once: a slot whose call returns an error, the slots
            // before it as chosen and those after it empty, is drawn again, at most `REDRAWS`
            // times, then left empty.
            for (k, s) in e.slots.iter().enumerate() {
                let empty = none(e, s).to_string();
                let mut pending: Vec<usize> = (0..argss.len())
                    .filter(|&i| argss[i][s.start] != empty)
                    .collect();
                for round in 0..=REDRAWS {
                    if pending.is_empty() {
                        break;
                    }
                    let tried: Vec<Vec<String>> = pending
                        .iter()
                        .map(|&i| {
                            let mut a = argss[i].clone();
                            for later in &e.slots[k + 1..] {
                                a[later.start] = none(e, later).to_string();
                            }
                            a
                        })
                        .collect();
                    let ok = ok_each(&plan.setup, setup, &tried);
                    pending = pending
                        .into_iter()
                        .zip(ok)
                        .filter(|(_, ok)| !ok)
                        .map(|(i, _)| i)
                        .collect();
                    for &i in &pending {
                        if round < REDRAWS {
                            for p in s.clone() {
                                argss[i][p] = draw(&mut rngs[i], &e.params[p], keys[i]);
                            }
                        } else {
                            argss[i][s.start] = empty.clone();
                        }
                    }
                }
            }
        }
        calls.extend(argss.into_iter().map(|args| Call {
            func: e.call.clone(),
            args,
        }));
    }
    Calls {
        format: CALLS_FORMAT.into(),
        calls,
    }
}

/// How an entry's seeds fared in one run of its calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    /// The function's key.
    pub func: String,
    pub seeds: usize,
    /// Seeds whose setup calls all returned without an error, so that the function was called.
    pub reached: usize,
    /// Of those, the seeds whose line is a result (`ok …`), not an error.
    pub ok: usize,
}

/// Per entry, how many of its calls in `calls` reached the function and how many printed a
/// result, `out` being the evaluator's output for `calls`, one line per call. Which calls
/// reached the function, the evaluator tells by running their setup calls alone
/// ([`Entry::setup`]); an entry without one reaches it on every call.
pub fn reach(plan: &Plan, calls: &Calls, out: &str) -> Vec<Reach> {
    let lines: Vec<&str> = out.lines().collect();
    plan.entries
        .iter()
        .map(|e| {
            let mine: Vec<usize> = (0..calls.calls.len())
                .filter(|&i| calls.calls[i].func == e.call)
                .collect();
            let reached = match &e.setup {
                Some(setup) => {
                    let argss: Vec<Vec<String>> =
                        mine.iter().map(|&i| calls.calls[i].args.clone()).collect();
                    ok_each(&plan.setup, setup, &argss)
                }
                None => vec![true; mine.len()],
            };
            Reach {
                func: e.func.clone(),
                seeds: mine.len(),
                reached: reached.iter().filter(|r| **r).count(),
                ok: mine
                    .iter()
                    .zip(&reached)
                    .filter(|&(&i, &r)| r && lines.get(i).is_some_and(|l| l.starts_with("ok")))
                    .count(),
            }
        })
        .collect()
}

/// The calls file's canonical text: two-space indentation, a final newline.
pub fn calls_json(c: &Calls) -> String {
    let mut s = serde_json::to_string_pretty(c).expect("calls serialize");
    s.push('\n');
    s
}

/// The call at 0-based index `i`, named by entry and seed.
fn which(plan: &Plan, calls: &Calls, i: usize) -> String {
    let Some(c) = calls.calls.get(i) else {
        return "past the last call".into();
    };
    let first = calls
        .calls
        .iter()
        .position(|x| x.func == c.func)
        .unwrap_or(i);
    let func = plan
        .entries
        .iter()
        .find(|e| e.call == c.func)
        .map_or(c.func.as_str(), |e| e.func.as_str());
    format!(
        "call {}: {func} through {} seed {} ({})",
        i + 1,
        c.func,
        i - first + 1,
        c.args.join(" ")
    )
}

/// Every output equal to the first, or the first difference (module doc § Comparison). Each
/// output is `(lang, text)`.
pub fn compare(plan: &Plan, calls: &Calls, outputs: &[(&str, &str)]) -> Result<(), String> {
    let Some(((base, reference), rest)) = outputs.split_first() else {
        return Ok(());
    };
    let aligned = reference.lines().count() == calls.calls.len();
    for (lang, text) in rest {
        if text == reference {
            continue;
        }
        let mut a = reference.lines();
        let mut b = text.lines();
        let mut n = 0;
        loop {
            n += 1;
            let (x, y) = (a.next(), b.next());
            if x != y || (x.is_none() && y.is_none()) {
                let call = if aligned {
                    which(plan, calls, n - 1)
                } else {
                    "the lines are not one per call".into()
                };
                return Err(format!(
                    "{lang} differs from {base} at line {n}: {base} {:?}, {lang} {:?}; {call}",
                    x.unwrap_or("<end>"),
                    y.unwrap_or("<end>")
                ));
            }
        }
    }
    Ok(())
}
