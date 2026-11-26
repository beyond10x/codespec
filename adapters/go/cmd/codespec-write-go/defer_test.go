package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The stmt-defer case written as Go holds its defers as Go defers, vets, and reads back to the
// same cir/1.
func TestDeferCaseFixedPoint(t *testing.T) {
	want, err := os.ReadFile(filepath.Join(repo, "conformance", "cases", "stmt-defer", "case.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out, src := writeDoc(t, want)
	for _, line := range []string{"defer show(\"first\", n)", "defer show(\"second\", n)", "defer func() {"} {
		if !strings.Contains(src, line) {
			t.Fatalf("%q not written:\n%s", line, src)
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
