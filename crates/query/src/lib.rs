//! cq, the query language of `codespec query`: a typed Datalog with named-field record patterns
//! over a fact base derived from a set of `cir/1` documents. The design is
//! `docs/query-language.md`; this crate holds § 4 (the fact base), § 5.1 (rules) and § 7 (the
//! engine). Path patterns (§ 5.2) and properties (§ 5.3) are refused by name.
//!
//! # The fact base
//!
//! [`FactBase::build`] computes every relation [`schema_text`] lists, read as one program, in an
//! order that does not depend on the order of the documents. Every statement and expression is a
//! node `<module path>.<function key>#<index path>`: the index path is the node's place in its
//! function's tree, statement 3, its `then` branch, statement 0, field `x`
//! (`net/http.Client.Do#3.then.0.x`); a list's element carries its index (`rhs.0`, `args.1`),
//! a call of a call list its place in the list (`#4.calls.0`, an opaque function's
//! `<function>#calls.0`). Positions are `<module path>/<file>:<line>`.
//!
//! [`load`] caches the fact base next to the first document as `.cq/<digest>.facts`, the digest
//! the SHA-256 of the codespec version and the SHA-256 of each document, sorted: the same
//! documents in any order give the same file, a changed one another. A second run reads the file
//! instead of the documents. [`load_with`] adds `codespec effects`' summaries and barrier, for
//! `effect` and `syscall`, to the input and to the digest.
//!
//! `reaches_path` ([`WAITING`]) is the witness of a path pattern: the schema names it as not
//! built yet, and a query naming it is refused.
//!
//! # Unknown
//!
//! What the fact base cannot see is a [`Site`] in `unknown`: an opaque function, a `foreign`
//! statement without a call list, a call of a function value or through an interface (`calls`
//! links neither); the same place in a document given twice is one site. Each site comes with
//! the facts it could hold: an opaque function could make any call, hold any statement and
//! expression; a foreign statement without a call list any call and expression; an unlinked call
//! could reach any function. A negated atom holds definitely when no fact satisfies it and none
//! of those facts would (the relation is evaluated a second time over them); when one of those
//! would, or when every fact that satisfies it only possibly holds, it holds possibly, resting on
//! those sites, and so does every row that rests on it, printed `possible: <site> at <pos>
//! (<reason>)`. A definite fact refutes it. An aggregate over facts that rest on sites rests on
//! them too.
//!
//! # The language
//!
//! The grammar is in `syntax.rs`. A rule is `head(X, …) :- body.`; a body joins atoms, positional
//! (`reaches(F, G)`, in schema order for a fact-base relation) or record patterns
//! (`call{func: F, callee: C}`: fields by name, any order, omitted fields free, `field: ~"re"` a
//! regular expression, `field: in @names.list` a names list), negated atoms (`!p(X)`),
//! conditions (`==`, `!=`, `<`, `<=`, `>`, `>=`, `=` which binds, `X ~ "re"`, `X in @names.l`,
//! `!(cond)`), arithmetic on integers (`+`, `-`, `*`), disjunctions `(a or b)`, a type's fields
//! as `T.bits`, aggregates `N = count{body}` and `M = min{X : body}` (`max`, `sum`, `collect`),
//! grouped by the variables the body shares with the rest of the rule (the body binds them
//! itself: it is evaluated on its own, `_` and every field a record pattern leaves out a variable
//! of its own, so `count{call{func: F, callee: C}}` counts the calls), and the builtins
//! `word(S, I, W)` (the words of a name as `codespec check` splits them), `int(S, N)`,
//! `replace(S, From, To, R)` and `concat(A, B, R)`. `names never-fail = ["…", …].` declares a
//! names list that [`Names`] (`--names`, the file `codespec check --names` reads) may replace;
//! `X in @names.l` holds for an entry equal to `X`, one ending in `*` that `X` starts with, or one
//! starting with `*` that `X` ends with, and with `X` unbound gives each entry as written. One
//! `?- body.` is the query: its rows are the bindings of its variables outside aggregates and
//! negations, in the order they first occur, sorted.
//!
//! Evaluation is bottom-up and semi-naive, stratum by stratum; a negation or an aggregate inside
//! a recursion is refused. [`explain`] prints the rules under the stratum of their heads.
//!
//! # Limits
//!
//! Hostile input ends in an error, never in a crash: parentheses, negated conditions,
//! aggregates and the operators of a term nest at most [`MAX_NESTING`] deep; a body holds at most
//! [`MAX_BODY`] literals; the disjunctions of one rule expand into at most [`MAX_BODIES`]
//! bodies; a regular expression's groups nest at most [`MAX_GROUPS`] deep, its counts are at most
//! [`MAX_COUNT`] and it compiles to at most [`MAX_PROGRAM`] instructions, matched by a Pike VM in
//! time linear in the string. Strata come from an SCC computed without recursion, so a chain of
//! rules of any length is safe; an index key holds any number of columns.
//!
//! # Gaps
//!
//! - `calls` links the static calls only: no dispatch candidates under CHA or RTA. `reached` is
//!   `codespec reach` with `rta` from every `main.main` and `init`.
//! - `--explain` prints the rules by stratum, not one derivation tree per finding (§ 7).
//! - The checks under `checks/` give `codespec check`'s findings on every fixture of
//!   `crates/check` they run on, but: `insecure-random` takes neither `--strict` nor `--report`;
//!   `unreached` neither `--entry` nor `--exported`, and its generated-file names are fixed in
//!   the check, not the list `generated`; `case-fold-asymmetry` and `clone-aliasing` do not
//!   follow an accessor of an opaque or foreign function. `lock-without-unlock` waits for
//!   properties (§ 5.3) and the `defer` statement (§ 8), as `checks/waiting.json` says.

