//! The command line of `codespec`, as a clap definition, and the `measure` report. The binary
//! (`src/main.rs`) runs it; the documentation generator (`codespec-docs`) walks it for the CLI
//! reference.

pub mod edit;
pub mod eval;
pub mod measure;
pub mod query;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// The `codespec` command line.
#[derive(Parser)]
#[command(
    name = "codespec",
    version,
    about = "Read code into cir/1, check it, write code from it"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Cmd,
}

/// One `codespec` subcommand.
#[derive(Subcommand)]
pub enum Cmd {
    /// Validate a cir/1 document; with --canonical, also require its canonical encoding.
    Validate {
        /// The cir/1 document to read.
        file: PathBuf,
        /// Also require the file's bytes to be the document's canonical encoding.
        #[arg(long)]
        canonical: bool,
    },
    /// Print the JSON Schema of cir/1.
    Schema,
    /// Print the cir/1 feature vocabulary (codespec-features/1): every feature id an adapter can
    /// support, sorted by id.
    Features,
    /// Run the reader adapter for LANG over SOURCE and write the validated cir/1 module to OUT.
    Read {
        /// The source language; selects the adapter `codespec-read-<lang>`.
        #[arg(long)]
        lang: String,
        /// The source the adapter reads: for Go, one package directory.
        source: PathBuf,
        /// Where the validated cir/1 module is written, in canonical encoding.
        #[arg(long)]
        out: PathBuf,
        /// Run the adapter with --spans and also write the module's codespec-spans/1 document
        /// here, strictly read against the module and the source files; the cir/1 module written
        /// to OUT is the one a read without --spans gives. A reader without spans is refused. A
        /// span or unspanned entry whose `at` names no node of the module is dropped first, with
        /// one line on standard error each, in document order: "dropped span <at> <role>: names
        /// no node of the cir/1 document <module>"; the rest is read strictly.
        #[arg(long)]
        spans: Option<PathBuf>,
        /// Directory holding the adapter programs `codespec-read-<lang>` and `codespec-write-<lang>`.
        #[arg(long, env = "CODESPEC_ADAPTERS")]
        adapters: PathBuf,
    },
    /// Apply every pass (lookup-guard, then the facts) to a cir/1 module and write the result to OUT.
    Pass {
        /// The cir/1 module to rewrite.
        file: PathBuf,
        /// Where the rewritten module is written, in canonical encoding.
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate a cir/1 module and run the writer adapter for LANG, writing into OUT.
    Write {
        /// The target language; selects the adapter `codespec-write-<lang>`.
        #[arg(long)]
        lang: String,
        /// The cir/1 module to write.
        file: PathBuf,
        /// The directory the adapter writes into.
        #[arg(long)]
        out: PathBuf,
        /// Directory holding the adapter programs `codespec-read-<lang>` and `codespec-write-<lang>`.
        #[arg(long, env = "CODESPEC_ADAPTERS")]
        adapters: PathBuf,
    },
    /// Run the conformance kit against the adapter codespec-ROLE-LANG, by the claim its --describe
    /// manifest prints, and write a codespec-conformance-report/1 to OUT; exit 1 when it does not
    /// conform. With --check REPORT, run again for the report's adapter and exit 1 naming every
    /// feature whose entry differs.
    Conform {
        /// The adapter's language; selects codespec-ROLE-LANG and the runner codespec-run-LANG.
        #[arg(long, required_unless_present = "check", conflicts_with = "check")]
        lang: Option<String>,
        /// reader or writer.
        #[arg(
            long,
            value_parser = ["reader", "writer"],
            required_unless_present = "check",
            conflicts_with = "check"
        )]
        role: Option<String>,
        /// Where the report is written; a missing directory is created.
        #[arg(long, required_unless_present = "check", conflicts_with = "check")]
        out: Option<PathBuf>,
        /// A report to check: run again and compare.
        #[arg(long)]
        check: Option<PathBuf>,
        /// The kit directory: kit.json and cases/.
        #[arg(long, default_value = "conformance")]
        kit: PathBuf,
        /// A reader's corpus: one directory per kit case, its source and calls.json. Defaults to
        /// KIT/reader/LANG.
        #[arg(long)]
        corpus: Option<PathBuf>,
        /// Directory holding the adapter, its runner and, for a reader, codespec-write-go and
        /// codespec-run-go, and codespec-write-cir and codespec-run-cir, which judge a case the Go
        /// writer refuses.
        #[arg(long, env = "CODESPEC_ADAPTERS")]
        adapters: PathBuf,
        /// Where each case is written, read and built; emptied first. Defaults to
        /// target/conform/ROLE-LANG.
        #[arg(long)]
        work: Option<PathBuf>,
        /// Seconds one program (adapter, runner) may run before it is killed and the case
        /// recorded as `timeout`.
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    /// Measure a reader's output: counts read and refused functions per artefact by refusal
    /// category, then the totals and the share read.
    ///
    /// An artefact is a source file, the part of a position before its last `:`. A function is
    /// read when its body holds no `foreign` statement and refused when it holds one; a refusal
    /// outside every function counts under its file and category, and is a refused function too
    /// when it refuses a method's declaration (a call shape `T(…)` or `T.m(…)`, such as a Groovy
    /// hook `T.methodMissing(…)`). A constructor `New<T>` a JVM reader made at its class's position is not
    /// counted. The category is the text of the refusal's reason before the first `: ` when it is
    /// one of the Groovy reader's eight (dynamic, closure, groovy-semantics, type, unresolved,
    /// gorm, spring, unsupported), and uncategorised otherwise. Files and categories are in byte
    /// order; the same document gives the same bytes.
    Measure {
        /// The module to measure: a cir/1 document.
        module: PathBuf,
        /// text, a table with one row per artefact; or json, codespec-measure/1 in canonical
        /// encoding.
        #[arg(long, value_parser = ["text", "json"], default_value = "text")]
        format: String,
    },
    /// Report what each function of one module, or of several read as one program, may do outside
    /// its own memory: per function, its effects (db, env, fs.read, fs.write, io, net, proc, rand,
    /// stdin, stdout, time), with a barrier the syscalls it reaches, where each comes from, and
    /// every reason that set may be incomplete, as codespec-effects-report/1.
    ///
    /// A function's effects are those of every extern it calls, as the summaries state them, of
    /// println (stdout), of every syscall it reaches, and of every function or method it calls,
    /// to a fixed point over all modules given. An extern naming the import path of a module
    /// given is a call of that module's function; a call through an interface or a function
    /// value reaches every function of the set that could stand there. An extern no summary
    /// names, a foreign statement whose calls are not listed, a method on a type no module
    /// declares, a function without a body the barrier does not resolve, and a call of a
    /// function that has one of these, are each listed as unknown with a position, never taken
    /// as pure. The same modules, summaries and barrier give the same bytes.
    Effects {
        /// The modules to analyse, cir/1 documents, read as one program.
        #[arg(required = true)]
        modules: Vec<PathBuf>,
        /// A codespec-effects/1 summary of functions outside the modules; repeat for several.
        /// Where two name one function, the first counts.
        #[arg(long = "summaries", value_name = "FILE")]
        summaries: Vec<PathBuf>,
        /// A codespec-barrier/1 document: the functions that enter the operating system and
        /// what each syscall does.
        #[arg(long, value_name = "FILE")]
        barrier: Option<PathBuf>,
        /// Report only the module with this import path or name; repeat for several. All when
        /// absent.
        #[arg(long = "report", value_name = "MODULE")]
        report: Vec<String>,
        /// Also count, per reported function, the unknown reasons of every function it reaches.
        #[arg(long)]
        roots: bool,
        /// An entry function, `<path>.<name>` or `<path>.<type>.<method>`; repeat for several.
        /// With an entry, dispatch defaults to rta.
        #[arg(long = "entry", value_name = "FUNC")]
        entry: Vec<String>,
        /// A module, by import path or name, whose every exported function is an entry; repeat
        /// for several.
        #[arg(long = "exported", value_name = "MODULE")]
        exported: Vec<String>,
        /// How a call through an interface or a function value is resolved: cha, every
        /// candidate of the set (the default without entries); rta, only the types the code
        /// reached from the entries makes and the function values it uses (the default with
        /// entries).
        #[arg(long, value_parser = ["cha", "rta"])]
        dispatch: Option<String>,
    },
    /// List, per module, the functions the entry functions reach and those they do not, as
    /// codespec-reach-report/1: the tree-shaking result and the dead-code report.
    ///
    /// A function is reached by a call, by being used as a value, or as a candidate of a call
    /// through an interface or a function value. The entries are every --entry, every exported
    /// function of each --exported module, or when neither is given `main.main`. The Go init
    /// functions and variable initializers of the packages a program imports, directly or
    /// through other packages of the set (its recorded `imports`), are entries too, for each
    /// entry that program holds; an entry no program holds, or a set without `imports`, takes
    /// those of every module. The report's `roots` says which, per entry: `program` or `all`.
    /// An entry naming no function of the set is refused. The same modules give the same bytes.
    Reach {
        /// The modules to analyse, cir/1 documents, read as one program.
        #[arg(required = true)]
        modules: Vec<PathBuf>,
        /// An entry function, `<path>.<name>` or `<path>.<type>.<method>`; repeat for several.
        #[arg(long = "entry", value_name = "FUNC")]
        entry: Vec<String>,
        /// A module, by import path or name, whose every exported function is an entry.
        #[arg(long = "exported", value_name = "MODULE")]
        exported: Vec<String>,
        /// cha: every candidate of the set; rta (default): only types made and function values
        /// used by reached code.
        #[arg(long, value_parser = ["cha", "rta"], default_value = "rta")]
        dispatch: String,
        /// List only the module with this import path or name; repeat for several.
        #[arg(long = "report", value_name = "MODULE")]
        report: Vec<String>,
    },
    /// Compare the code facts of one program at two revisions and print what changed, as
    /// codespec-diff-report/1: functions added and removed, signatures, effects, syscalls by call
    /// site, raises, reach, entries and unknown reasons. Exit 0 with no delta, 1 with deltas, 2 on
    /// an input error.
    ///
    /// Each directory's *.cir.json documents are read strictly as one program; a directory with
    /// none, a document strict reading refuses and two documents of one directory with the same
    /// module path are input errors. Both revisions are analysed as `codespec effects` and
    /// `codespec reach` analyse them, with the same options. The entries of a revision are every
    /// --entry, every exported function of each --exported module and every function its
    /// documents record as an entry; without --entry and --exported, main.main too; and the Go init
    /// functions and variable initializers that run with them, as `codespec reach` takes them. A
    /// delta names a function as `<module path>.<key>` and a call by its call-site id
    /// `<callee>@<n>`, never by a file or a line: a change that only moves lines gives no delta.
    /// The same inputs give the same bytes.
    /// The format is the crate doc of b10x-codespec-diff.
    Diff {
        /// The old revision: a directory of cir/1 documents.
        old: PathBuf,
        /// The new revision: a directory of cir/1 documents.
        new: PathBuf,
        /// A codespec-effects/1 summary of functions outside the modules; repeat for several.
        /// Where two name one function, the first counts.
        #[arg(long = "summaries", value_name = "FILE")]
        summaries: Vec<PathBuf>,
        /// A codespec-barrier/1 document: the functions that enter the operating system and
        /// what each syscall does.
        #[arg(long, value_name = "FILE")]
        barrier: Option<PathBuf>,
        /// An entry function, `<path>.<name>` or `<path>.<type>.<method>`, in both revisions;
        /// repeat for several.
        #[arg(long = "entry", value_name = "FUNC")]
        entry: Vec<String>,
        /// A module, by import path or name, whose every exported function is an entry; repeat
        /// for several.
        #[arg(long = "exported", value_name = "MODULE")]
        exported: Vec<String>,
        /// cha: every candidate of the set; rta (default): only types made and function values
        /// used by reached code.
        #[arg(long, value_parser = ["cha", "rta"], default_value = "rta")]
        dispatch: String,
        /// Report only the functions of the module with this import path or name; repeat for
        /// several. All when absent.
        #[arg(long = "report", value_name = "MODULE")]
        report: Vec<String>,
        /// text, one line per delta; or json, codespec-diff-report/1.
        #[arg(long, value_parser = ["text", "json"], default_value = "text")]
        format: String,
    },
    /// Print the call tree from one function as codespec-trace/1: its statements, each if, switch
    /// and loop a branch node with its condition, each call expanded into the callee's tree, a
    /// call through an interface or a function value a dispatch node listing every candidate,
    /// syscalls and summarised externs at the leaves.
    ///
    /// Calls are resolved as `codespec effects` resolves them, over all modules given, with
    /// --from the only entry for rta. Each function is expanded once, where it is called fewest
    /// calls from --from; every other call of it is a ref node. A function more than --depth calls
    /// from --from is a cut node. Whatever `codespec effects` lists as unknown is an unknown node
    /// where it occurs. --from naming no function of the set is refused. The same modules,
    /// summaries and barrier give the same bytes.
    Trace {
        /// The modules to analyse, cir/1 documents, read as one program.
        #[arg(required = true)]
        modules: Vec<PathBuf>,
        /// The function the tree starts from, `<path>.<name>` or `<path>.<type>.<method>`.
        #[arg(long, value_name = "FUNC")]
        from: String,
        /// How many calls deep functions are expanded.
        #[arg(long, default_value_t = 8)]
        depth: usize,
        /// A codespec-effects/1 summary of functions outside the modules; repeat for several.
        #[arg(long = "summaries", value_name = "FILE")]
        summaries: Vec<PathBuf>,
        /// A codespec-barrier/1 document: the functions that enter the operating system and
        /// what each syscall does.
        #[arg(long, value_name = "FILE")]
        barrier: Option<PathBuf>,
        /// cha: every candidate of the set; rta (default): only types made and function values
        /// used by code reached from --from.
        #[arg(long, value_parser = ["cha", "rta"], default_value = "rta")]
        dispatch: String,
    },
    /// Run one built-in defect check over modules read as one program and print one finding per
    /// line: position, function, check and the evidence it matched (the calls and their
    /// positions). Exit 0 with no finding, 1 with findings, 2 on an input error.
    ///
    /// A check reads only cir/1: read bodies, the call lists of foreign statements and opaque
    /// functions, method sets and types. --list prints every check, one line on what it finds,
    /// and the name lists it matches calls against; --names replaces lists. What each check
    /// matches exactly is the crate doc of b10x-codespec-check.
    Check {
        /// The check to run; --list names them.
        #[arg(required_unless_present = "list")]
        check: Option<String>,
        /// The modules to check, cir/1 documents, read as one program.
        #[arg(required_unless_present = "list")]
        modules: Vec<PathBuf>,
        /// Print every check, what it finds and its name lists, and exit.
        #[arg(long, conflicts_with = "check")]
        list: bool,
        /// text, one line per finding; or json, codespec-check-report/1.
        #[arg(long, value_parser = ["text", "json"], default_value = "text")]
        format: String,
        /// insecure-random: report every call into a random package.
        #[arg(long)]
        strict: bool,
        /// A JSON object of list name to an array of entries, each replacing that list.
        #[arg(long, value_name = "FILE")]
        names: Option<PathBuf>,
        /// Report only findings in the module with this import path or name, or under it;
        /// repeat for several. All when absent.
        #[arg(long = "report", value_name = "MODULE")]
        report: Vec<String>,
        /// unreached: an entry function, `<path>.<name>` or `<path>.<type>.<method>`; repeat for
        /// several.
        #[arg(long = "entry", value_name = "FUNC")]
        entry: Vec<String>,
        /// unreached: a module, by import path or name, whose every exported function is an
        /// entry; repeat for several.
        #[arg(long = "exported", value_name = "MODULE")]
        exported: Vec<String>,
    },
    /// Answer a question over modules read as one program, written in cq, a typed Datalog with
    /// named-field record patterns over the fact base --schema prints, and print one row per
    /// answer. Exit 0 with no row, 1 with rows, 2 on an error.
    ///
    /// The fact base is built once per set of documents and cached as <digest>.facts in
    /// $CODESPEC_CACHE_DIR, else $XDG_CACHE_HOME/codespec/facts, else ~/.cache/codespec/facts,
    /// the digest a SHA-256 of the documents and the codespec version; never in or under a
    /// document's directory, and read only after the documents are validated. Rules
    /// may recurse, negate (stratified) and aggregate (count, min, max, sum, collect). A recursive
    /// rule that makes a value by arithmetic or a builtin from a value the recursion derived is
    /// refused, unless the value is found in a relation outside the recursion or is a bounded
    /// step: D + c (c a positive integer) with D, or the value made, bounded from above by a fact
    /// outside the recursion or a literal (D < M, D <= M, M > D, M >= D), or D - c bounded from
    /// below (D > M, D >= M, M < D, M <= D); a depth limit is reach(F, H, D2) :- reach(F, G, D),
    /// depth_limit(M), D < M, ..., D2 = D + 1. A row that
    /// rests on a negation crossing code the fact base cannot see (an opaque function, a foreign
    /// statement without a call list, an unresolved call) is marked possible, with the unknown
    /// sites. Path patterns and properties are refused.
    Query(query::Args),
    /// Change source code with a cq edit program: each row of an `edit` item's body is one
    /// operation (rename, rename_field, retarget, delete), planned on cir/1 and applied to the
    /// source by its spans, over Rust and Go. A dry run prints a unified diff (paths relative to
    /// the current directory); --apply writes it; --format json prints codespec-edit-report/1.
    /// Exit 0 when every row is planned and verified (and written with --apply), or there is no
    /// row; 1 when a row is refused, verification fails or a file changed since it was read,
    /// each reason on standard error, nothing written; 2 on an error (the program, a reader, an
    /// input).
    ///
    /// Each directory is read with `codespec-read-<lang> --spans`, the reader started in it; the
    /// fact base is built over the set as `codespec query` builds it, with source_name from the
    /// span documents. A possible row is refused with the unknown sites it rests on; so is a row
    /// whose target is exported, without --closed-world, and a row the reader could not see
    /// whole (a call list or foreign source naming the target). Before anything is printed, the
    /// root the reader resolves against (Rust: the Cargo workspace root, the nearest directory
    /// above the crate whose Cargo.toml has [workspace], else the crate directory; Go: the
    /// directory of the go.mod above the package, a relative replace path in the copy's go.mod
    /// made absolute) is copied without .git and target into the cache directory
    /// ($CODESPEC_CACHE_DIR/edit, else $XDG_CACHE_HOME/codespec/edit, else
    /// ~/.cache/codespec/edit), the edited files are written there, the copied directories are
    /// read again, and the set read must equal the expected cir/1 byte for byte; the first
    /// differing path is printed otherwise. An edited Go file that was gofmt-clean when read is
    /// run through gofmt and its whitespace changes join the diff; a gofmt change outside
    /// whitespace refuses the set. With --build the checked copy is then compiled (Go: go build
    /// ./... and go vet ./... over the go.mod root with GOWORK=off; Rust: cargo check
    /// --all-targets with the target directory inside the copy); a failure is status unverified
    /// with the first 20 lines of the compiler's output. With --apply each file is then written
    /// through a temporary file in its directory and a rename, only when every file read still
    /// has the SHA-256 it was read with. The report holds the set's status (no-row, planned,
    /// applied, refused, unverified, stale), each row's status and reasons, each edited file's
    /// SHA-256 before and after, and the diff; no time. A reader that gives no spans is refused,
    /// naming it.
    Edit(edit::Args),
    /// Run the calls of a codespec-calls/1 file against a cir/1 module with the evaluator and
    /// print one line per call, as the conformance kit's runners print them. Exit 0 when every
    /// call is made, 1 when the run stops (a foreign statement executed, a raise reached, an
    /// extern without a built-in handler, a panic, the step budget), naming the node by its id on
    /// standard error,
    /// 2 when the module or the calls are refused. Limits: calls nest at most 1000 deep, and
    /// statements and expressions at most 10,000 levels counted through calls; a type
    /// nests at most 100 deep inside a value; a zero value takes at most 4,194,304 cells; a
    /// module calling an undeclared function, with a wrong argument count, naming an undeclared
    /// type or a type holding itself, or an argument outside its parameter's width, is refused.
    ///
    /// With --session instead of --calls, the calls of a codespec-session/1 file run in order,
    /// reusing results earlier calls bound, and standard output is the run's trace,
    /// codespec-eval-trace/1: one JSON object per call, return, effect, panic and stop. A panic
    /// ends its call, not the session. Externs go to their handlers: os.Getenv reads only the
    /// --env given; an extern without one stops the run unless a --stub answers for it, and
    /// standard error names every site a stub answered at.
    ///
    /// A fault plan, --fault or --faults (codespec-faults/1), makes the effect at a node id (an
    /// extern call or a send) fail at a call count, before a stub or handler answers: error,
    /// value, closed, block or panic. The trace says where it fired, what its call left changed
    /// in older memory, and whether its error was propagated, kept, handled or dropped, or live
    /// when the run stopped. --each runs the
    /// session once per site of the --sites file and call count up to --max-nth, a send closed,
    /// a call returning an error given one, another call panicking, and writes each trace into
    /// --out, printing its path; exit 1 when a run stopped.
    Eval {
        /// The cir/1 module to run.
        module: PathBuf,
        /// The calls to make, a codespec-calls/1 document.
        #[arg(long, required_unless_present = "session", conflicts_with = "session")]
        calls: Option<PathBuf>,
        /// The session to run, a codespec-session/1 document; prints its trace.
        #[arg(long)]
        session: Option<PathBuf>,
        /// A variable of the run's environment, KEY=VALUE; repeat for several. The run sees no
        /// other.
        #[arg(long, value_name = "KEY=VALUE")]
        env: Vec<String>,
        /// A stub answering in place of a handler: SITE=VALUES, SITE an extern's key (every call
        /// of it) or a call's node id (that call), VALUES one JSON value per result, separated by
        /// spaces (null for a nil error); repeat for several.
        #[arg(long, value_name = "SITE=VALUES")]
        stub: Vec<String>,
        /// A fault: SITE:NTH=DO, SITE an effect's node id, NTH a call count from 1 or *, DO
        /// error, value, closed, block or panic, a JSON message or values after it; repeat for
        /// several sites.
        #[arg(long, value_name = "SITE:NTH=DO", requires = "session")]
        fault: Vec<String>,
        /// A fault plan, a codespec-faults/1 document.
        #[arg(long, requires = "session")]
        faults: Option<PathBuf>,
        /// Run the session once per site of --sites and call count up to --max-nth, each trace
        /// written into --out.
        #[arg(
            long,
            requires_all = ["session", "sites", "out"],
            conflicts_with_all = ["fault", "faults"]
        )]
        each: bool,
        /// The sites --each faults: one node id per line.
        #[arg(long, requires = "each")]
        sites: Option<PathBuf>,
        /// The last call count --each faults at each site.
        #[arg(long, default_value_t = 1, requires = "each")]
        max_nth: u64,
        /// The directory --each writes its traces into.
        #[arg(long, requires = "each")]
        out: Option<PathBuf>,
        /// The statements, calls and loop iterations the run may execute before it stops.
        #[arg(long, default_value_t = 10_000_000)]
        max_steps: u64,
    },
    /// Build every implementation EXAMPLE/harness.toml lists, run each on one generated trace per
    /// seed and require byte-equal output; exit 1 naming the seed, implementation and first
    /// differing line otherwise.
    Harness {
        /// The example directory holding harness.toml.
        example: PathBuf,
        /// How many seeds, 0 to SEEDS-1, each giving one trace.
        #[arg(long, default_value_t = 200)]
        seeds: u64,
        /// Commands per trace.
        #[arg(long, default_value_t = 300)]
        commands: usize,
        /// Directory the implementations are built into; `{build}` in harness.toml.
        #[arg(long, default_value = "target/harness")]
        build_dir: PathBuf,
    },
}
