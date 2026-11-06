//! Runs a codespec harness trace against the Rust output of the ledger example and prints one
//! line per command. The trace and output formats are defined in the doc of `crates/harness`;
//! this driver prints exactly what the Java and Go drivers print.

use std::fmt::Display;
use std::io::{BufRead, BufWriter, Write};
use std::process::exit;

use ledger::{Kind, Ledger};

fn main() {
    let stdin = std::io::stdin().lock();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut ledger: Option<Ledger> = None;
    for (n, line) in stdin.lines().enumerate() {
        let n = n + 1;
        let line = line.unwrap_or_else(|e| fail(n, &e.to_string()));
        let f: Vec<&str> = line.split(' ').collect();
        if f[0] == "new" && f.len() == 1 {
            ledger = Some(ledger::new_ledger());
            writeln!(out, "new").expect("stdout");
            continue;
        }
        let Some(l) = ledger.as_mut() else {
            fail(n, "command before new")
        };
        let long = |s: &str| -> i64 { s.parse().unwrap_or_else(|e| fail(n, &format!("{e}"))) };
        let int = |s: &str| -> i32 { s.parse().unwrap_or_else(|e| fail(n, &format!("{e}"))) };
        match (f[0], f.len()) {
            ("open", 2) => result(
                &mut out,
                "open",
                l.open(f[1].to_string()).map(|v| format!(" {v}")),
            ),
            ("post", 4) => {
                let r = l.post(f[1].to_string(), kind(n, f[2]), long(f[3]));
                result(&mut out, "post", r.map(|()| String::new()))
            }
            ("award", 3) => {
                let r = l.award(f[1].to_string(), int(f[2]));
                result(&mut out, "award", r.map(|v| format!(" {v}")))
            }
            ("find", 2) => match l.find(f[1].to_string()) {
                Some(a) => writeln!(out, "find found {}", a.borrow().describe()),
                None => writeln!(out, "find missing"),
            },
            ("largest", 2) => match l.find(f[1].to_string()) {
                Some(a) => writeln!(out, "largest {}", a.borrow().largest()),
                None => writeln!(out, "largest missing"),
            },
            ("reach", 3) => {
                let target = long(f[2]);
                match l.find(f[1].to_string()) {
                    Some(a) => writeln!(out, "reach {}", a.borrow().reach(target)),
                    None => writeln!(out, "reach missing"),
                }
            }
            ("balances", 1) => writeln!(out, "balances {}", l.balances()),
            ("tallied", 1) => writeln!(out, "tallied {}", l.tallied()),
            ("count", 1) => writeln!(out, "count {}", l.count()),
            _ => fail(n, &format!("unknown command {line:?}")),
        }
        .expect("stdout");
    }
    out.flush().expect("stdout");
}

fn kind(line: usize, s: &str) -> Kind {
    match s {
        "deposit" => Kind::DEPOSIT,
        "withdrawal" => Kind::WITHDRAWAL,
        "fee" => Kind::FEE,
        _ => fail(line, &format!("unknown kind {s}")),
    }
}

fn result(out: &mut impl Write, cmd: &str, r: Result<String, impl Display>) -> std::io::Result<()> {
    match r {
        Ok(v) => writeln!(out, "{cmd} ok{v}"),
        Err(e) => writeln!(out, "{cmd} err {e}"),
    }
}

fn fail(line: usize, msg: &str) -> ! {
    eprintln!("driver: trace line {line}: {msg}");
    exit(2)
}
