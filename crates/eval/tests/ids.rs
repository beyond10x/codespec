//! Node ids (`docs/query-language.md` § 4.1, the crate doc of `codespec_eval::id`) on the kit's
//! modules: every statement and expression of every function has one, no two the same.

use std::collections::BTreeSet;
use std::path::Path;

use codespec_conformance::load_cases;
use codespec_eval::id::{Node, node_id};
use codespec_model::{Stmt, walk};

#[test]
fn every_node_of_every_kit_function_has_its_own_id() {
    let cases = load_cases(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases"))
        .expect("the kit reads");
    let mut nodes = 0;
    for case in &cases {
        let m = &case.module;
        for f in &m.funcs {
            let mut ids = BTreeSet::new();
            let mut each = |n: Node<'_>| {
                let (id, pos) = node_id(m, f, n).expect("a node of the function has an id");
                assert!(!pos.is_empty(), "{id}: no position");
                assert!(ids.insert(id.clone()), "{id} twice in {}", case.name);
                nodes += 1;
            };
            walk::walk_stmts(&f.body, &mut |s| each(Node::Stmt(s)));
            walk::body_exprs(&f.body, &mut |e| each(Node::Expr(e)));
        }
    }
    assert!(nodes > 1000, "only {nodes} nodes");
}

#[test]
fn an_id_is_the_path_through_the_bodys_field_names() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let case = load_cases(&dir)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "sem-switch-break")
        .unwrap();
    let m = &case.module;
    let f = m.funcs.iter().find(|f| f.name == "Until").unwrap();
    let mut breaks = Vec::new();
    walk::walk_stmts(&f.body, &mut |s| {
        if let Stmt::Break { .. } = s {
            breaks.push(node_id(m, f, Node::Stmt(s)).unwrap());
        }
    });
    assert_eq!(
        breaks,
        [(
            "sem_switch_break.Until#1.body.0.cases.0.body.0".to_string(),
            "case.go:10".to_string()
        )]
    );
}
