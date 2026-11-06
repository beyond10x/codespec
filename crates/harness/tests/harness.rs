//! The harness's own acceptance: reproducible traces without `Audit`, traces that reach the error
//! paths, a strict manifest, and an original whose output does not depend on Go map order.

use std::path::PathBuf;

use codespec_harness::{
    Manifest, Options, SplitMix64, TraceKind, build, fees_trace, inventory_trace, ledger_trace,
    run_driver, series_trace, trace,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_generator_is_splitmix64() {
    let mut r = SplitMix64(0);
    assert_eq!(r.next_u64(), 0xe220_a839_7b1d_cdaf);
    assert_eq!(r.next_u64(), 0x6e78_9e6a_a1b9_65f4);
    assert_eq!(r.next_u64(), 0x06c4_5d18_8009_454f);
}

#[test]
fn one_seed_gives_the_same_trace_bytes_on_every_run() {
    for seed in [0, 1, 17, 199, u64::MAX] {
        assert_eq!(trace(seed, 300).into_bytes(), trace(seed, 300).into_bytes());
    }
    assert_ne!(trace(0, 300), trace(1, 300));
    assert_eq!(trace(3, 300).lines().count(), 300);
}

#[test]
fn a_trace_is_pinned_across_platforms() {
    assert_eq!(trace(0, 12), PINNED_SEED_0);
}

const PINNED_SEED_0: &str = "new
new
get o-2
pending
place carol 391 true
pay o-1
place Ω -1000 false
get x
place alice 860 false
pending
get o-4
cancel o-2
";

#[test]
fn no_trace_for_seeds_0_to_199_contains_audit() {
    for seed in 0..200 {
        let t = trace(seed, 300);
        assert!(
            !t.to_ascii_lowercase().contains("audit"),
            "seed {seed} contains audit"
        );
    }
}

#[test]
fn traces_start_with_new_and_reach_every_command_and_error_path() {
    let mut seen = std::collections::BTreeSet::new();
    let mut non_positive = 0;
    let mut extreme = 0;
    let mut unknown_id = 0;
    for seed in 0..200 {
        let t = trace(seed, 300);
        assert!(t.starts_with("new\n"), "seed {seed}");
        let mut placed = 0i64;
        for line in t.lines() {
            let f: Vec<&str> = line.split(' ').collect();
            seen.insert(f[0].to_string());
            match f[0] {
                "new" => placed = 0,
                "place" => {
                    let total: i64 = f[2].parse().unwrap();
                    if total <= 0 {
                        non_positive += 1;
                    } else {
                        placed += 1;
                    }
                    if total == i64::MAX || total == i64::MIN {
                        extreme += 1;
                    }
                }
                "pay" | "ship" | "cancel" | "get" => {
                    let known = f[1]
                        .strip_prefix("o-")
                        .and_then(|n| n.parse::<i64>().ok())
                        .is_some_and(|n| n >= 1 && n <= placed && f[1] == format!("o-{n}"));
                    if !known {
                        unknown_id += 1;
                    }
                }
                _ => {}
            }
        }
    }
    let want: Vec<String> = ["cancel", "get", "new", "pay", "pending", "place", "ship"]
        .map(String::from)
        .to_vec();
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), want);
    assert!(non_positive > 100, "{non_positive} non-positive totals");
    assert!(extreme > 10, "{extreme} extreme totals");
    assert!(unknown_id > 100, "{unknown_id} unknown ids");
}

#[test]
fn the_manifest_is_strict() {
    let ok = "[[implementation]]\nname = \"a\"\nbuild = [\"true\"]\nrun = [\"cat\"]\n";
    let m = Manifest::parse(ok).unwrap();
    assert_eq!(m.implementation.len(), 1);
    assert_eq!(m.implementation[0].name, "a");
    assert!(Manifest::parse(&format!("{ok}extra = 1\n")).is_err());
    assert!(Manifest::parse("[[implementation]]\nname = \"a\"\nrun = [\"cat\"]\n").is_err());
    assert!(Manifest::parse("").is_err());
    let dup = format!("{ok}{ok}");
    assert!(Manifest::parse(&dup).is_err());
    let empty_run = "[[implementation]]\nname = \"a\"\nbuild = [\"true\"]\nrun = []\n";
    assert!(Manifest::parse(empty_run).is_err());
}

