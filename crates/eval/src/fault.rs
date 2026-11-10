//! Fault plans (`docs/evaluator.md` § 6.2): a fault answers in place of the effect at one site and
//! call count, as LFI injects at a library's interface, and the session's trace
//! (`session` module doc) says where it fired, what its call had written before it and what
//! became of it.
//!
//! # The plan
//!
//! ```json
//! {"format": "codespec-faults/1", "faults": [
//!   {"at": "orders.Service.Pay#3", "nth": 1, "do": "closed"},
//!   {"at": "envcheck.Mode#0.rhs.0", "nth": "*", "do": "error", "message": "no space left on device"}
//! ]}
//! ```
//!
//! On the command line one fault is `<at>:<nth>=<do>`, a message or the values after the kind
//! as JSON separated by white space: `orders.Service.Pay#3:1=closed`, `x.F#0.rhs.0:*=error "disk
//! full"`, `x.G#0.rhs.0:2=value 1 null`.
//!
//! `at` is the node id of an effect site ([`crate::id`]): an `extern` or `extern_method` call,
//! or a `send` statement. `nth` is the call count of that site in the run at which the fault
//! answers, from 1, or `*` for every call. `do` is one of:
//!
//! | `do` | at | the site |
//! |---|---|---|
//! | `error` | an extern call whose last result is `error` | returns that error, `message` (default `injected fault`), the other results zero |
//! | `value` | an extern call | returns `values`, one JSON value per result as a stub's |
//! | `closed` | a send | panics `send on closed channel`, as Go's send on a closed channel |
//! | `block` | either | never returns: the run stops `blocked`, `no receiver` at a send, `the call does not return` at a call |
//! | `panic` | either | panics with `message` (default `injected panic`) |
//!
//! A fault answers before a stub or a handler would. Refused before anything runs: another
//! format, an unknown field, an object giving one key twice, an `nth` below 1, a `do` the site
//! cannot take, values its call's results cannot hold, a site that is no effect site of the
//! module, a second fault for one site, a stub each of whose sites a fault of every call answers
//! (it answers no call), and a plan given to a run of a calls file, which writes no trace to say
//! what it did.
//!
//! # What the trace says
//!
//! When a fault answers, the trace has a `fault` event. When the session's call it answered in
//! ends (it returns, panics, or the run stops), an `update` event for each place of memory older
//! than the call (a field, element or map entry reached through a pointer, a map or a bound
//! receiver) that the call left holding other than before, with what it holds then and the
//! statement that wrote it last: the partial update a caller that recovers sees. A write the call
//! undid is none; a pointer the call stored shows what it points to at the end.
//!
//! An `error` fault's error carries the fault it came from, and so does what holds it: an
//! `errorf` that wraps it, a value of an error type with it in a field, a struct, array, slice or
//! map with it inside, a pointer to one. The trace follows it, in later calls too:
//!
//! | event | the error |
//! |---|---|
//! | `tested` | is, or an error wrapping it is, an operand of `==` or `!=`, a `switch`'s tag or one of its case values |
//! | `propagated` | is held by a result of a `return` |
//! | `wrapped` | is, or an error wrapping it is, an argument of `errorf` or `sprintf` |
//! | `dropped` | is, or an error wrapping it is, bound to `_` or a result of a call a statement discards |
//!
//! Then each fault that answered in the call ends in one `summary` event, its `outcome`: `panic,
//! not handled in module` (`closed`, `panic`: `cir/1` has no `recover`), `blocked`, `answered`
//! (`value`), and for `error`, the first that holds of: `propagated to <entry>` when the call
//! returns it to the session; `kept at <node>` when the call left it in older memory, `node` the
//! statement that wrote it there; `handled at <node>` or `dropped at <node>` at its first test or
//! drop; `live at stop` when the run stopped first; else `dropped at <site>`, it went out of scope
//! unread. A fault the run never reaches gets a summary `not reached` at the end of the trace,
//! before a `stop`.
//!
//! # Each site
//!
//! [`run_each`] runs a session once per site of a list and call count from 1 to a most, each
//! site given the fault Go defines for it: `closed` at a send, `error` at an extern call whose
//! last result is an error, `panic` at another call (`codespec eval --each`).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use codespec_model::{Expr, Func, Module, Stmt, Type, walk};

use crate::id::{Node, func_id, node_id, var_node_id};
use crate::interp::{Interp, R};
use crate::trace::{J, line};
use crate::value::{ErrorVal, HeapId, Place, Value};
use crate::{Options, Run, Session, StopKind};

