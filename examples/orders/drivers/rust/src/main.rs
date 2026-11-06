//! Runs a codespec harness trace against the Rust output of the orders example and prints one line
//! per command result, then one line per message the call sent. The trace and output formats are
//! defined in the doc of `crates/harness`; this driver prints exactly what the Go driver prints.

use std::io::{BufRead, BufWriter, Write};
use std::process::exit;
use std::sync::mpsc::{Receiver, channel};

use orders::{Message, Order, OrderID, Service};

fn main() {
    let stdin = std::io::stdin().lock();
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut svc: Option<(Service, Receiver<Message>)> = None;
    for (n, line) in stdin.lines().enumerate() {
        let n = n + 1;
        let line = line.unwrap_or_else(|e| fail(n, &e.to_string()));
        let f: Vec<&str> = line.split(' ').collect();
        if f[0] == "new" && f.len() == 1 {
            let (tx, rx) = channel();
            svc = Some((orders::new(tx), rx));
            writeln!(out, "new").expect("stdout");
            continue;
        }
        let Some((s, rx)) = svc.as_mut() else {
            fail(n, "command before new")
        };
        match (f[0], f.len()) {
            ("place", 4) => {
                let total: i64 = f[2].parse().unwrap_or_else(|e| fail(n, &format!("{e}")));
                let gift: bool = f[3].parse().unwrap_or_else(|e| fail(n, &format!("{e}")));
                match s.place(f[1].to_string(), total, gift) {
                    Ok(id) => writeln!(out, "place ok {id}"),
                    Err(e) => writeln!(out, "place err {e}"),
                }
                .expect("stdout");
            }
            ("pay", 2) => result(&mut out, "pay", s.pay(OrderID(f[1].to_string()))),
            ("ship", 3) => result(
                &mut out,
                "ship",
                s.ship(OrderID(f[1].to_string()), f[2].to_string()),
            ),
            ("cancel", 2) => result(&mut out, "cancel", s.cancel(OrderID(f[1].to_string()))),
            ("get", 2) => match s.get(OrderID(f[1].to_string())) {
                Some(o) => writeln!(out, "get found {}", order(&o)),
                None => writeln!(out, "get missing"),
            }
            .expect("stdout"),
            ("pending", 1) => {
                let mut ps = s.pending_orders();
                ps.sort_by(|a, b| a.id.0.cmp(&b.id.0));
                write!(out, "pending {}", ps.len()).expect("stdout");
                for o in &ps {
                    write!(out, " | {}", order(o)).expect("stdout");
                }
                writeln!(out).expect("stdout");
            }
            _ => fail(n, &format!("unknown command {line:?}")),
        }
        for m in rx.try_iter() {
            match m {
                Message::OrderPlaced(e) => writeln!(
                    out,
                    "event OrderPlaced id={} customer={} total={}",
                    e.id, e.customer, e.total
                ),
                Message::OrderPaid(e) => writeln!(out, "event OrderPaid id={}", e.id),
                Message::OrderShipped(e) => writeln!(
                    out,
                    "event OrderShipped id={} tracking={}",
                    e.id, e.tracking
                ),
                Message::OrderCancelled(e) => writeln!(out, "event OrderCancelled id={}", e.id),
            }
            .expect("stdout");
        }
    }
    out.flush().expect("stdout");
}

fn result(out: &mut impl Write, cmd: &str, r: Result<(), orders::Error>) {
    match r {
        Ok(()) => writeln!(out, "{cmd} ok"),
        Err(e) => writeln!(out, "{cmd} err {e}"),
    }
    .expect("stdout");
}

fn order(o: &Order) -> String {
    format!(
        "id={} customer={} total={} status={} tracking={} gift={}",
        o.id, o.customer, o.total, o.status, o.tracking, o.gift
    )
}

fn fail(line: usize, msg: &str) -> ! {
    eprintln!("driver: trace line {line}: {msg}");
    exit(2)
}
