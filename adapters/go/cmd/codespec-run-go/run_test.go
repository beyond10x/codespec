package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

// The embedded driver is the conformance kit's, byte for byte: one driver prints every case.
func TestDriverIsTheConformanceDriver(t *testing.T) {
	kit, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "conformance", "driver", "go", "main.go"))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(kit, driver) {
		t.Fatal("driver.go.txt differs from conformance/driver/go/main.go; copy it over")
	}
}

// The argument encoding refuses what check_argument in crates/conformance refuses.
func TestArgumentsOutsideTheEncodingAreRefused(t *testing.T) {
	for _, a := range []string{"01", "-0", "+1", "9223372036854775808", "1.0", ""} {
		if checkArgument(a, "int") == nil {
			t.Errorf("int %q accepted", a)
		}
	}
	if err := checkArgument("-9223372036854775808", "int"); err != nil {
		t.Error(err)
	}
	for _, a := range []string{`"a\"b"`, `"a\\b"`, "\"é\"", "abc"} {
		if checkArgument(a, "string") == nil {
			t.Errorf("string %q accepted", a)
		}
	}
	if checkArgument("ch", "chan") == nil {
		t.Error("chan \"ch\" accepted")
	}
}

// A calls.json with an unknown field or no calls is refused.
func TestCallsWithAnUnknownFieldOrNoCallsAreRefused(t *testing.T) {
	dir := t.TempDir()
	for name, doc := range map[string]string{
		"extra.json": `{"format":"codespec-calls/1","calls":[{"func":"F","args":[],"extra":1}]}`,
		"empty.json": `{"format":"codespec-calls/1","calls":[]}`,
		"other.json": `{"format":"codespec-calls/2","calls":[{"func":"F","args":[]}]}`,
	} {
		p := filepath.Join(dir, name)
		if err := os.WriteFile(p, []byte(doc), 0o644); err != nil {
			t.Fatal(err)
		}
		if _, err := readCalls(p); err == nil {
			t.Errorf("%s accepted", name)
		}
	}
}
