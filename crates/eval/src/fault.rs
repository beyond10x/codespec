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
//! format, an unknown field, an `nth` below 1, a `do` the site cannot take, values its call's
//! results cannot hold, a site that is no effect site of the module, a second fault for one site,
//! and a plan given to a run of a calls file, which writes no trace to say what it did.
//!
//! # What the trace says
//!
//! When a fault answers, the trace has a `fault` event, then an `update` event for each statement
//! of the session's call that wrote, before the fault, into memory older than the call (a field,
//! element or map entry reached through a pointer, a map or a bound receiver), with the last
//! value it wrote: the partial update a caller that recovers sees. An `error` fault's error, and
//! an `errorf` that wraps it, carries the fault it came from, and the trace follows it:
//!
//! | event | the error |
//! |---|---|
//! | `tested` | is an operand of `==` or `!=`, a `switch`'s tag or one of its case values |
//! | `propagated` | is a result of a `return` |
//! | `wrapped` | is an argument of `errorf` or `sprintf` |
//! | `dropped` | is bound to `_`, or a result of a call a statement discards |
//!
//! When the session's call ends, each fault that fired in it ends in one `summary` event, its
//! `outcome`: `panic, not handled in module` (`closed`, `panic`: `cir/1` has no `recover`),
//! `blocked`, `answered` (`value`), and for `error` `propagated to <entry>` when the call returns
//! it to the session, else `handled at <node>` or `dropped at <node>` at the first test or drop of
//! it, else `dropped at <site>`: it went out of scope unread. A fault the run never reaches gets a
//! summary `not reached` at the end of the trace, before a `stop`.
//!
//! # Each site
//!
//! [`run_each`] runs a session once per site of a list and call count from 1 to a most, each
//! site given the fault Go defines for it: `closed` at a send, `error` at an extern call whose
//! last result is an error, `panic` at another call (`codespec eval --each`).

use std::collections::HashMap;
use std::rc::Rc;

use codespec_model::{Expr, Func, Module, Stmt, Type, walk};

use crate::id::{Node, func_id, node_id, var_node_id};
use crate::interp::{Interp, R};
use crate::trace::{J, line};
use crate::value::{ErrorVal, HeapId, Value};
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
    let doc: V = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
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
/// would be one, `max_nth` 0, and faults in `opts`.
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
        let file = trace_file(s, 1);
        if files.contains(&file) {
            return Err(format!("site {s}: its trace file {file} is another site's"));
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
    updates: u64,
    /// Decided when it answered: every kind's but `error`'s.
    outcome: Option<String>,
    /// `handled at <node>` or `dropped at <node>`: the first test or drop of its error.
    first: Option<String>,
}

/// One statement's write into memory older than the session's call.
struct Write {
    stmt: usize,
    node: String,
    pos: String,
    value: String,
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
    writes: Vec<Write>,
}

impl Faults<'_> {
    /// Whether a write into `cell` is a partial update to record.
    pub(crate) fn older(&self, cell: HeapId) -> bool {
        self.mark.is_some_and(|m| cell < m)
    }
}

/// The fault an error value came from.
fn origin(v: &Value<'_>) -> Option<usize> {
    match v {
        Value::Error(e) => match e.as_ref() {
            ErrorVal::Message { origin, .. } => *origin,
            _ => None,
        },
        Value::Iface(_, x) => origin(x),
        _ => None,
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
        let updates: Vec<String> = self
            .host
            .faults
            .writes
            .iter()
            .map(|w| {
                line(&[
                    ("event", J::Str("update")),
                    ("node", J::Str(&w.node)),
                    ("pos", J::Str(&w.pos)),
                    ("value", J::Str(&w.value)),
                ])
            })
            .collect();
        let count = updates.len() as u64;
        for u in updates {
            self.host.emit(u);
        }
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
            updates: count,
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

    /// Writes the trace's `event` (`tested`, `propagated`, `wrapped`, `dropped`) at `node` when
    /// `v` is an error a fault gave, and gives that fault.
    pub(crate) fn traced(&mut self, event: &str, v: &Value<'m>, node: Node<'m>) -> Option<usize> {
        if self.host.faults.fired.is_empty() {
            return None;
        }
        let k = origin(v)?;
        let (id, pos) = self.site(node);
        let f = &mut self.host.faults.fired[k];
        let verb = match event {
            "tested" => Some("handled"),
            "dropped" => Some("dropped"),
            _ => None,
        };
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
        Some(k)
    }

    /// Records a write of `v` by the running statement into memory older than the session's call.
    pub(crate) fn record_write(&mut self, v: &Value<'m>) {
        let Some(s) = self.cur else {
            return;
        };
        let value = self.render(v);
        let stmt = s as *const Stmt as usize;
        if let Some(w) = self.host.faults.writes.iter_mut().find(|w| w.stmt == stmt) {
            w.value = value;
            return;
        }
        let (node, pos) = self.site(Node::Stmt(s));
        self.host.faults.writes.push(Write {
            stmt,
            node,
            pos,
            value,
        });
    }

    /// A session's call begins: writes into older memory are partial updates from here.
    pub(crate) fn begin_call(&mut self) {
        if !self.host.faults.armed.is_empty() {
            self.host.faults.mark = Some(self.heap_len());
            self.host.faults.writes.clear();
        }
    }

    /// A session's call ended, returning `results` from `entry` if it returned: one `summary`
    /// for each fault that answered in it.
    pub(crate) fn end_call(&mut self, entry: Option<&'m Func>, results: &[Value<'m>]) {
        self.host.faults.mark = None;
        self.host.faults.writes.clear();
        for k in std::mem::take(&mut self.host.faults.pending) {
            let f = &self.host.faults.fired[k];
            let outcome = match (&f.outcome, entry) {
                (Some(o), _) => o.clone(),
                (None, Some(e)) if results.iter().any(|v| origin(v) == Some(k)) => {
                    format!("propagated to {}", func_id(self.m, e))
                }
                (None, _) => f
                    .first
                    .clone()
                    .unwrap_or_else(|| format!("dropped at {}", f.site)),
            };
            let summary = line(&[
                ("event", J::Str("summary")),
                ("fault", J::Str(&f.site)),
                ("nth", J::Num(f.nth)),
                ("do", J::Str(f.act)),
                ("outcome", J::Str(&outcome)),
                ("updates", J::Num(f.updates)),
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
