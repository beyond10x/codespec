//! Two crates read and queried together: a call of a crate's own function links within its own
//! document, though the other declares a function of the same name.

use codespec_model::Module;
use codespec_query::{FactBase, Names, parse, run};

/// A crate `name` whose `walk` calls itself, refused whole for its bound so its calls are
/// listed, beside a `step` read whole that calls `walk`.
fn krate(name: &str) -> Module {
    let src = "pub fn walk<T: Clone>(n: u32, t: T) -> u32 {\n    if n == 0 { 0 } else { walk(n - 1, t) }\n}\n\npub fn step(n: u32) -> u32 {\n    n + 1\n}\n";
    let m = codespec_read_rust::read_source(name, "lib.rs", src).expect("read");
    Module::from_json(m.to_json().as_bytes()).expect("the module is valid cir/1")
}

fn answer(query: &str, ms: &[Module]) -> String {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    let facts = FactBase::build(ms);
    run(&program, &facts, &Names::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_text()
}

#[test]
fn a_listed_call_of_a_crate_s_own_function_links_within_its_document() {
    let ms = [krate("a"), krate("b")];
    for (me, other) in [("a", "b"), ("b", "a")] {
        let text = answer(
            &format!(r#"?- call{{site: S, func: "{me}.Walk"}}, calls{{site: S, target: C}}."#),
            &ms,
        );
        assert!(text.contains(&format!("{me}.Walk")), "{me}: {text}");
        assert!(!text.contains(&format!("{other}.Walk")), "{me}: {text}");
        let reach = answer(&format!(r#"?- reaches("{me}.Walk", T)."#), &ms);
        assert!(reach.contains(&format!("{me}.Walk")), "{me}: {reach}");
        assert!(!reach.contains(&format!("{other}.Walk")), "{me}: {reach}");
    }
}