/// The format of a fault plan.
pub const FAULTS_FORMAT: &str = "codespec-faults/1";

const DEFAULT_ERROR: &str = "injected fault";
const DEFAULT_PANIC: &str = "injected panic";

/// The call count of its site at which a fault answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nth {
    /// That call, counting from 1.
    At(u64),
    /// Every call.
    Every,
}

impl Nth {
    fn parse(s: &str) -> Result<Nth, String> {
        if s == "*" {
            return Ok(Nth::Every);
        }
        match s.parse::<u64>() {
            Ok(n) if n >= 1 => Ok(Nth::At(n)),
            _ => Err(format!("nth {s:?} is neither a call count from 1 nor *")),
        }
    }

    fn hits(self, call: u64) -> bool {
        match self {
            Nth::At(n) => n == call,
            Nth::Every => true,
        }
    }
}

/// What a fault does at its site (module doc).
#[derive(Debug, Clone, PartialEq)]
pub enum FaultDo {
    /// The call returns this message as its last result, an error.
    Error(String),
    /// The call returns these values, one per result.
    Value(Vec<serde_json::Value>),
    Closed,
    Block,
    /// The call or send panics with this message.
    Panic(String),
}

impl FaultDo {
    pub fn name(&self) -> &'static str {
        match self {
            FaultDo::Error(_) => "error",
            FaultDo::Value(_) => "value",
            FaultDo::Closed => "closed",
            FaultDo::Block => "block",
            FaultDo::Panic(_) => "panic",
        }
    }

    fn read(
        kind: &str,
        message: Option<String>,
        values: Option<Vec<serde_json::Value>>,
    ) -> Result<FaultDo, String> {
        let only = |what: &str, ok: bool| {
            if ok {
                Ok(())
            } else {
                Err(format!("do {kind} takes no {what}"))
            }
        };
        Ok(match kind {
            "error" | "panic" => {
                only("values", values.is_none())?;
                let default = if kind == "error" {
                    DEFAULT_ERROR
                } else {
                    DEFAULT_PANIC
                };
                let message = message.unwrap_or_else(|| default.to_string());
                if kind == "error" {
                    FaultDo::Error(message)
                } else {
                    FaultDo::Panic(message)
                }
            }
            "value" => {
                only("message", message.is_none())?;
                FaultDo::Value(values.unwrap_or_default())
            }
            "closed" | "block" => {
                only("message", message.is_none())?;
                only("values", values.is_none())?;
                if kind == "closed" {
                    FaultDo::Closed
                } else {
                    FaultDo::Block
                }
            }
            other => {
                return Err(format!(
                    "do {other:?} is none of error, value, closed, block, panic"
                ));
            }
        })
    }
}

/// One entry of a fault plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    /// The node id of the effect site.
    pub at: String,
    pub nth: Nth,
    pub act: FaultDo,
}

impl Fault {
    /// Reads the command line's `<at>:<nth>=<do>[ <JSON>…]` (module doc).
    pub fn parse(s: &str) -> Result<Fault, String> {
        let bad = |why: &str| format!("fault {s:?}: {why}");
        let (left, right) = s
            .split_once('=')
            .ok_or_else(|| bad("not <site>:<nth>=<do>"))?;
        let (at, nth) = left
            .rsplit_once(':')
            .ok_or_else(|| bad("not <site>:<nth>=<do>"))?;
        if at.is_empty() {
            return Err(bad("no site"));
        }
        let nth = Nth::parse(nth).map_err(|e| bad(&e))?;
        let right = right.trim();
        let (kind, rest) = right.split_once(char::is_whitespace).unwrap_or((right, ""));
        let args = serde_json::Deserializer::from_str(rest)
            .into_iter::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| bad(&e.to_string()))?;
        let (message, values) = match kind {
            "value" => (None, Some(args)),
            _ => match args.as_slice() {
                [] => (None, None),
                [serde_json::Value::String(m)] if kind == "error" || kind == "panic" => {
                    (Some(m.clone()), None)
                }
                _ if kind == "error" || kind == "panic" => {
                    return Err(bad("takes at most one message, a JSON string"));
                }
                _ => {
                    let why = FaultDo::read(kind, None, None)
                        .err()
                        .unwrap_or_else(|| format!("do {kind} takes nothing after it"));
                    return Err(bad(&why));
                }
            },
        };
        Ok(Fault {
            at: at.to_string(),
            nth,
            act: FaultDo::read(kind, message, values).map_err(|e| bad(&e))?,
        })
    }
}

