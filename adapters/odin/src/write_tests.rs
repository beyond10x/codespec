use super::*;

fn module() -> Module {
    Module::from_json(include_bytes!(
        "../../../conformance/cases/op-add/case.cir.json"
    ))
    .unwrap()
}

#[test]
fn writes_scalars_and_refuses_missing_facts_and_conflicting_names() {
    let mut m = module();
    let source = render(&m).unwrap();
    assert!(source.contains("Add :: proc("));
    assert!(source.contains("codespec_v_a + codespec_v_b"));
    m.funcs[0].name = "codespec_arg_a".into();
    assert!(render(&m).unwrap_err().contains("identifier"));
    m.facts = None;
    assert!(render(&m).unwrap_err().contains("codespec pass"));
}

#[test]
fn checks_the_whole_module_before_writing_any_file() {
    let mut m = module();
    m.funcs[0].body.push(Stmt::Foreign {
        lang: "odin".into(),
        src: "unsupported".into(),
        reason: "unsupported".into(),
        pos: "module.odin:2".into(),
        calls: None,
    });
    assert!(render(&m).is_err());
}

#[test]
fn refuses_ambiguous_go_octal_instead_of_changing_the_value() {
    let mut m = module();
    let t = m.funcs[0].results[0].clone();
    m.funcs[0].body = vec![Stmt::Return {
        results: vec![Expr::Lit {
            value: "010".into(),
            t,
        }],
        pos: "case.go:3".into(),
    }];
    assert!(render(&m).unwrap_err().contains("signed decimal"));
}

#[test]
fn division_preserves_the_signed_minimum_value() {
    let m = Module::from_json(include_bytes!(
        "../../../conformance/cases/sem-int-wrap/case.cir.json"
    ))
    .unwrap();
    let source = render(&m).unwrap();
    assert!(source.contains("codespec_div_int(codespec_v_a, codespec_v_b)"));
    assert!(source.contains("a == min(int) && b == -1 { return a }"));
    assert!(source.contains("a == min(int) && b == -1 { return 0 }"));
}
