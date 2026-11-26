//! What changed in the code facts of one program between two revisions: the units, their
//! signatures, effects, syscalls by call site, raises, reach, entries and `unknown` reasons.
//! `codespec diff <old-dir> <new-dir>` prints the report this crate defines.
//!
//! # The input
//!
//! Each revision is a directory: every `*.cir.json` file directly in it is one `cir/1` document,
//! read strictly, and the documents are one program ([`read_dir`]). A directory with no document,
//! a document strict reading refuses and two documents with the same module path (the `path`, or
//! the `name` of a module without one) are refused, naming the directory or the document.
//!
//! Each revision's facts are computed with the code `codespec effects` and `codespec reach` use
//! (`b10x-codespec-effects`), with the same summaries, barrier, entries and dispatch for both. The
//! entries of a revision are the entries of [`Options`] (`--entry`, `--exported`) and every
//! function its documents record in `Module.entries` (`func` of the module `Entry.path` names, or
//! of the document's own module without one; an entry naming a function the set does not hold is
//! no entry for reach); with no `--entry` and no `--exported`, `main.main` of each `main` module
//! too, as `reach` takes it; and the Go `init` functions and variable initializers that run with
//! them, as `reach` takes them (those of the programs that hold an entry, or of every module).
//!
//! # The report, `codespec-diff-report/1`
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-diff-report/1` |
//! | `dispatch` | string | `cha` or `rta`, as both sides were analysed |
//! | `deltas` | array | one object per delta, sorted by `unit`, then `kind`, then `site` (`callee`, then `n`), then `value` |
//! | `unknowns` | array, only with `--unknowns summary` and omitted when empty | one object per kind and reason class of the `unknown-*` deltas, sorted by `kind`, then `class` (below) |
//!
//! A delta is `{kind, unit, site, value, old, new, callers}`; `site`, `value`, `old`, `new` and
//! `callers` are omitted when they do not apply or are empty:
//!
//! | `kind` | `unit` | other fields |
//! |---|---|---|
//! | `unit-added`, `unit-removed` | the function | none |
//! | `signature` | the function | `old`, `new`: its type as `codespec effects` spells a signature (`func(string) (int, error)`), receiver first; parameter names are not part of it |
//! | `effect-added`, `effect-removed` | the function | `value`: an effect of the vocabulary (`db`, `env`, `fs.read`, `fs.write`, `io`, `net`, `proc`, `rand`, `stdin`, `stdout`, `time`) the function has in one revision only |
//! | `syscall-added`, `syscall-removed` | the function | `site`: the call-site id (`{callee, n}`, the crate doc of `b10x-codespec-effects`, § Call sites) of the function's own call that leads there; `value`: the syscall |
//! | `raise-added`, `raise-removed` | the function | `value`: an extern type `path.name` in its `raises` in one revision only |
//! | `reach-gained`, `reach-lost` | the function | none: reached from the entries in the new revision only, or in the old only; a unit added or removed gives no reach delta |
//! | `entry-added`, `entry-removed` | the entry's function | `value`: `<kind> <method> <on>` (`http GET /api/items`); `<kind> <on>` without a method (outside `http`, or an `http` entry for any method: `scheduled every=5m`) |
//! | `unknown-added`, `unknown-removed` | the function | `value`: an `unknown` reason of `codespec effects` (its text, without its position) present in one revision only, except an inherited one (§ Inherited reasons); `callers`: the functions incomplete through it in that revision, sorted |
//!
//! `unit` is `<module path>.<function key>` (the module's `path`, or its `name` without one), for
//! functions and opaque functions alike. What runs when a module loads is a unit too: its
//! variable initializers (and the calls of its `inits`) are `<module path>.init:vars` (the name
//! `init:vars` of `b10x-codespec-effects`, which no function key can be, since a Go `init`
//! function has the key `init`), with their effects, syscalls, raises and `unknown` reasons and
//! no signature or reach delta; and every Go `init` function of a module is the one unit
//! `<module path>.init`, its facts the union of theirs. A function is compared only with the function of the
//! same unit; an opaque function has no signature, so a unit that is opaque in either revision
//! gives no `signature` delta. No field holds a file or a line: a change that only moves lines,
//! or moves a function to another file of its module, gives no delta. With [`Options::report`],
//! only the units of the modules it names, by `path` or `name`, give deltas.
//!
//! The encoding is canonical: two-space indentation, a final newline; the same inputs give the
//! same bytes. The text form ([`Report::to_text`]) prints one line per delta in the same order,
//! its fields separated by one space, a missing one skipped, the site as `<callee>@<n>`:
//! `syscall-removed example.com/probe.Load syscall.Syscall@1 getpid`, `old -> new` for a
//! signature, and `callers=<n>` last, the length of `callers`, when it is not empty (the names
//! are in the JSON form only).
//!
//! ## Inherited reasons
//!
//! A reason is reported at the function whose own body gives it. `calls <func> which is
//! incomplete` is inherited when `<func>` is a unit of the revision (a callee of the function's
//! own module named without its path, as `codespec effects` names it, is qualified with it), or
//! a function literal (`A.lambda@1`, `analyze_program` of `b10x-codespec-effects`) each of
//! whose reasons is `calls <func> which is incomplete` naming a unit or such a literal, or a
//! reason its holder has too (the effects report counts the body of a literal in a function's
//! body as that function's, so the holder's own delta gives it), to a fixed point. The function
//! only inherits the callee's incompleteness. Such a reason present in one
//! revision only is a delta only when a call of the function's own that leads to the callee
//! exists in that revision alone: its call site (`{callee, n}`, § Call sites of
//! `b10x-codespec-effects`), or, for a call through an interface or a function value, that site
//! with the callee among its candidates. So a function whose only difference is that its callee
//! became incomplete (or complete) gets no delta; a function that starts calling an incomplete
//! callee keeps `unknown-added <unit> calls <func> which is incomplete`. Every other reason is
//! the function's own, a delta as above; that includes `calls <func> which is incomplete` naming
//! a literal whose own body gives a reason its holder does not have (a literal of a `foreign`
//! statement's call list) or calls one that does, variable initializers, or a function of a
//! module that
//! [`Options::report`] leaves out, which are no unit of the report.
//!
//! `callers` of an `unknown-added` delta (of the old revision for `unknown-removed`) lists every
//! function incomplete through the delta's function in that revision: each function or literal
//! whose reason `calls <func> which is incomplete` names it, and each that names one of those,
//! to a fixed point, the functions of them only. A caller through an interface or a function
//! value is one, and so are the holder of a literal that calls it and every function whose call
//! through a function value may reach that literal; a literal whose own reasons its holder has
//! is incomplete through its holder, so whoever may call the literal is a caller of the holder.
//! A caller with two calls of it is listed once.
//!
//! ```text
//! unknown-added example.com/probe.Lib extern without summary: example.org/lib.Do callers=3
//! ```
//!
//! ## `--unknowns summary`
//!
//! `--unknowns list`, the default, gives the report above. With `--unknowns summary`
//! ([`Report::summarize_unknowns`]) `deltas` holds no `unknown-added` or `unknown-removed`
//! delta; `unknowns` holds them instead, one entry `{kind, class, count, units, examples}` per
//! kind and reason class: `class` is the class `b10x-codespec-effects` gives the reason
//! (`Unknown::class`: the text up to its first `: `, a reason without one its own class,
//! `calls <func> which is incomplete` and `syscall number <n> has no name` one class each),
//! `count` the deltas of that class, `units` the distinct functions they are of, `examples` the
//! first three distinct subjects in the order of the deltas (the part of the reason after its
//! class: after the first `: `, the `<func>` or `<n>`; the function for a reason that is its
//! whole class). A reason names a callee of the function's own module without the module's
//! path (`calls Raw which is incomplete`), so an example of `calls <func> which is incomplete`
//! adds the path of the unit's module (`example.com/p.Raw`) unless the callee already starts
//! with the path of another module of the program; two functions of one name in two modules
//! are two examples. The summary counts the `unknown-*` deltas `list` gives, after § Inherited
//! reasons: an inherited reason makes no delta there either, so the class `calls <func> which
//! is incomplete` counts only the functions whose own call of an incomplete function is new (or
//! gone), and the reasons that stay their function's own; the functions that inherit a reason
//! are counted nowhere, and `callers` is not kept. The other deltas are unchanged. The text form
//! prints the deltas, then
//! `<kind> <class> count=<count> units=<units>` per entry, each example below it on a line of
//! its own indented by two spaces:
//!
//! ```text
//! unknown-added extern without summary count=3 units=3
//!   example.org/lib.A
//!   example.org/lib.B
//!   example.org/lib.C
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use codespec_effects::{Barrier, Call, CallSite, DispatchMode, FuncEffects, Summary};
use codespec_model::Module;
use serde::{Deserialize, Serialize};