/// Reads a `codespec-faults/1` plan.
pub fn read_faults(bytes: &[u8]) -> Result<Vec<Fault>, String> {
    use serde_json::Value as V;
    let doc = crate::trace::read_json(bytes)?;
    let V::Object(top) = doc else {
        return Err("a fault plan is a JSON object".into());
    };
    let format = top.get("format").and_then(V::as_str).unwrap_or("");
    if format != FAULTS_FORMAT {
        return Err(format!(
            "format is {format:?}, this reader reads {FAULTS_FORMAT:?}"
        ));
    }
    if let Some(k) = top.keys().find(|k| *k != "format" && *k != "faults") {
        return Err(format!("unknown field {k}"));
    }
    let Some(V::Array(faults)) = top.get("faults") else {
        return Err("no faults array".into());
    };
    if faults.is_empty() {
        return Err("no faults".into());
    }
    let mut out = Vec::with_capacity(faults.len());
    for (i, f) in faults.iter().enumerate() {
        let n = i + 1;
        let V::Object(f) = f else {
            return Err(format!("fault {n} is not an object"));
        };
        if let Some(k) = f
            .keys()
            .find(|k| !matches!(k.as_str(), "at" | "nth" | "do" | "message" | "values"))
        {
            return Err(format!("fault {n}: unknown field {k}"));
        }
        let at = match f.get("at") {
            Some(V::String(s)) if !s.is_empty() => s.clone(),
            _ => return Err(format!("fault {n}: at is not a non-empty string")),
        };
        let nth = match f.get("nth") {
            Some(V::String(s)) if s == "*" => Nth::Every,
            Some(V::Number(x)) => {
                Nth::parse(&x.to_string()).map_err(|e| format!("fault {n}: {e}"))?
            }
            _ => {
                return Err(format!(
                    "fault {n}: nth is neither a call count from 1 nor \"*\""
                ));
            }
        };
        let kind = f
            .get("do")
            .and_then(V::as_str)
            .ok_or_else(|| format!("fault {n}: do is not a string"))?;
        let message = match f.get("message") {
            None => None,
            Some(V::String(s)) => Some(s.clone()),
            Some(_) => return Err(format!("fault {n}: message is not a string")),
        };
        let values = match f.get("values") {
            None => None,
            Some(V::Array(xs)) => Some(xs.clone()),
            Some(_) => return Err(format!("fault {n}: values is not an array")),
        };
        let act = FaultDo::read(kind, message, values).map_err(|e| format!("fault {n}: {e}"))?;
        out.push(Fault { at, nth, act });
    }
    Ok(out)
}

/// Reads a sites file: one node id per line, blank lines skipped.
pub fn read_sites(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        if l.contains(char::is_whitespace) {
            return Err(format!("line {}: {l:?} is not one node id", i + 1));
        }
        out.push(l.to_string());
    }
    Ok(out)
}

/// One run of [`run_each`]: its site, call count, the file name its trace goes to, and the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EachRun {
    pub site: String,
    pub nth: u64,
    /// `<site>.nth<n>.eval-trace.jsonl`, each character of the site outside letters, digits,
    /// `.`, `_` and `-` written `-`.
    pub file: String,
    pub run: Run,
}