mod engine;
mod facts;
mod graph;
mod regex;
mod sha256;
mod syntax;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use engine::MAX_BODIES;
pub use facts::{FACTS_FORMAT, FactBase, RELATIONS, WAITING, schema_text};
pub use regex::{MAX_COUNT, MAX_GROUPS, MAX_PROGRAM};
pub use syntax::{MAX_BODY, MAX_NESTING, Program};

/// The format of the JSON report.
pub const REPORT_FORMAT: &str = "codespec-query-report/1";

/// A value of a row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<Value>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::List(l) => {
                let items: Vec<String> = l.iter().map(Value::to_string).collect();
                write!(f, "[{}]", items.join(", "))
            }
        }
    }
}

/// A place the fact base cannot see: its node, its position and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub site: String,
    pub pos: String,
    pub reason: String,
}

/// Where [`load`] got the fact base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Built,
    Cache,
}

/// A fact base [`load`] built or read back.
pub struct Loaded {
    pub facts: FactBase,
    pub source: Source,
    pub digest: String,
    pub cache: PathBuf,
}

/// The cache key of a set of documents: the SHA-256 of the codespec version and each document's
/// SHA-256, sorted.
pub fn digest(documents: &[Vec<u8>]) -> String {
    digest_with(documents, &[], None)
}

/// The cache key of a set of documents read with effect summaries and a barrier: as [`digest`],
/// then `summary <SHA-256>` for each summary, sorted, and `barrier <SHA-256>`.
pub fn digest_with(documents: &[Vec<u8>], summaries: &[Vec<u8>], barrier: Option<&[u8]>) -> String {
    let mut each: Vec<String> = documents.iter().map(|d| sha256::hex(d)).collect();
    each.sort();
    let mut key = format!("{}\n{}\n", FACTS_FORMAT, env!("CARGO_PKG_VERSION"));
    for e in each {
        key += &e;
        key.push('\n');
    }
    let mut sums: Vec<String> = summaries.iter().map(|d| sha256::hex(d)).collect();
    sums.sort();
    for e in sums {
        key += &format!("summary {e}\n");
    }
    if let Some(b) = barrier {
        key += &format!("barrier {}\n", sha256::hex(b));
    }
    sha256::hex(key.as_bytes())
}

/// The fact base of the documents at `paths`: read from `<dir of the first>/.cq/<digest>.facts`
/// when it is there and current, else built from the documents (each read strictly) and written
/// there. A cache that cannot be written is left out.
pub fn load(paths: &[PathBuf]) -> Result<Loaded, String> {
    load_with(paths, &[], None)
}