/// The format of the report [`diff`] gives.
pub const FORMAT: &str = "codespec-diff-report/1";

/// A `codespec-diff-report/1` document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub format: String,
    /// `cha` or `rta`.
    pub dispatch: String,
    pub deltas: Vec<Delta>,
    /// Only after [`Report::summarize_unknowns`]: the `unknown-*` deltas, one entry per kind
    /// and class.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknowns: Vec<UnknownClass>,
    /// The labels of the modules of both revisions, which [`diff`] sets: a reason names a
    /// callee of its own module without its label, and the summary's examples add it. Not part
    /// of the document or of equality.
    #[serde(skip)]
    labels: Vec<String>,
}

impl PartialEq for Report {
    fn eq(&self, other: &Self) -> bool {
        (&self.format, &self.dispatch, &self.deltas, &self.unknowns)
            == (
                &other.format,
                &other.dispatch,
                &other.deltas,
                &other.unknowns,
            )
    }
}

impl Eq for Report {}

/// The `unknown-added` or `unknown-removed` deltas of one reason class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownClass {
    pub kind: String,
    /// [`codespec_effects::Unknown::class`] of their reasons.
    pub class: String,
    /// The deltas.
    pub count: usize,
    /// The distinct functions they are of.
    pub units: usize,
    /// The first three distinct subjects, in the deltas' order.
    pub examples: Vec<String>,
}

