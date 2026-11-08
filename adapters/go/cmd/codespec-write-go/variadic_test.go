package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Each variadic case written as Go vets and reads back to the same cir/1: a variadic last
// parameter, calls of it with no, one and several arguments, a variadic extern, and a slice
// passed spread to a module function, an extern and append.
func TestVariadicCasesAreFixedPoints(t *testing.T) {
	for _, c := range []struct {
		dir  string
		want []string
	}{
		{"decl-variadic", []string{"func sum(xs ...int) int", "sum() + sum(a) + sum(a, b, 3)", `path.Join(a, b, "c")`}},
		{"expr-spread", []string{"func sum(xs ...int) int", "append(xs, xs...)", "sum(ys...)", "path.Join(parts...)"}},
	} {
		want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", c.dir, "case.cir.json"))
		if err != nil {
			t.Fatal(err)
		}
		out, src := writeDoc(t, want)
		for _, w := range c.want {
			if !strings.Contains(src, w) {
				t.Fatalf("%s: %q not written:\n%s", c.dir, w, src)
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
			t.Fatalf("%s: reading the written package is not a fixed point: first difference at %s", c.dir, d)
		}
	}
}