/// [`load`], `effect` and `syscall` from the `codespec-effects/1` summaries at `summaries` and
/// the `codespec-barrier/1` barrier at `barrier`, each part of the cache key.
pub fn load_with(
    paths: &[PathBuf],
    summaries: &[PathBuf],
    barrier: Option<&Path>,
) -> Result<Loaded, String> {
    let first = paths.first().ok_or("no documents")?;
    let read = |p: &Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let mut docs = Vec::with_capacity(paths.len());
    for p in paths {
        docs.push(read(p)?);
    }
    let mut sums = Vec::with_capacity(summaries.len());
    let mut sum_bytes = Vec::with_capacity(summaries.len());
    for p in summaries {
        let bytes = read(p)?;
        sums.push(
            codespec_effects::summary_from_json(&bytes)
                .map_err(|e| format!("{}: {e}", p.display()))?,
        );
        sum_bytes.push(bytes);
    }
    let (bar, bar_bytes) = match barrier {
        Some(p) => {
            let bytes = read(p)?;
            let b = codespec_effects::barrier_from_json(&bytes)
                .map_err(|e| format!("{}: {e}", p.display()))?;
            (Some(b), Some(bytes))
        }
        None => (None, None),
    };
    let digest = digest_with(&docs, &sum_bytes, bar_bytes.as_deref());
    let dir = first
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), |d| d.to_path_buf())
        .join(".cq");
    let cache = dir.join(format!("{digest}.facts"));
    if let Ok(bytes) = std::fs::read(&cache)
        && let Ok(facts) = FactBase::from_bytes(&bytes)
    {
        return Ok(Loaded {
            facts,
            source: Source::Cache,
            digest,
            cache,
        });
    }
    let mut modules = Vec::with_capacity(docs.len());
    for (p, d) in paths.iter().zip(&docs) {
        modules.push(
            codespec_model::Module::from_json(d).map_err(|e| format!("{}: {e}", p.display()))?,
        );
    }
    let facts = FactBase::build_with(&modules, &sums, bar.as_ref());
    if std::fs::create_dir_all(&dir).is_ok() {
        let tmp = dir.join(format!("{digest}.facts.{}", std::process::id()));
        if std::fs::write(&tmp, facts.to_bytes()).is_ok() && std::fs::rename(&tmp, &cache).is_err()
        {
            let _ = std::fs::remove_file(&tmp);
        }
    }
    Ok(Loaded {
        facts,
        source: Source::Built,
        digest,
        cache,
    })
}

/// The names lists a names file replaces, by list name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    pub(crate) lists: BTreeMap<String, Vec<String>>,
}

impl Names {
    /// A names file: a JSON object of list name to an array of strings, as `codespec check --names`
    /// reads it. Each replaces the list of that name a module declares; a list the module does not
    /// declare is left alone, so one file serves every check.
    pub fn from_json(bytes: &[u8]) -> Result<Names, String> {
        let lists: BTreeMap<String, Vec<String>> = serde_json::from_slice(bytes)
            .map_err(|e| format!("names: an object of list name to an array of strings: {e}"))?;
        Ok(Names { lists })
    }
}

/// Parse a cq module. A path pattern (§ 5.2) or a property (§ 5.3) is refused, naming it.
pub fn parse(src: &str) -> Result<Program, String> {
    syntax::parse(src)
}

/// The program's rules in the order of their strata, then its query.
pub fn explain(p: &Program) -> Result<String, String> {
    engine::explain(p)
}

/// One row of an answer: its values, and the unknown sites it rests on (none: definite).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub values: Vec<Value>,
    pub unknown: Vec<Site>,
}

/// What a query gives: its variables and its rows, sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub vars: Vec<String>,
    pub rows: Vec<Row>,
}

impl Answer {
    /// One line per row: its values separated by tabs (`true` for a query without variables),
    /// then, for a possible row, `possible: <site> at <pos> (<reason>)`, sites joined by `; `.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for r in &self.rows {
            let mut cells: Vec<String> = r.values.iter().map(Value::to_string).collect();
            if cells.is_empty() {
                cells.push("true".into());
            }
            if !r.unknown.is_empty() {
                let sites: Vec<String> = r
                    .unknown
                    .iter()
                    .map(|s| format!("{} at {} ({})", s.site, s.pos, s.reason))
                    .collect();
                cells.push(format!("possible: {}", sites.join("; ")));
            }
            out += &cells.join("\t");
            out.push('\n');
        }
        out
    }

    /// `codespec-query-report/1`: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        use serde_json::{Value as J, json};
        fn j(v: &Value) -> J {
            match v {
                Value::Bool(b) => J::from(*b),
                Value::Int(n) => J::from(*n),
                Value::Str(s) => J::from(s.as_str()),
                Value::List(l) => J::Array(l.iter().map(j).collect()),
            }
        }
        let rows: Vec<J> = self
            .rows
            .iter()
            .map(|r| {
                let mut o = json!({
                    "values": r.values.iter().map(j).collect::<Vec<_>>(),
                    "certainty": if r.unknown.is_empty() { "definite" } else { "possible" },
                });
                if !r.unknown.is_empty() {
                    o["unknown"] = r
                        .unknown
                        .iter()
                        .map(|s| json!({"site": s.site, "pos": s.pos, "reason": s.reason}))
                        .collect();
                }
                o
            })
            .collect();
        let doc = json!({"format": REPORT_FORMAT, "vars": self.vars, "rows": rows});
        let mut s = serde_json::to_string_pretty(&doc).expect("a report always serializes");
        s.push('\n');
        s
    }
}

/// Run the query of `p` over `facts`, the names lists of `names` replacing the module's own.
pub fn run(p: &Program, facts: &FactBase, names: &Names) -> Result<Answer, String> {
    let c = engine::compile(p, names)?;
    Ok(engine::evaluate(&c, facts))
}
