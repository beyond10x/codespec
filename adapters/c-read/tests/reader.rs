//! `codespec-read-c` on C sources written for one property each: what it refuses, what it links,
//! what it does not claim, and how it fails.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use codespec_conformance::conform::reader_corpus;
use codespec_conformance::features_used;
use codespec_model::{Expr, Module, Stmt, Type, manifest_from_json, walk};

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// A fresh source directory `target/c-read-tests/<name>/<name>` holding `files`.
fn source(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let work = root().join("target/c-read-tests").join(name);
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join(name);
    std::fs::create_dir_all(&dir).expect("source dir");
    for (f, text) in files {
        std::fs::write(dir.join(f), text).expect("write source");
    }
    dir
}

fn read_with(dir: &Path, path: Option<&str>) -> Output {
    let mut cmd = Command::new(READ_C);
    cmd.arg(dir);
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    cmd.output().expect("run codespec-read-c")
}

/// Read `dir`: exit 0 and a module strict reading accepts.
fn read(dir: &Path) -> Module {
    let o = read_with(dir, None);
    assert!(
        o.status.success(),
        "codespec-read-c failed ({}):\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    Module::from_json(&o.stdout).expect("a strict cir/1 module")
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} is in the source"))
        + 1
}

fn foreigns(m: &Module) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in &m.funcs {
        walk::walk_stmts(&f.body, &mut |s| {
            if let Stmt::Foreign { src, pos, .. } = s {
                out.push((pos.clone(), src.clone()));
            }
        });
    }
    out
}

#[test]
fn a_goto_reads_with_one_refusal_at_its_line_and_exits_zero() {
    let text = "long count(long n) {\n    long i = 0;\nagain:\n    i++;\n    if (i < n) {\n        goto again;\n    }\n    return i;\n}\n";
    let dir = source("goto", &[("count.c", text)]);
    let m = read(&dir);
    let at = format!("count.c:{}", line_of(text, "goto again"));
    assert_eq!(m.refusals.len(), 1, "{:?}", m.refusals);
    assert_eq!(m.refusals[0].pos, at, "{:?}", m.refusals);
    assert_eq!(foreigns(&m), vec![(at, "goto again;".to_string())]);
}

#[test]
fn the_reader_does_not_claim_int_wrap_for_a_signed_addition_that_overflows() {
    let text = "#include <limits.h>\n\nlong add(long a, long b) {\n    return a + b;\n}\n\nlong over(void) {\n    return add(LONG_MAX, 1);\n}\n";
    let dir = source("wrap", &[("wrap.c", text)]);
    let m = read(&dir);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let used = features_used(&m).expect("features of the module");
    assert!(
        used.contains("sem.int.wrap") && used.contains("op.add"),
        "the signed + is read: {used:?}"
    );
    let o = Command::new(READ_C)
        .arg("--describe")
        .output()
        .expect("--describe");
    assert!(o.status.success());
    let claimed = manifest_from_json(&o.stdout)
        .expect("a strict manifest")
        .claimed();
    assert!(claimed.contains("op.add"), "{claimed:?}");
    assert!(!claimed.contains("sem.int.wrap"), "{claimed:?}");
}

#[test]
fn without_clang_on_path_the_reader_exits_1_and_names_it() {
    let dir = source("noclang", &[("a.c", "long one(void) { return 1; }\n")]);
    let empty = root().join("target/c-read-tests/noclang/empty-path");
    std::fs::create_dir_all(&empty).expect("empty dir");
    let o = read_with(&dir, Some(empty.to_str().expect("UTF-8 path")));
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(o.stdout.is_empty(), "{o:?}");
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "codespec-read-c: clang not found\n"
    );
}