/// One difference between the two revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delta {
    pub kind: String,
    /// `<module path>.<function key>`.
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<CallSite>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<String>,
    /// Of an `unknown-added` or `unknown-removed` delta: the functions incomplete through
    /// `unit` in the revision that has the reason, sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<String>,
}

impl Report {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Report always serializes");
        s.push('\n');
        s
    }

    /// One line per delta, its fields separated by one space, a missing one skipped,
    /// `callers=<n>` last when it has callers, then one line per entry of `unknowns`, each
    /// example on a line of its own indented by two spaces; empty without either.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for d in &self.deltas {
            let mut fields = vec![d.kind.clone(), d.unit.clone()];
            fields.extend(d.site.as_ref().map(CallSite::to_string));
            fields.extend(d.value.clone());
            match (&d.old, &d.new) {
                (Some(old), Some(new)) => fields.push(format!("{old} -> {new}")),
                (old, new) => fields.extend(old.iter().chain(new).cloned()),
            }
            if !d.callers.is_empty() {
                fields.push(format!("callers={}", d.callers.len()));
            }
            out.push_str(&fields.join(" "));
            out.push('\n');
        }
        for u in &self.unknowns {
            let (kind, class, count, units) = (&u.kind, &u.class, u.count, u.units);
            out.push_str(&format!("{kind} {class} count={count} units={units}\n"));
            for e in &u.examples {
                out.push_str(&format!("  {e}\n"));
            }
        }
        out
    }

    /// `--unknowns summary`: move the `unknown-added` and `unknown-removed` deltas into
    /// `unknowns`, one entry per kind and reason class, sorted by kind, then class.
    pub fn summarize_unknowns(&mut self) {
        type Group = (usize, BTreeSet<String>, Vec<String>);
        let mut groups: BTreeMap<(String, String), Group> = BTreeMap::new();
        self.deltas.retain(|d| {
            if d.kind != "unknown-added" && d.kind != "unknown-removed" {
                return true;
            }
            let reason = d.value.as_deref().unwrap_or_default();
            let class = codespec_effects::Unknown::class(reason);
            let subject = codespec_effects::Unknown::subject(reason).unwrap_or(&d.unit);
            let subject = if class == INCOMPLETE {
                qualified(&self.labels, &d.unit, subject)
            } else {
                subject.to_owned()
            };
            let (count, units, examples) = groups
                .entry((d.kind.clone(), class.to_owned()))
                .or_default();
            *count += 1;
            units.insert(d.unit.clone());
            if examples.len() < 3 && !examples.contains(&subject) {
                examples.push(subject);
            }
            false
        });
        self.unknowns.extend(groups.into_iter().map(
            |((kind, class), (count, units, examples))| UnknownClass {
                kind,
                class,
                count,
                units: units.len(),
                examples,
            },
        ));
    }
}

