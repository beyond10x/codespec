package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The expr-extern case written as Go and read back gives the same cir/1, and vets.
func TestExternCaseFixedPoint(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", "expr-extern", "case.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out, src := writeDoc(t, want)
	if !strings.Contains(src, "strings.Repeat(s, n) + strconv.Itoa(n)") {
		t.Fatalf("extern calls not written as package calls:\n%s", src)
	}
	vet(t, out)
	got := goTool(t, "run", "../codespec-read-go", out)
	var a, b any
	if err := json.Unmarshal(want, &a); err != nil {
		t.Fatal(err)
	}
	delete(a.(map[string]any), "facts") // case.cir.json is passed; the reader writes no facts
	if err := json.Unmarshal(got, &b); err != nil {
		t.Fatal(err)
	}
	if d := firstDiff("$", strip(a, true), strip(b, true)); d != "" {
		t.Fatalf("reading the written package is not a fixed point: first difference at %s", d)
	}
}

// A module function named like an extern's package, and two extern paths ending in one element,
// each get the package imported under a name nothing else holds.
const externNames = `{
  "format": "cir/1", "name": "m",
  "reader": {"language": "go", "name": "test", "version": "0"},
  "types": [],
  "funcs": [
    {"name": "strings", "exported": false, "params": [], "results": [{"k": "string"}], "pos": "m.go:1",
     "body": [{"k": "return", "results": [{"k": "lit", "value": "\"x\"", "t": {"k": "string"}}], "pos": "m.go:2"}]},
    {"name": "F", "exported": true, "params": [{"name": "s", "t": {"k": "string"}}], "results": [{"k": "string"}], "pos": "m.go:3",
     "body": [{"k": "return", "results": [{"k": "binop", "op": "+",
       "x": {"k": "extern", "path": "strings", "name": "ToUpper", "args": [{"k": "var", "name": "s", "t": {"k": "string"}}], "t": {"k": "string"}},
       "y": {"k": "binop", "op": "+",
         "x": {"k": "extern", "path": "example.com/a/text", "name": "A", "args": [], "t": {"k": "string"}},
         "y": {"k": "extern", "path": "example.com/b/text", "name": "B", "args": [], "t": {"k": "string"}},
         "t": {"k": "string"}},
       "t": {"k": "string"}}], "pos": "m.go:4"}]}
  ],
  "externs": [
    {"path": "example.com/a/text", "name": "A", "params": [], "results": [{"k": "string"}]},
    {"path": "example.com/b/text", "name": "B", "params": [], "results": [{"k": "string"}]},
    {"path": "strings", "name": "ToUpper", "params": [{"k": "string"}], "results": [{"k": "string"}]}
  ],
  "layout": ["func:strings", "func:F"]
}`

func TestExternPackagesAreImportedUnderNamesNothingElseHolds(t *testing.T) {
	_, src := writeDoc(t, []byte(externNames))
	for _, want := range []string{
		`strings_ "strings"`,
		`"example.com/a/text"`,
		`text_ "example.com/b/text"`,
		"strings_.ToUpper(s) + (text.A() + text_.B())",
	} {
		if !strings.Contains(src, want) {
			t.Fatalf("missing %q in:\n%s", want, src)
		}
	}
}
