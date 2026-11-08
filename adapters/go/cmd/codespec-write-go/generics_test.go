package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The decl-generic case written as Go and read back gives the same cir/1, and vets: a generic type
// and function are declared with their type parameters, a method of a generic type names them in
// its receiver, and a call and a type use write their type arguments out.
func TestGenericCaseFixedPoint(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", "decl-generic", "case.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out, src := writeDoc(t, want)
	for _, w := range []string{
		"type Pair[T any] struct {",
		"func (p Pair[T]) First() T {",
		"func sum[T Sizer](x T, y T) int {",
		"p := Pair[Item]{",
		"sum[Item](p.First(), p.b)",
	} {
		if !strings.Contains(src, w) {
			t.Fatalf("missing %q in:\n%s", w, src)
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
