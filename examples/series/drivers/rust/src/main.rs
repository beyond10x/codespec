//! Runs a codespec harness trace against the Rust output of the series example and prints one
//! line per command. The trace and output formats are defined in the doc of `crates/harness`; this
//! driver prints exactly what the Go driver prints.

use std::fmt::Display;
use std::io::{BufRead, BufWriter, Write};
use std::process::exit;

use series::Series;

fn main() {
    let stdin = std::io::stdin().lock();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut svc: Option<Series> = None;
    for (n, line) in stdin.lines().enumerate() {
        let n = n + 1;
        let line = line.unwrap_or_else(|e| fail(n, &e.to_string()));
        let f: Vec<&str> = line.split(' ').collect();
        if f[0] == "new" && f.len() == 1 {
            svc = Some(series::new());
            writeln!(out, "new").expect("stdout");
            continue;
        }
        let Some(s) = svc.as_mut() else {
            fail(n, "command before new")
        };
        let arg = || -> i64 { f[1].parse().unwrap_or_else(|e| fail(n, &format!("{e}"))) };
        match (f[0], f.len()) {
            ("push", 2) => writeln!(out, "push {}", s.push(arg())),
            ("sum", 1) => writeln!(out, "sum {}", s.sum()),
            ("mean", 1) => result(&mut out, "mean", s.mean()),
            ("median", 1) => result(&mut out, "median", s.median()),
            ("sorted", 1) => {
                let vs = s.sorted();
                write!(out, "sorted {}", vs.len()).expect("stdout");
                for v in &vs {
                    write!(out, " {v}").expect("stdout");
                }
                writeln!(out)
            }
            ("reverse", 1) => {
                s.reverse();
                writeln!(out, "reverse")
            }
            ("trend", 1) => writeln!(out, "trend {}", s.trend()),
            ("window", 2) => result(&mut out, "window", s.max_window(arg())),
            ("gcd", 1) => writeln!(out, "gcd {}", s.gcd()),
            ("evens", 1) => writeln!(out, "evens {}", s.even_sum()),
            ("drop", 2) => writeln!(out, "drop {}", s.drop(arg())),
            ("len", 1) => writeln!(out, "len {}", s.len()),
            ("steps", 2) => result(&mut out, "steps", series::steps(arg())),
            ("prime", 2) => writeln!(out, "prime {}", series::is_prime(arg())),
            ("bucket", 2) => writeln!(out, "bucket {}", series::bucket(arg())),
            ("residue", 2) => writeln!(out, "residue {}", series::residue(arg())),
            _ => fail(n, &format!("unknown command {line:?}")),
        }
        .expect("stdout");
    }
    out.flush().expect("stdout");
}

fn result(out: &mut impl Write, cmd: &str, r: Result<i64, impl Display>) -> std::io::Result<()> {
    match r {
        Ok(v) => writeln!(out, "{cmd} ok {v}"),
        Err(e) => writeln!(out, "{cmd} err {e}"),
    }
}

fn fail(line: usize, msg: &str) -> ! {
    eprintln!("driver: trace line {line}: {msg}");
    exit(2)
}