#[test]
fn the_original_run_twice_on_one_seed_gives_the_same_bytes() {
    let opts = Options {
        example: root().join("examples/orders"),
        build_dir: root().join("target/harness-test/twice"),
        seeds: 1,
        commands: 300,
    };
    let built = build(&opts).unwrap();
    let original = built.iter().find(|b| b.name == "original").unwrap();
    let t = trace(5, 300);
    let first = run_driver(original, &t).unwrap();
    let second = run_driver(original, &t).unwrap();
    assert_eq!(first, second);
    let text = String::from_utf8(first).unwrap();
    let most_pending = text
        .lines()
        .filter_map(|l| l.strip_prefix("pending "))
        .filter_map(|l| l.split(' ').next()?.parse::<usize>().ok())
        .max()
        .unwrap();
    assert!(
        most_pending >= 3,
        "seed 5 lists at most {most_pending} pending orders"
    );
}

#[test]
fn the_manifest_names_its_trace_and_refuses_an_unknown_one() {
    let ok = "[[implementation]]\nname = \"a\"\nbuild = [\"true\"]\nrun = [\"cat\"]\n";
    assert_eq!(Manifest::parse(ok).unwrap().trace, TraceKind::Orders);
    let inv = format!("trace = \"inventory\"\n{ok}");
    assert_eq!(Manifest::parse(&inv).unwrap().trace, TraceKind::Inventory);
    assert!(Manifest::parse(&format!("trace = \"bogus\"\n{ok}")).is_err());
}

#[test]
fn inventory_traces_are_reproducible_and_reach_every_command_and_error_path() {
    let mut seen = std::collections::BTreeSet::new();
    let (mut bad_price, mut bad_qty, mut unknown_sku) = (0, 0, 0);
    for seed in 0..200 {
        let t = inventory_trace(seed, 300);
        assert_eq!(t, inventory_trace(seed, 300));
        assert_eq!(t.lines().count(), 300);
        assert!(t.starts_with("new\n"), "seed {seed}");
        for l in t.lines() {
            let f: Vec<&str> = l.split(' ').collect();
            assert!(f.iter().all(|x| !x.is_empty()), "empty field in {l:?}");
            seen.insert(f[0].to_string());
            match f[0] {
                "add" if f[4].parse::<i64>().unwrap() <= 0 => bad_price += 1,
                "receive" | "reserve" if f[2].parse::<i64>().unwrap() <= 0 => bad_qty += 1,
                _ => {}
            }
            if matches!(f[0], "receive" | "reserve" | "find" | "stock") && f[1] == "x9" {
                unknown_sku += 1;
            }
        }
    }
    let want: Vec<String> = [
        "add", "find", "low", "new", "receive", "reserve", "reserved", "stock", "value",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), want);
    assert!(bad_price > 100, "{bad_price} non-positive prices");
    assert!(bad_qty > 100, "{bad_qty} non-positive quantities");
    assert!(unknown_sku > 100, "{unknown_sku} unknown skus");
}

#[test]
fn the_manifest_selects_the_series_trace() {
    let ok = "trace = \"series\"\n[[implementation]]\nname = \"a\"\nbuild = [\"true\"]\nrun = [\"cat\"]\n";
    assert_eq!(Manifest::parse(ok).unwrap().trace, TraceKind::Series);
}

