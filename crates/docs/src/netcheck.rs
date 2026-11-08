//! The worked static-analysis example `examples/netcheck`: a Go module with a TCP server and a
//! client, read with every package it imports and mapped down to syscalls.
//!
//! `task examples-netcheck` runs `codespec effects`, `reach` and `trace` over the linked set into a
//! work directory, then `codespec-docs netcheck --work <dir>` ([`reduce`]) writes the checked-in
//! files: the effects reports without `effect_sites` (and, for `cha`, without syscall positions),
//! the reach reports as printed, and the call trees, the deep ones pruned to the path that ends in
//! one syscall. The page ([`outputs`]) and the tests read only those checked-in files.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use codespec_effects::{
    FuncEffects, ReachReport, Report, Trace, TraceNode, barrier_from_json, trace_from_json,
};

use crate::showcase::{fence, read};
use crate::{Result, SOURCE, TREE, cell, front_matter};

/// Where the page lands, relative to the repository root.
pub const PAGE: &str = "website/docs/examples/netcheck.md";

const DIR: &str = "examples/netcheck";
const MODULE: &str = "example.com/netcheck";
const SERVER: &str = "example.com/netcheck/cmd/server";
const CLIENT: &str = "example.com/netcheck/cmd/client";
const BARRIER: &str = "summaries/go-linux-amd64.barrier.json";

pub const READ: &str = "examples/netcheck/read.txt";
pub const EFFECTS: &str = "examples/netcheck/netcheck.effects.json";
pub const EFFECTS_CHA: &str = "examples/netcheck/netcheck.effects-cha.json";
pub const REACH: &str = "examples/netcheck/netcheck.reach.json";
pub const REACH_CHA: &str = "examples/netcheck/netcheck.reach-cha.json";
pub const CLIENT_TRACE: &str = "examples/netcheck/client.trace.json";
pub const CLIENT_CONNECT: &str = "examples/netcheck/client-connect.trace.json";
pub const SERVER_ACCEPT: &str = "examples/netcheck/server-accept4.trace.json";

const SOURCES: [&str; 3] = [
    "examples/netcheck/source/netcheck.go",
    "examples/netcheck/source/cmd/server/main.go",
    "examples/netcheck/source/cmd/client/main.go",
];

/// `node` with only the branches that end in the syscall `name`; `None` when none does.
pub fn prune(node: &TraceNode, name: &str) -> Option<TraceNode> {
    let keep = |nodes: &[TraceNode]| -> Vec<TraceNode> {
        nodes.iter().filter_map(|n| prune(n, name)).collect()
    };
    let some = |v: Vec<TraceNode>, f: &dyn Fn(Vec<TraceNode>) -> TraceNode| {
        if v.is_empty() { None } else { Some(f(v)) }
    };
    match node {
        TraceNode::Syscall { name: s, .. } => (s == name).then(|| node.clone()),
        TraceNode::Call {
            pos,
            name: n,
            def,
            body,
        } => some(keep(body), &|body| TraceNode::Call {
            pos: pos.clone(),
            name: n.clone(),
            def: def.clone(),
            body,
        }),
        TraceNode::Loop { pos, cond, body } => some(keep(body), &|body| TraceNode::Loop {
            pos: pos.clone(),
            cond: cond.clone(),
            body,
        }),
        TraceNode::Lambda { pos, body } => some(keep(body), &|body| TraceNode::Lambda {
            pos: pos.clone(),
            body,
        }),
        TraceNode::Dispatch {
            pos,
            call,
            candidates,
            ..
        } => some(keep(candidates), &|candidates| TraceNode::Dispatch {
            pos: pos.clone(),
            call: call.clone(),
            candidates,
            unknown: Vec::new(),
        }),
        TraceNode::If {
            pos,
            cond,
            then,
            otherwise,
        } => {
            let (then, otherwise) = (keep(then), keep(otherwise));
            (!then.is_empty() || !otherwise.is_empty()).then(|| TraceNode::If {
                pos: pos.clone(),
                cond: cond.clone(),
                then,
                otherwise,
            })
        }
        TraceNode::Switch { pos, tag, cases } => {
            let cases: Vec<_> = cases
                .iter()
                .filter_map(|c| {
                    let body = keep(&c.body);
                    (!body.is_empty()).then(|| codespec_effects::TraceCase {
                        values: c.values.clone(),
                        body,
                    })
                })
                .collect();
            (!cases.is_empty()).then(|| TraceNode::Switch {
                pos: pos.clone(),
                tag: tag.clone(),
                cases,
            })
        }
        _ => None,
    }
}

fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

fn report(text: &str, what: &str) -> Result<Report> {
    serde_json::from_str(text).map_err(|e| format!("{what}: {e}"))
}

fn reach_report(text: &str, what: &str) -> Result<ReachReport> {
    serde_json::from_str(text).map_err(|e| format!("{what}: {e}"))
}

/// Write the checked-in files of `examples/netcheck` from the full outputs in `work`:
/// `effects.json`, `effects-cha.json`, `reach.json`, `reach-cha.json`, `client.trace.json`,
/// `client-deep.trace.json` and `server-deep.trace.json`.
pub fn reduce(root: &Path, work: &Path) -> Result<()> {
    let write = |relative: &str, content: String| {
        fs::write(root.join(relative), content).map_err(|e| format!("writing {relative}: {e}"))
    };
    let mut rta = report(&read_file(&work.join("effects.json"))?, "effects.json")?;
    for f in &mut rta.funcs {
        f.effect_sites.clear();
    }
    write(EFFECTS, rta.to_json())?;
    let mut cha = report(
        &read_file(&work.join("effects-cha.json"))?,
        "effects-cha.json",
    )?;
    for f in &mut cha.funcs {
        f.effect_sites.clear();
        for s in &mut f.syscalls {
            s.at.clear();
        }
    }
    write(EFFECTS_CHA, cha.to_json())?;
    for (from, to) in [("reach.json", REACH), ("reach-cha.json", REACH_CHA)] {
        write(
            to,
            reach_report(&read_file(&work.join(from))?, from)?.to_json(),
        )?;
    }
    let client = trace_from_json(read_file(&work.join("client.trace.json"))?.as_bytes())?;
    write(CLIENT_TRACE, client.to_json())?;
    for (from, syscall, to) in [
        ("client-deep.trace.json", "connect", CLIENT_CONNECT),
        ("server-deep.trace.json", "accept4", SERVER_ACCEPT),
    ] {
        let mut t = trace_from_json(read_file(&work.join(from))?.as_bytes())?;
        t.root =
            prune(&t.root, syscall).ok_or_else(|| format!("{from}: no path ends in {syscall}"))?;
        write(to, t.to_json())?;
    }
    Ok(())
}

/// The checked-in outputs, read.
pub struct Outputs {
    pub read: String,
    pub rta: Report,
    pub cha: Report,
    pub reach: ReachReport,
    pub reach_cha: ReachReport,
    pub client: Trace,
    pub connect: Trace,
    pub accept: Trace,
}

pub fn load(root: &Path) -> Result<Outputs> {
    let trace =
        |p: &str| trace_from_json(read(root, p)?.as_bytes()).map_err(|e| format!("{p}: {e}"));
    Ok(Outputs {
        read: read(root, READ)?,
        rta: report(&read(root, EFFECTS)?, EFFECTS)?,
        cha: report(&read(root, EFFECTS_CHA)?, EFFECTS_CHA)?,
        reach: reach_report(&read(root, REACH)?, REACH)?,
        reach_cha: reach_report(&read(root, REACH_CHA)?, REACH_CHA)?,
        client: trace(CLIENT_TRACE)?,
        connect: trace(CLIENT_CONNECT)?,
        accept: trace(SERVER_ACCEPT)?,
    })
}

/// The function `name` of the module `path` in `r`.
pub fn func<'a>(r: &'a Report, path: &str, name: &str) -> Result<&'a FuncEffects> {
    r.funcs
        .iter()
        .find(|f| f.path == path && f.name == name)
        .ok_or_else(|| format!("no {path}.{name} in the report"))
}

