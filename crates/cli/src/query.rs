//! `codespec query`: a cq query over a set of `cir/1` documents (crate `b10x-codespec-query`).

use std::path::PathBuf;
use std::process::ExitCode;

/// The arguments of `codespec query`.
#[derive(clap::Args)]
pub struct Args {
    /// The query, then the documents; with -f, only the documents.
    pub args: Vec<String>,
    /// A cq module to run: rules, names lists and one query (`?- …`).
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<PathBuf>,
    /// Print every relation of the fact base with its fields and their types, and exit.
    #[arg(long, conflicts_with_all = ["file", "explain"])]
    pub schema: bool,
    /// Print the query's rules in the order of their strata instead of its rows.
    #[arg(long)]
    pub explain: bool,
    /// text, one line per row (its values, then `possible: …` when it rests on code the fact
    /// base cannot see); or json, codespec-query-report/1.
    #[arg(long, value_parser = ["text", "json"], default_value = "text")]
    pub format: String,
    /// A JSON object of list name to an array of strings, each replacing a names list the
    /// module declares.
    #[arg(long, value_name = "FILE")]
    pub names: Option<PathBuf>,
}

/// Run `codespec query`: exit 0 with no row, 1 with rows; an error is the caller's 2.
pub fn run(a: &Args) -> Result<ExitCode, String> {
    if a.schema {
        print!("{}", codespec_query::schema_text());
        return Ok(ExitCode::SUCCESS);
    }
    let (src, documents) = match &a.file {
        Some(path) => (
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?,
            &a.args[..],
        ),
        None => match a.args.split_first() {
            Some((q, docs)) => (q.clone(), docs),
            None => return Err(String::from("query: give a query or -f FILE")),
        },
    };
    let program = codespec_query::parse(&src)?;
    if a.explain {
        print!("{}", codespec_query::explain(&program)?);
        return Ok(ExitCode::SUCCESS);
    }
    if documents.is_empty() {
        return Err(String::from("query: no documents"));
    }
    let names = match &a.names {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            codespec_query::Names::from_json(&bytes)
                .map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => codespec_query::Names::default(),
    };
    let paths: Vec<PathBuf> = documents.iter().map(PathBuf::from).collect();
    let loaded = codespec_query::load(&paths)?;
    let answer = codespec_query::run(&program, &loaded.facts, &names)?;
    if a.format == "json" {
        print!("{}", answer.to_json());
    } else {
        print!("{}", answer.to_text());
    }
    Ok(if answer.rows.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
