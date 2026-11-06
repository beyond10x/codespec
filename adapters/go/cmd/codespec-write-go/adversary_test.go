package main

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// Adversary cases for story:go-writer. Each one is a cir/1 document, either built by hand or read
// by codespec-read-go from a Go source that compiles, and each asserts that the written Go compiles
// or behaves as the document says.

// readSource writes src as <name>.go into a fresh directory and returns what codespec-read-go
// prints for it.
func readSource(t *testing.T, name, src string) []byte {
	t.Helper()
	dir := filepath.Join(scratch(t), "src")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, name+".go"), []byte(src), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("go", "run", "../codespec-read-go", dir)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	out, err := cmd.Output()
	if err != nil {
		t.Fatalf("the source compiles but codespec-read-go failed: %v\n%s", err, stderr.String())
	}
	return out
}

// writeOut writes doc into a fresh directory under scratch and returns that directory and the source.
func writeOut(t *testing.T, doc []byte) (string, string) {
	t.Helper()
	out := filepath.Join(scratch(t), "out")
	if err := run(bytes.NewReader(doc), out); err != nil {
		t.Fatalf("write refused a valid document: %v", err)
	}
	files, _ := filepath.Glob(filepath.Join(out, "*.go"))
	if len(files) != 1 {
		t.Fatalf("want one Go file in %s, got %v", out, files)
	}
	src, err := os.ReadFile(files[0])
	if err != nil {
		t.Fatal(err)
	}
	return files[0], string(src)
}

// compiles type-checks the written file with go vet and reports its output on failure.
func compiles(t *testing.T, file, src string) {
	t.Helper()
	out, err := exec.Command("go", "vet", file).CombinedOutput()
	if err != nil {
		t.Errorf("written Go does not compile: %v\n%s\nwritten:\n%s", err, out, src)
	}
}

// runMain builds the written main package into scratch and returns its stdout.
func runMain(t *testing.T, file, src string) string {
	t.Helper()
	bin := filepath.Join(filepath.Dir(file), "prog")
	if out, err := exec.Command("go", "build", "-o", bin, file).CombinedOutput(); err != nil {
		t.Fatalf("written Go does not build: %v\n%s\nwritten:\n%s", err, out, src)
	}
	out, err := exec.Command(bin).Output()
	if err != nil {
		t.Fatalf("written program failed: %v", err)
	}
	return string(out)
}

func roundTrip(t *testing.T, name, src string) {
	t.Helper()
	doc := readSource(t, name, src)
	file, written := writeOut(t, doc)
	compiles(t, file, written)
}

// An untyped constant is read with its default type, and the writer declares it with that type, so
// every use in a context of another type stops compiling.
func TestAdversaryUntypedConstUsedAsInt64Compiles(t *testing.T) {
	roundTrip(t, "c", `package c

const Max = 10

func Over(x int64) bool { return x > Max }
`)
}

func TestAdversaryUntypedConstUsedAsNamedStringCompiles(t *testing.T) {
	roundTrip(t, "s", `package s

type Code string

const Prefix = "a"

func Is(c Code) bool { return c == Prefix }
`)
}

// A foreign Go statement holding a spread call is written without its `...`, which changes which
// arguments the call receives: it still compiles, and prints something else.
func TestAdversaryForeignSpreadCallKeepsEllipsis(t *testing.T) {
	doc := readSource(t, "main", `package main

import "fmt"

func main() {
	xs := []any{1, 2}
	fmt.Printf("%v|%v\n", xs...)
}
`)
	file, written := writeOut(t, doc)
	if got := runMain(t, file, written); got != "1|2\n" {
		t.Errorf("foreign fmt.Printf(..., xs...) prints %q, the source prints %q\nwritten:\n%s", got, "1|2\n", written)
	}
}

// A foreign local type alias is written as a defined type, which loses the aliased type's methods.
func TestAdversaryForeignTypeAliasStaysAlias(t *testing.T) {
	roundTrip(t, "al", `package al

type T struct{}

func (T) Name() string { return "t" }

func F() string {
	type A = T
	xs := []A{{}}
	return xs[0].Name()
}
`)
}

// A local variable named like a package, anywhere in the function, stops the writer importing that
// package for a foreign statement that uses the package in another scope.
func TestAdversaryForeignImportDespiteLocalOfSameName(t *testing.T) {
	roundTrip(t, "v", `package v

import "errors"

func Validate(items []string) (int, error) {
	if len(items) == 0 {
		return 0, errors.New("no items")
	}
	errors := []string{}
	for _, it := range items {
		if it == "" {
			errors = append(errors, it)
		}
	}
	return len(errors), nil
}
`)
}

// Named results and a bare return: cir/1 keeps neither the names nor a way to say so.
func TestAdversaryNamedResultsCompile(t *testing.T) {
	roundTrip(t, "nr", `package nr

func F() (n int) {
	n = 1
	return
}
`)
}

// An embedded field is read as a field named after its type, so a promoted field stops resolving.
// The reader now refuses such a struct, so this asserts the refusal instead of a compiling round
// trip: the struct is listed as a refusal and is not read as a struct with a field named A.
func TestAdversaryEmbeddedFieldPromotionCompiles(t *testing.T) {
	doc := readSource(t, "em", `package em

type A struct{ X int }

type B struct{ A }

func F(b B) int { return b.X }
`)
	var m struct {
		Types    []struct{ Name string }
		Refusals []struct{ What, Reason string }
	}
	if err := json.Unmarshal(doc, &m); err != nil {
		t.Fatal(err)
	}
	for _, td := range m.Types {
		if td.Name == "B" {
			t.Errorf("struct B with an embedded field was read as a struct")
		}
	}
	refused := false
	for _, r := range m.Refusals {
		refused = refused || (r.What == "B" && strings.Contains(r.Reason, "embedded"))
	}
	if !refused {
		t.Errorf("no refusal names the embedded field of B: %+v", m.Refusals)
	}
}

