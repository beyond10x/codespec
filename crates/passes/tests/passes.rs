use std::path::{Path, PathBuf};

use codespec_model::{
    Case, ChanDir, Expr, Facts, Field, FieldInit, Func, IntWidth, Module, Param, Reader, Stmt,
    Type, TypeDecl,
};
use codespec_passes::{facts, lookup_guard, run};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn int() -> Type {
    Type::Int {
        width: IntWidth::Platform,
        signed: true,
    }
}

fn named(n: &str) -> Type {
    Type::Named {
        targs: Vec::new(),
        name: n.into(),
    }
}

fn ptr(t: Type) -> Type {
    Type::Ptr { elem: Box::new(t) }
}

fn map(k: Type, v: Type) -> Type {
    Type::Map {
        key: Box::new(k),
        elem: Box::new(v),
    }
}

fn slice(t: Type) -> Type {
    Type::Slice { elem: Box::new(t) }
}

fn chan(t: Type) -> Type {
    Type::Chan {
        elem: Box::new(t),
        dir: ChanDir::Send,
    }
}

fn var(n: &str, t: Type) -> Expr {
    Expr::Var { name: n.into(), t }
}

fn lit(v: &str, t: Type) -> Expr {
    Expr::Lit { value: v.into(), t }
}

fn field(x: Expr, n: &str, t: Type) -> Expr {
    Expr::Field {
        x: Box::new(x),
        name: n.into(),
        t,
    }
}

fn unop(op: &str, x: Expr, t: Type) -> Expr {
    Expr::Unop {
        op: op.into(),
        x: Box::new(x),
        t,
    }
}

fn not(x: Expr) -> Expr {
    unop("!", x, Type::Bool)
}

fn index_ok(m: Expr, k: Expr) -> Expr {
    let elem = match m.t() {
        Type::Map { elem, .. } => (**elem).clone(),
        _ => Type::Unit,
    };
    Expr::Index {
        x: Box::new(m),
        index: Box::new(k),
        t: Type::Tuple {
            elems: vec![elem, Type::Bool],
        },
    }
}

fn p(n: u32) -> String {
    format!("m.go:{n}")
}

fn ret(results: Vec<Expr>, n: u32) -> Stmt {
    Stmt::Return { results, pos: p(n) }
}

fn let_(names: &[&str], rhs: Vec<Expr>, n: u32) -> Stmt {
    Stmt::Let {
        names: names.iter().map(|s| s.to_string()).collect(),
        rhs,
        pos: p(n),
    }
}

fn if_(cond: Expr, then: Vec<Stmt>, n: u32) -> Stmt {
    Stmt::If {
        cond,
        then,
        otherwise: vec![],
        pos: p(n),
    }
}

fn func(name: &str, params: Vec<Param>, results: Vec<Type>, body: Vec<Stmt>) -> Func {
    Func {
        tparams: Vec::new(),
        name: name.into(),
        exported: true,
        recv: None,
        params,
        results,
        raises: vec![],
        body,
        doc: String::new(),
        pos: p(1),
    }
}

fn method(recv: Type, name: &str, results: Vec<Type>, body: Vec<Stmt>) -> Func {
    Func {
        recv: Some(Param {
            name: "s".into(),
            t: recv,
            variadic: false,
        }),
        ..func(name, vec![], results, body)
    }
}

fn param(n: &str, t: Type) -> Param {
    Param {
        name: n.into(),
        t,
        variadic: false,
    }
}

fn strukt(name: &str, fields: Vec<(&str, Type)>) -> TypeDecl {
    TypeDecl::Struct {
        tparams: Vec::new(),
        name: name.into(),
        exported: true,
        fields: fields
            .into_iter()
            .map(|(n, t)| Field {
                name: n.into(),
                t,
                embedded: false,
            })
            .collect(),
        doc: String::new(),
        pos: p(1),
    }
}

fn module(types: Vec<TypeDecl>, funcs: Vec<Func>) -> Module {
    Module {
        format: "cir/1".into(),
        name: "m".into(),
        reader: Reader {
            language: "go".into(),
            name: "test".into(),
            version: "0".into(),
        },
        doc: String::new(),
        types,
        consts: vec![],
        errors: vec![],
        vars: vec![],
        funcs,
        externs: vec![],
        layout: vec![],
        facts: None,
        refusals: vec![],
        path: String::new(),
        opaque: vec![],
        methods: vec![],
        linknames: vec![],
        inits: vec![],
        interfaces: vec![],
        entries: vec![],
        beans: vec![],
        injects: vec![],
        transactional: vec![],
        persistence: vec![],
    }
}

