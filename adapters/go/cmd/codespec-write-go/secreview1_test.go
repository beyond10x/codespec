package main

// Security review of codespec-write-go. A doc comment of the document is text in the written
// file, and the module name is its file name: neither may become a build directive or a file name
// the go command treats as something other than one ordinary source file of the package. Each
// case asserts on the written text or the refusal only; nothing written here is built or run.

import (
	"bytes"
	"encoding/json"
	"go/build"
	"go/build/constraint"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

// buildLines is every line of src the go command reads as a build constraint.
func buildLines(src string) []string {
	var out []string
	for _, line := range strings.Split(src, "\n") {
		if constraint.IsGoBuild(line) || constraint.IsPlusBuild(line) {
			out = append(out, line)
		}
	}
	return out
}

// A doc comment line `+build ignore` is written as `// +build ignore`; go/format, which the
// writer runs over the file, takes every such comment anywhere in the file for a build constraint
// and writes `//go:build ignore` at its top.
func TestSecreview1ADocCommentIsNeverABuildConstraint(t *testing.T) {
	for i, c := range []struct {
		path []string
		doc  string
	}{
		{[]string{"funcs", "0", "doc"}, "Add adds.\n+build ignore"},
		{[]string{"funcs", "0", "doc"}, "+build secreview_never_set"},
	} {
		t.Run(strconv.Itoa(i), func(t *testing.T) {
			doc := docAt(t, "conformance/cases/op-add/case.cir.json")
			set(t, doc, c.path, c.doc)
			b, err := json.Marshal(doc)
			if err != nil {
				t.Fatal(err)
			}
			_, src := writeDoc(t, b)
			if lines := buildLines(src); len(lines) > 0 {
				t.Errorf("the doc %q became the build constraint %q:\n%s", c.doc, lines, src)
			}
		})
	}
}

// The module name is the file name <name>.go. A name the go command reads as a constraint
// (_windows, _arm64), a test file (_test) or a file to skip (a leading _) passes
// token.IsIdentifier; either it is refused or the file written is one `go build` compiles here.
func TestSecreview1AModuleNameIsNeverAFileNameTheGoCommandSkips(t *testing.T) {
	for _, name := range []string{"op_add_windows", "op_add_arm64", "op_add_test", "_op_add"} {
		t.Run(name, func(t *testing.T) {
			doc := docAt(t, "conformance/cases/op-add/case.cir.json")
			set(t, doc, []string{"name"}, name)
			b, err := json.Marshal(doc)
			if err != nil {
				t.Fatal(err)
			}
			out := filepath.Join(scratch(t), "out")
			if err := run(bytes.NewReader(b), out); err != nil {
				return // refused: nothing written
			}
			file := name + ".go"
			ok, err := build.Default.MatchFile(out, file)
			if err != nil {
				t.Fatal(err)
			}
			if !ok || strings.HasSuffix(file, "_test.go") {
				t.Errorf("module %q was written as %s, which `go build` does not compile (matches %s/%s: %v)", name, file, build.Default.GOOS, build.Default.GOARCH, ok)
			}
		})
	}
}