#[test]
fn series_traces_are_reproducible_and_reach_every_command_and_edge() {
    let mut seen = std::collections::BTreeSet::new();
    let (mut extreme, mut bad_window, mut bad_steps, mut longest) = (0, 0, 0, 0usize);
    for seed in 0..200 {
        let t = series_trace(seed, 300);
        assert_eq!(t, series_trace(seed, 300));
        assert_eq!(t.lines().count(), 300);
        assert!(t.starts_with("new\n"), "seed {seed}");
        let mut pushed = 0usize;
        for l in t.lines() {
            let f: Vec<&str> = l.split(' ').collect();
            assert!(f.iter().all(|x| !x.is_empty()), "empty field in {l:?}");
            seen.insert(f[0].to_string());
            match f[0] {
                "new" => pushed = 0,
                "push" => {
                    pushed += 1;
                    longest = longest.max(pushed);
                    let v: i64 = f[1].parse().unwrap();
                    if v == i64::MAX || v == i64::MIN {
                        extreme += 1;
                    }
                }
                "window" if f[1].parse::<i64>().unwrap() <= 0 => bad_window += 1,
                "steps" if f[1].parse::<i64>().unwrap() <= 0 => bad_steps += 1,
                _ => {}
            }
        }
    }
    let want: Vec<String> = [
        "bucket", "drop", "evens", "gcd", "len", "mean", "median", "new", "prime", "push",
        "residue", "reverse", "sorted", "steps", "sum", "trend", "window",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), want);
    assert!(extreme > 100, "{extreme} pushes of an int64 extreme");
    assert!(bad_window > 100, "{bad_window} non-positive windows");
    assert!(bad_steps > 50, "{bad_steps} non-positive steps");
    assert!(longest >= 30, "the longest series has {longest} values");
}

#[test]
fn a_series_trace_is_pinned_across_platforms() {
    assert_eq!(series_trace(0, 12), PINNED_SERIES_SEED_0);
}

const PINNED_SERIES_SEED_0: &str = "new
new
steps -5
bucket 31
residue 50
push 1
push -262
push -4611686018427387904
steps 3019
window 4
steps 4611686018427387904
mean
";

#[test]
fn an_inventory_trace_is_pinned_across_platforms() {
    assert_eq!(inventory_trace(0, 12), PINNED_INVENTORY_SEED_0);
}

const PINNED_INVENTORY_SEED_0: &str = "new
new
stock c3
low 53
receive a1 27
value tool
add c3 hammer supply 5000
stock d4
reserve Ω5 0
stock b2
find x9
low 7
";

#[test]
fn the_manifest_selects_the_ledger_trace() {
    let ok = "trace = \"ledger\"\n[[implementation]]\nname = \"a\"\nbuild = [\"true\"]\nrun = [\"cat\"]\n";
    assert_eq!(Manifest::parse(ok).unwrap().trace, TraceKind::Ledger);
}

/// What one ledger trace reaches, counted by a model of the example's `Ledger`: each error path
/// and each integer edge crossed.
#[derive(Default, Debug)]
struct LedgerReach {
    unknown_account: u32,
    duplicate_account: u32,
    non_positive_amount: u32,
    insufficient_funds: u32,
    long_past_max: u32,
    long_past_min: u32,
    int_past_max: u32,
    int_past_min: u32,
}

/// Runs the trace against a model of `examples/ledger/source`: `open` refuses an open id,
/// `post` refuses a non-positive amount, then an unknown id, then a withdrawal above the balance;
/// a fee may overdraw; `award` refuses an unknown id. Balances are `long`, points `int`.
fn ledger_reach(t: &str, r: &mut LedgerReach) {
    let mut accounts: std::collections::BTreeMap<String, (i64, i32)> = Default::default();
    for l in t.lines() {
        let f: Vec<&str> = l.split(' ').collect();
        match f[0] {
            "new" => accounts.clear(),
            "open" => {
                if accounts.contains_key(f[1]) {
                    r.duplicate_account += 1;
                } else {
                    accounts.insert(f[1].to_string(), (0, 0));
                }
            }
            "post" => {
                let amount: i64 = f[3].parse().unwrap();
                if amount <= 0 {
                    r.non_positive_amount += 1;
                    continue;
                }
                let Some((balance, _)) = accounts.get_mut(f[1]) else {
                    r.unknown_account += 1;
                    continue;
                };
                match f[2] {
                    "deposit" => {
                        if balance.checked_add(amount).is_none() {
                            r.long_past_max += 1;
                        }
                        *balance = balance.wrapping_add(amount);
                    }
                    "withdrawal" if *balance < amount => r.insufficient_funds += 1,
                    "withdrawal" | "fee" => {
                        if balance.checked_sub(amount).is_none() {
                            r.long_past_min += 1;
                        }
                        *balance = balance.wrapping_sub(amount);
                    }
                    k => panic!("kind {k}"),
                }
            }
            "award" => {
                let n: i32 = f[2].parse().unwrap();
                let Some((_, points)) = accounts.get_mut(f[1]) else {
                    r.unknown_account += 1;
                    continue;
                };
                if points.checked_add(n).is_none() {
                    if n > 0 {
                        r.int_past_max += 1;
                    } else {
                        r.int_past_min += 1;
                    }
                }
                *points = points.wrapping_add(n);
            }
            _ => {}
        }
    }
}