/// Runs `session` once per site of `sites` and call count from 1 to `max_nth`, each with the one
/// fault Go defines for its site (module doc § Each site). Refused before anything runs: no
/// site, a site given twice or that is no effect site of the module, two sites whose trace files
/// would be one (letter case ignored), `max_nth` 0, and faults in `opts`.
pub fn run_each(
    m: &Module,
    session: &Session,
    sites: &[String],
    max_nth: u64,
    opts: &Options,
) -> Result<Vec<EachRun>, String> {
    if max_nth == 0 {
        return Err("--max-nth is 0: a call count starts at 1".into());
    }
    if !opts.faults.is_empty() {
        return Err("--each makes its own fault per site and call count; give no other".into());
    }
    if sites.is_empty() {
        return Err("no site to fault".into());
    }
    let all = effect_sites(m);
    let mut planned = Vec::with_capacity(sites.len());
    let mut files: Vec<String> = Vec::new();
    for (i, s) in sites.iter().enumerate() {
        if sites[..i].contains(s) {
            return Err(format!("site {s}: given twice"));
        }
        let site = all.iter().find(|x| x.id == *s).ok_or_else(|| no_site(s))?;
        let act = match site.kind {
            Kind::Send => FaultDo::Closed,
            Kind::Extern { error: true } => FaultDo::Error(DEFAULT_ERROR.into()),
            Kind::Extern { error: false } => FaultDo::Panic(DEFAULT_PANIC.into()),
        };
        // Compared without case: on a file system that ignores it, two names differing only in
        // case are one file, and one trace would overwrite the other.
        let file = trace_file(s, 1).to_lowercase();
        if files.contains(&file) {
            return Err(format!(
                "site {s}: its trace file {} is another site's, letter case aside",
                trace_file(s, 1)
            ));
        }
        files.push(file);
        planned.push((s, act));
    }
    let mut out = Vec::with_capacity(planned.len());
    for (s, act) in planned {
        for nth in 1..=max_nth {
            let mut o = opts.clone();
            o.faults = vec![Fault {
                at: s.clone(),
                nth: Nth::At(nth),
                act: act.clone(),
            }];
            let run = crate::run_session(m, session, &o)?;
            out.push(EachRun {
                site: s.clone(),
                nth,
                file: trace_file(s, nth),
                run,
            });
        }
    }
    Ok(out)
}

fn trace_file(site: &str, nth: u64) -> String {
    let name: String = site
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{name}.nth{nth}.eval-trace.jsonl")
}

fn no_site(at: &str) -> String {
    format!(
        "{at}: no effect site of the module is {at}: a fault answers at an extern call or a send"
    )
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Send,
    /// An extern call; `error` when its last result is an error.
    Extern {
        error: bool,
    },
}

/// A site a fault may answer at: its node's address, id, position and kind, and for a call its
/// type.
struct EffectSite<'m> {
    addr: usize,
    id: String,
    pos: String,
    kind: Kind,
    t: &'m Type,
}

/// Every effect site of `m`: its extern calls, then its sends.
fn effect_sites(m: &Module) -> Vec<EffectSite<'_>> {
    let mut out = Vec::new();
    for (e, _, id, pos) in crate::host::extern_sites(m) {
        let error = match e.t() {
            Type::Error => true,
            Type::Tuple { elems } => elems.last() == Some(&Type::Error),
            _ => false,
        };
        out.push(EffectSite {
            addr: e as *const Expr as usize,
            id,
            pos,
            kind: Kind::Extern { error },
            t: e.t(),
        });
    }
    for f in &m.funcs {
        for s in sends(&f.body) {
            if let Some((id, pos)) = node_id(m, f, Node::Stmt(s)) {
                out.push(send_site(s, id, pos));
            }
        }
    }
    for v in &m.vars {
        if let Some(init) = &v.init {
            let mut bodies = Vec::new();
            walk::walk_expr(init, &mut |e| {
                if let Expr::Lambda { body, .. } = e {
                    bodies.push(body.as_slice());
                }
            });
            for body in bodies {
                let mut here = Vec::new();
                walk::walk_stmts(body, &mut |s| {
                    if matches!(s, Stmt::Send { .. }) {
                        here.push(s);
                    }
                });
                for s in here {
                    if let Some((id, pos)) = var_node_id(m, v, Node::Stmt(s)) {
                        out.push(send_site(s, id, pos));
                    }
                }
            }
        }
    }
    out
}

fn send_site(s: &Stmt, id: String, pos: String) -> EffectSite<'_> {
    EffectSite {
        addr: s as *const Stmt as usize,
        id,
        pos,
        kind: Kind::Send,
        t: &Type::Unit,
    }
}

/// The `send` statements of `body`, those of the function literals in it included.
fn sends(body: &[Stmt]) -> Vec<&Stmt> {
    let mut out = Vec::new();
    let mut bodies = vec![body];
    walk::body_exprs(body, &mut |e| {
        if let Expr::Lambda { body, .. } = e {
            bodies.push(body.as_slice());
        }
    });
    for b in bodies {
        walk::walk_stmts(b, &mut |s| {
            if matches!(s, Stmt::Send { .. }) {
                out.push(s);
            }
        });
    }
    out
}

/// A fault armed at its site.
struct Armed<'m> {
    id: String,
    pos: String,
    nth: Nth,
    act: FaultDo,
    send: bool,
    /// What a `value` fault returns.
    value: Option<Value<'m>>,
    calls: u64,
    fired: bool,
}

