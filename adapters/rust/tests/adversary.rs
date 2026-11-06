//! Adversary cases for `codespec-write-rust`: small Go packages read by the Go reader, passed
//! through `codespec pass`, written as Rust, then built and run next to the Go original.
//!
//! A case asserts one of two contracts the writer states: the written crate builds under
//! `-D warnings` (Taskfile `examples-rust`), or the written `Run` prints what the Go `Run` prints,
//! unless the writer lists `Run` in `refusals.json`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn base() -> PathBuf {
    root().join("target/adversary-rust-writer")
}

fn run(cmd: &mut Command, what: &str) -> std::process::Output {
    cmd.output()
        .unwrap_or_else(|e| panic!("{what}: cannot start: {e}"))
}

fn ok(cmd: &mut Command, what: &str) -> Vec<u8> {
    let o = run(cmd, what);
    assert!(
        o.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    o.stdout
}

/// The Go reader, built once into the scratch directory.
fn reader() -> &'static Path {
    static R: OnceLock<PathBuf> = OnceLock::new();
    R.get_or_init(|| {
        let bin = base().join("bin/codespec-read-go");
        ok(
            Command::new("go")
                .args(["build", "-o"])
                .arg(&bin)
                .arg("./cmd/codespec-read-go")
                .current_dir(root().join("adapters/go")),
            "go build codespec-read-go",
        );
        bin
    })
}

/// The `codespec` CLI, for `codespec pass`.
fn codespec() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        ok(
            Command::new(cargo)
                .args(["build", "--quiet", "--locked", "-p", "b10x-codespec-cli"])
                .current_dir(root()),
            "cargo build codespec",
        );
        root().join("target/debug/codespec")
    })
}

struct Probe {
    dir: PathBuf,
    refusals: String,
}

/// Read `src` (package `probe`) with the Go reader, run the passes, and write it as Rust.
fn probe(name: &str, src: &str) -> Probe {
    probe_edited(name, src, &|_| {})
}

/// As [`probe`], with `edit` applied to the reader's document before the passes run.
fn probe_edited(name: &str, src: &str, edit: &dyn Fn(&mut serde_json::Value)) -> Probe {
    let dir = base().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("go/probe")).unwrap();
    std::fs::create_dir_all(dir.join("go/main")).unwrap();
    std::fs::write(dir.join("go/go.mod"), "module probe\n\ngo 1.23\n").unwrap();
    std::fs::write(dir.join("go/probe/probe.go"), src).unwrap();
    std::fs::write(
        dir.join("go/main/main.go"),
        "package main\n\nimport \"probe/probe\"\n\nfunc main() { probe.Run() }\n",
    )
    .unwrap();

    let read = ok(
        Command::new(reader()).arg(dir.join("go/probe")),
        "codespec-read-go",
    );
    let mut doc: serde_json::Value = serde_json::from_slice(&read).unwrap();
    edit(&mut doc);
    std::fs::write(dir.join("read.cir.json"), doc.to_string()).unwrap();
    ok(
        Command::new(codespec())
            .arg("pass")
            .arg(dir.join("read.cir.json"))
            .arg("--out")
            .arg(dir.join("pass.cir.json")),
        "codespec pass",
    );
    let doc = std::fs::read(dir.join("pass.cir.json")).unwrap();
    let mut child = Command::new(WRITER)
        .arg(dir.join("rust"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(&doc).unwrap();
    }
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "the writer refused the whole module (exit {:?}):\n{}",
        o.status.code(),
        String::from_utf8_lossy(&o.stderr)
    );
    let refusals = std::fs::read_to_string(dir.join("rust/refusals.json")).unwrap();
    Probe { dir, refusals }
}

impl Probe {
    fn lib(&self) -> String {
        std::fs::read_to_string(self.dir.join("rust/src/lib.rs")).unwrap()
    }

    fn refused(&self, key: &str) -> bool {
        self.refusals.contains(&format!("\"name\": \"{key}\""))
    }

    fn go_stdout(&self) -> String {
        let out = ok(
            Command::new("go")
                .args(["run", "./main"])
                .current_dir(self.dir.join("go")),
            "go run",
        );
        String::from_utf8(out).unwrap()
    }