/// The class whose callee a reason names without its module's label when it is the module of
/// the function holding the call.
const INCOMPLETE: &str = "calls <func> which is incomplete";

/// The callee `callee` of a reason of `unit`, with the label of `unit`'s module before it unless
/// it starts with the label of another module of `labels` (a callee of another module): the
/// longest label `unit` starts with. `callee` as it is when no label fits.
fn qualified(labels: &[String], unit: &str, callee: &str) -> String {
    let under = |name: &str, l: &str| name.strip_prefix(l).is_some_and(|r| r.starts_with('.'));
    let Some(own) = labels
        .iter()
        .filter(|l| under(unit, l))
        .max_by_key(|l| l.len())
    else {
        return callee.to_owned();
    };
    if labels.iter().any(|l| l != own && under(callee, l)) {
        callee.to_owned()
    } else {
        format!("{own}.{callee}")
    }
}

/// What both revisions are analysed with.
#[derive(Debug, Default, Clone)]
pub struct Options<'a> {
    /// The `codespec-effects/1` summaries; where two name one function, the first counts.
    pub summaries: &'a [Summary],
    /// The barrier; without one, a function without a body is `unknown`.
    pub barrier: Option<&'a Barrier>,
    /// Give deltas only for the units of the modules listed by `path` or `name`; all when empty.
    pub report: Vec<String>,
    /// Entry functions, `<path>.<key>`, in both revisions.
    pub entries: Vec<String>,
    /// Modules, by `path` or `name`, whose every exported function is an entry.
    pub exported: Vec<String>,
    /// How a call through an interface or a function value is resolved.
    pub dispatch: DispatchMode,
}

/// The documents of one revision: every `*.cir.json` directly in `dir`, read strictly, in the
/// order of their file names. A directory without one, a document strict reading refuses and two
/// documents with one module path are refused, naming the directory or the document.
pub fn read_dir(dir: &Path) -> Result<Vec<Module>, String> {
    let listing = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut paths = Vec::new();
    for entry in listing {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let named = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".cir.json"));
        if named && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "{}: no supported source: no *.cir.json document, nothing was compared",
            dir.display()
        ));
    }
    let mut seen: BTreeMap<String, &Path> = BTreeMap::new();
    let mut modules = Vec::with_capacity(paths.len());
    for path in &paths {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let m = Module::from_json(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let label = label(&m).to_owned();
        if let Some(first) = seen.get(&label) {
            return Err(format!(
                "{}: {} and {} both hold the module {label}",
                dir.display(),
                first.display(),
                path.display()
            ));
        }
        seen.insert(label, path);
        modules.push(m);
    }
    Ok(modules)
}