/// A fault that answered.
struct Fired {
    site: String,
    nth: u64,
    act: &'static str,
    /// Decided when it answered: every kind's but `error`'s.
    outcome: Option<String>,
    /// `handled at <node>` or `dropped at <node>`: the first test or drop of its error.
    first: Option<String>,
}

/// A place in memory older than the session's call that the call wrote: a value's place, or
/// the entry at a position of a map (entries keep their positions: a map only grows).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Loc {
    Place(Place),
    Entry(HeapId, usize),
}

/// What a place held before the call first wrote it (`None`: a map entry the call added), and
/// the statement that wrote it last.
struct Written<'m> {
    at: Loc,
    before: Option<Value<'m>>,
    stmt: usize,
}

/// The run's fault plan and what its faults did.
#[derive(Default)]
pub(crate) struct Faults<'m> {
    /// The armed faults by the address of their site's node, and those addresses in plan order.
    armed: HashMap<usize, Armed<'m>>,
    order: Vec<usize>,
    fired: Vec<Fired>,
    /// The faults that answered in the session's call running now.
    pending: Vec<usize>,
    /// The memory's size when the session's call began: a cell below it is older than the call.
    mark: Option<HeapId>,
    /// The places of older memory the call wrote, in the order of their first write.
    written: Vec<Written<'m>>,
    index: HashMap<Loc, usize>,
    /// The id and position of each statement that wrote, by its address.
    ids: HashMap<usize, (String, String)>,
}

impl Faults<'_> {
    /// Whether a write into `cell` is one into memory older than the session's call.
    pub(crate) fn older(&self, cell: HeapId) -> bool {
        self.mark.is_some_and(|m| cell < m)
    }

    /// Whether a fault of every call (`nth *`) is armed at the node at `addr`.
    pub(crate) fn every_call(&self, addr: usize) -> bool {
        self.armed.get(&addr).is_some_and(|a| a.nth == Nth::Every)
    }
}

/// Whether `v` is an error, or an interface holding one.
fn is_error(v: &Value<'_>) -> bool {
    match v {
        Value::Error(_) => true,
        Value::Iface(_, x) => is_error(x),
        _ => false,
    }
}

/// Whether `a` and `b` are the same contents of one place: equal values, the same slice header,
/// function or error.
fn same(a: &Value<'_>, b: &Value<'_>) -> bool {
    let all = |xs: &[Value<'_>], ys: &[Value<'_>]| {
        xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same(x, y))
    };
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (
            Value::Slice {
                arr: a1,
                off: o1,
                len: l1,
                cap: c1,
            },
            Value::Slice {
                arr: a2,
                off: o2,
                len: l2,
                cap: c2,
            },
        ) => (a1, o1, l1, c1) == (a2, o2, l2, c2),
        (Value::Func(f), Value::Func(g)) => Rc::ptr_eq(f, g),
        (Value::Opaque(x), Value::Opaque(y)) => x == y,
        (Value::Host(k, x), Value::Host(l, y)) => k == l && same(x, y),
        (Value::Struct(n, xs), Value::Struct(m, ys)) => n == m && all(xs, ys),
        (Value::Array(xs), Value::Array(ys)) | (Value::Tuple(xs), Value::Tuple(ys)) => all(xs, ys),
        (Value::Iface(t, x), Value::Iface(u, y)) => t == u && same(x, y),
        (Value::Error(x), Value::Error(y)) => {
            Rc::ptr_eq(x, y)
                || match (x.as_ref(), y.as_ref()) {
                    (ErrorVal::Sentinel { name: n, .. }, ErrorVal::Sentinel { name: m, .. }) => {
                        n == m
                    }
                    (ErrorVal::Value(t, v), ErrorVal::Value(u, w)) => t == u && same(v, w),
                    _ => false,
                }
        }
        (Value::MapData(_), _) | (Value::ChanData(..), _) => false,
        _ => crate::interp::equal(a, b),
    }
}