    /// Build the written crate as a module of a binary that calls `run()`, and run it.
    fn rust_stdout(&self) -> Result<String, String> {
        let lib = self.dir.join("rust/src/lib.rs");
        let main = self.dir.join("driver.rs");
        std::fs::write(
            &main,
            format!(
                "#[path = {:?}]\nmod probe;\nfn main() {{ probe::run(); }}\n",
                lib.display().to_string()
            ),
        )
        .unwrap();
        let bin = self.dir.join("driver");
        let o = run(
            Command::new("rustc")
                .args(["--edition", "2024", "-A", "warnings", "-o"])
                .arg(&bin)
                .arg(&main)
                .current_dir(&self.dir),
            "rustc driver",
        );
        if !o.status.success() {
            return Err(format!(
                "the written crate does not compile:\n{}\n{}",
                String::from_utf8_lossy(&o.stderr),
                self.lib()
            ));
        }
        let o = run(&mut Command::new(&bin), "driver");
        if !o.status.success() {
            return Err(format!(
                "the written Run panicked or failed (stdout {:?}):\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ));
        }
        Ok(String::from_utf8(o.stdout).unwrap())
    }

    /// Build the written crate as a library with `-D warnings`, as `task examples-rust` does.
    fn lint(&self) -> Result<(), String> {
        let o = run(
            Command::new("rustc")
                .args([
                    "--edition",
                    "2024",
                    "--crate-type",
                    "lib",
                    "--crate-name",
                    "probe",
                    "-D",
                    "warnings",
                    "--out-dir",
                ])
                .arg(self.dir.join("lint"))
                .arg(self.dir.join("rust/src/lib.rs"))
                .current_dir(&self.dir),
            "rustc -D warnings",
        );
        if o.status.success() {
            Ok(())
        } else {
            Err(format!(
                "{}\n{}",
                String::from_utf8_lossy(&o.stderr),
                self.lib()
            ))
        }
    }
}

/// The written `Run` prints what the Go `Run` prints, or the writer lists `Run` as refused.
fn same_as_go(name: &str, src: &str) {
    same_as_go_edited(name, src, &|_| {});
}

/// As [`same_as_go`], the reader's document changed by `edit` into an equivalent one.
fn same_as_go_edited(name: &str, src: &str, edit: &dyn Fn(&mut serde_json::Value)) {
    let p = probe_edited(name, src, edit);
    if p.refused("Run") {
        return;
    }
    let want = p.go_stdout();
    match p.rust_stdout() {
        Ok(got) => assert_eq!(
            got,
            want,
            "Rust and Go disagree; refusals: {}\n{}",
            p.refusals,
            p.lib()
        ),
        Err(e) => panic!("Go prints {want:?}, but {e}"),
    }
}

/// The writer refuses nothing, the written crate builds under `-D warnings`, and the written `Run`
/// prints what the Go `Run` prints.
///
/// The Go reader types a `nil` only where it is returned (a `nil` operand of `==` is Go's untyped
/// nil, which has no cir/1 form), so a probe spells a nil pointer `none()`, a function returning
/// `nil`, and each call of it becomes the `nil` expression of its type before the passes run: the
/// same program, as a reader that types every `nil` (the Java reader's `null`) writes it.
fn written_same_as_go(name: &str, src: &str) {
    fn nil_for_none(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                if o.get("k").and_then(|k| k.as_str()) == Some("call")
                    && o.get("name").and_then(|n| n.as_str()) == Some("none")
                {
                    let t = o["t"].clone();
                    *v = serde_json::json!({ "k": "nil", "t": t });
                    return;
                }
                o.values_mut().for_each(nil_for_none);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(nil_for_none),
            _ => {}
        }
    }
    let p = probe_edited(name, src, &nil_for_none);
    assert_eq!(p.refusals.trim(), "[]", "refused:\n{}", p.lib());
    if let Err(e) = p.lint() {
        panic!("the written crate warns under -D warnings:\n{e}");
    }
    let want = p.go_stdout();
    match p.rust_stdout() {
        Ok(got) => assert_eq!(got, want, "Rust and Go disagree:\n{}", p.lib()),
        Err(e) => panic!("Go prints {want:?}, but {e}"),
    }
}

/// The written crate builds under `-D warnings`.
fn builds_clean(name: &str, src: &str) {
    let p = probe(name, src);
    if let Err(e) = p.lint() {
        panic!("the written crate warns under -D warnings:\n{e}");
    }
}

// ----- (b) Go value semantics against Rust clones --------------------------------------------

#[test]
fn a_copied_struct_shares_its_slice_with_the_original() {
    same_as_go(
        "struct-copy-slice",
        r#"package probe

import "fmt"

type Bag struct{ Xs []int }

func Run() {
	a := Bag{}
	a.Xs = append(a.Xs, 1)
	b := a
	b.Xs[0] = 9
	fmt.Println(a.Xs[0])
}
"#,
    );
}

#[test]
fn a_range_over_a_slice_sees_writes_made_during_the_loop() {
    same_as_go(
        "range-live-slice",
        r#"package probe

import "fmt"

func Run() {
	var xs []int
	xs = append(xs, 1)
	xs = append(xs, 2)
	for i, v := range xs {
		if i == 0 {
			xs[1] = 100
		}
		fmt.Println(v)
	}
}
"#,
    );
}

#[test]
fn a_go_clone_method_is_not_what_a_value_copy_calls() {
    same_as_go(
        "clone-method",
        r#"package probe

import "fmt"

type Bag struct{ N int }

func (b Bag) Clone() Bag { return Bag{N: b.N + 100} }

func Run() {
	a := Bag{N: 1}
	b := a
	fmt.Println(b.N)
}
"#,
    );
}

// ----- (b) Display and formatting ------------------------------------------------------------

#[test]
fn the_string_method_of_an_int_enum_is_what_println_prints() {
    same_as_go(
        "enum-string-method",
        r#"package probe

import "fmt"

type Color int

const (
	Red Color = iota
	Green
)

func (c Color) String() string { return "a color" }

func Run() {
	fmt.Println(Green)
}
"#,
    );
}

#[test]
fn a_pointer_receiver_string_method_is_not_used_for_a_value() {
    same_as_go(
        "ptr-string-method",
        r#"package probe

import "fmt"

type Temp struct{ C int }

func (t *Temp) String() string { return "hot" }

func Run() {
	t := Temp{C: 5}
	fmt.Println(t)
}
"#,
    );
}

#[test]
fn a_zero_padded_string_verb_pads_with_zeros() {
    same_as_go(
        "zero-pad-string",
        r#"package probe

import "fmt"

func Run() {
	fmt.Println(fmt.Sprintf("[%05s]", "ab"))
}
"#,
    );
}

#[test]
fn a_width_on_a_displayed_newtype_pads_it() {
    same_as_go(
        "width-newtype",
        r#"package probe

import "fmt"

type ID string

func Run() {
	id := ID("ab")
	fmt.Println(fmt.Sprintf("[%6s]", id))
}
"#,
    );
}

// ----- (b) Rc<RefCell<T>>: borrows that Go does not have -------------------------------------

#[test]
fn a_map_store_through_a_shared_pointer_reads_the_same_pointer() {
    same_as_go(
        "shared-map-insert",
        r#"package probe

import "fmt"

type Bag struct {
	N int
	M map[string]int
}

type Holder struct{ B *Bag }

func Run() {
	p := &Bag{M: map[string]int{}}
	p.N = 3
	p.M["a"] = p.N
	fmt.Println(p.M["a"])
}
"#,
    );
}

#[test]
fn an_append_through_a_shared_pointer_reads_the_same_pointer() {
    same_as_go(
        "shared-append",
        r#"package probe

import "fmt"

type Bag struct {
	N  int
	Xs []int
}

type Holder struct{ B *Bag }

func Run() {
	p := &Bag{N: 3}
	p.Xs = append(p.Xs, p.N)
	fmt.Println(len(p.Xs), p.Xs[0])
}
"#,
    );
}

#[test]
fn a_mutating_method_on_a_shared_value_reads_it_through_its_argument() {
    same_as_go(
        "shared-reentrant",
        r#"package probe

import "fmt"

type Bag struct{ N int }

type Holder struct{ B *Bag }

func (b *Bag) AddFrom(h Holder) { b.N += h.B.N }

func Run() {
	p := &Bag{N: 2}
	h := Holder{B: p}
	p.AddFrom(h)
	fmt.Println(p.N)
}
"#,
    );
}

// ----- (b) names the writer introduces -------------------------------------------------------

#[test]
fn the_switch_tag_binding_does_not_shadow_a_local_named_tag() {
    // The Go reader keeps `switch` as foreign, so the reader's `if x+1 == 2 { … }` is rewritten
    // into the cir/1 `switch` it is equivalent to: `switch x + 1 { case 2: … }`.
    same_as_go_edited(
        "switch-tag-shadow",
        r#"package probe

import "fmt"

func Run() {
	tag := 5
	x := 1
	if x+1 == 2 {
		fmt.Println(tag)
	}
}
"#,
        &|doc| {
            let run = doc["funcs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|f| f["name"] == "Run")
                .unwrap();
            let s = run["body"][2].clone();
            assert_eq!(s["k"], "if", "{s}");
            run["body"][2] = serde_json::json!({
                "k": "switch",
                "tag": s["cond"]["x"],
                "cases": [{ "values": [s["cond"]["y"]], "body": s["then"] }],
                "pos": s["pos"],
            });
        },
    );
}

#[test]
fn a_shared_store_binding_does_not_shadow_a_local_named_value() {
    same_as_go(
        "store-value-shadow",
        r#"package probe

import "fmt"

type Bag struct {
	N  int
	Xs []int
}

type Holder struct{ B *Bag }

func Run() {
	p := &Bag{N: 2}
	p.Xs = append(p.Xs, 0)
	p.Xs = append(p.Xs, 0)
	p.Xs = append(p.Xs, 0)
	value := 1
	p.Xs[value] = p.N
	fmt.Println(p.Xs[1], p.Xs[2])
}
"#,
    );
}

#[test]
fn run_and_an_unexported_run_get_different_rust_names() {
    same_as_go(
        "run-and-run",
        r#"package probe

import "fmt"

func run() int { return 7 }

func Run() { fmt.Println(run()) }
"#,
    );
}

#[test]
fn an_enum_with_a_value_method_compiles() {
    same_as_go(
        "enum-value-method",
        r#"package probe

import "fmt"

type Level int

const (
	Low Level = iota
	High
)

func (l Level) Value() int { return 42 }

func Run() { fmt.Println(High.Value()) }
"#,
    );
}

#[test]
fn a_value_receiver_method_compares_and_returns_its_receiver() {
    same_as_go(
        "value-receiver-self",
        r#"package probe

import "fmt"

type Color int

const (
	Red Color = iota
	Green
)

func (c Color) Flip() Color {
	if c == Red {
		return Green
	}
	return c
}

func Run() { fmt.Println(Red.Flip() == Green) }
"#,
    );
}

// ----- (d) dropped without a refusal ---------------------------------------------------------

#[test]
fn an_init_function_runs_before_run() {
    same_as_go(
        "init-func",
        r#"package probe

import "fmt"

func init() { fmt.Println("boot") }

func Run() { fmt.Println("run") }
"#,
    );
}

#[test]
fn a_negative_constant_does_not_refuse_the_whole_module() {
    same_as_go(
        "negative-const",
        r#"package probe

import "fmt"

const Offset = -3

func Run() { fmt.Println(Offset) }
"#,
    );
}

// ----- (a) builds under -D warnings ----------------------------------------------------------

#[test]
fn a_var_assigned_on_every_path_builds_without_warnings() {
    builds_clean(
        "var-every-path",
        r#"package probe

import "fmt"

func Pick(c bool) string {
	var s string
	if c {
		s = "a"
	} else {
		s = "b"
	}
	return s
}

func Run() { fmt.Println(Pick(true)) }
"#,
    );
}

#[test]
fn a_shadowing_binding_of_a_mutated_name_builds_without_warnings() {
    builds_clean(
        "shadow-mut",
        r#"package probe

import "fmt"

func Run() {
	x := 1
	x++
	fmt.Println(x)
	if x > 0 {
		x := 3
		fmt.Println(x)
	}
}
"#,
    );
}

#[test]
fn a_helper_of_a_refused_function_builds_without_warnings() {
    builds_clean(
        "refused-caller-helper",
        r#"package probe

import "fmt"

func helper() int { return 1 }

func Audit() { defer fmt.Println(helper()) }

func Run() { fmt.Println(2) }
"#,
    );
}

// ----- nil pointers (story rust-writer-nil-pointer) --------------------------------------------

#[test]
fn a_pointer_local_and_a_map_lookup_of_a_pointer_compare_with_nil_as_go_does() {
    written_same_as_go(
        "nil-local-lookup",
        r#"package probe

import "fmt"

func none() *Node { return nil }

type Node struct {
	Val  int
	Next *Node
}

func localIsNil(set bool) bool {
	var p *Node
	if set {
		p = &Node{Val: 1}
	}
	return p == none()
}

func lookupIsNil(key string) bool {
	m := map[string]*Node{}
	m["a"] = &Node{Val: 2}
	return m[key] == none()
}

func lookupVal(key string) int {
	m := map[string]*Node{}
	m["a"] = &Node{Val: 2}
	n := m[key]
	if n != none() {
		return n.Val
	}
	return -1
}

func Run() {
	fmt.Println(localIsNil(false), localIsNil(true))
	fmt.Println(lookupIsNil("a"), lookupIsNil("b"))
	fmt.Println(lookupVal("a"), lookupVal("b"))
}
"#,
    );
}

#[test]
fn a_pointer_field_never_assigned_reads_as_nil() {
    written_same_as_go(
        "nil-field",
        r#"package probe

import "fmt"

func none() *Node { return nil }

type Node struct {
	Val  int
	Next *Node
}

type Box struct {
	Inner *Node
}

func Run() {
	b := Box{}
	fmt.Println(b.Inner == none())
	b.Inner = &Node{Val: 7}
	fmt.Println(b.Inner == none(), b.Inner.Val)
	q := b.Inner
	q.Val = 9
	fmt.Println(b.Inner.Val, q.Next == none())
}
"#,
    );
}

#[test]
fn a_chain_of_pointers_ends_at_nil() {
    written_same_as_go(
        "nil-chain",
        r#"package probe

import "fmt"

func none() *Node { return nil }

type Node struct {
	Val  int
	Next *Node
}

func chain(n int) int {
	var head *Node
	for i := 1; i <= n; i++ {
		head = &Node{Val: i, Next: head}
	}
	sum := 0
	for p := head; p != none(); p = p.Next {
		sum += p.Val
	}
	return sum
}

func Run() {
	fmt.Println(chain(0), chain(3))
}
"#,
    );
}

#[test]
fn a_pointer_to_a_local_writes_the_local() {
    written_same_as_go(
        "pointer-to-local",
        r#"package probe

import "fmt"

func bump(p *int) {
	*p = *p + 1
}

func Run() {
	n := 1
	p := &n
	bump(p)
	bump(&n)
	n++
	fmt.Println(n, *p)
}
"#,
    );
}

// ----- evaluation order -----------------------------------------------------------------------

/// `s.pick().label = s.stamp()`: Go calls `pick`, the target's receiver, before `stamp`, the
/// value. Binding the value first would print "value first"; the writer refuses `Run` instead.
#[test]
fn an_assignment_through_a_call_receiver_with_a_call_in_its_value_is_refused() {
    let src = r#"package probe

import "fmt"

type Shelf struct {
	label string
	picks int
	next  *Shelf
}

func (s *Shelf) pick() *Shelf {
	s.picks++
	return s.next
}

func (s *Shelf) stamp() string {
	if s.picks == 0 {
		return "value first"
	}
	return "receiver first"
}

func Run() {
	s := &Shelf{}
	s.next = &Shelf{}
	s.pick().label = s.stamp()
	fmt.Println(s.next.label)
}
"#;
    let p = probe("call-receiver-order", src);
    assert!(
        p.refused("Run") && p.refusals.contains("goes through a call"),
        "Run not refused for its evaluation order; refusals: {}\n{}",
        p.refusals,
        p.lib()
    );
    same_as_go("call-receiver-order-run", src);
}
