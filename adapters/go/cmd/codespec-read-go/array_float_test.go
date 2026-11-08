package main

import (
	"sort"
	"strings"
	"testing"
)

func funcNamed(t *testing.T, m *Module, name string) Func {
	t.Helper()
	for _, f := range m.Funcs {
		if f.Name == name {
			return f
		}
	}
	t.Fatalf("no function %s; refusals %v", name, m.Refusals)
	return Func{}
}

// A fixed-size array and float32/float64 are cir/1 types: a function using them reads with no
// foreign statement, its float literals in their shortest round-trip spelling.
func TestArraysAndFloatsReadWithoutForeign(t *testing.T) {
	m := loadSource(t, `package p

func Weight(i int) float64 {
	var w [3]float64
	var h [2]float32
	w[0] = 0.10
	w[1] = w[0]*3 + 1e3
	h[1] = 1e-7
	return w[i] / 2
}
`)
	f := funcNamed(t, m, "Weight")
	for _, s := range f.Body {
		if s.K == "foreign" {
			t.Fatalf("foreign statement %q: %s", s.Src, s.Reason)
		}
	}
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	var floats []string
	var walk func(v any)
	walk = func(v any) {
		switch v := v.(type) {
		case node:
			if v["k"] == "float" {
				if val, ok := v["value"].(string); ok {
					floats = append(floats, val)
				}
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
	sort.Strings(floats)
	if got, want := strings.Join(floats, " "), "0.1 1000 1e-7 2 3"; got != want {
		t.Fatalf("float literals %q, want %q", got, want)
	}
	w := f.Body[0].T
	if w.K != "array" || w.Len != 3 || w.Elem.K != "float" || w.Elem.Bits != 64 {
		t.Fatalf("var w has type %+v, want an array of 3 float64", *w)
	}
}

// uintptr and unsafe.Pointer stay outside cir/1: a statement converting to uintptr stays foreign,
// and the rest of the function is read.
func TestAConversionToUintptrStaysForeign(t *testing.T) {
	m := loadSource(t, `package p

func Addr(n int) int {
	x := uintptr(n)
	_ = x
	return n + 1
}
`)
	f := funcNamed(t, m, "Addr")
	if len(f.Body) != 3 {
		t.Fatalf("body has %d statements, want 3", len(f.Body))
	}
	if s := f.Body[0]; s.K != "foreign" || !strings.Contains(s.Src, "uintptr(n)") {
		t.Fatalf("first statement %+v, want foreign holding uintptr(n)", s)
	}
	if s := f.Body[2]; s.K != "return" {
		t.Fatalf("last statement %+v, want return", s)
	}
}
