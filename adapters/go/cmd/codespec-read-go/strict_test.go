package main

import (
	"strings"
	"testing"
)

// undeclared lists every reference in the encoded document v to a type or interface of the
// package that the document does not declare: a `named` type, an iface_call of this package.
// cir/1's strict reading refuses an iface_call to an undeclared interface; a named type with no
// declaration is the same defect one step earlier.
func undeclared(t *testing.T, m *Module) []string {
	t.Helper()
	doc := encoded(t, m)
	declared := map[string]bool{}
	for _, d := range list(doc["types"]) {
		declared[obj(d)["name"].(string)] = true
	}
	for _, d := range list(doc["interfaces"]) {
		if p, _ := obj(d)["path"].(string); p == "" {
			declared[obj(d)["name"].(string)] = true
		}
	}
	var out []string
	var walk func(v any)
	walk = func(v any) {
		switch v := v.(type) {
		case map[string]any:
			if v["k"] == "named" && !declared[v["name"].(string)] {
				out = append(out, "named "+v["name"].(string))
			}
			if p, _ := v["path"].(string); v["k"] == "iface_call" && p == "" && !declared[v["iface"].(string)] {
				out = append(out, "iface_call "+v["iface"].(string)+"."+v["name"].(string))
			}
			for _, x := range v {
				walk(x)
			}
		case []any:
			for _, x := range v {
				walk(x)
			}
		}
	}
	walk(doc)
	return out
}

// A field embedded through an alias is named after the alias, not after the type cir/1 writes
// for it, and an embedded `error` is named `error`; cir/1 names an embedded field after its
// `named` or `extern` type, so each such struct is refused on its own, with every declaration
// using it, and a struct embedding a type under its own name is still read.
func TestAStructEmbeddingThroughAnAliasOrEmbeddingErrorIsRefusedOnItsOwn(t *testing.T) {
	t.Setenv(importPathEnv, "example.com/p")
	m := loadSource(t, `package p

import "strings"

type local = strings.Builder

type ViaAlias struct{ local }

type ViaPointer struct{ *local }

type WithError struct {
	error
	n int
}

type Plain struct{ strings.Builder }

func Use(v ViaAlias) int { return v.Len() }

func Size(p Plain) int { return p.Len() }
`)
	var declared []string
	for _, td := range m.Types {
		declared = append(declared, td.Name)
		for _, f := range td.Fields {
			if f.Embedded && embeddedName(f.T) != f.Name {
				t.Errorf("embedded field %s of %s is not named after its type %+v", f.Name, td.Name, f.T)
			}
		}
	}
	if got := strings.Join(declared, " "); got != "Plain" {
		t.Errorf("types declared %q, want only Plain", got)
	}
	reasons := map[string]string{}
	for _, r := range m.Refusals {
		reasons[r.What] = r.Reason
	}
	for what, want := range map[string]string{
		"ViaAlias":   "embedded field local is not named after its type strings.Builder",
		"ViaPointer": "embedded field local is not named after its type *strings.Builder",
		"WithError":  "embedded field error is not named after its type error",
		"Use":        "parameter v of type example.com/p.ViaAlias has no cir/1 type",
	} {
		if !strings.Contains(reasons[what], want) {
			t.Errorf("refusal of %s: %q, want it to say %q (refusals %v)", what, reasons[what], want, m.Refusals)
		}
	}
	if len(m.Funcs) != 1 || m.Funcs[0].Name != "Size" {
		t.Errorf("funcs %v, want only Size", m.Funcs)
	}
}

// A type declared inside a function has no cir/1 declaration: a call through a local interface,
// a value of a local struct, are foreign, and the document names no type or interface it does
// not declare.
func TestATypeDeclaredInsideAFunctionIsNeverNamed(t *testing.T) {
	t.Setenv(importPathEnv, "example.com/p")
	m := loadSource(t, `package p

func Clone(in any) []byte {
	type marshaler interface {
		MarshalBinary() ([]byte, error)
	}
	m, ok := in.(marshaler)
	if !ok {
		return nil
	}
	b, _ := m.MarshalBinary()
	return b
}

func First(n int) int {
	type pair struct{ a, b int }
	var p pair
	p.a = n
	return p.a
}
`)
	if got := undeclared(t, m); len(got) != 0 {
		t.Fatalf("references to undeclared types: %v", got)
	}
	if len(m.Refusals) == 0 {
		t.Fatalf("no refusal for the local types")
	}
}