#[test]
fn libc_calls_are_externs_with_their_declared_signatures_and_effects_link_them() {
    let text = "#include <stdio.h>\n#include <stdlib.h>\n\nlong hello(long n) {\n    printf(\"hello %ld\\n\", n);\n    return labs(n);\n}\n";
    let dir = source("libc", &[("hello.c", text)]);
    let m = read(&dir);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let int64 = Type::Int {
        width: codespec_model::IntWidth::W64,
        signed: true,
    };
    let keys: Vec<String> = m.externs.iter().map(|x| x.key()).collect();
    assert_eq!(keys, ["libc.labs", "libc.printf"]);
    let labs = &m.externs[0];
    assert_eq!(labs.params, vec![int64.clone()]);
    assert_eq!(labs.results, vec![int64.clone()]);
    let printf = &m.externs[1];
    assert!(printf.variadic);
    assert_eq!(
        printf.params,
        vec![
            Type::String,
            Type::Slice {
                elem: Box::new(Type::Any)
            }
        ]
    );
    let f = &m.funcs[0];
    let Stmt::Expr { x, .. } = &f.body[0] else {
        panic!(
            "the printf call is an expression statement: {:?}",
            f.body[0]
        );
    };
    let Expr::Extern {
        path, name, args, ..
    } = x
    else {
        panic!("printf is an extern call: {x:?}");
    };
    assert_eq!((path.as_str(), name.as_str()), ("libc", "printf"));
    assert_eq!(
        args[0],
        Expr::Lit {
            value: "\"hello %ld\\n\"".into(),
            t: Type::String
        }
    );

    let summary = codespec_effects::summary_from_json(
        br#"{"format": "codespec-effects/1", "language": "c", "externs": [{"path": "libc", "name": "labs", "effects": []}, {"path": "libc", "name": "printf", "effects": ["stdout"]}]}"#,
    )
    .expect("a summary");
    let report = codespec_effects::analyze(&m, &[summary]);
    let hello = report
        .funcs
        .iter()
        .find(|f| f.name == "hello")
        .expect("hello in the report");
    assert_eq!(hello.effects, ["stdout"]);
    assert!(hello.unknown.is_empty(), "{:?}", hello.unknown);
}

#[test]
fn constructs_outside_the_subset_are_kept_with_a_refusal_at_their_line() {
    let text = r#"#include <stdarg.h>

union Num {
    long i;
    double d;
};

long first(long n) {
    long *p = &n;
    p = p + 1;
    return n;
}

long sum(int n, ...) {
    return n;
}

long viaVolatile(volatile long x) {
    return x;
}

long fenced(long x) {
    __asm__ volatile("" ::: "memory");
    return x;
}

long walk(long n) {
    long s = 0;
    s = s + n;
    return s;
}
"#;
    let dir = source("outside", &[("outside.c", text)]);
    let m = read(&dir);
    let mut got: Vec<(usize, String)> = m
        .refusals
        .iter()
        .map(|r| {
            let line = r
                .pos
                .strip_prefix("outside.c:")
                .and_then(|l| l.parse().ok())
                .unwrap_or_else(|| panic!("a position in outside.c: {r:?}"));
            (line, r.reason.clone())
        })
        .collect();
    got.sort();
    let want = [
        (line_of(text, "union Num"), "union"),
        (line_of(text, "p = p + 1"), "pointer arithmetic"),
        (line_of(text, "long sum"), "variadic"),
        (line_of(text, "long viaVolatile"), "volatile"),
        (line_of(text, "__asm__"), "inline assembly"),
    ];
    assert_eq!(got.len(), want.len(), "{:#?}", m.refusals);
    for ((line, reason), (want_line, word)) in got.iter().zip(want) {
        assert!(
            *line == want_line && reason.contains(word),
            "refusal at line {line} ({reason}), expected line {want_line} naming {word}: {:#?}",
            m.refusals
        );
    }
    let opaque: Vec<&str> = m.opaque.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(opaque, ["sum", "viaVolatile"]);
    let read: Vec<&str> = m.funcs.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(read, ["first", "fenced", "walk"]);
    assert_eq!(foreigns(&m).len(), 2, "{:#?}", foreigns(&m));
}

/// Read `text` as `<name>.c`: no refusal, and the module's features.
fn read_clean(name: &str, text: &str) -> (Module, std::collections::BTreeSet<String>) {
    let dir = source(name, &[(&format!("{name}.c"), text)]);
    let m = read(&dir);
    assert!(m.refusals.is_empty(), "{:#?}", m.refusals);
    let used = features_used(&m).expect("features of the module");
    (m, used)
}

fn int64() -> Type {
    Type::Int {
        width: codespec_model::IntWidth::W64,
        signed: true,
    }
}

fn named(n: &str) -> Type {
    Type::Named {
        name: n.into(),
        targs: Vec::new(),
    }
}

/// Every expression of the function `name`, in walk order.
fn exprs_of<'m>(m: &'m Module, name: &str) -> Vec<&'m Expr> {
    let f = m
        .funcs
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is read: {:#?}", m.opaque));
    let mut out = Vec::new();
    walk::body_exprs(&f.body, &mut |e| out.push(e));
    out
}

