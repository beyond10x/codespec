package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"
)

const repo = "../../../.."

func TestMain(m *testing.M) {
	if os.Getenv("CODESPEC_WRITE_GO_RUN_MAIN") == "1" {
		main()
		os.Exit(0)
	}
	os.Exit(m.Run())
}

// scratch is a fresh directory under the repository's target/, never the system temp directory.
func scratch(t *testing.T) string {
	t.Helper()
	dir, err := filepath.Abs(filepath.Join(repo, "target", "go-writer-test", t.Name()))
	if err != nil {
		t.Fatal(err)
	}
	if err := os.RemoveAll(dir); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	return dir
}

func writeDoc(t *testing.T, doc []byte) (string, string) {
	t.Helper()
	out := scratch(t)
	if err := run(bytes.NewReader(doc), out); err != nil {
		t.Fatalf("write: %v", err)
	}
	var m Module
	if err := json.Unmarshal(doc, &m); err != nil {
		t.Fatal(err)
	}
	src, err := os.ReadFile(filepath.Join(out, m.Name+".go"))
	if err != nil {
		t.Fatal(err)
	}
	return out, string(src)
}

func goTool(t *testing.T, args ...string) []byte {
	t.Helper()
	cmd := exec.Command("go", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go %s: %v\n%s", strings.Join(args, " "), err, out)
	}
	return out
}

func vet(t *testing.T, dir string) {
	t.Helper()
	files, err := filepath.Glob(filepath.Join(dir, "*.go"))
	if err != nil || len(files) == 0 {
		t.Fatalf("no Go files in %s: %v", dir, err)
	}
	goTool(t, append([]string{"vet"}, files...)...)
}

// strip drops the fields a fixed point does not compare: the top-level reader and every pos.
func strip(v any, top bool) any {
	switch v := v.(type) {
	case map[string]any:
		out := map[string]any{}
		for k, x := range v {
			if k == "pos" || (top && k == "reader") {
				continue
			}
			out[k] = strip(x, false)
		}
		return out
	case []any:
		out := make([]any, len(v))
		for i, x := range v {
			out[i] = strip(x, false)
		}
		return out
	}
	return v
}

func firstDiff(path string, a, b any) string {
	switch av := a.(type) {
	case map[string]any:
		bv, ok := b.(map[string]any)
		if !ok {
			return path
		}
		keys := map[string]bool{}
		for k := range av {
			keys[k] = true
		}
		for k := range bv {
			keys[k] = true
		}
		var ks []string
		for k := range keys {
			ks = append(ks, k)
		}
		sort.Strings(ks)
		for _, k := range ks {
			if d := firstDiff(path+"."+k, av[k], bv[k]); d != "" {
				return d
			}
		}
		return ""
	case []any:
		bv, ok := b.([]any)
		if !ok || len(av) != len(bv) {
			return fmt.Sprintf("%s (length)", path)
		}
		for i := range av {
			if d := firstDiff(fmt.Sprintf("%s[%d]", path, i), av[i], bv[i]); d != "" {
				return d
			}
		}
		return ""
	}
	if !reflect.DeepEqual(a, b) {
		return fmt.Sprintf("%s: %v != %v", path, a, b)
	}
	return ""
}

func TestOrdersFixedPoint(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "examples", "orders", "orders.read.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out, _ := writeDoc(t, want)
	got := goTool(t, "run", "../codespec-read-go", out)
	var a, b any
	if err := json.Unmarshal(want, &a); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(got, &b); err != nil {
		t.Fatal(err)
	}
	if d := firstDiff("$", strip(a, true), strip(b, true)); d != "" {
		t.Fatalf("reading the written package is not a fixed point: first difference at %s", d)
	}
}

func TestSeriesFixedPoint(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "examples", "series", "series.read.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out, _ := writeDoc(t, want)
	got := goTool(t, "run", "../codespec-read-go", out)
	var a, b any
	if err := json.Unmarshal(want, &a); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(got, &b); err != nil {
		t.Fatal(err)
	}
	if d := firstDiff("$", strip(a, true), strip(b, true)); d != "" {
		t.Fatalf("reading the written package is not a fixed point: first difference at %s", d)
	}
}

// A cir/1 break leaves the loop; inside a switch Go's unlabeled break would leave the switch.
const breakInSwitch = `{
  "format": "cir/1", "name": "m",
  "reader": {"language": "rust", "name": "test", "version": "0"},
  "types": [],
  "funcs": [{"name": "F", "exported": true, "params": [], "results": [], "pos": "m.rs:1",
    "body": [{"k": "for", "body": [
      {"k": "switch", "cases": [{"values": [], "body": [{"k": "break", "pos": "m.rs:4"}]}], "pos": "m.rs:3"}
    ], "pos": "m.rs:2"}]}],
  "layout": ["func:F"]
}`

