//! The harness's own acceptance: reproducible traces without `Audit`, traces that reach the error
//! paths, a strict manifest, and an original whose output does not depend on Go map order.

use std::path::PathBuf;

use codespec_harness::{
    Manifest, Options, SplitMix64, TraceKind, build, inventory_trace, run_driver, trace,
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
