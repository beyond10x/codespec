package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// A package with an import path records every import path of its files, a blank and a dot
// import included, once each across files, sorted; one without an import path records none.
func TestImportsAreEveryPathOfEveryFileOnceSorted(t *testing.T) {
	dir := t.TempDir()
	files := map[string]string{
		"a.go": "package p\n\nimport (\n\t_ \"embed\"\n\t. \"strings\"\n\t\"os\"\n)\n\nfunc A() string { _ = os.Args; return ToUpper(\"a\") }\n",
		"b.go": "package p\n\nimport \"os\"\nimport \"errors\"\n\nfunc B() error { _ = os.Args; return errors.New(\"b\") }\n",
	}
	for name, src := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(src), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	t.Setenv(importPathEnv, "example.com/p")
	m, err := load(dir)
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"embed", "errors", "os", "strings"}
	if !reflect.DeepEqual(m.Imports, want) {
		t.Fatalf("imports %v, want %v", m.Imports, want)
	}
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	b, _ := json.Marshal(doc)
	if !strings.Contains(string(b), `"imports":["embed","errors","os","strings"]`) {
		t.Fatalf("no imports in %s", b)
	}

	t.Setenv(importPathEnv, "")
	bare := loadSource(t, "package q\n\nimport \"os\"\n\nfunc Q() { _ = os.Args }\n")
	if len(bare.Imports) != 0 {
		t.Fatalf("a package without an import path records %v", bare.Imports)
	}
}