func TestBreakInsideASwitchLeavesTheLoopByLabel(t *testing.T) {
	_, src := writeDoc(t, []byte(breakInSwitch))
	if !strings.Contains(src, "loop:\n\tfor {") || !strings.Contains(src, "break loop") {
		t.Fatalf("break inside a switch is not a labeled break of the loop:\n%s", src)
	}
}

const foreignRust = `{
  "format": "cir/1", "name": "m",
  "reader": {"language": "rust", "name": "test", "version": "0"},
  "types": [],
  "funcs": [{"name": "F", "exported": true, "params": [], "results": [], "pos": "m.rs:3",
    "body": [{"k": "foreign", "lang": "rust", "src": "unsafe { x() }", "reason": "unsafe", "pos": "m.rs:7"}]}],
  "layout": ["func:F"]
}`

func TestRefusesForeignRust(t *testing.T) {
	out := scratch(t)
	cmd := exec.Command(os.Args[0], out)
	cmd.Env = append(os.Environ(), "CODESPEC_WRITE_GO_RUN_MAIN=1")
	cmd.Stdin = strings.NewReader(foreignRust)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	err := cmd.Run()
	ee, ok := err.(*exec.ExitError)
	if !ok || ee.ExitCode() != 1 {
		t.Fatalf("want exit 1, got %v; stderr: %s", err, stderr.String())
	}
	if !strings.Contains(stderr.String(), "m.rs:7") {
		t.Fatalf("refusal does not name the statement's pos m.rs:7: %q", stderr.String())
	}
	if files, _ := filepath.Glob(filepath.Join(out, "*.go")); len(files) != 0 {
		t.Fatalf("a refused module wrote %v", files)
	}
}

const lookupElse = `{
  "format": "cir/1", "name": "lk",
  "reader": {"language": "go", "name": "test", "version": "0"},
  "types": [],
  "funcs": [{"name": "Get", "exported": true, "pos": "lk.go:1",
    "params": [{"name": "m", "t": {"k": "map", "key": {"k": "string"}, "elem": {"k": "int", "width": "platform", "signed": true}}},
               {"name": "k", "t": {"k": "string"}}],
    "results": [{"k": "int", "width": "platform", "signed": true}, {"k": "bool"}],
    "body": [
      {"k": "lookup_else", "name": "v", "pos": "lk.go:2",
       "map": {"k": "var", "name": "m", "t": {"k": "map", "key": {"k": "string"}, "elem": {"k": "int", "width": "platform", "signed": true}}},
       "index": {"k": "var", "name": "k", "t": {"k": "string"}},
       "then": [{"k": "return", "pos": "lk.go:3", "results": [
         {"k": "lit", "value": "0", "t": {"k": "int", "width": "platform", "signed": true}},
         {"k": "lit", "value": "false", "t": {"k": "bool"}}]}]},
      {"k": "lookup_else", "name": "w", "pos": "lk.go:4",
       "map": {"k": "var", "name": "m", "t": {"k": "map", "key": {"k": "string"}, "elem": {"k": "int", "width": "platform", "signed": true}}},
       "index": {"k": "lit", "value": "\"other\"", "t": {"k": "string"}},
       "then": [{"k": "return", "pos": "lk.go:5", "results": [
         {"k": "var", "name": "v", "t": {"k": "int", "width": "platform", "signed": true}},
         {"k": "lit", "value": "false", "t": {"k": "bool"}}]}]},
      {"k": "return", "pos": "lk.go:6", "results": [
        {"k": "binop", "op": "+", "t": {"k": "int", "width": "platform", "signed": true},
         "x": {"k": "var", "name": "v", "t": {"k": "int", "width": "platform", "signed": true}},
         "y": {"k": "var", "name": "w", "t": {"k": "int", "width": "platform", "signed": true}}},
        {"k": "lit", "value": "true", "t": {"k": "bool"}}]}
    ]}],
  "layout": ["func:Get"]
}`

func TestLookupElseCompiles(t *testing.T) {
	out, src := writeDoc(t, []byte(lookupElse))
	vet(t, out)
	if !strings.Contains(src, "v, ok := m[k]") {
		t.Fatalf("lookup_else not written as a comma-ok lookup:\n%s", src)
	}
}

