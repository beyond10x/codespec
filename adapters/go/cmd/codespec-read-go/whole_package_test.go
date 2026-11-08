package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// A package of a real repository is read even where single declarations are outside cir/1: each
// such declaration is refused with its reason, a test file is not read, and every other function
// is in the document.
func TestAPackageIsReadWithEachDeclarationOutsideCir1RefusedOnItsOwn(t *testing.T) {
	dir := t.TempDir()
	files := map[string]string{
		"p.go": `package p

import "unsafe"

type Guarded struct {
	mu unsafe.Pointer
	n  int
}

func Sum[T comparable](x T) T { return x }

func Fast(x int) int

func Double(x int) int { return x + x }
`,
		"p_test.go": `package p

func Helper() int { return 1 }
`,
		"other_linux.go": `//go:build ignore

package p

func Ignored() int { return 2 }
`,
	}
	for name, src := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(src), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	m, err := load(dir)
	if err != nil {
		t.Fatal(err)
	}
	var funcs []string
	for _, f := range m.Funcs {
		funcs = append(funcs, f.Name)
	}
	if len(funcs) != 1 || funcs[0] != "Double" {
		t.Fatalf("functions %v, want only Double", funcs)
	}
	want := map[string]string{
		"Guarded": "field mu of type unsafe.Pointer has no cir/1 type",
		"Sum":     "constraint of type parameter T: comparable has no cir/1 type",
		"Fast":    "function without a Go body",
	}
	for what, reason := range want {
		found := false
		for _, r := range m.Refusals {
			if strings.Contains(r.What, what) && strings.Contains(r.Reason, reason) {
				found = true
			}
		}
		if !found {
			t.Errorf("no refusal of %s with %q; refusals %v", what, reason, m.Refusals)
		}
	}
	for _, r := range m.Refusals {
		if strings.Contains(r.What, "Helper") || strings.Contains(r.What, "Ignored") {
			t.Errorf("a test file or an excluded file was read: %v", r)
		}
	}
}

// A local of type pointer-to-a-refused-struct (runtime's `var gp *g`) makes its statement foreign
// rather than failing the encoding, and a file that builds only with cgo is not read.
func TestAPointerToARefusedTypeAndACgoOnlyFileDoNotFailThePackage(t *testing.T) {
	dir := t.TempDir()
	files := map[string]string{
		"p.go": `package p

import "unsafe"

type g struct{ mu unsafe.Pointer }

func Count() int {
	var gp *g
	_ = gp
	return 1
}
`,
		"c.go": `//go:build cgo

package p

func FromC() int { return undefinedInPureGo }
`,
	}
	for name, src := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(src), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	m, err := load(dir)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := encode(m); err != nil {
		t.Fatalf("encode: %v", err)
	}
	found := false
	for _, r := range m.Refusals {
		if strings.Contains(r.Reason, "type outside cir/1: g") {
			found = true
		}
	}
	if !found {
		t.Fatalf("want the var statement refused naming g; refusals %v", m.Refusals)
	}
}