#[test]
fn ledger_traces_reach_every_command_each_error_path_and_both_edges_of_int_and_long() {
    let mut seen = std::collections::BTreeSet::new();
    let mut r = LedgerReach::default();
    for seed in 0..200 {
        let t = ledger_trace(seed, 300);
        assert_eq!(t, ledger_trace(seed, 300));
        assert_eq!(t.lines().count(), 300);
        assert!(t.starts_with("new\n"), "seed {seed}");
        for l in t.lines() {
            let f: Vec<&str> = l.split(' ').collect();
            assert!(f.iter().all(|x| !x.is_empty()), "empty field in {l:?}");
            seen.insert(f[0].to_string());
        }
        ledger_reach(&t, &mut r);
    }
    let want: Vec<String> = [
        "award", "balances", "count", "find", "largest", "new", "open", "post", "reach", "tallied",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), want);
    for (what, n) in [
        ("unknown account", r.unknown_account),
        ("duplicate account", r.duplicate_account),
        ("non-positive amount", r.non_positive_amount),
        ("insufficient funds", r.insufficient_funds),
        ("long past its maximum", r.long_past_max),
        ("long past its minimum", r.long_past_min),
        ("int past its maximum", r.int_past_max),
        ("int past its minimum", r.int_past_min),
    ] {
        assert!(n >= 50, "{what}: {n} times over seeds 0-199; {r:?}");
    }
}

#[test]
fn a_ledger_trace_is_pinned_across_platforms() {
    assert_eq!(ledger_trace(0, 12), PINNED_LEDGER_SEED_0);
}

const PINNED_LEDGER_SEED_0: &str = "new
new
largest B
balances
open B
count
balances
new
post bob fee 926
open ｚ
reach zoë 1661
post ｚ withdrawal 139
";

#[test]
fn the_manifest_selects_the_fees_trace() {
    let ok =
        "trace = \"fees\"\n[[implementation]]\nname = \"a\"\nbuild = [\"true\"]\nrun = [\"cat\"]\n";
    assert_eq!(Manifest::parse(ok).unwrap().trace, TraceKind::Fees);
}

/// What one fees trace reaches, counted by a model of the example's `LateFeeService`: each branch
/// of `assess`, each `ItemKind`, and each integer edge crossed.
#[derive(Default, Debug)]
struct FeesReach {
    waived: u32,
    graced: u32,
    capped: u32,
    clamped: u32,
    charged: u32,
    book: u32,
    disc: u32,
    tool: u32,
    long_past_max: u32,
    long_past_min: u32,
    int_past_max: u32,
    int_past_min: u32,
}

