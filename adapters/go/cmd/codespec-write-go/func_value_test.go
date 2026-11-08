package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Each function-value case written as Go vets and reads back to the same cir/1: a function type,
// a module function and a method used as values, a function literal, a call of a function value.
func TestFunctionValueCasesAreFixedPoints(t *testing.T) {
	for _, c := range []struct{ dir, want string }{
		{"type-func", "func run(f func(int) int, n int) int"},
		{"expr-func-value", "g := c.Add"},
		{"expr-lambda", "mul := func(x int) int {"},
		{"expr-call-value", "return r.Check(n)"},
	} {
		want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", c.dir, "case.cir.json"))
		if err != nil {
			t.Fatal(err)
		}
		out, src := writeDoc(t, want)
		if !strings.Contains(src, c.want) {
			t.Fatalf("%s: %q not written:\n%s", c.dir, c.want, src)
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
			t.Fatalf("%s: reading the written package is not a fixed point: first difference at %s", c.dir, d)
		}
	}
}