/// The module's `path`, or its `name` without one.
fn label(m: &Module) -> &str {
    if m.path.is_empty() { &m.name } else { &m.path }
}

/// The keys of the functions and opaque functions of `m`, in that order.
fn keys(m: &Module) -> impl Iterator<Item = String> + '_ {
    m.funcs
        .iter()
        .map(|f| f.key())
        .chain(m.opaque.iter().map(|o| o.key()))
}

/// The facts of one unit in one revision.
#[derive(Default)]
struct Unit {
    signature: Option<String>,
    effects: BTreeSet<String>,
    syscalls: BTreeSet<(Option<CallSite>, String)>,
    raises: BTreeSet<String>,
    reached: bool,
    unknown: BTreeSet<String>,
    /// Its calls of functions of the set, whether the callee is incomplete or not.
    calls: BTreeSet<Call>,
    /// Each reason `calls <func> which is incomplete` whose `<func>` is a unit or a function
    /// literal of the revision, with that unit or literal: the function is incomplete through
    /// it.
    through: BTreeMap<String, String>,
    /// The reasons of `through` the function inherits: those naming a unit, or a literal that
    /// is incomplete only through what it calls or through reasons its holder already has.
    inherited: BTreeSet<String>,
    /// Of a literal: the unit that holds it, when that unit has every reason of the literal
    /// outside `through` (a holder's report counts the bodies of its literals as its own).
    covered_by: Option<String>,
}

/// The facts of one revision: each unit's, each entry as `(unit, value)`, and each function
/// literal's `unknown`, `through` and `inherited`, keyed `<module path>.<holder>.lambda@<n>`
/// (no function key holds an `@`).
struct Facts {
    units: BTreeMap<String, Unit>,
    entries: BTreeSet<(String, String)>,
    literals: BTreeMap<String, Unit>,
}

/// The entries of the revision `modules`: those of `opts`, `main.main` of each `main` module
/// without them, and every recorded entry naming a function of the set.
fn entries(modules: &[Module], opts: &Options) -> Vec<String> {
    let mut out = opts.entries.clone();
    let holds = |l: &str, key: &str| {
        modules
            .iter()
            .any(|m| label(m) == l && keys(m).any(|k| k == key))
    };
    if opts.entries.is_empty() && opts.exported.is_empty() {
        for m in modules {
            if m.name == "main" && keys(m).any(|k| k == "main") {
                out.push(format!("{}.main", label(m)));
            }
        }
    }
    for m in modules {
        for e in &m.entries {
            let l = if e.path.is_empty() { label(m) } else { &e.path };
            if holds(l, &e.func) {
                out.push(format!("{l}.{}", e.func));
            }
        }
    }
    out
}

/// Add what the effects report says of one function to the unit `u`.
fn absorb(u: &mut Unit, f: &FuncEffects) {
    u.effects.extend(f.effects.iter().cloned());
    u.raises.extend(f.raises.iter().cloned());
    u.unknown.extend(f.unknown.iter().map(|x| x.reason.clone()));
    u.calls.extend(f.calls.iter().cloned());
    for s in &f.syscalls {
        if s.sites.is_empty() {
            u.syscalls.insert((None, s.name.clone()));
        }
        for site in &s.sites {
            u.syscalls.insert((Some(site.clone()), s.name.clone()));
        }
    }
}

