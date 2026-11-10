package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

// A cir/1 document is untrusted input: every name and literal it holds lands in Go source text.
// Each case below puts something into one that is not a Go identifier or literal and asserts the
// writer refuses it, naming the value, and writes no file anywhere.

func docAt(t *testing.T, file string) map[string]any {
	t.Helper()
	b, err := os.ReadFile(filepath.Join(repo, file))
	if err != nil {
		t.Fatal(err)
	}
	var v map[string]any
	if err := json.Unmarshal(b, &v); err != nil {
		t.Fatal(err)
	}
	return v
}

// set replaces the value at path (object keys and array indices) and fails when there is none.
func set(t *testing.T, v any, path []string, to any) {
	t.Helper()
	for i, p := range path {
		last := i == len(path)-1
		switch c := v.(type) {
		case map[string]any:
			if _, ok := c[p]; !ok {
				t.Fatalf("no %q at %v", p, path[:i+1])
			}
			if last {
				c[p] = to
				return
			}
			v = c[p]
		case []any:
			n, err := strconv.Atoi(p)
			if err != nil || n >= len(c) {
				t.Fatalf("no index %q at %v", p, path[:i+1])
			}
			if last {
				c[n] = to
				return
			}
			v = c[n]
		default:
			t.Fatalf("nothing to index at %v", path[:i+1])
		}
	}
}

// refusedNaming writes doc into a directory below a fresh scratch directory and asserts the write
// fails naming want, and that no file is left in the scratch directory.
func refusedNaming(t *testing.T, doc map[string]any, want string) {
	t.Helper()
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	base := scratch(t)
	err = run(bytes.NewReader(b), filepath.Join(base, "out"))
	if err == nil {
		t.Fatalf("the writer accepted %q", want)
	}
	if !strings.Contains(err.Error(), strconv.Quote(want)) {
		t.Errorf("the refusal does not name %q: %v", want, err)
	}
	_ = filepath.Walk(base, func(p string, info os.FileInfo, _ error) error {
		if info != nil && !info.IsDir() {
			t.Errorf("a refused document left %s", p)
		}
		return nil
	})
}

func TestANameThatIsNotAGoIdentifierIsRefusedAndNothingIsWritten(t *testing.T) {
	for i, c := range []struct {
		path []string
		bad  string
	}{
		{[]string{"name"}, "orders\n\nimport _ \"os\"\n\nvar injected = 1\n\nvar _"},
		{[]string{"name"}, "../evil"},
		{[]string{"name"}, "a/b"},
		{[]string{"name"}, `a\b`},
		{[]string{"name"}, ".hidden"},
		{[]string{"name"}, ".."},
		{[]string{"name"}, "_"},
		{[]string{"types", "2", "fields", "1", "name"}, "Customer string; Injected"},
		{[]string{"funcs", "2", "name"}, "Place() {}\nfunc Injected"},
		{[]string{"funcs", "2", "params", "0", "name"}, "customer, injected"},
		{[]string{"funcs", "2", "body", "2", "names", "0"}, "id = 1; injected"},
		{[]string{"funcs", "2", "params", "0", "name"}, "r#customer"},
		{[]string{"types", "2", "fields", "1", "name"}, "Customer$"},
	} {
		// Numbered: a subtest name becomes a scratch path, and these values are paths.
		t.Run(strconv.Itoa(i), func(t *testing.T) {
			doc := docAt(t, "examples/orders/orders.cir.json")
			set(t, doc, c.path, c.bad)
			if c.path[0] == "funcs" && c.path[2] == "name" {
				set(t, doc, []string{"layout", "12"}, "func:Service."+c.bad)
			}
			refusedNaming(t, doc, c.bad)
		})
	}
}

func TestALiteralThatIsNotOneGoLiteralIsRefusedAndNothingIsWritten(t *testing.T) {
	const bad = "0 + func() int64 { return 1 }()"
	for _, k := range []string{"lit", "float"} {
		t.Run(k, func(t *testing.T) {
			doc := docAt(t, "examples/orders/orders.cir.json")
			set(t, doc, []string{"funcs", "2", "body", "0", "cond", "y", "k"}, k)
			set(t, doc, []string{"funcs", "2", "body", "0", "cond", "y", "value"}, bad)
			refusedNaming(t, doc, bad)
		})
	}
}

// sorted_keys prints the map's types as Go text and parses the helper it builds from it, so a
// type name there must be checked before it is printed, not only in the tree written at the end.
func TestATypeNameInASortedKeysHelperIsRefused(t *testing.T) {
	const bad = "int) []string { return nil }(nil); _ = func(m map[string]int"
	doc := docAt(t, "conformance/cases/intrinsic-sorted-keys/case.cir.json")
	set(t, doc, []string{"funcs", "0", "body", "5", "over", "args", "0", "t", "elem"}, map[string]any{"k": "named", "name": bad})
	refusedNaming(t, doc, bad)
}

// A document the strict reader would refuse still reaches this writer when it is run on its
// own; a missing field is a refusal, never a panic.
func TestADocumentMissingARequiredPartIsRefusedNotAPanic(t *testing.T) {
	doc := docAt(t, "examples/orders/orders.cir.json")
	set(t, doc, []string{"types", "2", "fields", "1", "t"}, map[string]any{"k": "slice"})
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	base := scratch(t)
	if err := run(bytes.NewReader(b), filepath.Join(base, "out")); err == nil {
		t.Fatal("the writer accepted a slice type without an element type")
	}
	if _, err := os.Stat(filepath.Join(base, "out")); !os.IsNotExist(err) {
		t.Errorf("a refused document left %s", filepath.Join(base, "out"))
	}
}

// codespec-read-go writes an unnamed parameter as _; it is written back as _.
func TestAParameterNamedUnderscoreIsWritten(t *testing.T) {
	doc := docAt(t, "conformance/cases/op-add/case.cir.json")
	fn := doc["funcs"].([]any)[0].(map[string]any)
	params := fn["params"].([]any)
	int := params[0].(map[string]any)["t"]
	fn["params"] = append(params, map[string]any{"name": "_", "t": int})
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	_, src := writeDoc(t, b)
	if !strings.Contains(src, "func Add(a int, b int, _ int) int") && !strings.Contains(src, "func Add(a, b, _ int) int") {
		t.Errorf("the parameter _ is not written:\n%s", src)
	}
}