fn m_var() -> Expr {
    var("m", map(Type::String, int()))
}

fn guarded_lookup(after: Vec<Stmt>) -> Vec<Stmt> {
    let mut body = vec![
        let_(
            &["v", "ok"],
            vec![index_ok(m_var(), var("k", Type::String))],
            2,
        ),
        if_(
            not(var("ok", Type::Bool)),
            vec![ret(vec![lit("0", int())], 3)],
            2,
        ),
    ];
    body.extend(after);
    body
}

// lookup-guard

#[test]
fn lookup_guard_rewrites_a_guarded_comma_ok_lookup() {
    let body = guarded_lookup(vec![ret(vec![var("v", int())], 5)]);
    let out = lookup_guard(&body);
    assert_eq!(
        out,
        vec![
            Stmt::LookupElse {
                name: "v".into(),
                map: m_var(),
                index: var("k", Type::String),
                then: vec![ret(vec![lit("0", int())], 3)],
                pos: p(2),
            },
            ret(vec![var("v", int())], 5),
        ]
    );
}

#[test]
fn lookup_guard_rewrites_inside_nested_blocks() {
    let inner = guarded_lookup(vec![ret(vec![var("v", int())], 5)]);
    let body = vec![Stmt::Switch {
        tag: None,
        cases: vec![Case {
            values: vec![],
            body: vec![Stmt::ForRange {
                key: "_".into(),
                val: "x".into(),
                over: var("xs", slice(int())),
                body: inner,
                pos: p(1),
            }],
        }],
        pos: p(1),
    }];
    let out = lookup_guard(&body);
    let Stmt::Switch { cases, .. } = &out[0] else {
        panic!("not a switch")
    };
    let Stmt::ForRange { body, .. } = &cases[0].body[0] else {
        panic!("not a for_range")
    };
    assert!(matches!(body[0], Stmt::LookupElse { .. }), "{body:?}");
}

#[test]
fn lookup_guard_leaves_a_lookup_whose_ok_is_read_later() {
    let body = guarded_lookup(vec![ret(vec![var("v", int()), var("ok", Type::Bool)], 5)]);
    assert_eq!(lookup_guard(&body), body);
}

#[test]
fn lookup_guard_leaves_a_guard_that_does_not_end_in_return() {
    let body = vec![
        let_(
            &["v", "ok"],
            vec![index_ok(m_var(), var("k", Type::String))],
            2,
        ),
        if_(
            not(var("ok", Type::Bool)),
            vec![Stmt::Incr {
                x: var("n", int()),
                pos: p(3),
            }],
            2,
        ),
        ret(vec![var("v", int())], 5),
    ];
    assert_eq!(lookup_guard(&body), body);
}

#[test]
fn lookup_guard_leaves_a_guard_followed_by_foreign_code() {
    let body = guarded_lookup(vec![Stmt::Foreign {
        lang: "go".into(),
        src: "go use(ok)".into(),
        reason: "goroutine".into(),
        pos: p(4),
        calls: None,
    }]);
    assert_eq!(lookup_guard(&body), body);
}

// shared

#[test]
fn shared_lists_a_struct_stored_by_reference_in_a_map_field() {
    let m = module(
        vec![
            strukt("Item", vec![("n", int())]),
            strukt(
                "Store",
                vec![("items", map(Type::String, ptr(named("Item"))))],
            ),
        ],
        vec![],
    );
    assert_eq!(facts(&m).shared, vec!["Item".to_string()]);
}

#[test]
fn shared_lists_a_struct_held_by_reference_in_a_local_slice() {
    let m = module(
        vec![strukt("Item", vec![("n", int())])],
        vec![func(
            "f",
            vec![],
            vec![],
            vec![Stmt::Var {
                name: "xs".into(),
                t: slice(ptr(named("Item"))),
                pos: p(2),
            }],
        )],
    );
    assert_eq!(facts(&m).shared, vec!["Item".to_string()]);
}