/// The facts of the revision `modules`.
fn facts(modules: &[Module], opts: &Options) -> Result<Facts, String> {
    let eopts = codespec_effects::Options {
        barrier: opts.barrier,
        report: opts.report.clone(),
        roots: false,
        entries: entries(modules, opts),
        exported: opts.exported.clone(),
        dispatch: opts.dispatch,
    };
    codespec_effects::check_entries(modules, &eopts)?;
    let reported_label = |l: &str| {
        opts.report.is_empty()
            || modules
                .iter()
                .filter(|m| label(m) == l)
                .any(|m| opts.report.iter().any(|r| *r == m.path || *r == m.name))
            || opts.report.iter().any(|r| r == l)
    };
    let mut units: BTreeMap<String, Unit> = BTreeMap::new();
    for m in modules.iter().filter(|m| reported_label(label(m))) {
        for f in &m.funcs {
            units
                .entry(format!("{}.{}", label(m), f.key()))
                .or_default()
                .signature
                .get_or_insert_with(|| codespec_effects::signature(m, f));
        }
    }

    // The report lists the functions module by module in the order given, each key of a module
    // once: walk the modules alongside it.
    let codespec_effects::Program {
        report,
        at_load,
        literals: literal_effects,
    } = codespec_effects::analyze_program(modules, opts.summaries, &eopts);
    let mut consumed: Vec<BTreeSet<String>> = vec![BTreeSet::new(); modules.len()];
    let mut mi = 0;
    for f in &report.funcs {
        while mi < modules.len()
            && !(reported_label(label(&modules[mi]))
                && modules[mi].path == f.path
                && !consumed[mi].contains(&f.name)
                && keys(&modules[mi]).any(|k| k == f.name))
        {
            mi += 1;
        }
        let Some(m) = modules.get(mi) else {
            return Err(format!(
                "the effects report names {}.{}, which no module of the set holds",
                f.path, f.name
            ));
        };
        consumed[mi].insert(f.name.clone());
        absorb(
            units.entry(format!("{}.{}", label(m), f.name)).or_default(),
            f,
        );
    }
    // The variable initializers are the unit `<module>.init:vars`; a Go `init` after the first
    // joins the unit `<module>.init` of the first.
    for (mi, f) in &at_load {
        absorb(
            units
                .entry(format!("{}.{}", label(&modules[*mi]), f.name))
                .or_default(),
            f,
        );
    }

    let mut literals: BTreeMap<String, Unit> = BTreeMap::new();
    for (mi, f) in &literal_effects {
        absorb(
            literals
                .entry(format!("{}.{}", label(&modules[*mi]), f.name))
                .or_default(),
            f,
        );
    }
    attribute(modules, &mut units, &mut literals);

    let reach = codespec_effects::reach(modules, &eopts)?;
    for r in &reach.modules {
        let l = if r.path.is_empty() { &r.name } else { &r.path };
        for key in &r.reached {
            if let Some(u) = units.get_mut(&format!("{l}.{key}")) {
                u.reached = true;
            }
        }
    }

    let mut entries = BTreeSet::new();
    for m in modules {
        for e in &m.entries {
            let l = if e.path.is_empty() { label(m) } else { &e.path };
            if reported_label(l) {
                entries.insert((
                    format!("{l}.{}", e.func),
                    if e.method.is_empty() {
                        format!("{} {}", e.kind, e.on)
                    } else {
                        format!("{} {} {}", e.kind, e.method, e.on)
                    },
                ));
            }
        }
    }
    Ok(Facts {
        units,
        entries,
        literals,
    })
}

