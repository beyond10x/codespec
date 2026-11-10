package main

import (
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readAsMain reads dir as main does: load, encode, write the JSON. An error is main's exit 1.
func readAsMain(dir string) error {
	m, err := load(dir)
	if err != nil {
		return err
	}
	doc, err := encode(m)
	if err != nil {
		return err
	}
	enc := json.NewEncoder(io.Discard)
	enc.SetEscapeHTML(false)
	return enc.Encode(doc)
}

// The Go reader's depth is bounded where its libraries bound it, and a source past a bound is a
// refusal, never a crash: go/parser refuses nesting past 100 000 levels (maxNestLev, counting
// the levels a chain of binary operators builds), and encoding/json refuses to write a document
// nested past 10 000 levels (maxNestingDepth), which a chain of 99 000 operators the parser takes
// builds.
func TestASourceNestedPastTheLibrariesBoundsIsRefusedNotACrash(t *testing.T) {
	shapes := []struct{ name, body, want string }{
		{"parentheses", strings.Repeat("(", 100_001) + "a" + strings.Repeat(")", 100_001), "exceeded max nesting depth"},
		{"binary", strings.Repeat("a+", 99_000) + "a", "exceeded max depth"},
	}
	for _, s := range shapes {
		dir := t.TempDir()
		src := "package p\n\nfunc F(a int) int { return " + s.body + " }\n"
		if err := os.WriteFile(filepath.Join(dir, "p.go"), []byte(src), 0o644); err != nil {
			t.Fatal(err)
		}
		err := readAsMain(dir)
		if err == nil || !strings.Contains(err.Error(), s.want) {
			t.Errorf("%s: want a refusal holding %q, got %v", s.name, s.want, err)
		}
	}
}
