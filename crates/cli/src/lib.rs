//! The command line of `codespec`, as a clap definition, and the `measure` report. The binary
//! (`src/main.rs`) runs it; the documentation generator (`codespec-docs`) walks it for the CLI
//! reference.

pub mod eval;
pub mod measure;

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
        /// codespec-run-go.
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
    /// function of each --exported module, or when neither is given `main.main`; every Go init
    /// function and every module's variable initializers are entries too. An entry naming no
    /// function of the set is refused. The same modules give the same bytes.
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
    /// Run the calls of a codespec-calls/1 file against a cir/1 module with the evaluator and
    /// print one line per call, as the conformance kit's runners print them. Exit 0 when every
    /// call is made, 1 when the run stops (a foreign statement executed, an extern without a
    /// built-in handler, a panic, the step budget), naming the node by its id on standard error,
    /// 2 when the module or the calls are refused.
    Eval {
        /// The cir/1 module to run.
        module: PathBuf,
        /// The calls to make, a codespec-calls/1 document.
        #[arg(long)]
        calls: PathBuf,
        /// The statements and calls the run may execute before it stops.
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
