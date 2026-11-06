//! The command line of `codespec`, as a clap definition. The binary (`src/main.rs`) runs it; the
//! documentation generator (`codespec-docs`) walks it for the CLI reference.

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