// append(a, b...) is read as the intrinsic append(a, b); the spread is gone.
func TestAdversaryAppendSpreadCompiles(t *testing.T) {
	roundTrip(t, "ap", `package ap

func Cat(a, b []int) []int { return append(a, b...) }
`)
}

// A channel receive is read as unop "<-", which the writer refuses.
func TestAdversaryChannelReceiveIsWritten(t *testing.T) {
	roundTrip(t, "rc", `package rc

func Recv(ch chan int) int { return <-ch }
`)
}

const lookupShadowsConst = `{
  "format": "cir/1", "name": "main",
  "reader": {"language": "go", "name": "test", "version": "0"},
  "types": [],
  "consts": [{"name": "ok", "t": {"k": "bool"}, "value": {"k": "lit", "value": "true", "t": {"k": "bool"}}, "pos": "m.go:1"}],
  "funcs": [
    {"name": "Get", "exported": true, "pos": "m.go:2",
     "params": [{"name": "m", "t": {"k": "map", "key": {"k": "string"}, "elem": {"k": "int", "width": "platform", "signed": true}}},
                {"name": "k", "t": {"k": "string"}}],
     "results": [{"k": "int", "width": "platform", "signed": true}, {"k": "bool"}],
     "body": [
       {"k": "lookup_else", "name": "v", "pos": "m.go:3",
        "map": {"k": "var", "name": "m", "t": {"k": "map", "key": {"k": "string"}, "elem": {"k": "int", "width": "platform", "signed": true}}},
        "index": {"k": "var", "name": "k", "t": {"k": "string"}},
        "then": [{"k": "return", "pos": "m.go:4", "results": [
          {"k": "lit", "value": "0", "t": {"k": "int", "width": "platform", "signed": true}},
          {"k": "const", "name": "ok", "t": {"k": "bool"}}]}]},
       {"k": "return", "pos": "m.go:5", "results": [
         {"k": "var", "name": "v", "t": {"k": "int", "width": "platform", "signed": true}},
         {"k": "const", "name": "ok", "t": {"k": "bool"}}]}]},
    {"name": "main", "exported": false, "pos": "m.go:6", "params": [], "results": [],
     "body": [{"k": "expr", "pos": "m.go:7", "x": {"k": "intrinsic", "name": "println", "t": {"k": "unit"}, "args": [
       {"k": "call", "name": "Get", "t": {"k": "tuple", "elems": [{"k": "int", "width": "platform", "signed": true}, {"k": "bool"}]}, "args": [
         {"k": "make", "t": {"k": "map", "key": {"k": "string"}, "elem": {"k": "int", "width": "platform", "signed": true}}},
         {"k": "lit", "value": "\"x\"", "t": {"k": "string"}}]}]}}]}
  ],
  "layout": ["const:ok", "func:Get", "func:main"]
}`

// The comma-ok name lookup_else invents must not shadow a module-level declaration the `then`
// branch reads: the document returns the constant ok (true), the written Go the lookup's false.
func TestAdversaryLookupElseOkDoesNotShadowModuleConst(t *testing.T) {
	file, written := writeOut(t, []byte(lookupShadowsConst))
	if got := runMain(t, file, written); got != "0 true\n" {
		t.Errorf("Get on a missing key prints %q, the document says %q\nwritten:\n%s", got, "0 true\n", written)
	}
}

func letModule(rhs, result string) string {
	return `{
  "format": "cir/1", "name": "lt",
  "reader": {"language": "rust", "name": "test", "version": "0"},
  "types": [],
  "funcs": [{"name": "F", "exported": true, "pos": "lt.rs:1", "params": [], "results": [` + result + `],
    "body": [
      {"k": "let", "names": ["x"], "rhs": [` + rhs + `], "pos": "lt.rs:2"},
      {"k": "return", "pos": "lt.rs:3", "results": [{"k": "var", "name": "x", "t": ` + result + `}]}]}],
  "layout": ["func:F"]
}`
}

// `let x: i64 = -1`: a negative literal of a sized type, bound by :=, is not converted.
func TestAdversaryLetNegativeInt64Compiles(t *testing.T) {
	i64 := `{"k": "int", "width": "64", "signed": true}`
	file, written := writeOut(t, []byte(letModule(
		`{"k": "unop", "op": "-", "t": `+i64+`, "x": {"k": "lit", "value": "1", "t": `+i64+`}}`, i64)))
	compiles(t, file, written)
}

// `let x: u8 = 0` written from a zero expression.
func TestAdversaryLetZeroUint8Compiles(t *testing.T) {
	u8 := `{"k": "int", "width": "8", "signed": false}`
	file, written := writeOut(t, []byte(letModule(`{"k": "zero", "t": `+u8+`}`, u8)))
	compiles(t, file, written)
}

// A zero pointer bound by := is `x := nil`, which Go rejects.
func TestAdversaryLetZeroPointerCompiles(t *testing.T) {
	p := `{"k": "ptr", "elem": {"k": "int", "width": "platform", "signed": true}}`
	file, written := writeOut(t, []byte(letModule(`{"k": "zero", "t": `+p+`}`, p)))
	if strings.Contains(written, "x := nil") {
		t.Errorf("zero pointer bound as `x := nil`")
	}
	compiles(t, file, written)
}
