//! Modules refused before anything runs (crate doc § Refusals), beyond the adversary's cases:
//! a call with another number of arguments than the function takes, and a type the module does
//! not declare.

use std::path::Path;

use codespec_conformance::{Case, load_cases};
use codespec_eval::{Options, run_calls};
use codespec_model::{Expr, IntWidth, Stmt, Type};

fn op_add() -> Case {
    load_cases(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases"))
        .unwrap()
        .into_iter()
        .find(|c| c.name == "op-add")
        .unwrap()
}

const INT: Type = Type::Int {
    width: IntWidth::Platform,
    signed: true,
};

#[test]
fn a_call_with_a_wrong_argument_count_is_refused() {
    let mut case = op_add();
    let key = case.module.funcs[0].key();
    let Stmt::Return { results, .. } = &mut case.module.funcs[0].body[0] else {
        panic!("op-add returns");
    };
    results[0] = Expr::Call {
        name: key,
        targs: Vec::new(),
        args: vec![Expr::Lit {
            value: "1".into(),
            t: INT,
        }],
        t: INT,
    };
    let r = run_calls(&case.module, &case.calls, &Options::default());
    let e = r.expect_err("refused");
    assert!(e.contains("with 1 arguments; it takes 2"), "{e}");
}

#[test]
fn an_undeclared_type_is_refused() {
    let mut case = op_add();
    let ret = case.module.funcs[0].body[0].clone();
    case.module.funcs[0].body = vec![
        Stmt::Var {
            name: "v".into(),
            t: Type::Named {
                name: "Nowhere".into(),
                targs: Vec::new(),
            },
            pos: "case.go:5".into(),
        },
        ret,
    ];
    let r = run_calls(&case.module, &case.calls, &Options::default());
    let e = r.expect_err("refused");
    assert!(e.contains("declares no type Nowhere"), "{e}");
}
