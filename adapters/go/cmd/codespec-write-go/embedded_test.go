package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The decl-embedded case written as Go declares the embedded field without a name, every
// promoted access is the explicit path the reader wrote, and reading the written package back
// gives the same cir/1.
func TestEmbeddedCaseFixedPoint(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", "decl-embedded", "case.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out, src := writeDoc(t, want)
	for _, s := range []string{"\tCounter\n", "t.Counter.Bump(t.Step)", "return t.Counter.N"} {
		if !strings.Contains(src, s) {
			t.Fatalf("no %q in the written package:\n%s", s, src)
		}
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

// A field marked embedded is written as an embedded field: the type alone, a pointer to it
// starred, an extern type qualified by its package.
func TestAnEmbeddedFieldIsWrittenWithoutAName(t *testing.T) {
	doc := `{
  "format": "cir/1", "name": "m",
  "reader": {"language": "go", "name": "test", "version": "0"},
  "types": [
    {"k": "struct", "name": "Inner", "exported": true, "fields": [{"name": "N", "t": {"k": "int", "width": "platform", "signed": true}}], "pos": "m.go:1"},
    {"k": "struct", "name": "Outer", "exported": true, "fields": [
      {"name": "Inner", "t": {"k": "ptr", "elem": {"k": "named", "name": "Inner"}}, "embedded": true},
      {"name": "Mutex", "t": {"k": "extern", "path": "sync", "name": "Mutex"}, "embedded": true},
      {"name": "Name", "t": {"k": "string"}}
    ], "pos": "m.go:2"}
  ],
  "funcs": [],
  "layout": ["type:Inner", "type:Outer"]
}`
	out, src := writeDoc(t, []byte(doc))
	for _, s := range []string{"\t*Inner\n", "\tsync.Mutex\n", "\tName "} {
		if !strings.Contains(src, s) {
			t.Fatalf("no %q in the written package:\n%s", s, src)
		}
	}
	vet(t, out)
}