#[test]
fn shared_lists_a_struct_held_by_reference_as_a_type_argument() {
    let boxed = Type::Named {
        name: "Box".into(),
        targs: vec![ptr(named("Item"))],
    };
    let m = module(
        vec![strukt("Item", vec![("n", int())])],
        vec![func(
            "f",
            vec![],
            vec![],
            vec![Stmt::Var {
                name: "b".into(),
                t: boxed,
                pos: p(2),
            }],
        )],
    );
    assert_eq!(facts(&m).shared, vec!["Item".to_string()]);
}

#[test]
fn shared_omits_a_struct_stored_by_value() {
    let m = module(
        vec![
            strukt("Item", vec![("n", int())]),
            strukt("Store", vec![("items", map(Type::String, named("Item")))]),
        ],
        vec![func(
            "f",
            vec![param("i", ptr(named("Item")))],
            vec![],
            vec![],
        )],
    );
    assert!(facts(&m).shared.is_empty());
}

// messages

fn send(chan_t: Type, x: Expr) -> Stmt {
    Stmt::Send {
        chan: var("c", chan_t),
        x,
        pos: p(2),
    }
}

fn ev(name: &str) -> Expr {
    Expr::Struct {
        name: name.into(),
        fields: vec![FieldInit {
            name: "n".into(),
            value: lit("1", int()),
        }],
        t: named(name),
    }
}