/// Runs the trace against a model of `examples/fees/source`: `assess` spends a waiver first, then
/// charges nothing within the grace period, then adds the kind's daily rate (10 when unset or 0) once
/// per day, at most 365 days, stopping at a positive cap. Rates and fees are `long`, days and waivers
/// `int`; both wrap.
fn fees_reach(t: &str, r: &mut FeesReach) {
    #[derive(Default)]
    struct Model {
        rates: std::collections::BTreeMap<String, i64>,
        grace: i32,
        cap: i64,
        waivers: i32,
        collected: i64,
    }
    let mut m = Model::default();
    let past = |r: &mut FeesReach, int: bool, up: bool| match (int, up) {
        (false, true) => r.long_past_max += 1,
        (false, false) => r.long_past_min += 1,
        (true, true) => r.int_past_max += 1,
        (true, false) => r.int_past_min += 1,
    };
    for l in t.lines() {
        let f: Vec<&str> = l.split(' ').collect();
        match f[0] {
            "new" => m = Model::default(),
            "rate" => {
                m.rates.insert(f[1].to_string(), f[2].parse().unwrap());
            }
            "grace" => m.grace = f[1].parse().unwrap(),
            "cap" => m.cap = f[1].parse().unwrap(),
            "waive" => {
                let n: i32 = f[1].parse().unwrap();
                if m.waivers.checked_add(n).is_none() {
                    past(r, true, n > 0);
                }
                m.waivers = m.waivers.wrapping_add(n);
            }
            "assess" => {
                match f[1] {
                    "book" => r.book += 1,
                    "disc" => r.disc += 1,
                    "tool" => r.tool += 1,
                    k => panic!("kind {k}"),
                }
                if m.waivers > 0 {
                    m.waivers -= 1;
                    r.waived += 1;
                    continue;
                }
                let late: i32 = f[2].parse().unwrap();
                if late.checked_sub(m.grace).is_none() {
                    past(r, true, m.grace < 0);
                }
                let mut days = late.wrapping_sub(m.grace);
                if days <= 0 {
                    r.graced += 1;
                    continue;
                }
                if days > 365 {
                    r.clamped += 1;
                    days = 365;
                }
                let rate = match m.rates.get(f[1]).copied().unwrap_or(0) {
                    0 => 10,
                    v => v,
                };
                let mut fee = 0i64;
                let mut capped = false;
                for _ in 0..days {
                    if fee.checked_add(rate).is_none() {
                        past(r, false, rate > 0);
                    }
                    fee = fee.wrapping_add(rate);
                    if m.cap > 0 && fee >= m.cap {
                        fee = m.cap;
                        capped = true;
                        break;
                    }
                }
                if capped {
                    r.capped += 1;
                } else {
                    r.charged += 1;
                }
                if m.collected.checked_add(fee).is_none() {
                    past(r, false, fee > 0);
                }
                m.collected = m.collected.wrapping_add(fee);
            }
            _ => {}
        }
    }
}

#[test]
fn fees_traces_reach_every_command_each_branch_and_both_edges_of_int_and_long() {
    let mut seen = std::collections::BTreeSet::new();
    let mut r = FeesReach::default();
    for seed in 0..200 {
        let t = fees_trace(seed, 300);
        assert_eq!(t, fees_trace(seed, 300));
        assert_eq!(t.lines().count(), 300);
        assert!(t.starts_with("new\n"), "seed {seed}");
        for l in t.lines() {
            let f: Vec<&str> = l.split(' ').collect();
            assert!(f.iter().all(|x| !x.is_empty()), "empty field in {l:?}");
            seen.insert(f[0].to_string());
        }
        fees_reach(&t, &mut r);
    }
    let want: Vec<String> = [
        "assess", "cap", "grace", "new", "rate", "rates", "stats", "waive",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), want);
    for (what, n) in [
        ("waived", r.waived),
        ("within grace", r.graced),
        ("capped", r.capped),
        ("clamped to 365 days", r.clamped),
        ("charged below the cap", r.charged),
        ("BOOK", r.book),
        ("DISC", r.disc),
        ("TOOL", r.tool),
        ("long past its maximum", r.long_past_max),
        ("long past its minimum", r.long_past_min),
        ("int past its maximum", r.int_past_max),
        ("int past its minimum", r.int_past_min),
    ] {
        assert!(n >= 50, "{what}: {n} times over seeds 0-199; {r:?}");
    }
}

#[test]
fn a_fees_trace_is_pinned_across_platforms() {
    let first: String = fees_trace(0, 300)
        .lines()
        .take(5)
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(first, PINNED_FEES_SEED_0);
}

const PINNED_FEES_SEED_0: &str = "new
new
assess disc 43
grace 14
rates
";
