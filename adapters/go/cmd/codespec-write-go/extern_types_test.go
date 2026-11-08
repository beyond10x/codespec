package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The type-extern and expr-extern-method cases written as Go and read back give the same cir/1,
// and vet: an extern type is written as the package's type, an extern method call as a call of
// the method on its receiver.
func TestExternTypeCasesFixedPoint(t *testing.T) {
	for _, c := range []struct{ name, want string }{
		{"type-extern", "text *strings.Builder"},
		{"expr-extern-method", "b.WriteString(s)"},
	} {
		t.Run(c.name, func(t *testing.T) {
			want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", c.name, "case.cir.json"))
			if err != nil {
				t.Fatal(err)
			}
			out, src := writeDoc(t, want)
			if !strings.Contains(src, c.want) {
				t.Fatalf("missing %q in:\n%s", c.want, src)
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
		})
	}
}
