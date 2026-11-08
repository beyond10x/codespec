package main

import (
	"strings"
	"testing"
)

// A composite literal, `new`, a conversion, a constant of a named type, a `var` zero value and
// the struct fields each make holds by value are listed as `new`, once per type, only for a
// package with an import path; an interface type never is.
func TestMadeTypesAreListedOncePerConstruct(t *testing.T) {
	src := `package p

import (
	"io"
	"unsafe"
)

type Inner struct{ n int }

type Outer struct {
	in  Inner
	ptr *Ptr
}

type Ptr struct{}

type Level int

const Debug Level = 1

type Conv struct{}

type Zero struct{}

func Raw(p unsafe.Pointer, w io.Writer) {
	o := &Outer{}
	o2 := Outer{}
	q := new(Ptr)
	l := Debug
	var z Zero
	_, _, _, _, _ = o, o2, q, l, z
}
`
	t.Setenv(importPathEnv, "example.com/p")
	doc := encoded(t, loadSource(t, src))
	raw := findFunc(doc, "Raw", "opaque")
	if raw == nil {
		t.Fatalf("Raw is not opaque: %v", doc["opaque"])
	}
	var made []string
	for _, c := range callKeys(list(raw["calls"])) {
		if strings.HasPrefix(c, "new:") {
			made = append(made, c)
		}
	}
	if got := strings.Join(made, ","); got != "new:Outer,new:Inner,new:Ptr,new:Level,new:Zero" {
		t.Fatalf("made %s", got)
	}

	t.Setenv(importPathEnv, "")
	doc = encoded(t, loadSource(t, src))
	for _, c := range callKeys(list(findFunc(doc, "Raw", "opaque")["calls"])) {
		if strings.HasPrefix(c, "new:") {
			t.Fatalf("a package without an import path lists %s", c)
		}
	}
}