#[test]
fn a_typedef_of_an_integer_type_is_a_newtype_and_a_cast_to_it_a_conversion() {
    let text = "typedef long Celsius;\n\nCelsius warm(long n) {\n    Celsius c = (Celsius)n;\n    return c + 1;\n}\n\nlong plain(long n) {\n    Celsius c = n;\n    return n;\n}\n";
    let dir = source("newtype", &[("newtype.c", text)]);
    let m = read(&dir);
    assert_eq!(
        m.types,
        vec![codespec_model::TypeDecl::Newtype {
            name: "Celsius".into(),
            exported: true,
            tparams: Vec::new(),
            under: int64(),
            doc: String::new(),
            pos: "newtype.c:1".into(),
        }]
    );
    let warm = &m.funcs[0];
    assert_eq!(warm.results, vec![named("Celsius")]);
    let Stmt::Let { rhs, .. } = &warm.body[0] else {
        panic!("a let: {:?}", warm.body[0]);
    };
    assert_eq!(
        rhs[0],
        Expr::Conv {
            name: "Celsius".into(),
            x: Box::new(Expr::Var {
                name: "n".into(),
                t: int64()
            }),
            t: named("Celsius"),
        }
    );
    let Stmt::Return { results, .. } = &warm.body[1] else {
        panic!("a return: {:?}", warm.body[1]);
    };
    let Expr::Binop { y, t, .. } = &results[0] else {
        panic!("c + 1 is a binop: {results:?}");
    };
    assert_eq!((t, y.t()), (&named("Celsius"), &named("Celsius")));
    let used = features_used(&m).unwrap();
    for id in ["decl.newtype", "expr.conv", "type.named"] {
        assert!(used.contains(id), "{id}: {used:?}");
    }
    assert_eq!(m.refusals.len(), 1, "{:#?}", m.refusals);
    assert_eq!(
        m.refusals[0].pos,
        format!("newtype.c:{}", line_of(text, "Celsius c = n;")),
        "a long initializing a Celsius without a cast is refused: {:#?}",
        m.refusals
    );
}