impl<'m> Interp<'m> {
    /// Arms `faults` at their sites, refusing what the module doc § The plan refuses.
    pub(crate) fn install_faults(&mut self, faults: &[Fault]) -> Result<(), String> {
        if faults.is_empty() {
            return Ok(());
        }
        let sites = effect_sites(self.m);
        for (i, f) in faults.iter().enumerate() {
            let at = &f.at;
            if faults[..i].iter().any(|g| g.at == *at) {
                return Err(format!("fault {at}: given twice; one site takes one fault"));
            }
            let site = sites
                .iter()
                .find(|s| s.id == *at)
                .ok_or_else(|| format!("fault {}", no_site(at)))?;
            let mut value = None;
            match (&f.act, site.kind) {
                (FaultDo::Closed, Kind::Extern { .. }) => {
                    return Err(format!(
                        "fault {at}: closed is a send's fault, and {at} is an extern call"
                    ));
                }
                (FaultDo::Error(_), Kind::Send) => {
                    return Err(format!(
                        "fault {at}: a send gives no error; error is a fault of a call returning one"
                    ));
                }
                (FaultDo::Error(_), Kind::Extern { error: false }) => {
                    return Err(format!(
                        "fault {at}: error: the call's last result is not an error"
                    ));
                }
                (FaultDo::Value(_), Kind::Send) => {
                    return Err(format!("fault {at}: a send gives no value"));
                }
                (FaultDo::Value(vs), Kind::Extern { .. }) => {
                    value = Some(
                        self.stub_value(vs, site.t)
                            .map_err(|why| format!("fault {at}: {why}"))?,
                    );
                }
                _ => {}
            }
            self.host.faults.order.push(site.addr);
            self.host.faults.armed.insert(
                site.addr,
                Armed {
                    id: site.id.clone(),
                    pos: site.pos.clone(),
                    nth: f.nth,
                    act: f.act.clone(),
                    send: matches!(site.kind, Kind::Send),
                    value,
                    calls: 0,
                    fired: false,
                },
            );
        }
        Ok(())
    }

    /// What a fault armed at the node at `addr`, of the type `t`, does at this call of it;
    /// `None` when none answers here. Counts the call.
    pub(crate) fn fault_at(
        &mut self,
        addr: usize,
        t: &Type,
        node: Node<'m>,
    ) -> Option<R<'m, Value<'m>>> {
        let faults = &mut self.host.faults;
        if faults.armed.is_empty() {
            return None;
        }
        let a = faults.armed.get_mut(&addr)?;
        a.calls += 1;
        if !a.nth.hits(a.calls) {
            return None;
        }
        a.fired = true;
        let (site, pos, nth, act, send, value) = (
            a.id.clone(),
            a.pos.clone(),
            a.calls,
            a.act.clone(),
            a.send,
            a.value.clone(),
        );
        let k = self.host.faults.fired.len();
        let shown: Vec<String> = match &value {
            None | Some(Value::Unit) => Vec::new(),
            Some(Value::Tuple(xs)) => xs.iter().map(|x| self.render(x)).collect(),
            Some(v) => vec![self.render(v)],
        };
        let mut fields = vec![
            ("event", J::Str("fault")),
            ("node", J::Str(&site)),
            ("pos", J::Str(&pos)),
            ("nth", J::Num(nth)),
            ("do", J::Str(act.name())),
        ];
        match &act {
            FaultDo::Error(m) | FaultDo::Panic(m) => fields.push(("message", J::Str(m))),
            FaultDo::Value(_) => fields.push(("results", J::List(&shown))),
            FaultDo::Closed | FaultDo::Block => {}
        }
        let fault_line = line(&fields);
        self.host.emit(fault_line);
        let outcome = match act {
            FaultDo::Closed | FaultDo::Panic(_) => Some("panic, not handled in module".into()),
            FaultDo::Block => Some("blocked".into()),
            FaultDo::Value(_) => Some("answered".into()),
            FaultDo::Error(_) => None,
        };
        self.host.faults.fired.push(Fired {
            site,
            nth,
            act: act.name(),
            outcome,
            first: None,
        });
        self.host.faults.pending.push(k);
        Some(match act {
            FaultDo::Closed => Err(self.panic(node, "send on closed channel")),
            FaultDo::Panic(m) => Err(self.panic(node, m)),
            FaultDo::Block => Err(self.halt(
                StopKind::Blocked,
                node,
                if send {
                    "no receiver"
                } else {
                    "the call does not return"
                },
            )),
            FaultDo::Value(_) => Ok(value.expect("install read a value fault's values")),
            FaultDo::Error(m) => self.fault_error(t, m, k, node),
        })
    }