// lenAsInt64 is what codespec-read-rust reads from `self.values.len() as i64`: a `len` typed int64.
const lenAsInt64 = `{
  "format": "cir/1", "name": "ln",
  "reader": {"language": "rust", "name": "test", "version": "0"},
  "types": [],
  "funcs": [{"name": "Count", "exported": true, "pos": "ln.rs:1",
    "params": [{"name": "xs", "t": {"k": "slice", "elem": {"k": "int", "width": "64", "signed": true}}}],
    "results": [{"k": "int", "width": "64", "signed": true}],
    "body": [{"k": "return", "pos": "ln.rs:2", "results": [
      {"k": "intrinsic", "name": "len", "t": {"k": "int", "width": "64", "signed": true},
       "args": [{"k": "var", "name": "xs", "t": {"k": "slice", "elem": {"k": "int", "width": "64", "signed": true}}}]}]}]}],
  "layout": ["func:Count"]
}`

func TestALenOfAnotherIntTypeIsConverted(t *testing.T) {
	out, src := writeDoc(t, []byte(lenAsInt64))
	vet(t, out)
	if !strings.Contains(src, "return int64(len(xs))\n") {
		t.Fatalf("a len typed int64 is not written as int64(len(xs)):\n%s", src)
	}
}

func intT() string { return `{"k": "int", "width": "platform", "signed": true}` }

func v(name string) string { return fmt.Sprintf(`{"k": "var", "name": %q, "t": %s}`, name, intT()) }

func bin(op, x, y string) string {
	return fmt.Sprintf(`{"k": "binop", "op": %q, "t": %s, "x": %s, "y": %s}`, op, intT(), x, y)
}

func un(op, x string) string {
	return fmt.Sprintf(`{"k": "unop", "op": %q, "t": %s, "x": %s}`, op, intT(), x)
}

func TestPrecedenceAndPlatformInt(t *testing.T) {
	cases := []struct{ expr, want string }{
		{bin("*", bin("+", v("a"), v("b")), v("c")), "(a + b) * c"},
		{bin("+", v("a"), bin("*", v("b"), v("c"))), "a + b*c"},
		{bin("-", v("a"), bin("-", v("b"), v("c"))), "a - (b - c)"},
		{bin("-", bin("-", v("a"), v("b")), v("c")), "a - b - c"},
		{un("-", bin("+", v("a"), v("b"))), "-(a + b)"},
		{bin("<<", v("a"), bin("+", v("b"), v("c"))), "a << (b + c)"},
	}
	var funcs, layout []string
	for i, c := range cases {
		name := fmt.Sprintf("F%d", i)
		funcs = append(funcs, fmt.Sprintf(`{"name": %q, "exported": true, "pos": "p.go:%d",
          "params": [{"name": "a", "t": %s}, {"name": "b", "t": %s}, {"name": "c", "t": %s}],
          "results": [%s],
          "body": [{"k": "return", "pos": "p.go:%d", "results": [%s]}]}`, name, i, intT(), intT(), intT(), intT(), i, c.expr))
		layout = append(layout, fmt.Sprintf("%q", "func:"+name))
	}
	doc := fmt.Sprintf(`{"format": "cir/1", "name": "p", "reader": {"language": "go", "name": "test", "version": "0"},
      "types": [{"k": "struct", "name": "W", "exported": true, "pos": "p.go:0", "fields": [
        {"name": "N", "t": %s}, {"name": "U", "t": {"k": "int", "width": "64", "signed": false}}]}],
      "funcs": [%s], "layout": ["type:W", %s]}`, intT(), strings.Join(funcs, ","), strings.Join(layout, ","))
	out, src := writeDoc(t, []byte(doc))
	vet(t, out)
	for _, c := range cases {
		if !strings.Contains(src, "return "+c.want+"\n") {
			t.Errorf("want `return %s` in:\n%s", c.want, src)
		}
	}
	if !strings.Contains(src, "func F0(a int, b int, c int) int") || !strings.Contains(src, "U uint64") {
		t.Errorf("platform-width int not written as int, or unsigned 64 not as uint64:\n%s", src)
	}
}

func TestForeignGoIsVerbatim(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "examples", "orders", "orders.read.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	_, src := writeDoc(t, want)
	verbatim := "\tgo func() {\n\t\tfor _, o := range s.orders {\n\t\t\tfmt.Println(o.ID)\n\t\t}\n\t}()\n"
	if !strings.Contains(src, verbatim) {
		t.Fatalf("foreign Go statement not written verbatim:\n%s", src)
	}
}