#[test]
fn float_and_double_are_floats_and_their_literals_float_literals() {
    let text = "#include <stdbool.h>\n\nbool over(bool big) {\n    double x;\n    float y = 0.1f;\n    x = 0.25;\n    if (big) {\n        x = 1.5e300;\n        y = -y * 3.0f;\n    }\n    return x / 4.0 > 0.5 && y < 1e21f;\n}\n";
    let (m, used) = read_clean("floats", text);
    let f64t = Type::Float {
        bits: codespec_model::FloatBits::W64,
    };
    let f32t = Type::Float {
        bits: codespec_model::FloatBits::W32,
    };
    assert_eq!(
        m.funcs[0].body[0],
        Stmt::Var {
            name: "x".into(),
            t: f64t.clone(),
            pos: "floats.c:4".into()
        }
    );
    let floats: Vec<(String, Type)> = exprs_of(&m, "over")
        .into_iter()
        .filter_map(|e| match e {
            Expr::Float { value, t } => Some((value.clone(), t.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        floats,
        [
            ("0.1".to_string(), f32t.clone()),
            ("0.25".to_string(), f64t.clone()),
            ("1.5e300".to_string(), f64t.clone()),
            ("3".to_string(), f32t.clone()),
            ("4".to_string(), f64t.clone()),
            ("0.5".to_string(), f64t.clone()),
            ("1e21".to_string(), f32t.clone()),
        ]
    );
    for id in [
        "type.float",
        "expr.float",
        "op.div",
        "op.gt",
        "op.lt",
        "unop.neg",
    ] {
        assert!(used.contains(id), "{id}: {used:?}");
    }
}

#[test]
fn a_local_array_is_an_array_and_a_subscript_of_it_an_index() {
    let text = "long pick(long i) {\n    long a[3] = {0};\n    a[0] = 10;\n    a[1] = a[0] * 2;\n    return a[i];\n}\n";
    let (m, used) = read_clean("array", text);
    let arr = Type::Array {
        len: 3,
        elem: Box::new(int64()),
    };
    assert_eq!(
        m.funcs[0].body[0],
        Stmt::Var {
            name: "a".into(),
            t: arr.clone(),
            pos: "array.c:2".into()
        }
    );
    let Stmt::Return { results, .. } = &m.funcs[0].body[3] else {
        panic!("a return: {:?}", m.funcs[0].body[3]);
    };
    assert_eq!(
        results[0],
        Expr::Index {
            x: Box::new(Expr::Var {
                name: "a".into(),
                t: arr
            }),
            index: Box::new(Expr::Var {
                name: "i".into(),
                t: int64()
            }),
            t: int64(),
        }
    );
    for id in ["type.array", "expr.index"] {
        assert!(used.contains(id), "{id}: {used:?}");
    }
    let dir = source(
        "pointer-index",
        &[(
            "pointer-index.c",
            "long at(long n) {\n    long *p = &n;\n    return p[0];\n}\n",
        )],
    );
    let m = read(&dir);
    assert_eq!(m.refusals.len(), 1, "{:#?}", m.refusals);
    assert!(
        m.refusals[0].reason.contains("pointer"),
        "indexing a pointer is pointer arithmetic: {:#?}",
        m.refusals
    );
}

#[test]
fn a_pointer_is_a_ptr_an_address_a_reference_a_star_a_dereference_and_zero_nil() {
    let text = "#include <stdbool.h>\n\nstruct Box {\n    long v;\n};\n\nlong bump(long n) {\n    long *p = &n;\n    *p = *p + 1;\n    return n;\n}\n\nlong boxed(long v) {\n    struct Box *b = &(struct Box){v};\n    return b->v;\n}\n\nbool found(long n) {\n    long *p = 0;\n    if (n > 0) {\n        p = &n;\n    }\n    return p != 0;\n}\n";
    let (m, used) = read_clean("pointers", text);
    let ptr = |t: Type| Type::Ptr { elem: Box::new(t) };
    let n = Expr::Var {
        name: "n".into(),
        t: int64(),
    };
    let p = Expr::Var {
        name: "p".into(),
        t: ptr(int64()),
    };
    let bump = &m.funcs[0];
    assert_eq!(
        bump.body[0],
        Stmt::Let {
            names: vec!["p".into()],
            rhs: vec![Expr::Unop {
                op: "&".into(),
                x: Box::new(n.clone()),
                t: ptr(int64())
            }],
            pos: "pointers.c:8".into()
        }
    );
    let Stmt::Assign { lhs, .. } = &bump.body[1] else {
        panic!("an assign: {:?}", bump.body[1]);
    };
    assert_eq!(
        lhs[0],
        Expr::Unop {
            op: "*".into(),
            x: Box::new(p.clone()),
            t: int64()
        }
    );
    let Stmt::Return { results, .. } = &m.funcs[1].body[1] else {
        panic!("a return: {:?}", m.funcs[1].body[1]);
    };
    assert_eq!(
        results[0],
        Expr::Field {
            x: Box::new(Expr::Var {
                name: "b".into(),
                t: ptr(named("Box"))
            }),
            name: "v".into(),
            t: int64()
        },
        "b->v is a field of the pointer, as cir/1 reads a field through one"
    );
    let found = &m.funcs[2];
    assert_eq!(
        found.body[0],
        Stmt::Var {
            name: "p".into(),
            t: ptr(int64()),
            pos: "pointers.c:19".into()
        }
    );
    let Stmt::Return { results, .. } = &found.body[2] else {
        panic!("a return: {:?}", found.body[2]);
    };
    assert_eq!(
        results[0],
        Expr::Binop {
            op: "!=".into(),
            x: Box::new(p),
            y: Box::new(Expr::Nil { t: ptr(int64()) }),
            t: Type::Bool
        }
    );
    for id in [
        "type.ptr",
        "unop.ref",
        "unop.deref",
        "expr.nil",
        "expr.field",
    ] {
        assert!(used.contains(id), "{id}: {used:?}");
    }
}

#[test]
fn a_struct_literal_of_zeros_is_zero_and_its_address_a_reference_to_the_literal() {
    let text = "struct Point {\n    long x;\n    long y;\n};\n\nlong sum(long n) {\n    struct Point p = {n, n};\n    if (n <= 0) {\n        p = (struct Point){0};\n    }\n    struct Point *q = &(struct Point){0};\n    return p.x + q->y;\n}\n";
    let (m, used) = read_clean("zero", text);
    let point = named("Point");
    let Stmt::If { then, .. } = &m.funcs[0].body[1] else {
        panic!("an if: {:?}", m.funcs[0].body[1]);
    };
    let Stmt::Assign { rhs, .. } = &then[0] else {
        panic!("an assign: {then:?}");
    };
    assert_eq!(rhs[0], Expr::Zero { t: point.clone() });
    let Stmt::Let { rhs, .. } = &m.funcs[0].body[2] else {
        panic!("a let: {:?}", m.funcs[0].body[2]);
    };
    assert_eq!(
        rhs[0],
        Expr::Unop {
            op: "&".into(),
            x: Box::new(Expr::Struct {
                name: "Point".into(),
                fields: vec![codespec_model::FieldInit {
                    name: "x".into(),
                    value: Expr::Lit {
                        value: "0".into(),
                        t: int64()
                    }
                }],
                t: point.clone()
            }),
            t: Type::Ptr {
                elem: Box::new(point)
            }
        }
    );
    assert!(used.contains("expr.zero"), "{used:?}");
}

#[test]
fn a_function_pointer_is_a_func_a_function_name_a_func_ref_and_a_call_through_it_a_call_value() {
    let text = "#include <stdbool.h>\n\nstruct Rule {\n    bool (*check)(long);\n};\n\nstatic long twice(long n) {\n    return n * 2;\n}\n\nstatic long run(long (*f)(long), long n) {\n    return f(n);\n}\n\nstatic bool positive(long n) {\n    return n > 0;\n}\n\nlong apply(long n) {\n    return run(twice, n);\n}\n\nbool allowed(long n) {\n    struct Rule r = {positive};\n    return r.check(n);\n}\n";
    let (m, used) = read_clean("funcs", text);
    let func = |result: Type| Type::Func {
        params: vec![int64()],
        results: vec![result],
    };
    let run = m.funcs.iter().find(|f| f.name == "run").unwrap();
    assert_eq!(run.params[0].t, func(int64()));
    let Stmt::Return { results, .. } = &run.body[0] else {
        panic!("a return: {:?}", run.body[0]);
    };
    assert_eq!(
        results[0],
        Expr::CallValue {
            f: Box::new(Expr::Var {
                name: "f".into(),
                t: func(int64())
            }),
            args: vec![Expr::Var {
                name: "n".into(),
                t: int64()
            }],
            t: int64()
        }
    );
    assert!(
        exprs_of(&m, "apply").contains(&&Expr::FuncRef {
            name: "twice".into(),
            recv: None,
            t: func(int64())
        }),
        "{:#?}",
        exprs_of(&m, "apply")
    );
    let allowed = exprs_of(&m, "allowed");
    assert!(
        allowed
            .iter()
            .any(|e| matches!(e, Expr::CallValue { f, t: Type::Bool, .. }
            if matches!(f.as_ref(), Expr::Field { name, .. } if name == "check"))),
        "{allowed:#?}"
    );
    for id in ["type.func", "expr.func_ref", "expr.call_value"] {
        assert!(used.contains(id), "{id}: {used:?}");
    }
}

#[test]
fn the_c_corpus_holds_each_case_s_c_source_beside_its_calls() {
    let kit = root().join("conformance");
    let files = reader_corpus(&kit, "c").expect("a c corpus");
    let get = |rel: &str| {
        files
            .iter()
            .find(|(f, _)| f == rel)
            .map(|(_, b)| b.clone())
            .unwrap_or_else(|| panic!("no {rel} in the c corpus"))
    };
    let case = kit.join("cases/stmt-for");
    assert_eq!(
        get("stmt-for/case.c"),
        std::fs::read(case.join("c/case.c")).unwrap()
    );
    assert_eq!(
        get("stmt-for/calls.json"),
        std::fs::read(case.join("calls.json")).unwrap()
    );
    assert!(
        !files.iter().any(|(f, _)| f.starts_with("stmt-send/")),
        "a case without c/case.c has no corpus entry"
    );
}

#[test]
fn a_struct_pointing_to_itself_is_read_and_a_pointer_to_an_array_is_refused_as_one() {
    let text = "struct Node {\n    long value;\n    struct Node *next;\n};\n\nlong second(struct Node *n) {\n    return n->next->value;\n}\n\nlong head(long (*rows)[3]) {\n    return 0;\n}\n";
    let dir = source("self-pointer", &[("self-pointer.c", text)]);
    let m = read(&dir);
    assert_eq!(m.refusals.len(), 1, "{:#?}", m.refusals);
    let r = &m.refusals[0];
    assert_eq!(
        r.pos,
        format!("self-pointer.c:{}", line_of(text, "long head"))
    );
    assert!(
        r.reason.contains("a pointer to an array") && !r.reason.contains("function pointer"),
        "{r:#?}"
    );
    assert!(
        m.funcs.iter().any(|f| f.name == "second"),
        "{:#?}",
        m.opaque
    );
}
