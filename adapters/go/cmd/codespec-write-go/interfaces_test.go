package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The decl-interface, expr-iface-call and expr-to-iface cases written as Go and read back give the
// same cir/1, and vet: an interface is declared with its methods, a call through one is a method
// call on the value, and a conversion to one is the value itself, which Go converts implicitly.
func TestInterfaceCasesFixedPoint(t *testing.T) {
	for _, c := range []struct{ name, want string }{
		{"decl-interface", "type Shape interface {\n\tArea() int\n}"},
		{"expr-iface-call", "w.WriteString(s)"},
		{"expr-to-iface", "l = Tag{text: s}"},
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