pub fn syscalls(f: &FuncEffects) -> BTreeSet<&str> {
    f.syscalls.iter().map(|s| s.name.as_str()).collect()
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "none".into()
    } else {
        items
            .iter()
            .map(|i| format!("`{i}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn short(path: &str) -> &str {
    path.strip_prefix("example.com/").unwrap_or(path)
}

fn page(root: &Path) -> Result<String> {
    let o = load(root)?;
    let barrier = barrier_from_json(read(root, BARRIER)?.as_bytes())
        .map_err(|e| format!("{BARRIER}: {e}"))?;
    let documents = o.read.lines().count();
    let own: Vec<&str> = o.read.lines().filter(|l| l.starts_with(MODULE)).collect();
    let mut out = front_matter(
        "netcheck: a TCP server and client mapped to syscalls",
        "netcheck: down to syscalls",
        "A small Go module with a TCP server and a client, read with every package it imports; codespec effects, reach and trace over the set, with the syscalls each entry may reach, rapid type analysis against class-hierarchy analysis, and the call paths to connect and accept4.",
        4,
    );
    out.push_str("# netcheck: down to syscalls\n\n");
    out.push_str(&format!(
        "Generated by `codespec-docs` from the checked-in files of [`{DIR}`]({TREE}/{DIR}). `task examples-netcheck` writes them (Go 1.27.1, linux/amd64, no network); `task check` holds the page to them and checks the facts below.\n\n"
    ));
    out.push_str("## The module\n\n`example.com/netcheck` has three packages and imports only the standard library.\n\n\
| package | what it shows |\n|---|---|\n\
| `netcheck` | an interface `Prober` with two implementations: `TCPProber` dials, `FixedProber` does no IO; a generic `Count[T any]` that calls a function value; a variadic `CheckAll`; a function literal; a module variable; `Answer`, which only computes |\n\
| `cmd/server` | entry `main`: `net.Listen`, then `Accept` in a loop, each connection answered with `netcheck.Answer` |\n\
| `cmd/client` | entry `main`: `netcheck.CheckAll(netcheck.TCP(), addr)`, prints how many are up |\n\n");
    for s in SOURCES {
        out.push_str(&fence(
            "go",
            s.trim_start_matches("examples/netcheck/source/"),
            &read(root, s)?,
        ));
        out.push('\n');
    }
    out.push_str(&format!(
        "## 1. Read the module and its imports\n\n```shell-session\n$ scripts/read-repo.sh --deps examples/netcheck/source work/netcheck\n```\n\n\
`go list -deps` names {documents} packages: the module's 3 and {} of the standard library. Each is one `cir/1` document; the set is linked by import path. The module's own lines of [`read.txt`]({SOURCE}/{READ}):\n\n```text\n{}\n```\n\n\
No refusal: interfaces, the generic function, the variadic call, the function literal and the module variable are all `cir/1`.\n\n",
        documents - 3,
        own.join("\n")
    ));

    out.push_str(&format!(
        "## 2. Effects and syscalls from two entries\n\n```shell-session\n$ codespec effects work/netcheck/*.cir.json --barrier {BARRIER} \\\n    --entry {SERVER}.main --entry {CLIENT}.main \\\n    --report {MODULE} --report {SERVER} --report {CLIENT}\n```\n\n\
With entries, dispatch is rapid type analysis (`rta`). The same run with `--dispatch cha` resolves every call through an interface or a function value over the whole set. [`netcheck.effects.json`]({SOURCE}/{EFFECTS}) and [`netcheck.effects-cha.json`]({SOURCE}/{EFFECTS_CHA}) hold both reports without `effect_sites`; the `cha` one also without syscall positions.\n\n\
| function | effects (`rta`) | syscalls `rta` | syscalls `cha` | unknown `rta` |\n|---|---|---:|---:|---:|\n"
    ));
    for f in &o.rta.funcs {
        let c = func(&o.cha, &f.path, &f.name)?;
        out.push_str(&format!(
            "| `{}.{}` | {} | {} | {} | {} |\n",
            short(&f.path),
            f.name,
            list(&f.effects),
            f.syscalls.len(),
            c.syscalls.len(),
            f.unknown.len()
        ));
    }
    let server = func(&o.rta, SERVER, "main")?;
    let client = func(&o.rta, CLIENT, "main")?;
    let (s, c) = (syscalls(server), syscalls(client));
    let net: Vec<&str> = barrier
        .syscalls
        .iter()
        .filter(|x| x.effects.iter().any(|e| e == "net"))
        .map(|x| x.name.as_str())
        .filter(|n| s.contains(n) || c.contains(n))
        .collect();
    out.push_str(&format!(
        "\n### The entries\n\nThe syscalls the barrier maps to `net`, per entry (`rta`):\n\n| syscall | `server.main` | `client.main` |\n|---|---|---|\n{}\n",
        net.iter()
            .map(|n| format!(
                "| `{n}` | {} | {} |\n",
                if s.contains(n) { "yes" } else { "no" },
                if c.contains(n) { "yes" } else { "no" }
            ))
            .collect::<String>()
    ));
    let only = |a: &BTreeSet<&str>, b: &BTreeSet<&str>| {
        let v: Vec<String> = a.difference(b).map(|x| x.to_string()).collect();
        list(&v)
    };
    out.push_str(&format!(
        "| measure | value |\n|---|---|\n| syscalls only the server may reach | {} |\n| syscalls only the client may reach | {} |\n| syscalls both may reach | {} |\n\n",
        only(&s, &c),
        only(&c, &s),
        s.intersection(&c).count()
    ));
    out.push_str("Read a list as *may reach*, not as what a run does. The client only dials, yet `bind` and `listen` are on its list: `net.Dial` goes through `net.socket`, which also binds and listens when it makes a listening socket, and the analysis does not know which branch a run takes.\n\n");

    let answer = func(&o.rta, MODULE, "Answer")?;
    out.push_str(&format!(
        "### The function that only computes\n\n`Answer` calls `strings.TrimSpace` and switches on the result. Read down to the barrier, it has {} effects, {} syscalls and {} `unknown` entries: pure, with nothing over-approximated.\n\n",
        answer.effects.len(),
        answer.syscalls.len(),
        answer.unknown.len()
    ));

    out.push_str("### Dispatch: `rta` against `cha`\n\nEach call through an interface or a function value in the module, and how many functions of the set it may reach:\n\n| function | call | at | `rta` | `cha` |\n|---|---|---|---:|---:|\n");
    for f in &o.rta.funcs {
        let c = func(&o.cha, &f.path, &f.name)?;
        for d in &f.dispatch {
            let other = c
                .dispatch
                .iter()
                .find(|x| x.pos == d.pos && x.call == d.call)
                .map(|x| x.candidates.to_string())
                .unwrap_or_else(|| "none".into());
            out.push_str(&format!(
                "| `{}.{}` | `{}` | `{}` | {} | {} |\n",
                short(&f.path),
                f.name,
                cell(&d.call),
                d.pos,
                d.candidates,
                other
            ));
        }
    }
    out.push_str("\n`Prober.Probe` in `CheckAll` reaches only `TCPProber.Probe` under `rta`: no reached code makes a `FixedProber`. `func value func(T) bool` in `Count` has no candidate: a value of a generic signature is a current limit, listed as `unknown`, never taken as pure.\n\n");

    let unknown: Vec<String> = o
        .rta
        .funcs
        .iter()
        .flat_map(|f| {
            f.unknown.iter().map(move |u| {
                format!(
                    "| `{}.{}` | `{}` | {} |\n",
                    short(&f.path),
                    f.name,
                    u.pos,
                    cell(&u.reason)
                )
            })
        })
        .filter(|l| !l.contains("which is incomplete"))
        .collect();
    out.push_str(&format!(
        "Every `unknown` entry of the module that is not inherited from a callee (`calls … which is incomplete`):\n\n| function | at | reason |\n|---|---|---|\n{}\n",
        unknown.concat()
    ));

    out.push_str(&format!(
        "## 3. Reached and unreached functions\n\n```shell-session\n$ codespec reach work/netcheck/*.cir.json --entry {SERVER}.main --entry {CLIENT}.main \\\n    --report {MODULE} --report {SERVER} --report {CLIENT}\n```\n\n\
| module | reached `rta` | unreached `rta` | unreached `cha` |\n|---|---|---|---|\n"
    ));
    for m in &o.reach.modules {
        let c = o
            .reach_cha
            .modules
            .iter()
            .find(|x| x.path == m.path)
            .ok_or_else(|| format!("{} not in {REACH_CHA}", m.path))?;
        out.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            short(&m.path),
            list(&m.reached),
            list(&m.unreached),
            list(&c.unreached)
        ));
    }
    out.push_str(&format!(
        "\n[`netcheck.reach.json`]({SOURCE}/{REACH}), [`netcheck.reach-cha.json`]({SOURCE}/{REACH_CHA}). `FixedProber.Probe` is dead code from these entries; class-hierarchy analysis keeps it, because `Prober.Probe` could reach it.\n\n"
    ));

    out.push_str(&format!(
        "## 4. Call trees\n\n```shell-session\n$ codespec trace work/netcheck/*.cir.json --barrier {BARRIER} --from {CLIENT}.main --depth {}\n```\n\n\
[Call trees](/docs/examples/call-trace#netcheck) draws the client's tree {} calls deep, and the path from each entry to one syscall: `client.main` to `connect` ({} calls deep), `server.main` to `accept4` ({} calls deep). The full trees are {}; the checked-in [`client-connect.trace.json`]({SOURCE}/{CLIENT_CONNECT}) and [`server-accept4.trace.json`]({SOURCE}/{SERVER_ACCEPT}) keep only the branches that end in that syscall.\n",
        o.client.depth, o.client.depth, o.connect.depth, o.accept.depth,
        "several megabytes"
    ));
    Ok(out)
}

