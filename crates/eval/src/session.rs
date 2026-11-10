//! Sessions (`docs/evaluator.md` § 6.3): a `codespec-session/1` file's calls, each a top-level
//! function or a method on a result an earlier call bound, run in order against one module into
//! a `codespec-eval-trace/1` trace (§ 6.4).
//!
//! # The session
//!
//! ```json
//! {"format": "codespec-session/1", "calls": [
//!   {"bind": "s", "func": "New", "args": ["chan"]},
//!   {"bind": "id", "func": "Service.Place", "recv": "s", "args": ["\"ann\"", "30", "false"]},
//!   {"func": "Service.Pay", "recv": "s", "args": ["$id"]}
//! ]}
//! ```
//!
//! `func` is a function's key: its name, or `<Type>.<name>` for a method, which `recv` names
//! the bound receiver of. An argument is an integer, `true` or `false`, a string as a JSON string,
//! `chan` for a channel the host owns, or `$<name>` for the first result of an earlier call that
//! bound `<name>`. A bound value lives in a variable of its own, as `c := NewCounter()` in Go: a
//! method with a pointer receiver gets its address and changes it, a value receiver and an
//! argument get a copy. A call that does not return (it panics, or is skipped) unbinds its name.
//! A channel the host owns takes every value sent on it, each an `effect` of the trace, and never
//! holds one. Refused before anything runs: another format, an unknown field, a function the
//! module does not declare, a method without `recv` or a function with one, a generic function,
//! another number of arguments than it takes, an argument its parameter cannot hold, a name or
//! receiver no earlier call binds, a bound value of another type than its parameter's, and for an
//! interface parameter a bound value whose type lacks one of the interface's methods, or whose
//! methods or interface the module does not declare, so cannot decide.
//!
//! # The trace
//!
//! A first line `{"format":"codespec-eval-trace/1","module":<module path>}`, then one object
//! per event, its fields in this order:
//!
//! | `event` | fields | when |
//! |---|---|---|
//! | `call` | `n`, `func`, `recv` (a method's), `args` | a call of the session starts; `n` counts from 1 |
//! | `effect` | `node`, `pos`, `effect`: `extern` with `key`, `handler` (`builtin`, `env` or `stub`), `args`, `results`; `send` with `value`; `stdout` with `text` | the run leaves the module's memory |
//! | `return` | `n`, `bind` (if the call binds), `results` | the call returns |
//! | `panic` | `n`, `node`, `pos`, `reason` | the call panics: it ends, the session goes on |
//! | `skip` | `n`, `func`, `reason` | an argument or the receiver names a result its call never gave: it panicked or was skipped |
//! | `fault` | `node`, `pos`, `nth`, `do`, and `message` (`error`, `panic`) or `results` (`value`) | a fault of the plan answers at its site ([`crate::fault`]) |
//! | `update` | `node`, `pos`, `value` | the call a fault answered in ends (returns, panics or the run stops): one per place of memory older than the call that it left changed, its content now, `node` the statement that wrote it last; a write undone before the end is none |
//! | `tested`, `propagated`, `wrapped`, `dropped` | `node`, `pos`, `fault`, `nth` | an `error` fault's error is compared, returned, formatted or discarded, in that call or a later one |
//! | `summary` | `fault`, `nth` (`"*"` for a fault of every call never reached), `do`, `outcome`, `updates` | after the call's `update` events; or `not reached` for a fault that never answered, before a `stop` |
//! | `stop` | `n` (inside a call), `kind`, `node`, `pos`, `reason` | the run stops (crate doc § Stops, a `raise` among them), the session's later calls unrun; the last line |
//!
//! A value is written as Go source spells it, close enough to read: a string quoted, a struct
//! `Name{Field: value, …}`, a pointer `&` and what it points to (`&…` for a value it already
//! points through), a map `map[key: value, …]` in insertion order, a slice `[a, b]`, `nil`, a
//! channel `chan`, a function `func`. An error is written by what it holds, never through its
//! `Error` method: writing the trace runs none of the module's code. A sentinel or a message
//! (`errorf`) is `error("message")`, a value of a type with an `Error` method is
//! `error(Name{Field: value, …})` for a struct (`error(&Name{…})` through a pointer) and
//! `error(Name(value))` for another type.

use std::collections::HashMap;

use codespec_conformance::CHAN_CAPACITY;
use codespec_model::{ChanDir, Func, Module, Type, receiver_type_name};

