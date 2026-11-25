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
//! too, as `reach` takes it; every Go `init` and every module's variable initializers, as `reach`
//! takes them.
//!
//! # The report, `codespec-diff-report/1`
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-diff-report/1` |
//! | `dispatch` | string | `cha` or `rta`, as both sides were analysed |
//! | `deltas` | array | one object per delta, sorted by `unit`, then `kind`, then `site` (`callee`, then `n`), then `value` |
//!
//! A delta is `{kind, unit, site, value, old, new}`; `site`, `value`, `old` and `new` are omitted
//! when they do not apply:
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
//! | `unknown-added`, `unknown-removed` | the function | `value`: an `unknown` reason of `codespec effects` (its text, without its position) present in one revision only |
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
//! `syscall-removed example.com/probe.Load syscall.Syscall@1 getpid`, and `old -> new` for a
//! signature.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use codespec_effects::{Barrier, CallSite, DispatchMode, FuncEffects, Summary};
use codespec_model::Module;
use serde::{Deserialize, Serialize};

/// The format of the report [`diff`] gives.
pub const FORMAT: &str = "codespec-diff-report/1";

/// A `codespec-diff-report/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub format: String,
    /// `cha` or `rta`.
    pub dispatch: String,
    pub deltas: Vec<Delta>,
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
}

impl Report {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Report always serializes");
        s.push('\n');
        s
    }

    /// One line per delta, its fields separated by one space, a missing one skipped; empty
    /// without a delta.
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
            out.push_str(&fields.join(" "));
            out.push('\n');
        }
        out
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
}

/// The facts of one revision: each unit's, and each entry as `(unit, value)`.
struct Facts {
    units: BTreeMap<String, Unit>,
    entries: BTreeSet<(String, String)>,
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
    let (report, at_load) = codespec_effects::analyze_at_load(modules, opts.summaries, &eopts);
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
    Ok(Facts { units, entries })
}

/// The deltas from the revision `old` to the revision `new`, each read as one program. An entry
/// of `opts` or a module of `opts.exported` that a revision does not hold is refused, naming the
/// revision.
pub fn diff(old: &[Module], new: &[Module], opts: &Options) -> Result<Report, String> {
    let a = facts(old, opts).map_err(|e| format!("old revision: {e}"))?;
    let b = facts(new, opts).map_err(|e| format!("new revision: {e}"))?;
    let mut deltas = Vec::new();
    let delta = |kind: &str, unit: &str| Delta {
        kind: kind.into(),
        unit: unit.into(),
        site: None,
        value: None,
        old: None,
        new: None,
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
        sets(
            &mut deltas,
            unit,
            ["unknown-added", "unknown-removed"],
            &o.unknown,
            &n.unknown,
        );
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
    })
}
