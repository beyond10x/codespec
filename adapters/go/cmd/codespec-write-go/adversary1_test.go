package main

import (
	"bytes"
	"encoding/json"
	"path/filepath"
	"strings"
	"testing"
	"unicode"
)

// Adversary pass 1 on story:writers-validate-names: names valid in a source language come back
// as Go, and a refusal names its position without carrying the document's control characters.

func runDoc(t *testing.T, doc map[string]any) error {
	t.Helper()
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	return run(bytes.NewReader(b), filepath.Join(scratch(t), "out"))
}

// An assign with no left-hand side is admitted by strict reading (codespec validate exits 0 on
// it). The writer indexes lhs[0]; write recovers, but the message names no position, which
// main.go promises for every refusal.
func TestARecoveredPanicNamesThePositionOfWhatItCannotWrite(t *testing.T) {
	doc := docAt(t, "conformance/cases/assign-div/case.cir.json")
	set(t, doc, []string{"funcs", "0", "body", "0", "lhs"}, []any{})
	err := runDoc(t, doc)
	if err == nil {
		t.Fatal("an assignment to nothing was written")
	}
	if !strings.Contains(err.Error(), "case.go:5") {
		t.Errorf("the refusal names no position: %v", err)
	}
}

// A pos is document text that strict reading does not check; the refusal prints it raw.
func TestARefusalMessageHoldsNoControlCharacterFromTheDocument(t *testing.T) {
	doc := docAt(t, "conformance/cases/assign-div/case.cir.json")
	set(t, doc, []string{"funcs", "0", "pos"}, "case.go\x1b]0;owned\x07\x1b[2J:4")
	set(t, doc, []string{"funcs", "0", "body", "0", "pos"}, "case.go\x1b]0;owned\x07\x1b[2J:5")
	set(t, doc, []string{"funcs", "0", "body", "0", "rhs", "0", "k"}, "lit")
	fn := doc["funcs"].([]any)[0].(map[string]any)
	rhs := fn["body"].([]any)[0].(map[string]any)["rhs"].([]any)[0].(map[string]any)
	rhs["value"] = "1 1"
	delete(rhs, "name")
	err := runDoc(t, doc)
	if err == nil {
		t.Fatal(`the literal "1 1" was written`)
	}
	for _, r := range err.Error() {
		if unicode.IsControl(r) {
			t.Fatalf("the refusal holds a raw control character: %q", err.Error())
		}
	}
}

// Java `static long size(long type, long map)` reads to parameters named type and map, legal
// Java and among its most common names; codespec-read-rust renames Go keywords itself, the Java
// reader does not, and the writer refuses the whole module rather than renaming them.
func TestAParameterNamedLikeAGoKeywordIsWritten(t *testing.T) {
	t.Skip("story:writers-distinct-names")
	doc := docAt(t, "conformance/cases/op-add/case.cir.json")
	set(t, doc, []string{"funcs", "0", "params", "0", "name"}, "type")
	set(t, doc, []string{"funcs", "0", "params", "1", "name"}, "map")
	set(t, doc, []string{"funcs", "0", "body", "0", "results", "0", "x", "name"}, "type")
	set(t, doc, []string{"funcs", "0", "body", "0", "results", "0", "y", "name"}, "map")
	if err := runDoc(t, doc); err != nil {
		t.Fatalf("a module with legal Java parameter names is refused: %v", err)
	}
}
