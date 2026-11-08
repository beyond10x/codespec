package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Each case whose subject is an array or float feature, written as Go and read back, gives the
// same cir/1, and vets: arrays and float types are written as Go types, a float literal so that
// Go types it as a float.
func TestArrayAndFloatCasesFixedPoint(t *testing.T) {
	for _, c := range []struct{ name, want string }{
		{"type-array-float", "var w [3]float64"},
		{"type-float", "var y float32"},
		{"expr-float", "x = 1.5e300"},
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
			delete(a.(map[string]any), "facts")
			if err := json.Unmarshal(got, &b); err != nil {
				t.Fatal(err)
			}
			if d := firstDiff("$", strip(a, true), strip(b, true)); d != "" {
				t.Fatalf("reading the written package is not a fixed point: first difference at %s\n%s", d, src)
			}
		})
	}
}

// A float literal whose spelling Go would read as an integer is written so that Go reads a float.
const floatSpelling = `{
  "format": "cir/1", "name": "m",
  "reader": {"language": "go", "name": "test", "version": "0"},
  "types": [],
  "funcs": [
    {"name": "F", "exported": true, "params": [], "results": [{"k": "float", "bits": "64"}], "pos": "m.go:1",
     "body": [
       {"k": "let", "names": ["x"], "rhs": [{"k": "float", "value": "2", "t": {"k": "float", "bits": "64"}}], "pos": "m.go:2"},
       {"k": "let", "names": ["y"], "rhs": [{"k": "float", "value": "1e21", "t": {"k": "float", "bits": "32"}}], "pos": "m.go:3"},
       {"k": "return", "results": [{"k": "var", "name": "x", "t": {"k": "float", "bits": "64"}}], "pos": "m.go:4"}
     ]}
  ],
  "layout": ["func:F"]
}`

func TestAFloatLiteralIsWrittenAsAGoFloat(t *testing.T) {
	_, src := writeDoc(t, []byte(floatSpelling))
	for _, want := range []string{"x := 2.0", "y := float32(1e21)"} {
		if !strings.Contains(src, want) {
			t.Fatalf("missing %q in:\n%s", want, src)
		}
	}
}
