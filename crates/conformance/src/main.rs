//! `codespec-conformance`: prints the generated `kit.json`, lists the features a `cir/1` module
//! uses, reads an adapter manifest, and checks or regenerates the cases of a kit (crate doc of
//! `codespec_conformance`).

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::{Parser, Subcommand};
use codespec_conformance::{
    Tools, check_case, check_manifest, features_used, kit_json, load_cases,
};
use codespec_model::Module;

#[derive(Parser)]
#[command(name = "codespec-conformance", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the codespec-conformance-kit/1 document generated from a feature vocabulary.
    Kit {
        /// The codespec-features/1 vocabulary.
        #[arg(long, default_value = "schemas/cir-1.features.json")]
        features: PathBuf,
    },
    /// Print the feature ids a cir/1 module uses, one per line.
    Features {
        /// The cir/1 module.
        module: PathBuf,
    },
    /// Read an adapter's codespec-adapter/1 manifest strictly and print the feature ids it claims,
    /// one per line.
    Manifest {
        /// The manifest, as the adapter's --describe printed it.
        manifest: PathBuf,
        /// A cir/1 module the adapter read; every claimed feature must be used by one of them.
        #[arg(long = "read")]
        read: Vec<PathBuf>,
    },
    /// Read, pass and run every case, comparing with its case.cir.json and expected.txt.
    Check {
        /// The kit directory, holding cases/ and driver/go/.
        #[arg(long, default_value = "conformance")]
        kit: PathBuf,
        /// The codespec binary.
        #[arg(long)]
        codespec: PathBuf,
        /// The directory holding codespec-read-go.
        #[arg(long)]
        adapters: PathBuf,
        /// Where each case is read, passed and built.
        #[arg(long, default_value = "target/conformance")]
        build_dir: PathBuf,
        /// Only the cases with these names.
        #[arg(long = "case")]
        cases: Vec<String>,
        /// Rewrite case.cir.json and expected.txt instead of comparing with them.
        #[arg(long)]
        write: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec-conformance: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Cmd::Kit { features } => {
            let bytes =
                std::fs::read(&features).map_err(|e| format!("{}: {e}", features.display()))?;
            print!("{}", kit_json(&bytes)?);
            Ok(())
        }
        Cmd::Features { module } => {
            let bytes = std::fs::read(&module).map_err(|e| format!("{}: {e}", module.display()))?;
            let m = Module::from_json(&bytes).map_err(|e| format!("{}: {e}", module.display()))?;
            for id in features_used(&m)? {
                println!("{id}");
            }
            Ok(())
        }
        Cmd::Manifest { manifest, read } => {
            let bytes =
                std::fs::read(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
            let mut modules = Vec::new();
            for p in &read {
                let b = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
                modules.push(Module::from_json(&b).map_err(|e| format!("{}: {e}", p.display()))?);
            }
            let refs: Vec<&Module> = modules.iter().collect();
            let claimed = check_manifest(&bytes, &refs)
                .map_err(|e| format!("{}: {e}", manifest.display()))?;
            for id in claimed {
                println!("{id}");
            }
            Ok(())
        }
        Cmd::Check {
            kit,
            codespec,
            adapters,
            build_dir,
            cases,
            write,
        } => {
            let mut all = load_cases(&kit.join("cases"))?;
            if !cases.is_empty() {
                for name in &cases {
                    if !all.iter().any(|c| &c.name == name) {
                        return Err(format!("no case {name:?}"));
                    }
                }
                all.retain(|c| cases.contains(&c.name));
            }
            let tools = Tools {
                codespec: std::path::absolute(&codespec).map_err(|e| e.to_string())?,
                adapters: std::path::absolute(&adapters).map_err(|e| e.to_string())?,
                driver: kit.join("driver/go"),
            };
            let next = AtomicUsize::new(0);
            let failures = Mutex::new(Vec::new());
            let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
            std::thread::scope(|s| {
                for _ in 0..workers {
                    s.spawn(|| {
                        while let Some(case) = all.get(next.fetch_add(1, Ordering::SeqCst)) {
                            if let Err(e) = check_case(case, &tools, &build_dir, write) {
                                failures.lock().expect("no worker panics").push(e);
                            }
                        }
                    });
                }
            });
            let mut failures = failures.into_inner().expect("no worker panics");
            failures.sort();
            for f in &failures {
                eprintln!("FAIL {f}");
            }
            let verb = if write { "written" } else { "checked" };
            if failures.is_empty() {
                println!("{} cases {verb}", all.len());
                Ok(())
            } else {
                Err(format!("{} of {} cases failed", failures.len(), all.len()))
            }
        }
    }
}
