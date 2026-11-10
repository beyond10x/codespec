package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The kit case stmt-raise holds `raise` in Check: the writer has no mapping for it, refuses the
// function by name at the raise's position and writes nothing.
func TestARaiseIsRefusedNamingTheFunctionAndItsPosition(t *testing.T) {
	doc, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "conformance", "cases", "stmt-raise", "case.cir.json"))
	if err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(scratch(t), "stmt_raise")
	err = run(strings.NewReader(string(doc)), out)
	if err == nil {
		t.Fatal("the module was written")
	}
	for _, want := range []string{"case.go:12", "Check", "raise"} {
		if !strings.Contains(err.Error(), want) {
			t.Fatalf("the refusal %q does not name %s", err, want)
		}
	}
	if _, statErr := os.Stat(filepath.Join(out, "stmt_raise.go")); statErr == nil {
		t.Fatal("a source file was written beside the refusal")
	}
}
