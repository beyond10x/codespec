//! `b10x-codespec-eval`: a tree-walking interpreter over `cir/1` (`docs/evaluator.md` § 4, § 5).
//! [`run_calls`] runs the calls of a `codespec-calls/1` file against one module and prints what
//! the kit's native runners print for them; `codespec eval --calls` is its command line.
//! [`run_session`] runs a `codespec-session/1` file, whose calls reuse earlier results, and gives
//! its trace, `codespec-eval-trace/1` ([`session`]); `codespec eval --session`.
//!
//! # Handlers and stubs
//!
//! An extern call goes, in this order, to a [`Stub`] the user named for its node id, a stub for
//! its key, then its built-in handler (`builtins`: pure library functions as Go's, and
//! `os.Getenv` and `os.LookupEnv` reading [`Options::env`], empty unless given); with none of
//! them the run stops (`no handler`, E4). [`Run::stubbed`] names each site a stub answered at and
//! how often. A stub answers at `extern` and `extern_method` nodes; a method of an extern type
//! called through an interface goes to its handler. A stub naming no extern call of the module,
//! or values the call's results cannot hold, is refused before anything runs.
//!
//! # Output
//!
//! The encoding of the kit's driver, as the crate doc of `codespec_conformance` defines it: each
//! call one line after anything the function prints itself, `ok <v>…`, `err <Error()>` or `ok`,
//! then `sent <v>` for each value left on each channel argument. A run is deterministic (E2): the
//! same module and calls give the same bytes. A map is iterated in insertion order.
//!
//! # Stops
//!
//! A run never goes past what `cir/1` does not say (E4). It ends with a [`Stop`] naming the node
//! by its id (`docs/query-language.md` § 4.1, [`id`]), the position of its statement and a
//! reason, after the output of every call before it:
//!
//! | kind | when |
//! |---|---|
//! | `foreign` | a `foreign` statement is executed, or a variable's initializer is foreign source; one the run does not reach does not stop it |
//! | `no handler` | an extern with no built-in handler and no stub is called; the handlers are `path.Join`, `strconv.Itoa`, `strings.Repeat`, `strings.ToUpper`, `ToLower` and `TrimSpace` and `strings.Builder`'s `WriteString`, `Len` and `String`, each as Go's, and `os.Getenv` and `os.LookupEnv` on the run's environment |
//! | `no body` | a function the module declares without a body (`opaque`) is called |
//! | `panic` | Go would panic: an index out of range, a nil dereference, a zero divisor, a send on a closed channel, a handler's panic (a write to a copied non-zero `strings.Builder`) |
//! | `blocked` | a send on a full channel or a receive from an empty one, which only another goroutine could end |
//! | `step budget` | the run took more than [`Options::max_steps`] steps (default 10,000,000; each statement, call and loop iteration is one; an element or field of a value with a place, an array's `len` and a range's next iteration are read in place, so a step copies no more than Go copies), calls nest deeper than 1000, statements and expressions nest deeper than 10,000 levels counting through calls, or the evaluator's 512 MiB stack has less than 64 MiB left |
//! | `unsupported` | a construct the evaluator does not reproduce exactly: a `sprintf` verb other than `%d`, `%s`, `%v` (and `%w` outside `errorf`), a string outside ASCII in a result, a value the kit's driver cannot print, an `append` growing a slice of an extern type, a zero value a generic type's arguments make over the limit below, an entry added to a map during a range over it, a string outside ASCII to `strings.ToUpper`, `ToLower` or `TrimSpace` |
//!
//! A panic inside a function literal a module variable holds names its node as
//! `<module path>.<var>#init.…` ([`id`]).
//!
//! # Refusals
//!
//! Besides calls the kit's driver cannot make, [`run_calls`] refuses before anything runs (exit 2
//! of `codespec eval`):
//!
//! - a call of a function or method the module does not declare, or with another number of
//!   arguments than it takes;
//! - a type the module does not declare, a struct or newtype that holds itself, a type nested
//!   deeper than 100 inside a value;
//! - a type whose zero value takes more than 4,194,304 cells (one per scalar or reference);
//! - an argument of the calls file its parameter cannot hold: an integer outside its width.
//!
//! # Semantics
//!
//! Integers are kept within their type's width after each operation (`sem.int.wrap`), divide
//! and take the remainder truncated (`sem.rem.sign`); a float of 32 bits is rounded after each
//! operation. A `continue` in a three-clause `for` runs its `post` (`sem.continue.post`), each
//! iteration has its own copy of the variables `init` declares, a multi-target `assign` reads
//! every value before it stores any (`sem.multi_assign.order`) and a `break` leaves the innermost
//! loop, through a `switch` (`sem.switch.break`). Structs and arrays are values; a slice shares
//! its array with the slices `append` made from it while it has room, and grows to the capacity
//! Go's runtime gives it (`runtime.growslice` with its size classes, on amd64). `len` counts a
//! string's bytes. Module variables are initialised in Go's dependency order, then the `init`
//! functions run by file name and line, before the first call. A `range` reads each element,
//! entry value or rune when it reaches it. `fmt` calls a value's `Error` or `String` method for
//! `%v` and `%s`, but none of a value it reaches through an unexported struct field. A value
//! held as `error` and the same value held as another interface are equal.
//!
//! # Interfaces
//!
//! A concrete value entering a slot of interface type is converted to it, keeping its type, in
//! one place (`Interp::enter_slot`). The `cir/1` nodes that put a value into a typed slot, and so
//! the crossings that pass through it:
//!
//! - `assign`: each value into its target's type, one value per target or a multi-valued call
//!   spread over the targets;
//! - `return`: each result into the function's result type, a multi-valued call spread;
//! - a call's arguments into its parameters: `call`, `method`, `iface_call`, `call_value`
//!   of a function, a method value or a function literal, `extern` and `extern_method` with
//!   their declared parameters; a variadic tail's values into the slice's element type, a
//!   multi-valued call spread over the parameters;
//! - `send`: the value into the channel's element type;
//! - `struct`: each field's value into the field's type;
//! - `append`: each value into the slice's element type;
//! - `index` of a map (read, assign, comma-ok) and `lookup_else`: the key into the key type;
//! - `binop` `==` and `!=`: either operand into the other's interface type; `switch`: the tag
//!   and each case value likewise;
//! - `conv` to an interface type, `to_iface` (and `to_error` into `error`);
//! - a module variable's initializer into the variable's type.
//!
//! `let`, `for_range`, `lookup_else`'s name and a receive declare variables of the value's own
//! type, and a `var` holds a zero value: no crossing.
//!
//! # As a conformance adapter, and differential runs
//!
//! [`runner`] is the writer and runner pair `codespec-write-cir` / `codespec-run-cir` that
//! `codespec conform --lang cir --role writer` judges, with the claim and the reason for each
//! feature left out of it. [`diff`] makes per-function differential runs: seeded calls of every
//! exported function of a module, through generated probes for what a calls file cannot pass,
//! run by the evaluator and the native runners and compared byte for byte (`codespec-eval-diff`,
//! `task eval-diff`).
//!
//! # Limits
//!
//! - Constant expressions are computed as `cir/1` writes them, operator by operator in their
//!   type, where Go folds them exactly at compile time: `big * 4 / 8` with `big = 1 << 62`
//!   overflows, `0.1 + 0.2` prints `0.30000000000000004`. The Go reader is to fold them.
//! - Go's compiler may give a non-escaping slice of at most 32 bytes a backing store on the
//!   stack, with a capacity its escape analysis decides; the evaluator gives every slice the
//!   heap's capacity, so a program whose result depends on whether two such appends share an
//!   array may print otherwise than Go.
//! - Several `init` functions share the function key `init`, so the node ids inside them do
//!   too; a stop in one is told apart by its position only.

