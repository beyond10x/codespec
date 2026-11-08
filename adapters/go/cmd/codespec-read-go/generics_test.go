package main

import (
	"encoding/json"
	"strings"
	"testing"
)

// A generic type and function are declared with their type parameters, each constrained by an
// interface; a method of a generic type binds the type's parameters through its receiver; a call
// of a generic function and a use of a generic type carry their type arguments, inferred ones
// written out; a method called on a value of a type parameter is an interface call through its
// constraint; a method of another package's generic type is declared generic over its receiver.
func TestGenericDeclarationsAndInstantiationsAreRead(t *testing.T) {
	m := loadSource(t, `package p

import (
	"sync"
	"sync/atomic"
)

type Sizer interface {
	Size() int
}

type Item struct {
	n int
}

func (i Item) Size() int {
	return i.n
}

type store[T Sizer] struct {
	items map[string]T
	mu    sync.Mutex
}

func newStore[T Sizer]() *store[T] {
	return &store[T]{items: map[string]T{}}
}

func (s *store[T]) put(key string, x T) {
	s.items[key] = x
}

func (s *store[T]) total() int {
	n := 0
	for _, x := range s.items {
		n += x.Size()
	}
	return n
}

func Total(n int) int {
	s := newStore[Item]()
	s.put("a", Item{n: n})
	return s.total() + first(Item{n: 1}, Item{n: 2}).n
}

func first[T any](x T, y T) T {
	return x
}

type holder struct {
	p atomic.Pointer[Item]
}

func (h *holder) get() *Item {
	return h.p.Load()
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals: %+v", m.Refusals)
	}
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	src := string(b)
	param := `{"k":"param","name":"T"}`
	item := `{"k":"named","name":"Item"}`
	for _, want := range []string{
		`"k":"struct","name":"store","pos":"p.go:20","tparams":[{"constraint":{"k":"named","name":"Sizer"},"name":"T"}]`,
		`{"name":"items","t":{"elem":` + param + `,"k":"map","key":{"k":"string"}}}`,
		`"name":"newStore","params":[],"pos":"p.go:25","results":[{"elem":{"k":"named","name":"store","targs":[` + param + `]},"k":"ptr"}],"tparams":[{"constraint":{"k":"named","name":"Sizer"},"name":"T"}]`,
		`"recv":{"name":"s","t":{"elem":{"k":"named","name":"store","targs":[` + param + `]},"k":"ptr"}}`,
		`"iface":"Sizer","k":"iface_call","name":"Size","recv":{"k":"var","name":"x","t":` + param + `}`,
		`"k":"call","name":"newStore","t":{"elem":{"k":"named","name":"store","targs":[` + item + `]},"k":"ptr"},"targs":[` + item + `]`,
		`"k":"call","name":"first","t":` + item + `,"targs":[` + item + `]`,
		`"tparams":[{"constraint":{"k":"any"},"name":"T"}]`,
		`{"name":"p","t":{"k":"extern","name":"Pointer","path":"sync/atomic","targs":[` + item + `]}}`,
		`{"name":"Load","params":[],"path":"sync/atomic","recv":{"elem":{"k":"extern","name":"Pointer","path":"sync/atomic","targs":[` + param + `]},"k":"ptr"},"results":[{"elem":` + param + `,"k":"ptr"}]}`,
	} {
		if !strings.Contains(src, want) {
			t.Fatalf("module %s\nwant %s", src, want)
		}
	}
}

// A constraint cir/1 cannot state, a type set or `comparable`, is refused with its own reason,
// and so is the declaration it constrains; no reason calls it generic.
func TestATypeSetConstraintIsRefusedNamingWhy(t *testing.T) {
	m := loadSource(t, `package p

type Number interface {
	~int | ~int64
}

func double[T Number](x T) T {
	return x + x
}

func has[K comparable](m map[K]bool, k K) bool {
	return m[k]
}

type set[K comparable] struct {
	m map[K]bool
}
`)
	got := map[string]string{}
	for _, r := range m.Refusals {
		got[r.What] = r.Reason
		if strings.Contains(r.Reason, "generic") {
			t.Errorf("%s: reason %q names generics, not the construct", r.What, r.Reason)
		}
	}
	for name, want := range map[string]string{
		"Number": "interface with a type set (a constraint): outside cir/1",
		"double": "constraint of type parameter T: Number has no cir/1 type",
		"has":    "constraint of type parameter K: comparable has no cir/1 type",
		"set":    "constraint of type parameter K: comparable has no cir/1 type",
	} {
		if got[name] != want {
			t.Errorf("%s: reason %q, want %q", name, got[name], want)
		}
	}
}