#[test]
fn messages_lists_types_sent_on_a_channel_of_any_in_declaration_order() {
    let m = module(
        vec![
            strukt("A", vec![("n", int())]),
            strukt("B", vec![("n", int())]),
        ],
        vec![func(
            "f",
            vec![],
            vec![],
            vec![
                send(chan(Type::Any), ev("B")),
                send(chan(Type::Any), ev("A")),
            ],
        )],
    );
    assert_eq!(facts(&m).messages, vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn messages_omits_types_sent_on_a_typed_channel() {
    let m = module(
        vec![strukt("A", vec![("n", int())])],
        vec![func(
            "f",
            vec![],
            vec![],
            vec![send(chan(named("A")), ev("A"))],
        )],
    );
    assert!(facts(&m).messages.is_empty());
}

// fallible

fn errvar() -> Expr {
    Expr::Errvar {
        name: "ErrX".into(),
        t: Type::Error,
    }
}

#[test]
fn fallible_lists_a_function_whose_error_returns_carry_zero_values() {
    let body = vec![
        if_(
            var("bad", Type::Bool),
            vec![ret(vec![lit("\"\"", Type::String), errvar()], 2)],
            2,
        ),
        Stmt::LookupElse {
            name: "v".into(),
            map: var("m", map(Type::String, Type::String)),
            index: var("k", Type::String),
            then: vec![ret(vec![Expr::Zero { t: Type::String }, errvar()], 3)],
            pos: p(3),
        },
        ret(
            vec![var("v", Type::String), Expr::Nil { t: Type::Error }],
            4,
        ),
    ];
    let m = module(
        vec![],
        vec![func("f", vec![], vec![Type::String, Type::Error], body)],
    );
    assert_eq!(facts(&m).fallible, vec!["f".to_string()]);
}

#[test]
fn fallible_omits_a_function_returning_a_value_beside_an_error() {
    let body = vec![ret(vec![var("v", Type::String), errvar()], 2)];
    let m = module(
        vec![],
        vec![
            func("f", vec![], vec![Type::String, Type::Error], body),
            func(
                "g",
                vec![],
                vec![Type::Error, Type::String],
                vec![ret(
                    vec![Expr::Nil { t: Type::Error }, lit("\"\"", Type::String)],
                    2,
                )],
            ),
        ],
    );
    assert!(facts(&m).fallible.is_empty());
}

// comma_ok

#[test]
fn comma_ok_lists_a_function_returning_zero_false_or_value_true() {
    let body = vec![
        if_(
            var("bad", Type::Bool),
            vec![ret(
                vec![
                    Expr::Struct {
                        name: "T".into(),
                        fields: vec![],
                        t: named("T"),
                    },
                    lit("false", Type::Bool),
                ],
                2,
            )],
            2,
        ),
        ret(vec![var("x", named("T")), lit("true", Type::Bool)], 3),
    ];
    let m = module(
        vec![strukt("T", vec![("n", int())])],
        vec![func("f", vec![], vec![named("T"), Type::Bool], body)],
    );
    assert_eq!(facts(&m).comma_ok, vec!["f".to_string()]);
}

#[test]
fn comma_ok_omits_a_function_returning_a_value_with_false() {
    let body = vec![
        ret(vec![var("x", int()), lit("false", Type::Bool)], 2),
        ret(vec![var("x", int()), var("b", Type::Bool)], 3),
    ];
    let m = module(
        vec![],
        vec![func("f", vec![], vec![int(), Type::Bool], body)],
    );
    assert!(facts(&m).comma_ok.is_empty());
}

// read_only

fn recv_t() -> Type {
    ptr(named("S"))
}

fn s() -> Expr {
    var("s", recv_t())
}

fn s_types() -> Vec<TypeDecl> {
    vec![
        strukt("Item", vec![("n", int())]),
        strukt(
            "S",
            vec![
                ("n", int()),
                ("items", map(Type::String, ptr(named("Item")))),
                ("box", ptr(named("Box"))),
            ],
        ),
        strukt("Box", vec![("n", int())]),
    ]
}

#[test]
fn read_only_lists_a_method_that_only_reads_its_receiver_or_writes_a_shared_item() {
    let get = method(
        recv_t(),
        "Get",
        vec![int()],
        vec![ret(vec![field(s(), "n", int())], 2)],
    );
    let touch = method(
        recv_t(),
        "Touch",
        vec![],
        vec![
            Stmt::LookupElse {
                name: "it".into(),
                map: field(s(), "items", map(Type::String, ptr(named("Item")))),
                index: lit("\"a\"", Type::String),
                then: vec![ret(vec![], 3)],
                pos: p(3),
            },
            Stmt::Incr {
                x: field(var("it", ptr(named("Item"))), "n", int()),
                pos: p(4),
            },
        ],
    );
    let m = module(s_types(), vec![get, touch]);
    assert_eq!(
        facts(&m).read_only,
        vec!["S.Get".to_string(), "S.Touch".to_string()]
    );
}

#[test]
fn read_only_omits_methods_that_write_through_the_receiver_an_alias_or_a_call() {
    let bump = method(
        recv_t(),
        "Bump",
        vec![],
        vec![Stmt::Incr {
            x: field(s(), "n", int()),
            pos: p(2),
        }],
    );
    let calls = method(
        recv_t(),
        "Calls",
        vec![],
        vec![Stmt::Expr {
            x: Expr::Method {
                x: Box::new(s()),
                name: "Bump".into(),
                args: vec![],
                t: Type::Unit,
            },
            pos: p(3),
        }],
    );
    let alias = method(
        recv_t(),
        "Alias",
        vec![],
        vec![
            let_(
                &["p"],
                vec![unop("&", field(s(), "n", int()), ptr(int()))],
                4,
            ),
            Stmt::Assign {
                op: String::new(),
                lhs: vec![unop("*", var("p", ptr(int())), int())],
                rhs: vec![lit("1", int())],
                pos: p(5),
            },
        ],
    );
    let foreign = method(
        recv_t(),
        "Foreign",
        vec![],
        vec![Stmt::Foreign {
            lang: "go".into(),
            src: "go func() { s.n = 1 }()".into(),
            reason: "goroutine".into(),
            pos: p(6),
            calls: None,
        }],
    );
    let m = module(s_types(), vec![bump, calls, alias, foreign]);
    assert!(facts(&m).read_only.is_empty(), "{:?}", facts(&m).read_only);
}

/// `func (s *L) Hold() { s.mu.Lock() }`: an extern method may write its receiver, and the module
/// cannot see whether it does.
#[test]
fn read_only_omits_a_method_calling_an_extern_method_on_its_receiver() {
    let mutex = Type::Extern {
        targs: Vec::new(),
        path: "sync".into(),
        name: "Mutex".into(),
    };
    let l = ptr(named("L"));
    let hold = Func {
        recv: Some(param("s", l.clone())),
        ..func(
            "Hold",
            vec![],
            vec![],
            vec![Stmt::Expr {
                x: Expr::ExternMethod {
                    targs: Vec::new(),
                    path: "sync".into(),
                    type_name: "Mutex".into(),
                    name: "Lock".into(),
                    recv: Box::new(field(var("s", l), "mu", mutex.clone())),
                    args: vec![],
                    t: Type::Tuple { elems: vec![] },
                },
                pos: p(2),
            }],
        )
    };
    let m = module(vec![strukt("L", vec![("mu", mutex)])], vec![hold]);
    assert!(facts(&m).read_only.is_empty(), "{:?}", facts(&m).read_only);
}

/// `func (s *L) Flush() { s.w.Flush() }` and `func (s *L) Hand(c Sink) { c.Take(s) }` with `w`
/// and `c` of the interface `Sink`: the method that runs through an interface may write its
/// receiver or an argument, and the module cannot see which runs. `Peek`, which only reads a
/// field, stays read-only.
#[test]
fn read_only_omits_a_method_calling_through_an_interface_on_or_with_its_receiver() {
    let sink = named("Sink");
    let l = ptr(named("L"));
    let call = |recv: Expr, args: Vec<Expr>, name: &str| Stmt::Expr {
        x: Expr::IfaceCall {
            path: String::new(),
            iface: "Sink".into(),
            name: name.into(),
            recv: Box::new(recv),
            args,
            t: Type::Tuple { elems: vec![] },
        },
        pos: p(2),
    };
    let recv = |name: &str, params: Vec<Param>, body: Vec<Stmt>| Func {
        recv: Some(param("s", l.clone())),
        params,
        ..func(name, vec![], vec![], body)
    };
    let flush = recv(
        "Flush",
        vec![],
        vec![call(
            field(var("s", l.clone()), "w", sink.clone()),
            vec![],
            "Flush",
        )],
    );
    let hand = recv(
        "Hand",
        vec![param("c", sink.clone())],
        vec![call(
            var("c", sink.clone()),
            vec![Expr::ToIface {
                x: Box::new(var("s", l.clone())),
                t: sink.clone(),
            }],
            "Take",
        )],
    );
    let peek = Func {
        recv: Some(param("s", l.clone())),
        ..func(
            "Peek",
            vec![],
            vec![int()],
            vec![Stmt::Return {
                results: vec![field(var("s", l.clone()), "n", int())],
                pos: p(3),
            }],
        )
    };
    let mut m = module(
        vec![strukt("L", vec![("w", sink.clone()), ("n", int())])],
        vec![flush, hand, peek],
    );
    m.interfaces = vec![codespec_model::Interface {
        path: String::new(),
        name: "Sink".into(),
        exported: true,
        methods: vec![
            codespec_model::IfaceMethod {
                name: "Flush".into(),
                params: vec![],
                results: vec![],
            },
            codespec_model::IfaceMethod {
                name: "Take".into(),
                params: vec![sink],
                results: vec![],
            },
        ],
        doc: String::new(),
        pos: p(1),
    }];
    assert_eq!(facts(&m).read_only, ["L.Peek"]);
}

// displays

fn newtype(name: &str, under: Type) -> TypeDecl {
    TypeDecl::Newtype {
        tparams: Vec::new(),
        name: name.into(),
        exported: true,
        under,
        doc: String::new(),
        pos: p(1),
    }
}

#[test]
fn displays_lists_error_and_string_methods_and_string_newtypes() {
    let m = module(
        vec![
            strukt("E", vec![("n", int())]),
            newtype("Id", Type::String),
            TypeDecl::Enum {
                name: "State".into(),
                exported: true,
                under: Type::String,
                variants: vec![],
                doc: String::new(),
                pos: p(1),
            },
        ],
        vec![method(
            named("E"),
            "Error",
            vec![Type::String],
            vec![ret(vec![lit("\"e\"", Type::String)], 2)],
        )],
    );
    assert_eq!(
        facts(&m).displays,
        vec!["E".to_string(), "Id".to_string(), "State".to_string()]
    );
}

#[test]
fn displays_omits_int_newtypes_and_methods_not_returning_a_string() {
    let m = module(
        vec![strukt("E", vec![("n", int())]), newtype("Count", int())],
        vec![method(
            named("E"),
            "String",
            vec![int()],
            vec![ret(vec![lit("1", int())], 2)],
        )],
    );
    assert!(facts(&m).displays.is_empty());
}

// unordered_iteration

fn range(over: Expr, n: u32) -> Stmt {
    Stmt::ForRange {
        key: "k".into(),
        val: "_".into(),
        over,
        body: vec![],
        pos: p(n),
    }
}

#[test]
fn unordered_iteration_lists_loops_over_a_map() {
    let m = module(
        vec![],
        vec![func(
            "f",
            vec![],
            vec![],
            vec![if_(var("b", Type::Bool), vec![range(m_var(), 3)], 2)],
        )],
    );
    assert_eq!(facts(&m).unordered_iteration, vec![p(3)]);
}

#[test]
fn unordered_iteration_omits_loops_over_a_slice() {
    let m = module(
        vec![],
        vec![func(
            "f",
            vec![],
            vec![],
            vec![range(var("xs", slice(int())), 3)],
        )],
    );
    assert!(facts(&m).unordered_iteration.is_empty());
}

// the whole pipeline

fn orders_read() -> Module {
    let bytes = std::fs::read(root().join("examples/orders/orders.read.cir.json")).expect("read");
    Module::from_json(&bytes).expect("valid")
}

#[test]
fn passes_run_twice_give_the_same_bytes_as_once() {
    let once = run(&orders_read());
    let twice = run(&once);
    assert_eq!(once.to_json(), twice.to_json());
    let synthetic = module(
        s_types(),
        vec![func(
            "f",
            vec![],
            vec![int()],
            guarded_lookup(vec![ret(vec![var("v", int())], 5)]),
        )],
    );
    let once = run(&synthetic);
    assert_eq!(once.to_json(), run(&once).to_json());
}

#[test]
fn the_checked_in_orders_document_is_what_the_passes_produce() {
    let path = root().join("examples/orders/orders.cir.json");
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == run(&orders_read()).to_json(),
        "{} differs from the passes' output; run `task examples`",
        path.display()
    );
}

#[test]
fn orders_gets_its_lookups_rewritten_and_its_facts() {
    let m = run(&orders_read());
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(
        m.facts,
        Some(Facts {
            shared: s(&["Order"]),
            messages: s(&["OrderPlaced", "OrderPaid", "OrderShipped", "OrderCancelled"]),
            comma_ok: s(&["Service.Get"]),
            fallible: s(&[
                "Service.Place",
                "Service.Pay",
                "Service.Ship",
                "Service.Cancel"
            ]),
            read_only: s(&[
                "ErrWrongState.Error",
                "Service.Pay",
                "Service.Ship",
                "Service.Cancel",
                "Service.Get",
                "Service.PendingOrders",
            ]),
            displays: s(&["OrderID", "Status", "ErrWrongState"]),
            unordered_iteration: s(&["orders.go:123"]),
        })
    );
    let lookups = m
        .funcs
        .iter()
        .filter(|f| matches!(f.body.first(), Some(Stmt::LookupElse { .. })))
        .map(|f| f.key())
        .collect::<Vec<_>>();
    assert_eq!(
        lookups,
        s(&[
            "Service.Pay",
            "Service.Ship",
            "Service.Cancel",
            "Service.Get"
        ])
    );
}
#[test]
fn read_only_omits_a_method_whose_lambda_writes_the_receiver_and_one_holding_a_method_value() {
    let unit = Type::Func {
        params: vec![],
        results: vec![],
    };
    let lambda = method(
        recv_t(),
        "Later",
        vec![unit.clone()],
        vec![ret(
            vec![Expr::Lambda {
                params: vec![],
                results: vec![],
                captures: vec!["s".into()],
                body: vec![Stmt::Incr {
                    x: field(s(), "n", int()),
                    pos: p(3),
                }],
                t: unit.clone(),
            }],
            2,
        )],
    );
    let bound = method(
        recv_t(),
        "Bound",
        vec![unit.clone()],
        vec![ret(
            vec![Expr::FuncRef {
                name: "Later".into(),
                recv: Some(Box::new(s())),
                t: unit,
            }],
            5,
        )],
    );
    let m = module(s_types(), vec![lambda, bound]);
    assert!(facts(&m).read_only.is_empty(), "{:?}", facts(&m).read_only);
}
