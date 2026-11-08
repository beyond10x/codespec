package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The decl-var and expr-global cases written as Go and read back give the same cir/1, and vet:
// module vars are written as package-level `var` declarations and globals by name.
func TestModuleVarCasesFixedPoint(t *testing.T) {
	for _, c := range []struct{ name, want string }{
		{"decl-var", "var greeting string = \"hello, \"\n\nvar hits int\n"},
		{"expr-global", "total = total + n"},
	} {
		want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", c.name, "case.cir.json"))
		if err != nil {
			t.Fatal(err)
		}
		out, src := writeDoc(t, want)
		if !strings.Contains(src, c.want) {
			t.Fatalf("%s: missing %q in:\n%s", c.name, c.want, src)
		}
		vet(t, out)
		got := goTool(t, "run", "../codespec-read-go", out)
		var a, b any
		if err := json.Unmarshal(want, &a); err != nil {
			t.Fatal(err)
		}
		delete(a.(map[string]any), "facts")
		if err := json.Unmarshal(got, &b); err != nil {
			t.Fatal(err)
		}
		if d := firstDiff("$", strip(a, true), strip(b, true)); d != "" {
			t.Fatalf("%s: reading the written package is not a fixed point: first difference at %s", c.name, d)
		}
	}
}

// A var whose initializer the reader kept as foreign Go is written with that source as its
// initializer.
func TestAForeignInitializerIsWrittenVerbatim(t *testing.T) {
	doc := `{
  "format": "cir/1", "name": "m",
  "reader": {"language": "go", "name": "test", "version": "0"},
  "types": [],
  "vars": [
    {"name": "names", "t": {"k": "slice", "elem": {"k": "string"}},
     "foreign": {"lang": "go", "src": "[]string{\"a\", \"b\"}", "reason": "initializer outside cir/1"},
     "pos": "m.go:3"}
  ],
  "funcs": [
    {"name": "Count", "exported": true, "params": [], "results": [{"k": "int", "width": "platform", "signed": true}], "pos": "m.go:5",
     "body": [{"k": "return", "results": [{"k": "intrinsic", "name": "len",
       "args": [{"k": "global", "name": "names", "t": {"k": "slice", "elem": {"k": "string"}}}],
       "t": {"k": "int", "width": "platform", "signed": true}}], "pos": "m.go:6"}]}
  ],
  "layout": ["var:names", "func:Count"]
}`
	out, src := writeDoc(t, []byte(doc))
	for _, want := range []string{`var names []string = []string{"a", "b"}`, "return len(names)"} {
		if !strings.Contains(src, want) {
			t.Fatalf("missing %q in:\n%s", want, src)
		}
	}
	vet(t, out)
}