use crate::id::module_path;
use crate::interp::Interp;
use crate::trace::{J, line};
use crate::value::{HeapId, Place, Value};
use crate::{Options, Run, StopKind};

/// The format of a session file.
pub const SESSION_FORMAT: &str = "codespec-session/1";

/// The format of the trace a session gives.
pub const TRACE_FORMAT: &str = "codespec-eval-trace/1";

/// One call of a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCall {
    /// The name the call's first result is bound to.
    pub bind: Option<String>,
    /// The function's key: its name, or `<Type>.<name>` for a method.
    pub func: String,
    /// The bound name of a method's receiver.
    pub recv: Option<String>,
    pub args: Vec<String>,
}

/// A `codespec-session/1` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub calls: Vec<SessionCall>,
}

impl Session {
    /// Reads a session file: refused when it is not JSON, of another format, holds an unknown
    /// field, an object giving one key twice, or no call.
    pub fn from_json(bytes: &[u8]) -> Result<Session, String> {
        use serde_json::Value as V;
        let doc = crate::trace::read_json(bytes)?;
        let V::Object(top) = doc else {
            return Err("a session is a JSON object".into());
        };
        let format = top.get("format").and_then(V::as_str).unwrap_or("");
        if format != SESSION_FORMAT {
            return Err(format!(
                "format is {format:?}, this reader reads {SESSION_FORMAT:?}"
            ));
        }
        if let Some(k) = top.keys().find(|k| *k != "format" && *k != "calls") {
            return Err(format!("unknown field {k}"));
        }
        let Some(V::Array(calls)) = top.get("calls") else {
            return Err("no calls array".into());
        };
        if calls.is_empty() {
            return Err("no calls".into());
        }
        let mut out = Vec::with_capacity(calls.len());
        for (i, c) in calls.iter().enumerate() {
            let n = i + 1;
            let V::Object(c) = c else {
                return Err(format!("call {n} is not an object"));
            };
            if let Some(k) = c
                .keys()
                .find(|k| !matches!(k.as_str(), "bind" | "func" | "recv" | "args"))
            {
                return Err(format!("call {n}: unknown field {k}"));
            }
            let name = |field: &str| -> Result<Option<String>, String> {
                match c.get(field) {
                    None => Ok(None),
                    Some(V::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
                    Some(_) => Err(format!("call {n}: {field} is not a non-empty string")),
                }
            };
            let func = name("func")?.ok_or_else(|| format!("call {n}: no func"))?;
            let args = match c.get("args") {
                Some(V::Array(xs)) => xs
                    .iter()
                    .map(|x| {
                        x.as_str()
                            .map(str::to_string)
                            .ok_or_else(|| format!("call {n}: an argument is not a string"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(format!("call {n}: no args array")),
            };
            out.push(SessionCall {
                bind: name("bind")?,
                func,
                recv: name("recv")?,
                args,
            });
        }
        Ok(Session { calls: out })
    }
}

/// Runs `session` against `m` and gives its trace as [`Run::stdout`]. Refused (`Err`) before
/// anything runs: the module (crate doc § Refusals), the stubs, and a session that does not fit
/// the module (module doc).
pub fn run_session(m: &Module, session: &Session, opts: &Options) -> Result<Run, String> {
    crate::check::module(m)?;
    crate::on_eval_stack(|| run_on_this_thread(m, session, opts))
}

/// One argument of a call, checked against its parameter.
enum Arg {
    Lit(String),
    Chan,
    Bound(String),
}

/// A call checked against the module.
struct Planned<'m> {
    f: &'m Func,
    args: Vec<Arg>,
}

fn plan<'m>(it: &Interp<'m>, session: &Session) -> Result<Vec<Planned<'m>>, String> {
    let m = it.m;
    // The type of each name's value, as bound so far.
    let mut bound: HashMap<&str, &'m Type> = HashMap::new();
    let mut out = Vec::with_capacity(session.calls.len());
    for (i, c) in session.calls.iter().enumerate() {
        let n = i + 1;
        let at = |why: String| format!("call {n} ({}): {why}", c.func);
        let f = m
            .funcs
            .iter()
            .find(|f| f.key() == c.func)
            .ok_or_else(|| at(format!("the module has no function {}", c.func)))?;
        if !f.tparams.is_empty() {
            return Err(at(
                "a generic function cannot be called from a session".into()
            ));
        }
        match (&f.recv, &c.recv) {
            (None, Some(_)) => {
                return Err(at(format!("{} is not a method, it takes no recv", c.func)));
            }
            (Some(_), None) => return Err(at(format!("{} is a method: name its recv", c.func))),
            (Some(r), Some(name)) => {
                let t = bound.get(name.as_str()).ok_or_else(|| {
                    at(format!("the receiver {name} is bound by no earlier call"))
                })?;
                let strip = |t: &Type| match t {
                    Type::Ptr { elem } => (**elem).clone(),
                    t => t.clone(),
                };
                if let Type::Named { targs, .. } = strip(&r.t)
                    && !targs.is_empty()
                {
                    return Err(at("a method of a generic type cannot be called".into()));
                }
                if receiver_type_name(&strip(t)) != receiver_type_name(&strip(&r.t)) {
                    return Err(at(format!(
                        "the receiver {name} is not of the method's type"
                    )));
                }
            }
            (None, None) => {}
        }
        if f.params.len() != c.args.len() {
            return Err(at(format!(
                "{} takes {} arguments, the call passes {}",
                c.func,
                f.params.len(),
                c.args.len()
            )));
        }
        let mut args = Vec::with_capacity(c.args.len());
        for (a, p) in c.args.iter().zip(&f.params) {
            let wrong = || {
                at(format!(
                    "the argument {a} does not fit the parameter {}",
                    p.name
                ))
            };
            if p.variadic {
                return Err(at(
                    "a variadic function cannot be called from a session".into()
                ));
            }
            if let Some(name) = a.strip_prefix('$') {
                let t = bound
                    .get(name)
                    .ok_or_else(|| at(format!("{a} is bound by no earlier call")))?;
                if **t != p.t {
                    if !it.is_iface(&p.t) {
                        return Err(wrong());
                    }
                    implements(it, t, &p.t).map_err(|why| at(format!("{a}: {why}")))?;
                }
                args.push(Arg::Bound(name.to_string()));
                continue;
            }
            match it.resolve(&p.t) {
                Type::Chan { dir, .. } if a == "chan" && dir != ChanDir::Recv => {
                    args.push(Arg::Chan);
                }
                Type::Int { width, signed } => {
                    let fits = a.parse::<i128>().is_ok_and(|v| {
                        crate::interp::wrap(v, crate::interp::int_bits(width), signed) == v
                    });
                    if !fits {
                        return Err(wrong());
                    }
                    args.push(Arg::Lit(a.clone()));
                }
                Type::Bool if a == "true" || a == "false" => args.push(Arg::Lit(a.clone())),
                Type::String if serde_json::from_str::<String>(a).is_ok() => {
                    args.push(Arg::Lit(a.clone()));
                }
                _ => return Err(wrong()),
            }
        }
        if let Some(b) = &c.bind {
            let first = f
                .results
                .first()
                .ok_or_else(|| at(format!("{} gives no result to bind to {b}", c.func)))?;
            bound.insert(b.as_str(), first);
        }
        out.push(Planned { f, args });
    }
    Ok(out)
}

/// A method by its name, parameter types and result types.
type Method = (String, Vec<Type>, Vec<Type>);

/// The methods an interface type requires, each with its parameter and result types; `None`
/// for a type that is no interface the module declares.
fn iface_methods(it: &Interp<'_>, t: &Type) -> Option<Vec<Method>> {
    let decl = |path: &str, name: &str| {
        it.m.interface_decl(path, name).map(|i| {
            i.methods
                .iter()
                .map(|m| (m.name.clone(), m.params.clone(), m.results.clone()))
                .collect()
        })
    };
    match t {
        Type::Any => Some(Vec::new()),
        Type::Error => Some(vec![("Error".into(), Vec::new(), vec![Type::String])]),
        Type::Named { name, .. } => decl("", name),
        Type::Extern { path, name, .. } => decl(path, name),
        _ => None,
    }
}

/// Whether a value of the type `t` may enter a parameter of the interface type `iface`, as Go
/// decides it: every method `iface` requires is in `t`'s method set with the same parameter and
/// result types (a value type's set has its value-receiver methods, a pointer's all). Refused
/// with the reason otherwise, and where the module does not say: a type of another module.
fn implements(it: &Interp<'_>, t: &Type, iface: &Type) -> Result<(), String> {
    let need = iface_methods(it, iface).ok_or_else(|| {
        format!(
            "the module does not declare the interface {}, so cannot decide what implements it",
            crate::interp::type_name(iface)
        )
    })?;
    let have: Vec<Method> = if it.is_iface(t) {
        iface_methods(it, t).ok_or_else(|| {
            format!(
                "the module does not declare the interface {}",
                crate::interp::type_name(t)
            )
        })?
    } else {
        let (base, ptr) = match t {
            Type::Ptr { elem } => (elem.as_ref(), true),
            t => (t, false),
        };
        match base {
            Type::Named { name, .. } => {
                it.m.funcs
                    .iter()
                    .filter(|f| {
                        f.recv.as_ref().is_some_and(|r| {
                            let (rt, rptr) = match &r.t {
                                Type::Ptr { elem } => (elem.as_ref(), true),
                                t => (t, false),
                            };
                            receiver_type_name(rt) == name && (ptr || !rptr)
                        })
                    })
                    .map(|f| {
                        (
                            f.name.clone(),
                            f.params.iter().map(|p| p.t.clone()).collect(),
                            f.results.clone(),
                        )
                    })
                    .collect()
            }
            Type::Extern { .. } => {
                return Err(format!(
                    "{} is a type of another module, whose methods the module does not say",
                    crate::interp::type_name(t)
                ));
            }
            _ => Vec::new(),
        }
    };
    for (name, params, results) in &need {
        if !have
            .iter()
            .any(|(n, p, r)| n == name && p == params && r == results)
        {
            return Err(format!(
                "{} does not implement {}: no method {name} of its signature",
                crate::interp::type_name(t),
                crate::interp::type_name(iface)
            ));
        }
    }
    Ok(())
}

fn run_on_this_thread(m: &Module, session: &Session, opts: &Options) -> Result<Run, String> {
    let mut it = Interp::new(m, opts.max_steps);
    let base = 0u8;
    it.stack_base = &base as *const u8 as usize;
    it.install(opts)?;
    let planned = plan(&it, session)?;
    it.host.trace = Some(vec![line(&[
        ("format", J::Str(TRACE_FORMAT)),
        ("module", J::Str(module_path(m))),
    ])]);
    let mut stop = None;
    // The stop's line, written last, after the summaries of faults that never answered.
    let mut stop_text = None;
    if let Err(h) = it.init_globals().and_then(|()| it.run_inits()) {
        let s = h.stop(m);
        stop_text = Some(stop_line(None, &s));
        stop = Some(s);
    }
    it.end_call(None, &[], stop.is_some());
    // Each bound name's value, in a cell of its own, so a method on it changes it, and its type.
    let mut bound: HashMap<&str, (HeapId, &Type)> = HashMap::new();
    for (i, (c, p)) in session.calls.iter().zip(&planned).enumerate() {
        if stop.is_some() {
            break;
        }
        let n = (i + 1) as u64;
        let mut argv = Vec::with_capacity(p.args.len());
        let mut missing = None;
        for (a, param) in p.args.iter().zip(&p.f.params) {
            argv.push(match a {
                Arg::Lit(s) => literal(&it, s, &param.t),
                Arg::Chan => {
                    let id = it.alloc(Value::ChanData(
                        std::collections::VecDeque::new(),
                        CHAN_CAPACITY,
                        false,
                    ));
                    it.host.chans.push(id);
                    Value::Chan(id)
                }
                // Passed by value, as Go passes it: a copy of what the cell holds.
                Arg::Bound(name) => match bound.get(name.as_str()) {
                    Some(&(id, t)) => {
                        let v = it.cell(id);
                        it.enter_slot(v, t, &param.t)
                    }
                    None => {
                        missing = Some(name.clone());
                        break;
                    }
                },
            });
        }
        let recv = match (&c.recv, missing.is_none()) {
            (Some(name), true) => match bound.get(name.as_str()) {
                Some(&(id, t)) => Some((id, t)),
                None => {
                    missing = Some(name.clone());
                    None
                }
            },
            _ => None,
        };
        if let Some(name) = missing {
            // The call does not return, so it binds nothing: its name is unbound from here on,
            // as after a panic, never left holding an earlier call's value of another type.
            if let Some(b) = &c.bind {
                bound.remove(b.as_str());
            }
            it.host.emit(line(&[
                ("event", J::Str("skip")),
                ("n", J::Num(n)),
                ("func", J::Str(&c.func)),
                (
                    "reason",
                    J::Str(&format!("${name} is not bound: its call did not return")),
                ),
            ]));
            continue;
        }
        let shown: Vec<String> = argv.iter().map(|v| it.render(v)).collect();
        let mut fields = vec![
            ("event", J::Str("call")),
            ("n", J::Num(n)),
            ("func", J::Str(&c.func)),
        ];
        if let Some(r) = &c.recv {
            fields.push(("recv", J::Str(r)));
        }
        fields.push(("args", J::List(&shown)));
        it.host.emit(line(&fields));
        let depth = it.depth();
        it.begin_call();
        let result = match recv {
            Some((v, t)) => it
                .receiver(p.f, v, t)
                .and_then(|r| it.call_func(p.f, Some(r), argv, Vec::new(), None)),
            None => it.call_func(p.f, None, argv, Vec::new(), None),
        };
        match result {
            Ok(vs) => {
                let shown: Vec<String> = vs.iter().map(|v| it.render(v)).collect();
                let mut fields = vec![("event", J::Str("return")), ("n", J::Num(n))];
                if let Some(b) = &c.bind {
                    fields.push(("bind", J::Str(b)));
                }
                fields.push(("results", J::List(&shown)));
                it.host.emit(line(&fields));
                it.end_call(Some(p.f), &vs, false);
                if let (Some(b), Some(v), Some(t)) =
                    (&c.bind, vs.into_iter().next(), p.f.results.first())
                {
                    let id = it.alloc(v);
                    bound.insert(b.as_str(), (id, t));
                }
            }
            Err(h) => {
                it.unwind(depth);
                let s = h.stop(m);
                if s.kind == StopKind::Panic {
                    it.host.emit(line(&[
                        ("event", J::Str("panic")),
                        ("n", J::Num(n)),
                        ("node", J::Str(&s.node)),
                        ("pos", J::Str(&s.pos)),
                        ("reason", J::Str(&s.reason)),
                    ]));
                    it.end_call(None, &[], false);
                    if let Some(b) = &c.bind {
                        bound.remove(b.as_str());
                    }
                } else {
                    it.end_call(None, &[], true);
                    stop_text = Some(stop_line(Some(n), &s));
                    stop = Some(s);
                }
            }
        }
    }
    it.unreached();
    if let Some(t) = stop_text {
        it.host.emit(t);
    }
    let mut text = it.host.trace.take().unwrap_or_default().join("\n");
    text.push('\n');
    Ok(Run {
        stdout: text,
        stop,
        stubbed: std::mem::take(&mut it.host.stubbed),
    })
}

fn stop_line(n: Option<u64>, s: &crate::Stop) -> String {
    let mut fields = vec![("event", J::Str("stop"))];
    if let Some(n) = n {
        fields.push(("n", J::Num(n)));
    }
    fields.extend([
        ("kind", J::Str(s.kind.name())),
        ("node", J::Str(&s.node)),
        ("pos", J::Str(&s.pos)),
        ("reason", J::Str(&s.reason)),
    ]);
    line(&fields)
}

/// A literal argument [`plan`] checked against its parameter's type `t`.
fn literal<'m>(it: &Interp<'m>, a: &str, t: &Type) -> Value<'m> {
    match it.resolve(t) {
        Type::Int { .. } => Value::Int(a.parse().expect("checked by plan")),
        Type::Bool => Value::Bool(a == "true"),
        _ => Value::str(&serde_json::from_str::<String>(a).expect("checked by plan")),
    }
}

impl<'m> Interp<'m> {
    /// The receiver `f` takes, from the bound value of type `t` in the cell `id`: the cell's
    /// address for a pointer receiver on a value, so the method changes the bound value as Go's
    /// `c.Inc()` changes `c`; a copy of the value pointed to for a value receiver on a pointer;
    /// a copy of the value otherwise (a pointer copied still points where it did).
    fn receiver(&mut self, f: &'m Func, id: HeapId, t: &Type) -> crate::interp::R<'m, Value<'m>> {
        let wants_ptr = f.recv.as_ref().is_some_and(|r| self.is_ptr(&r.t));
        Ok(match (wants_ptr, self.is_ptr(t)) {
            (true, false) => Value::Ptr(Place::cell(id)),
            (false, true) => self.deref_at(&self.cell(id), f)?,
            _ => self.cell(id),
        })
    }

    /// A copy of what the cell `id` holds.
    fn cell(&self, id: HeapId) -> Value<'m> {
        self.read(&Place::cell(id)).cloned().unwrap_or(Value::Nil)
    }
}
