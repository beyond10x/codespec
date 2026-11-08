package main

import (
	"encoding/json"
	"strings"
	"testing"
)

// A method promoted from an embedded interface, the package's own or another package's, is a
// call through that interface on the explicit path to the embedded field, not a method or an
// extern method of the embedding struct; its method value has no function to name.
func TestAMethodPromotedFromAnEmbeddedInterfaceIsAnInterfaceCall(t *testing.T) {
	m := loadSource(t, `package p

import "io"

type Shape interface {
	Area() int
}

type Framed struct {
	Shape
	io.StringWriter
}

func Use(f Framed) int {
	f.WriteString("x")
	return f.Area()
}

func Value(f Framed) func() int {
	return f.Area
}
`)
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	all, err := json.Marshal(doc["funcs"])
	if err != nil {
		t.Fatal(err)
	}
	src := string(all)
	for _, want := range []string{
		`"iface":"Shape","k":"iface_call","name":"Area","recv":{"k":"field","name":"Shape","t":{"k":"named","name":"Shape"},"x":{"k":"var","name":"f"`,
		`"iface":"StringWriter","k":"iface_call","name":"WriteString","path":"io","recv":{"k":"field","name":"StringWriter","t":{"k":"extern","name":"StringWriter","path":"io"},"x":{"k":"var","name":"f"`,
	} {
		if !strings.Contains(src, want) {
			t.Fatalf("funcs %s\nwant %s", src, want)
		}
	}
	if strings.Contains(src, `"k":"func_ref"`) || strings.Contains(src, `"k":"extern_method"`) || strings.Contains(src, `"k":"method"`) {
		t.Fatalf("a promoted interface method read as a concrete method: %s", src)
	}
}
