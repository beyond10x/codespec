//! Adversary pass 2: the `sem.map.order` stability check against a small map.

use std::path::{Path, PathBuf};

use codespec_conformance::{MAP_ORDER_RUNS, load_cases, run_reference_stable};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The crate doc promises that a `sem.map.order` case whose output depends on iteration order is
/// refused. With two keys Go starts a small map's iteration at one of eight slots, so the second
/// key comes first in about one run in eight, and five runs agree with probability
/// (7/8)^5 + (1/8)^5, about 0.51. Ten independent checks of the same order-dependent case should
/// all refuse; each lets it through about half the time.
#[test]
fn adversary2_an_order_dependent_two_key_map_case_is_refused_every_time() {
    let mut case = load_cases(&root().join("conformance/cases"))
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|c| c.name == "sem-map-order")
        .expect("case sem-map-order");
    let build = root().join("target/conformance-test/adversary2-map-order");
    let dir = build.join("case");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("case.go"),
        "package sem_map_order\n\nfunc Squares(n int) int {\n\tm := map[int]int{}\n\tfor i := 0; i < 2; i++ {\n\t\tm[i] = i\n\t}\n\ts := 0\n\tp := 1\n\tfor k := range m {\n\t\ts += k * p\n\t\tp++\n\t}\n\treturn s\n}\n",
    )
    .unwrap();
    case.dir = dir;
    let attempts = 10;
    let mut accepted = Vec::new();
    for i in 0..attempts {
        if let Ok(out) = run_reference_stable(
            &case,
            &root().join("conformance/driver/go"),
            &build.join(format!("go-{i}")),
            MAP_ORDER_RUNS,
        ) {
            accepted.push(out.trim_end().to_string());
        }
    }
    assert!(
        accepted.is_empty(),
        "an order-dependent sem.map.order case passed the {MAP_ORDER_RUNS}-run stability check \
         in {} of {attempts} attempts, recording {accepted:?} as its expected output",
        accepted.len()
    );
}
