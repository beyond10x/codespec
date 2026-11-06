//! Runs a codespec harness trace against a Rust implementation of the inventory example and prints
//! one line per command. The same source drives the original crate (`drivers/rust-original`) and
//! the Rust output (`drivers/rust`); the trace and output formats are defined in the doc of
//! `crates/harness`, and the Go driver prints exactly the same.

use std::io::{BufRead, BufWriter, Write};
use std::process::exit;

use inventory::{Category, Error, Inventory};

fn main() {
    let stdin = std::io::stdin().lock();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut inv: Option<Inventory> = None;
    for (n, line) in stdin.lines().enumerate() {
        let n = n + 1;
        let line = line.unwrap_or_else(|e| fail(n, &e.to_string()));
        let f: Vec<&str> = line.split(' ').collect();
        if f[0] == "new" && f.len() == 1 {
            inv = Some(inventory::new());
            writeln!(out, "new").expect("stdout");
            continue;
        }
        let Some(inv) = inv.as_mut() else {
            fail(n, "command before new")
        };
        let int = |s: &str| -> i64 { s.parse().unwrap_or_else(|e| fail(n, &format!("{e}"))) };
        match (f[0], f.len()) {
            ("add", 5) => {
                let r = inv.add_item(
                    f[1].to_string(),
                    f[2].to_string(),
                    category(n, f[3]),
                    int(f[4]),
                );
                match r {
                    Ok(()) => writeln!(out, "add ok"),
                    Err(e) => writeln!(out, "add err {e}"),
                }
                .expect("stdout");
            }
            ("receive", 3) => level(
                &mut out,
                "receive",
                inv.receive(f[1].to_string(), int(f[2])),
            ),
            ("reserve", 3) => level(
                &mut out,
                "reserve",
                inv.reserve(f[1].to_string(), int(f[2])),
            ),
            ("find", 2) => match inv.find(f[1].to_string()) {
                Some(item) => writeln!(out, "find found {} {}", item.sku, item.label()),
                None => writeln!(out, "find missing"),
            }
            .expect("stdout"),
            ("stock", 2) => {
                writeln!(out, "stock {}", inv.stock_of(f[1].to_string())).expect("stdout")
            }
            ("value", 2) => {
                let c = category(n, f[1]);
                writeln!(out, "value {c} {}", inv.value_of(c)).expect("stdout");
            }
            ("low", 2) => {
                let mut skus = inv.low_stock(int(f[1]));
                skus.sort();
                write!(out, "low {}", skus.len()).expect("stdout");
                for s in &skus {
                    write!(out, " | {s}").expect("stdout");
                }
                writeln!(out).expect("stdout");
            }
            ("reserved", 1) => writeln!(out, "reserved {}", inv.reserved()).expect("stdout"),
            _ => fail(n, &format!("unknown command {line:?}")),
        }
    }
    out.flush().expect("stdout");
}

fn level(out: &mut impl Write, cmd: &str, r: Result<i64, Error>) {
    match r {
        Ok(v) => writeln!(out, "{cmd} ok {v}"),
        Err(e) => writeln!(out, "{cmd} err {e}"),
    }
    .expect("stdout");
}

fn category(line: usize, s: &str) -> Category {
    match s {
        "tool" => Category::Tool,
        "part" => Category::Part,
        "supply" => Category::Supply,
        _ => fail(line, &format!("unknown category {s:?}")),
    }
}

fn fail(line: usize, msg: &str) -> ! {
    eprintln!("driver: trace line {line}: {msg}");
    exit(2)
}