    /// The results of a call of type `t` an `error` fault gives: the error last, from the fault
    /// `k`, the others zero.
    fn fault_error(
        &mut self,
        t: &Type,
        text: String,
        k: usize,
        node: Node<'m>,
    ) -> R<'m, Value<'m>> {
        let err = Value::Error(Rc::new(ErrorVal::Message {
            text,
            origin: Some(k),
        }));
        match self.resolve(t) {
            Type::Tuple { elems } => {
                let mut vs = Vec::with_capacity(elems.len());
                for e in &elems[..elems.len() - 1] {
                    vs.push(self.zero(e, node)?);
                }
                vs.push(err);
                Ok(Value::Tuple(vs))
            }
            _ => Ok(err),
        }
    }

    /// The faults whose errors `v` holds, in the order met: inside an error (a wrapping
    /// `errorf`, a field of an error type), and through struct fields, array, slice and map
    /// elements and what pointers point to. Not `deep`, only when `v` is itself an error.
    fn holds(&self, v: &Value<'m>, deep: bool) -> Vec<usize> {
        let mut out = Vec::new();
        if !deep && !is_error(v) {
            return out;
        }
        let mut places: HashSet<Place> = HashSet::new();
        let mut slices: HashSet<(HeapId, usize, usize)> = HashSet::new();
        let mut maps: HashSet<HeapId> = HashSet::new();
        let mut work = vec![v.clone()];
        while let Some(x) = work.pop() {
            match x {
                Value::Error(e) => match e.as_ref() {
                    ErrorVal::Message {
                        origin: Some(k), ..
                    } => {
                        if !out.contains(k) {
                            out.push(*k);
                        }
                    }
                    ErrorVal::Value(_, inner) => work.push(inner.clone()),
                    _ => {}
                },
                Value::Iface(_, b) | Value::Host(_, b) => work.push(*b),
                Value::Struct(_, xs) | Value::Array(xs) | Value::Tuple(xs) => {
                    work.extend(xs.into_iter().rev());
                }
                Value::Ptr(p) => {
                    if let Some(y) = self.read(&p).cloned()
                        && places.insert(p)
                    {
                        work.push(y);
                    }
                }
                Value::Slice { arr, off, len, .. } => {
                    if slices.insert((arr, off, len))
                        && let Some(xs) = self.elems(&x)
                    {
                        work.extend(xs.into_iter().rev());
                    }
                }
                Value::Map(id) if maps.insert(id) => {
                    for (k, v) in self.map_entries(id).iter().rev() {
                        work.push(v.clone());
                        work.push(k.clone());
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Writes the trace's `event` (`tested`, `propagated`, `wrapped`, `dropped`) at `node` for
    /// each fault whose error `v` holds (an error itself, but for `propagated`: anything that
    /// holds one), and gives the first of them.
    pub(crate) fn traced(&mut self, event: &str, v: &Value<'m>, node: Node<'m>) -> Option<usize> {
        if self.host.faults.fired.is_empty() {
            return None;
        }
        let ks = self.holds(v, event == "propagated");
        if ks.is_empty() {
            return None;
        }
        let (id, pos) = self.site(node);
        let verb = match event {
            "tested" => Some("handled"),
            "dropped" => Some("dropped"),
            _ => None,
        };
        for &k in &ks {
            let f = &mut self.host.faults.fired[k];
            if let Some(verb) = verb
                && f.first.is_none()
            {
                f.first = Some(format!("{verb} at {id}"));
            }
            let (site, nth) = (f.site.clone(), f.nth);
            self.host.emit(line(&[
                ("event", J::Str(event)),
                ("node", J::Str(&id)),
                ("pos", J::Str(&pos)),
                ("fault", J::Str(&site)),
                ("nth", J::Num(nth)),
            ]));
        }
        ks.first().copied()
    }

    /// Notes that the running statement writes the place `p` of memory older than the session's
    /// call, keeping what it held before the call's first write of it.
    pub(crate) fn record_place(&mut self, p: &Place) {
        let before = self.read(p).cloned();
        self.record(Loc::Place(p.clone()), before);
    }

    /// Notes that the running statement sets the entry for `k` of the older map in `id`.
    pub(crate) fn record_entry(&mut self, id: HeapId, k: &Value<'m>) {
        let es = self.map_entries(id);
        let (pos, before) = match es.iter().position(|(ek, _)| crate::interp::equal(ek, k)) {
            Some(i) => (i, Some(es[i].1.clone())),
            None => (es.len(), None),
        };
        self.record(Loc::Entry(id, pos), before);
    }

    fn record(&mut self, at: Loc, before: Option<Value<'m>>) {
        let Some(s) = self.cur else {
            return;
        };
        let stmt = s as *const Stmt as usize;
        if !self.host.faults.ids.contains_key(&stmt) {
            let id = self.site(Node::Stmt(s));
            self.host.faults.ids.insert(stmt, id);
        }
        let faults = &mut self.host.faults;
        match faults.index.get(&at) {
            Some(&i) => faults.written[i].stmt = stmt,
            None => {
                faults.index.insert(at.clone(), faults.written.len());
                faults.written.push(Written { at, before, stmt });
            }
        }
    }

    /// A session's call begins: its writes into older memory are noted from here.
    pub(crate) fn begin_call(&mut self) {
        if !self.host.faults.armed.is_empty() {
            self.host.faults.mark = Some(self.heap_len());
            self.host.faults.written.clear();
            self.host.faults.index.clear();
        }
    }

    /// A session's call ended: it returned `results` from `entry`, or `entry` is `None` (a panic
    /// or, `stopped`, a stop). When a fault answered in it: an `update` for each place of older
    /// memory the call left changed, as it is now, then one `summary` per fault.
    pub(crate) fn end_call(
        &mut self,
        entry: Option<&'m Func>,
        results: &[Value<'m>],
        stopped: bool,
    ) {
        let faults = &mut self.host.faults;
        faults.mark = None;
        faults.index.clear();
        let written = std::mem::take(&mut faults.written);
        let pending = std::mem::take(&mut faults.pending);
        if pending.is_empty() {
            return;
        }
        // What the call left: each place it wrote that now holds other than before.
        let mut left: Vec<(usize, Value<'m>)> = Vec::new();
        for w in &written {
            let now = match &w.at {
                Loc::Place(p) => self.read(p).cloned(),
                Loc::Entry(id, pos) => self.map_entries(*id).get(*pos).map(|(_, v)| v.clone()),
            };
            let changed = match (&w.before, &now) {
                (Some(a), Some(b)) => !same(a, b),
                (None, None) => false,
                _ => true,
            };
            if let (true, Some(now)) = (changed, now) {
                left.push((w.stmt, now));
            }
        }
        let updates = left.len() as u64;
        for (stmt, now) in &left {
            let value = self.render(now);
            let (node, pos) = &self.host.faults.ids[stmt];
            let l = line(&[
                ("event", J::Str("update")),
                ("node", J::Str(node)),
                ("pos", J::Str(pos)),
                ("value", J::Str(&value)),
            ]);
            self.host.emit(l);
        }
        let returned: Vec<usize> = results.iter().flat_map(|v| self.holds(v, true)).collect();
        let kept: Vec<Vec<usize>> = left.iter().map(|(_, v)| self.holds(v, true)).collect();
        for k in pending {
            let f = &self.host.faults.fired[k];
            let at = |stmt: &usize| self.host.faults.ids[stmt].0.clone();
            let outcome = if let Some(o) = &f.outcome {
                o.clone()
            } else if let (Some(e), true) = (entry, returned.contains(&k)) {
                format!("propagated to {}", func_id(self.m, e))
            } else if let Some(i) = kept.iter().position(|ks| ks.contains(&k)) {
                format!("kept at {}", at(&left[i].0))
            } else if let Some(first) = &f.first {
                first.clone()
            } else if stopped {
                "live at stop".to_string()
            } else {
                format!("dropped at {}", f.site)
            };
            let summary = line(&[
                ("event", J::Str("summary")),
                ("fault", J::Str(&f.site)),
                ("nth", J::Num(f.nth)),
                ("do", J::Str(f.act)),
                ("outcome", J::Str(&outcome)),
                ("updates", J::Num(updates)),
            ]);
            self.host.emit(summary);
        }
    }

    /// A `summary` `not reached` for each fault that never answered, in plan order.
    pub(crate) fn unreached(&mut self) {
        let mut lines = Vec::new();
        for addr in &self.host.faults.order {
            let a = &self.host.faults.armed[addr];
            if a.fired {
                continue;
            }
            let nth = match a.nth {
                Nth::At(n) => J::Num(n),
                Nth::Every => J::Str("*"),
            };
            lines.push(line(&[
                ("event", J::Str("summary")),
                ("fault", J::Str(&a.id)),
                ("nth", nth),
                ("do", J::Str(a.act.name())),
                ("outcome", J::Str("not reached")),
                ("updates", J::Num(0)),
            ]));
        }
        for l in lines {
            self.host.emit(l);
        }
    }
}