pub fn outputs(root: &Path) -> Result<Vec<(&'static str, String)>> {
    Ok(vec![(PAGE, page(root)?)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn leaves(n: &TraceNode, out: &mut Vec<String>) {
        if let TraceNode::Syscall { name, .. } = n {
            out.push(name.clone());
        }
        for c in n.children() {
            leaves(c, out);
        }
        if let TraceNode::Switch { cases, .. } = n {
            for c in cases {
                for b in &c.body {
                    leaves(b, out);
                }
            }
        }
    }

    #[test]
    fn the_server_reaches_bind_listen_and_accept4_and_the_client_connect() {
        let o = load(&root()).unwrap();
        let server = syscalls(func(&o.rta, SERVER, "main").unwrap());
        let client = syscalls(func(&o.rta, CLIENT, "main").unwrap());
        for s in ["socket", "bind", "listen", "accept4"] {
            assert!(server.contains(s), "server.main lacks {s}");
        }
        assert!(client.contains("connect"), "client.main lacks connect");
        assert!(!client.contains("accept4"), "client.main reaches accept4");
        let diff: Vec<_> = server.difference(&client).copied().collect();
        assert_eq!(diff, ["accept4"]);
    }

    #[test]
    fn answer_only_computes() {
        let o = load(&root()).unwrap();
        for r in [&o.rta, &o.cha] {
            let a = func(r, MODULE, "Answer").unwrap();
            assert!(a.effects.is_empty() && a.syscalls.is_empty() && a.unknown.is_empty());
        }
    }

    #[test]
    fn rapid_type_analysis_reaches_fewer_candidates_than_class_hierarchy_analysis() {
        let o = load(&root()).unwrap();
        let probe = |r: &Report| {
            func(r, MODULE, "CheckAll")
                .unwrap()
                .dispatch
                .iter()
                .find(|d| d.call == "example.com/netcheck.Prober.Probe")
                .unwrap()
                .candidates
        };
        assert_eq!((probe(&o.rta), probe(&o.cha)), (1, 2));
        let client = |r: &Report| func(r, CLIENT, "main").unwrap().syscalls.len();
        assert!(client(&o.rta) < client(&o.cha));
        let unreached = |r: &ReachReport| {
            r.modules
                .iter()
                .find(|m| m.path == MODULE)
                .unwrap()
                .unreached
                .clone()
        };
        assert_eq!(unreached(&o.reach), ["FixedProber.Probe"]);
        assert!(unreached(&o.reach_cha).is_empty());
    }

    #[test]
    fn the_pruned_trees_end_in_their_syscall_and_the_module_reads_without_refusal() {
        let o = load(&root()).unwrap();
        for (t, s) in [(&o.connect, "connect"), (&o.accept, "accept4")] {
            let mut found = Vec::new();
            leaves(&t.root, &mut found);
            assert!(
                !found.is_empty() && found.iter().all(|x| x == s),
                "{found:?}"
            );
        }
        let own: Vec<&str> = o.read.lines().filter(|l| l.starts_with(MODULE)).collect();
        assert_eq!(own.len(), 3);
        assert!(
            own.iter()
                .all(|l| l.contains("| ok |") && l.ends_with("refusals 0"))
        );
    }

    #[test]
    fn prune_keeps_only_the_branches_that_end_in_the_syscall() {
        let t: Trace = serde_json::from_value(serde_json::json!({
            "format": "codespec-trace/1", "from": "m.F", "depth": 8, "dispatch": "rta",
            "root": {"k": "call", "pos": "m.go:1", "name": "m.F", "def": "m.go:1", "body": [
                {"k": "if", "pos": "m.go:2", "cond": "x > 0",
                 "then": [{"k": "syscall", "pos": "m.go:3", "name": "read", "effects": ["io"]}],
                 "else": [{"k": "syscall", "pos": "m.go:5", "name": "connect", "effects": ["net"]}]},
                {"k": "cut", "pos": "m.go:6", "name": "m.g"}
            ]}
        }))
        .unwrap();
        let p = prune(&t.root, "connect").unwrap();
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::json!({"k": "call", "pos": "m.go:1", "name": "m.F", "def": "m.go:1", "body": [
                {"k": "if", "pos": "m.go:2", "cond": "x > 0",
                 "else": [{"k": "syscall", "pos": "m.go:5", "name": "connect", "effects": ["net"]}]}
            ]})
        );
        assert!(prune(&t.root, "accept4").is_none());
    }
}