mod builtins;
mod check;
pub mod diff;
mod gofmt;
mod growth;
mod host;
pub mod id;
mod interp;
pub mod kit;
pub mod runner;
pub mod session;
mod trace;
mod value;

pub use session::{SESSION_FORMAT, Session, SessionCall, TRACE_FORMAT, run_session};

use codespec_conformance::{Calls, calls_go};
use codespec_model::Module;

/// How a run is bounded, and what it is given from outside the module.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// The statements and calls a run may execute before it stops (`step budget`).
    pub max_steps: u64,
    /// The environment `os.Getenv` and `os.LookupEnv` read, as `(key, value)`; the last of a
    /// key counts. Empty by default: a run sees none of the host's.
    pub env: Vec<(String, String)>,
    /// The stubs that answer in place of a handler (crate doc § Handlers and stubs).
    pub stubs: Vec<Stub>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_steps: 10_000_000,
            env: Vec::new(),
            stubs: Vec::new(),
        }
    }
}

/// A stub the user names (E3, E4): the results an extern call gives instead of its handler's,
/// at every call of an extern (`site` its key, `os.Getpid`) or at one call (`site` the call's
/// node id, `envcheck.Mode#0.rhs.0.args.0`).
#[derive(Debug, Clone, PartialEq)]
pub struct Stub {
    pub site: String,
    /// One JSON value per result: an integer, a float, `true` or `false`, a string, or `null`
    /// for a nil error, pointer, slice, map, channel, function or interface.
    pub values: Vec<serde_json::Value>,
}

