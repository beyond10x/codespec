package main

import (
	"go/types"
	"os"
	"path/filepath"
	"testing"
)

// Property: printable refuses every type the embedded driver cannot print. The driver's show
// handles signed integers, bool, string and slices; an unsigned integer makes it exit 2 at run
// time, after the build, instead of being refused when calls.go is generated.
func TestPrintableRefusesWhatTheDriverCannotPrint(t *testing.T) {
	for _, k := range []types.BasicKind{types.Uint, types.Uint8, types.Uint64} {
		if printable(types.Typ[k]) == nil {
			t.Errorf("%s accepted as printable; the driver's show exits 2 on it", types.Typ[k])
		}
	}
}

// Property: readCalls reads calls.json as read_calls in crates/conformance does (the crate doc
// says both runners read it that way). read_calls refuses an encoding other than the canonical
// one; encoding/json also matches keys case-insensitively and ignores what follows the object.
func TestReadCallsRefusesWhatReadCallsRefuses(t *testing.T) {
	dir := t.TempDir()
	canonical := "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {\n      \"func\": \"F\",\n      \"args\": []\n    }\n  ]\n}\n"
	for name, doc := range map[string]string{
		"compact.json":  `{"format":"codespec-calls/1","calls":[{"func":"F","args":[]}]}`,
		"upper.json":    "{\n  \"FORMAT\": \"codespec-calls/1\",\n  \"Calls\": [\n    {\n      \"Func\": \"F\",\n      \"Args\": []\n    }\n  ]\n}\n",
		"trailing.json": canonical + "{\"format\": \"other\"}\n",
		"null.json":     "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {\n      \"func\": \"F\",\n      \"args\": null\n    }\n  ]\n}\n",
	} {
		p := filepath.Join(dir, name)
		if err := os.WriteFile(p, []byte(doc), 0o644); err != nil {
			t.Fatal(err)
		}
		if _, err := readCalls(p); err == nil {
			t.Errorf("%s accepted; read_calls refuses it", name)
		}
	}
	p := filepath.Join(dir, "canonical.json")
	if err := os.WriteFile(p, []byte(canonical), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := readCalls(p); err != nil {
		t.Errorf("the canonical document is refused: %v", err)
	}
}
