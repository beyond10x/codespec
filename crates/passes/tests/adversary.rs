//! Adversary cases for story:passes-and-facts. Each asserts what the crate doc and docs/design.md
//! section 4 promise, or what a writer mapping the facts to Rust needs, on a module the existing
//! suite does not build.

use codespec_model::{
    ChanDir, Expr, FieldInit, Func, IntWidth, Module, Param, Reader, Stmt, Type, TypeDecl,
};
use codespec_passes::{facts, lookup_guard, run};

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

fn index(x: Expr, i: Expr, t: Type) -> Expr {
    Expr::Index {
        x: Box::new(x),
        index: Box::new(i),
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

fn assign(lhs: Vec<Expr>, rhs: Vec<Expr>, n: u32) -> Stmt {
    Stmt::Assign {
        op: String::new(),
        lhs,
        rhs,
        pos: p(n),
    }
}

fn if_not(ok: &str, then: Vec<Stmt>, n: u32) -> Stmt {
    Stmt::If {
        cond: unop("!", var(ok, Type::Bool), Type::Bool),
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
            .map(|(n, t)| codespec_model::Field {
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
    }
}

fn s_ptr() -> Type {
    ptr(named("S"))
}

fn s() -> Expr {
    var("s", s_ptr())
}

fn m_var() -> Expr {
    var("m", map(Type::String, int()))
}

// lookup-guard

/// Go: `v, ok := m["a"]; if !ok { return 0 }; w, ok := m["b"]; if !ok { return 0 }; return v + w`.
/// The second `:=` reuses `ok`. One pass rewrites only the second lookup, because the first one's
/// "rest of the block" still holds the second guard's `!ok`; a second pass then rewrites the first.
#[test]
fn adversary_lookup_guard_is_idempotent_when_two_lookups_share_ok() {
    let body = vec![
        let_(
            &["v", "ok"],
            vec![index_ok(m_var(), lit("\"a\"", Type::String))],
            2,
        ),
        if_not("ok", vec![ret(vec![lit("0", int())], 3)], 3),
        let_(
            &["w", "ok"],
            vec![index_ok(m_var(), lit("\"b\"", Type::String))],
            4,
        ),
        if_not("ok", vec![ret(vec![lit("0", int())], 5)], 5),
        ret(
            vec![Expr::Binop {
                op: "+".into(),
                x: Box::new(var("v", int())),
                y: Box::new(var("w", int())),
                t: int(),
            }],
            6,
        ),
    ];
    let once = lookup_guard(&body);
    let twice = lookup_guard(&once);
    assert_eq!(
        twice, once,
        "lookup-guard is not a fixed point after one pass"
    );

    let m = module(
        vec![],
        vec![func(
            "f",
            vec![param("m", map(Type::String, int()))],
            vec![int()],
            body,
        )],
    );
    let once = run(&m);
    assert_eq!(
        run(&once).to_json(),
        once.to_json(),
        "run(run(m)) differs from run(m)"
    );
}

/// Go: `v, ok := m[k]; if !ok { return v }; return v + 1`. The guard reads `v`, which in Go is the
/// zero value there. A `lookup_else` binds `name` only when the key is present (a Rust writer's
/// `let Some(v) = m.get(&k) else { … }`), so `then` reading `name` must keep the comma-ok form.
#[test]
fn adversary_lookup_guard_leaves_a_guard_whose_then_reads_the_looked_up_value() {
    let body = vec![
        let_(
            &["v", "ok"],
            vec![index_ok(m_var(), var("k", Type::String))],
            2,
        ),
        if_not("ok", vec![ret(vec![var("v", int())], 3)], 3),
        ret(vec![var("v", int())], 4),
    ];
    assert_eq!(lookup_guard(&body), body);
}

// read_only

/// `func (s *S) Items() map[string]int { return s.items }` and
/// `func (s *S) Put() { x := s.Items(); x["a"] = 1 }`: Put writes the receiver's map through the
/// alias a method call returned.
#[test]
fn adversary_read_only_omits_a_write_through_a_map_returned_by_a_receiver_method() {
    let items_t = map(Type::String, int());
    let items = method(
        s_ptr(),
        "Items",
        vec![items_t.clone()],
        vec![ret(vec![field(s(), "items", items_t.clone())], 2)],
    );
    let call = Expr::Method {
        x: Box::new(s()),
        name: "Items".into(),
        args: vec![],
        t: items_t.clone(),
    };
    let put = method(
        s_ptr(),
        "Put",
        vec![],
        vec![
            let_(&["x"], vec![call], 3),
            assign(
                vec![index(
                    var("x", items_t.clone()),
                    lit("\"a\"", Type::String),
                    int(),
                )],
                vec![lit("1", int())],
                4,
            ),
        ],
    );
    let m = module(
        vec![strukt("S", vec![("items", items_t)])],
        vec![items, put],
    );
    let ro = facts(&m).read_only;
    assert!(!ro.contains(&"S.Put".to_string()), "read_only = {ro:?}");
}

/// `type Holder struct { s *S }` makes S shared. `func (s *S) Reset() { t := s; t.n = 0 }` writes
/// its own receiver: the shared-pointer exemption is applied to the receiver's own type.
#[test]
fn adversary_read_only_omits_a_write_through_a_copy_of_a_shared_receiver() {
    let reset = method(
        s_ptr(),
        "Reset",
        vec![],
        vec![
            let_(&["t"], vec![s()], 2),
            assign(
                vec![field(var("t", s_ptr()), "n", int())],
                vec![lit("0", int())],
                3,
            ),
        ],
    );
    let m = module(
        vec![
            strukt("S", vec![("n", int())]),
            strukt("Holder", vec![("s", s_ptr())]),
        ],
        vec![reset],
    );
    let f = facts(&m);
    assert_eq!(f.shared, vec!["S".to_string()]);
    assert!(
        !f.read_only.contains(&"S.Reset".to_string()),
        "read_only = {:?}",
        f.read_only
    );
}

/// S shared as above. `func zero(t *S) { t.n = 0 }` and `func (s *S) Clear() { zero(s) }`: Clear
/// passes its receiver to a function that writes it.
#[test]
fn adversary_read_only_omits_a_shared_receiver_passed_to_a_writing_function() {
    let zero = func(
        "zero",
        vec![param("t", s_ptr())],
        vec![],
        vec![assign(
            vec![field(var("t", s_ptr()), "n", int())],
            vec![lit("0", int())],
            2,
        )],
    );
    let clear = method(
        s_ptr(),
        "Clear",
        vec![],
        vec![Stmt::Expr {
            x: Expr::Call {
                targs: Vec::new(),
                name: "zero".into(),
                args: vec![s()],
                t: Type::Unit,
            },
            pos: p(3),
        }],
    );
    let m = module(
        vec![
            strukt("S", vec![("n", int())]),
            strukt("Holder", vec![("s", s_ptr())]),
        ],
        vec![zero, clear],
    );
    let f = facts(&m);
    assert!(
        !f.read_only.contains(&"S.Clear".to_string()),
        "read_only = {:?}",
        f.read_only
    );
}

/// `func (s *S) Poke() { var xs []int; var ok bool; xs, ok = s.groups["a"]; xs[0] = 1 }`: the
/// crate doc names `assign` as an alias binding, and the two-name assign of a comma-ok index is not
/// recorded (only the equal-length form is), so the write into the receiver's slice is missed.
#[test]
fn adversary_read_only_omits_a_write_through_a_comma_ok_assign_alias() {
    let groups_t = map(Type::String, slice(int()));
    let poke = method(
        s_ptr(),
        "Poke",
        vec![],
        vec![
            Stmt::Var {
                name: "xs".into(),
                t: slice(int()),
                pos: p(2),
            },
            Stmt::Var {
                name: "ok".into(),
                t: Type::Bool,
                pos: p(3),
            },
            assign(
                vec![var("xs", slice(int())), var("ok", Type::Bool)],
                vec![index_ok(
                    field(s(), "groups", groups_t.clone()),
                    lit("\"a\"", Type::String),
                )],
                4,
            ),
            assign(
                vec![index(var("xs", slice(int())), lit("0", int()), int())],
                vec![lit("1", int())],
                5,
            ),
        ],
    );
    let m = module(vec![strukt("S", vec![("groups", groups_t)])], vec![poke]);
    let ro = facts(&m).read_only;
    assert!(!ro.contains(&"S.Poke".to_string()), "read_only = {ro:?}");
}

/// `func (s *S) Wipe() { c := *s; c.items["a"] = 0 }`: the struct copy shares the receiver's map,
/// so the write lands in the receiver. A struct value is never an alias under the crate doc's rule.
#[test]
fn adversary_read_only_omits_a_write_through_a_struct_copy_holding_the_receivers_map() {
    let items_t = map(Type::String, int());
    let wipe = method(
        s_ptr(),
        "Wipe",
        vec![],
        vec![
            let_(&["c"], vec![unop("*", s(), named("S"))], 2),
            assign(
                vec![index(
                    field(var("c", named("S")), "items", items_t.clone()),
                    lit("\"a\"", Type::String),
                    int(),
                )],
                vec![lit("0", int())],
                3,
            ),
        ],
    );
    let m = module(vec![strukt("S", vec![("items", items_t)])], vec![wipe]);
    let ro = facts(&m).read_only;
    assert!(!ro.contains(&"S.Wipe".to_string()), "read_only = {ro:?}");
}

// messages

/// `c <- &A{n: 1}` on a `chan any`: the type sent is A by reference. Neither `messages` nor
/// `shared` names A, so a writer's `enum Message` has no variant for a value the module sends.
#[test]
fn adversary_messages_lists_a_type_sent_by_pointer_on_a_channel_of_any() {
    let send = Stmt::Send {
        chan: var(
            "c",
            Type::Chan {
                elem: Box::new(Type::Any),
                dir: ChanDir::Send,
            },
        ),
        x: unop(
            "&",
            Expr::Struct {
                name: "A".into(),
                fields: vec![FieldInit {
                    name: "n".into(),
                    value: lit("1", int()),
                }],
                t: named("A"),
            },
            ptr(named("A")),
        ),
        pos: p(2),
    };
    let m = module(
        vec![strukt("A", vec![("n", int())])],
        vec![func("f", vec![], vec![], vec![send])],
    );
    let f = facts(&m);
    assert_eq!(f.messages, vec!["A".to_string()], "shared = {:?}", f.shared);
}
