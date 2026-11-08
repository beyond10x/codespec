package main

import (
	"encoding/json"
	"strings"
	"testing"
)

// An interface type of the package is declared with its whole method set; a method call through
// a value of an interface type, the package's own or another package's, is an interface call,
// and another package's interface is declared once it is called through; a concrete value given
// where an interface is expected (return, argument, assignment) is converted explicitly.
func TestInterfacesAreDeclaredAndCalledThrough(t *testing.T) {
	m := loadSource(t, `package p

import (
	"io"
	"strings"
)

// Shape has an area and a name.
type Shape interface {
	Name() string
	Area() int
}

type Square struct {
	side int
}

func (s Square) Area() int {
	return s.side * s.side
}

func (s Square) Name() string {
	return "square"
}

func Total(shapes []Shape) int {
	n := 0
	for _, s := range shapes {
		n += s.Area()
	}
	return n
}

func One(side int) Shape {
	return Square{side: side}
}

func Emit(w io.StringWriter, s string) {
	w.WriteString(s)
}

func Fill(b *strings.Builder) {
	Emit(b, "x")
}

func Keep(s Shape) Shape {
	var t Shape
	t = Square{side: 1}
	if s != nil {
		t = s
	}
	return t
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals: %+v", m.Refusals)
	}
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(doc["interfaces"])
	if err != nil {
		t.Fatal(err)
	}
	ifaces := string(b)
	for _, want := range []string{
		`{"doc":"Shape has an area and a name.","exported":true,"methods":[{"name":"Area","params":[],"results":[{"k":"int","signed":true,"width":"platform"}]},{"name":"Name","params":[],"results":[{"k":"string"}]}],"name":"Shape","pos":"p.go:9"}`,
		`{"exported":true,"methods":[{"name":"WriteString","params":[{"k":"string"}],"results":[{"k":"int","signed":true,"width":"platform"},{"k":"error"}]}],"name":"StringWriter","path":"io"}`,
	} {
		if !strings.Contains(ifaces, want) {
			t.Fatalf("interfaces %s\nwant %s", ifaces, want)
		}
	}
	if !has(m.Layout, "type:Shape") {
		t.Fatalf("layout %v has no type:Shape", m.Layout)
	}
	all, err := json.Marshal(doc["funcs"])
	if err != nil {
		t.Fatal(err)
	}
	src := string(all)
	for _, want := range []string{
		`"iface":"Shape","k":"iface_call","name":"Area","recv":{"k":"var","name":"s","t":{"k":"named","name":"Shape"}}`,
		`"iface":"StringWriter","k":"iface_call","name":"WriteString","path":"io","recv":{"k":"var","name":"w","t":{"k":"extern","name":"StringWriter","path":"io"}}`,
		`{"k":"to_iface","t":{"k":"named","name":"Shape"},"x":{"fields":[{"name":"side","value":{"k":"var","name":"side","t":{"k":"int","signed":true,"width":"platform"}}}],"k":"struct","name":"Square"`,
		`{"k":"to_iface","t":{"k":"extern","name":"StringWriter","path":"io"},"x":{"k":"var","name":"b","t":{"elem":{"k":"extern","name":"Builder","path":"strings"},"k":"ptr"}}}`,
		`"lhs":[{"k":"var","name":"t","t":{"k":"named","name":"Shape"}}],"pos":"p.go:48","rhs":[{"k":"to_iface"`,
		`"lhs":[{"k":"var","name":"t","t":{"k":"named","name":"Shape"}}],"pos":"p.go:50","rhs":[{"k":"var","name":"s"`,
	} {
		if !strings.Contains(src, want) {
			t.Fatalf("funcs %s\nwant %s", src, want)
		}
	}
}

// An interface cir/1 cannot declare is refused with its own reason: one with a type set (a
// constraint), one with a variadic method or a method of a type outside cir/1.
func TestAnInterfaceOutsideCir1IsRefusedNamingWhy(t *testing.T) {
	m := loadSource(t, `package p

import "unsafe"

type Number interface {
	~int | ~int64
}

type Logger interface {
	Log(args ...string)
}

type Raw interface {
	Ptr() unsafe.Pointer
}
`)
	got := map[string]string{}
	for _, r := range m.Refusals {
		got[r.What] = r.Reason
	}
	for name, want := range map[string]string{
		"Number": "interface with a type set (a constraint): outside cir/1",
		"Logger": "interface method Log is variadic: outside cir/1",
		"Raw":    "interface method Ptr: result of type unsafe.Pointer has no cir/1 type",
	} {
		if got[name] != want {
			t.Errorf("%s: reason %q, want %q", name, got[name], want)
		}
	}
}
