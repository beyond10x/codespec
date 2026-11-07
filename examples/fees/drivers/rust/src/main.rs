//! Runs a codespec harness trace against the Rust output of the fees example and prints one line
//! per command. The trace and output formats are defined in the doc of `crates/harness`; this
//! driver prints exactly what the Groovy and Go drivers print.

use std::io::{BufRead, BufWriter, Write};
use std::process::exit;

use fees::{ItemKind, LateFeeService};

fn main() {
    let stdin = std::io::stdin().lock();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut service: Option<LateFeeService> = None;
    for (n, line) in stdin.lines().enumerate() {
        let n = n + 1;
        let line = line.unwrap_or_else(|e| fail(n, &e.to_string()));
        let f: Vec<&str> = line.split(' ').collect();
        if f[0] == "new" && f.len() == 1 {
            service = Some(fees::new_late_fee_service());
            writeln!(out, "new").expect("stdout");
            continue;
        }
        let Some(s) = service.as_mut() else {
            fail(n, "command before new")
        };
        let long = |v: &str| -> i64 { v.parse().unwrap_or_else(|e| fail(n, &format!("{e}"))) };
        let int = |v: &str| -> i32 { v.parse().unwrap_or_else(|e| fail(n, &format!("{e}"))) };
        match (f[0], f.len()) {
            ("rate", 3) => writeln!(out, "rate {}", s.put_rate(kind(n, f[1]), long(f[2]))),
            ("grace", 2) => writeln!(out, "grace {}", s.grace_for(int(f[1]))),
            ("cap", 2) => writeln!(out, "cap {}", s.cap_at(long(f[1]))),
            ("waive", 2) => writeln!(out, "waive {}", s.waive(int(f[1]))),
            ("assess", 3) => writeln!(out, "assess {}", s.assess(kind(n, f[1]), int(f[2]))),
            ("rates", 1) => writeln!(out, "rates {}", s.rate_list()),
            ("stats", 1) => writeln!(out, "stats {}", s.stats()),
            _ => fail(n, &format!("unknown command {line:?}")),
        }
        .expect("stdout");
    }
    out.flush().expect("stdout");
}

fn kind(line: usize, s: &str) -> ItemKind {
    match s {
        "book" => ItemKind::BOOK,
        "disc" => ItemKind::DISC,
        "tool" => ItemKind::TOOL,
        _ => fail(line, &format!("unknown kind {s}")),
    }
}

fn fail(line: usize, msg: &str) -> ! {
    eprintln!("driver: trace line {line}: {msg}");
    exit(2)
}