/// Fill `through` and `inherited` of every unit and literal of one revision. A reason `calls
/// <func> which is incomplete` names a callee of the function's own module without its label:
/// qualify it. It is inherited when it names a unit, or a literal whose every reason is in its
/// `through` or is a reason of its holder (the holder reports it) and whose `through` names
/// units and such literals only, to a fixed point (a literal whose own body gives a reason no
/// holder reports, or that calls one that does, is not).
fn attribute(
    modules: &[Module],
    units: &mut BTreeMap<String, Unit>,
    literals: &mut BTreeMap<String, Unit>,
) {
    let labels: Vec<String> = modules.iter().map(|m| label(m).to_owned()).collect();
    let resolve = |key: &str, u: &Unit| -> BTreeMap<String, String> {
        u.unknown
            .iter()
            .filter_map(|reason| {
                if codespec_effects::Unknown::class(reason) != INCOMPLETE {
                    return None;
                }
                let callee = qualified(&labels, key, codespec_effects::Unknown::subject(reason)?);
                (units.contains_key(&callee) || literals.contains_key(&callee))
                    .then(|| (reason.clone(), callee))
            })
            .collect()
    };
    let unit_through: Vec<(String, BTreeMap<String, String>)> = units
        .iter()
        .map(|(k, u)| (k.clone(), resolve(k, u)))
        .collect();
    let literal_through: Vec<(String, BTreeMap<String, String>)> = literals
        .iter()
        .map(|(k, u)| (k.clone(), resolve(k, u)))
        .collect();
    for (k, t) in literal_through {
        if let Some(u) = literals.get_mut(&k) {
            u.through = t;
        }
    }
    for (k, t) in unit_through {
        if let Some(u) = units.get_mut(&k) {
            u.through = t;
        }
    }

    // A literal whose reasons outside `through` its holder has too: the holder's report counts
    // the literal's body as its own, so those reasons are deltas of the holder already.
    for (k, u) in literals.iter_mut() {
        let holder = k.find(".lambda@").map(|at| &k[..at]);
        u.covered_by = holder
            .and_then(|h| units.get(h).map(|hu| (h, hu)))
            .filter(|(_, hu)| {
                u.unknown
                    .iter()
                    .filter(|r| !u.through.contains_key(*r))
                    .all(|r| hu.unknown.contains(r))
            })
            .map(|(h, _)| h.to_owned());
    }

    // The literals with a reason of their own that no holder reports: one outside `through`,
    // then any that reaches one.
    let mut own: BTreeSet<String> = literals
        .iter()
        .filter(|(_, u)| u.unknown.len() != u.through.len() && u.covered_by.is_none())
        .map(|(k, _)| k.clone())
        .collect();
    loop {
        let more: Vec<String> = literals
            .iter()
            .filter(|(k, u)| !own.contains(*k) && u.through.values().any(|c| own.contains(c)))
            .map(|(k, _)| k.clone())
            .collect();
        if more.is_empty() {
            break;
        }
        own.extend(more);
    }
    let inherited = |u: &Unit| -> BTreeSet<String> {
        u.through
            .iter()
            .filter(|(_, callee)| !own.contains(*callee))
            .map(|(reason, _)| reason.clone())
            .collect()
    };
    for u in units.values_mut().chain(literals.values_mut()) {
        u.inherited = inherited(u);
    }
}

/// Of each unit and literal of `facts`, the units and literals incomplete through it directly.
fn inheritors(facts: &Facts) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut out: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (caller, u) in facts.units.iter().chain(&facts.literals) {
        for callee in u.through.values().chain(&u.covered_by) {
            out.entry(callee.as_str())
                .or_default()
                .insert(caller.as_str());
        }
    }
    out
}

/// The units incomplete through `unit`, from [`inheritors`] of the revision `facts`: every unit
/// or literal incomplete through it, and every one incomplete through those, to a fixed point;
/// the units of them sorted, without `unit` itself.
fn callers(facts: &Facts, inheritors: &BTreeMap<&str, BTreeSet<&str>>, unit: &str) -> Vec<String> {
    let mut found: BTreeSet<&str> = BTreeSet::new();
    let mut queue = vec![unit];
    while let Some(callee) = queue.pop() {
        for caller in inheritors.get(callee).into_iter().flatten() {
            if *caller != unit && found.insert(caller) {
                queue.push(caller);
            }
        }
    }
    found
        .into_iter()
        .filter(|k| facts.units.contains_key(*k))
        .map(str::to_owned)
        .collect()
}

