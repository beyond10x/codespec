//! `codespec_edit::source`: what the refusals read from a file's bytes.

use codespec_edit::source::{
    go_struct_has_tag, idents, impl_trait, receiver_mode, region, struct_attributes, tokens,
};

#[test]
fn a_static_s_region_holds_its_initializer() {
    let b = b"static PICK: fn() -> u32 = load;\n\nfn load() -> u32 {\n    4\n}\n";
    let t = tokens(b);
    let r = region(b, &t, 1, false).unwrap();
    assert_eq!(&b[r.0..r.1], b"static PICK: fn() -> u32 = load;");
}

#[test]
fn a_macro_rules_region_ends_at_its_closing_brace() {
    let b = b"/// Twice.\nmacro_rules! twice {\n    () => {\n        load() + load()\n    };\n}\n\nfn load() {}\n";
    let t = tokens(b);
    let r = region(b, &t, 1, false).unwrap();
    assert!(idents(b, &t, r).contains(&"load"));
    assert!(!idents(b, &t, r).contains(&"fn"));
}

#[test]
fn a_statement_s_region_ends_at_the_block_around_it() {
    let b = b"fn total() -> u32 {\n    twice!()\n}\n\nfn load() {}\n";
    let t = tokens(b);
    let r = region(b, &t, 2, false).unwrap();
    assert_eq!(&b[r.0..r.1], b"twice!()");
}

#[test]
fn a_go_statement_s_region_ends_at_its_line() {
    let b = b"func f() {\n\tgo stock.Total()\n\tLoad()\n}\n";
    let t = tokens(b);
    let r = region(b, &t, 2, true).unwrap();
    assert_eq!(&b[r.0..r.1], b"go stock.Total()");
}

#[test]
fn comments_and_strings_hold_no_identifier() {
    let b = b"static S: &str = \"load\"; // load\n/* load */ static C: char = 'l';\n";
    let t = tokens(b);
    let all = idents(b, &t, (0, b.len()));
    assert!(!all.contains(&"load"), "{all:?}");
}

#[test]
fn the_trait_of_the_impl_block_around_a_method() {
    let s = "impl<T: Clone> std::fmt::Display for W<T> {\n    fn fmt(&self) {}\n}\nimpl W { fn new() {} }\nimpl From<u8> for W { fn from(x: u8) -> W { W } }\n";
    let (b, t) = (s.as_bytes(), tokens(s.as_bytes()));
    assert_eq!(
        impl_trait(b, &t, s.find("fmt(").unwrap()).as_deref(),
        Some("Display")
    );
    assert_eq!(impl_trait(b, &t, s.find("new").unwrap()), None);
    assert_eq!(
        impl_trait(b, &t, s.find("from(").unwrap()).as_deref(),
        Some("From")
    );
}

#[test]
fn receiver_modes_as_types() {
    for (src, mode) in [
        ("fn f(&self) {}", "&Self"),
        ("fn f(&'a mut self, x: u8) {}", "&mut Self"),
        ("fn f(mut self) {}", "Self"),
        ("fn f(self: Box<Self>) {}", "Box<Self>"),
        ("fn f(self: &Self) {}", "&Self"),
        ("fn f<T>(self, t: T) {}", "Self"),
    ] {
        let b = src.as_bytes();
        assert_eq!(
            receiver_mode(b, &tokens(b), 4).as_deref(),
            Some(mode),
            "{src}"
        );
    }
    let b = b"fn f(x: u8) {}";
    assert_eq!(receiver_mode(b, &tokens(b), 4), None);
}

#[test]
fn a_rust_struct_s_attributes_and_a_go_struct_s_tag() {
    let b = b"/// doc\n#[derive(Debug, Clone)]\n#[serde(default)]\npub(crate) struct P {\n    n: u32,\n}\n";
    let a = struct_attributes(b, &tokens(b), 1, "P").unwrap();
    assert_eq!(
        a,
        vec![
            (
                "derive".to_string(),
                vec!["Debug".to_string(), "Clone".to_string()]
            ),
            ("serde".to_string(), vec![])
        ]
    );
    let g = b"type line struct {\n\tqty int `json:\"q\"`\n}\ntype bare struct{ n int }\n";
    let t = tokens(g);
    assert_eq!(go_struct_has_tag(g, &t, 1, "line"), Some(true));
    assert_eq!(go_struct_has_tag(g, &t, 1, "bare"), Some(false));
}