impl Stub {
    /// Reads `<site>=<values>`, the values JSON separated by white space: `os.Getpid=42`,
    /// `os.Hostname="h" null`, `log.Print=` for an extern without results.
    pub fn parse(s: &str) -> Result<Stub, String> {
        let (site, values) = s
            .split_once('=')
            .ok_or_else(|| format!("stub {s:?}: not <site>=<values>"))?;
        if site.is_empty() {
            return Err(format!("stub {s:?}: no site"));
        }
        let values = serde_json::Deserializer::from_str(values)
            .into_iter::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("stub {s:?}: {e}"))?;
        Ok(Stub {
            site: site.to_string(),
            values,
        })
    }

    /// Whether the site is a node id rather than an extern's key.
    pub fn is_node(&self) -> bool {
        self.site.contains('#')
    }
}

/// A call site a stub answered at, and how often (E4: the result says a stub answered).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stubbed {
    /// The extern's key.
    pub key: String,
    /// The call's node id and its statement's `file:line`.
    pub node: String,
    pub pos: String,
    pub calls: u64,
}

impl std::fmt::Display for Stubbed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stubbed {} at {} ({}): {} call{}",
            self.key,
            self.node,
            self.pos,
            self.calls,
            if self.calls == 1 { "" } else { "s" }
        )
    }
}

/// Why a run stopped (crate doc § Stops).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Foreign,
    NoHandler,
    NoBody,
    Panic,
    Blocked,
    Budget,
    Unsupported,
}

impl StopKind {
    pub fn name(self) -> &'static str {
        match self {
            StopKind::Foreign => "foreign",
            StopKind::NoHandler => "no handler",
            StopKind::NoBody => "no body",
            StopKind::Panic => "panic",
            StopKind::Blocked => "blocked",
            StopKind::Budget => "step budget",
            StopKind::Unsupported => "unsupported",
        }
    }
}

/// Where and why a run stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub kind: StopKind,
    /// The node's id: `<module path>.<function key>#<index path>`, the function's id for a stop
    /// outside its body, or `<module path>.<var>` for a variable's initializer.
    pub node: String,
    /// The `file:line` of the statement holding the node.
    pub pos: String,
    pub reason: String,
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stopped at {} ({}): {}: {}",
            self.node,
            self.pos,
            self.kind.name(),
            self.reason
        )
    }
}

/// What a run printed (a session's trace), where it stopped if it did not make every call, and
/// the sites a stub answered at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub stdout: String,
    pub stop: Option<Stop>,
    pub stubbed: Vec<Stubbed>,
}

/// The stack the evaluator runs on: room for [`interp::MAX_NEST`] levels of its own recursion
/// in a debug build (about 33 KiB a level), with a margin the run stops at besides.
pub(crate) const EVAL_STACK: usize = 512 << 20;

/// Run `calls` against `m` and give what it prints. Refused (`Err`) before anything runs: calls
/// that do not fit the module as the kit's driver requires (an exported top-level function,
/// arguments of its parameters' kinds, results the driver can print), and the refusals of the
/// crate doc § Refusals.
pub fn run_calls(m: &Module, calls: &Calls, opts: &Options) -> Result<Run, String> {
    calls_go(m, calls)?;
    check::module(m)?;
    check::calls(m, calls)?;
    on_eval_stack(|| run_on_this_thread(m, calls, opts))
}

/// `run` on a thread of its own with the evaluator's stack, [`EVAL_STACK`].
pub(crate) fn on_eval_stack<T: Send>(run: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .name("cir1-eval".into())
            .stack_size(EVAL_STACK)
            .spawn_scoped(s, run)
            .expect("a thread to evaluate on")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

fn run_on_this_thread(m: &Module, calls: &Calls, opts: &Options) -> Result<Run, String> {
    let mut it = interp::Interp::new(m, opts.max_steps);
    let base = 0u8;
    it.stack_base = &base as *const u8 as usize;
    it.install(opts)?;
    let mut stop = it
        .init_globals()
        .and_then(|()| it.run_inits())
        .err()
        .map(|h| h.stop(m));
    if stop.is_none() {
        for c in &calls.calls {
            let f = m
                .funcs
                .iter()
                .find(|f| f.name == c.func && f.recv.is_none())
                .ok_or_else(|| format!("no function {:?}", c.func))?;
            if let Err(h) = it.call_entry(f, &c.args) {
                stop = Some(h.stop(m));
                break;
            }
        }
    }
    let out = it.take_out();
    let stdout = match String::from_utf8(out) {
        Ok(s) => s,
        Err(e) => {
            let valid = e.utf8_error().valid_up_to();
            let mut bytes = e.into_bytes();
            bytes.truncate(valid);
            stop.get_or_insert(Stop {
                kind: StopKind::Unsupported,
                node: id::module_path(m).to_string(),
                pos: String::new(),
                reason: "the output is not UTF-8".into(),
            });
            String::from_utf8(bytes).expect("valid up to here")
        }
    };
    Ok(Run {
        stdout,
        stop,
        stubbed: std::mem::take(&mut it.host.stubbed),
    })
}