/// The deltas from the revision `old` to the revision `new`, each read as one program. An entry
/// of `opts` or a module of `opts.exported` that a revision does not hold is refused, naming the
/// revision.
pub fn diff(old: &[Module], new: &[Module], opts: &Options) -> Result<Report, String> {
    let a = facts(old, opts).map_err(|e| format!("old revision: {e}"))?;
    let b = facts(new, opts).map_err(|e| format!("new revision: {e}"))?;
    let (inheritors_a, inheritors_b) = (inheritors(&a), inheritors(&b));
    let mut deltas = Vec::new();
    let delta = |kind: &str, unit: &str| Delta {
        kind: kind.into(),
        unit: unit.into(),
        site: None,
        value: None,
        old: None,
        new: None,
        callers: Vec::new(),
    };
    let with_value = |kind: &str, unit: &str, value: &str| Delta {
        value: Some(value.into()),
        ..delta(kind, unit)
    };
    let sets = |deltas: &mut Vec<Delta>,
                unit: &str,
                [added, removed]: [&str; 2],
                old: &BTreeSet<String>,
                new: &BTreeSet<String>| {
        deltas.extend(new.difference(old).map(|v| with_value(added, unit, v)));
        deltas.extend(old.difference(new).map(|v| with_value(removed, unit, v)));
    };
    for unit in a.units.keys().filter(|u| !b.units.contains_key(*u)) {
        deltas.push(delta("unit-removed", unit));
    }
    for (unit, n) in &b.units {
        let Some(o) = a.units.get(unit) else {
            deltas.push(delta("unit-added", unit));
            continue;
        };
        if let (Some(old), Some(new)) = (&o.signature, &n.signature)
            && old != new
        {
            deltas.push(Delta {
                old: Some(old.clone()),
                new: Some(new.clone()),
                ..delta("signature", unit)
            });
        }
        sets(
            &mut deltas,
            unit,
            ["effect-added", "effect-removed"],
            &o.effects,
            &n.effects,
        );
        for (kind, from, to) in [
            ("syscall-added", &o.syscalls, &n.syscalls),
            ("syscall-removed", &n.syscalls, &o.syscalls),
        ] {
            deltas.extend(to.difference(from).map(|(site, name)| Delta {
                site: site.clone(),
                ..with_value(kind, unit, name)
            }));
        }
        sets(
            &mut deltas,
            unit,
            ["raise-added", "raise-removed"],
            &o.raises,
            &n.raises,
        );
        match (o.reached, n.reached) {
            (false, true) => deltas.push(delta("reach-gained", unit)),
            (true, false) => deltas.push(delta("reach-lost", unit)),
            _ => {}
        }
        // A reason the function inherits is a delta only when a call of its own that leads to
        // the callee exists in that revision alone; its callee's delta lists it as a caller.
        for (kind, from, to, facts, inheritors) in [
            ("unknown-added", o, n, &b, &inheritors_b),
            ("unknown-removed", n, o, &a, &inheritors_a),
        ] {
            for reason in to.unknown.difference(&from.unknown) {
                if to.inherited.contains(reason) {
                    let callee = codespec_effects::Unknown::subject(reason).unwrap_or_default();
                    let own_call_changed = to
                        .calls
                        .iter()
                        .any(|c| c.callee == callee && !from.calls.contains(c));
                    if !own_call_changed {
                        continue;
                    }
                }
                deltas.push(Delta {
                    callers: callers(facts, inheritors, unit),
                    ..with_value(kind, unit, reason)
                });
            }
        }
    }
    for (unit, value) in b.entries.difference(&a.entries) {
        deltas.push(with_value("entry-added", unit, value));
    }
    for (unit, value) in a.entries.difference(&b.entries) {
        deltas.push(with_value("entry-removed", unit, value));
    }
    deltas.sort_by(|x, y| {
        (&x.unit, &x.kind, &x.site, &x.value).cmp(&(&y.unit, &y.kind, &y.site, &y.value))
    });
    Ok(Report {
        format: FORMAT.into(),
        dispatch: match opts.dispatch {
            DispatchMode::Cha => "cha".into(),
            DispatchMode::Rta => "rta".into(),
        },
        deltas,
        unknowns: Vec::new(),
        labels: old
            .iter()
            .chain(new)
            .map(|m| label(m).to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    })
}
