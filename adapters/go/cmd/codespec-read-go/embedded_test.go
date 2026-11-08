package main

import (
	"strings"
	"testing"
)

// show spells the reads and calls an expression makes, the receiver of each spelled out:
// `s.Mutex.<sync.Mutex>Lock()` for an extern method call, `s.Inner.N` for a field access.
func show(e *Expr) string {
	if e == nil {
		return ""
	}
	switch e.K {
	case "var":
		return e.Name
	case "field":
		return show(e.X) + "." + e.Name
	case "method":
		return show(e.X) + "." + e.Name + "()"
	case "extern_method":
		return show(e.X) + ".<" + e.Path + "." + e.TypeName + ">" + e.Name + "()"
	case "func_ref":
		return show(e.X) + "." + e.Name
	}
	return e.K
}

// shown lists show of every expression a statement holds directly, depth first.
func shown(body []Stmt) []string {
	var out []string
	for _, s := range body {
		if s.X != nil {
			out = append(out, show(s.X))
		}
		for i := range s.Rhs {
			out = append(out, show(&s.Rhs[i]))
		}
		for i := range s.Results {
			out = append(out, show(&s.Results[i]))
		}
		out = append(out, shown(s.Then)...)
		out = append(out, shown(s.Body)...)
	}
	return out
}

// An embedded field is a field marked embedded, named after its type; a promoted field access or
// method call reads as the explicit path through the embedded field, so no consumer needs Go's
// promotion rules. The method set of the embedding type keeps the promoted methods, each with the
// call that runs.
func TestEmbeddedFieldsReadAndPromotionIsWrittenOut(t *testing.T) {
	t.Setenv(importPathEnv, "example.com/p")
	m := loadSource(t, `package p

import "sync"

type Inner struct {
	N int
}

func (i *Inner) Bump() int {
	i.N = i.N + 1
	return i.N
}

type S struct {
	sync.Mutex
	Inner
	Name string
}

func Get(s *S) int {
	s.Lock()
	n := s.N
	s.Bump()
	s.Unlock()
	return n
}

func Value(s S) func() int {
	return s.Bump
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	var s *TypeDecl
	for i := range m.Types {
		if m.Types[i].Name == "S" {
			s = &m.Types[i]
		}
	}
	if s == nil {
		t.Fatalf("S not declared: types %v", m.Types)
	}
	var fields []string
	for _, f := range s.Fields {
		fields = append(fields, f.Name+"/"+f.T.K+map[bool]string{true: "/embedded"}[f.Embedded])
	}
	if got, want := strings.Join(fields, " "), "Mutex/extern/embedded Inner/named/embedded Name/string"; got != want {
		t.Fatalf("fields of S %q, want %q", got, want)
	}
	var got []string
	for _, fn := range m.Funcs {
		got = append(got, shown(fn.Body)...)
	}
	for _, want := range []string{"s.Mutex.<sync.Mutex>Lock()", "s.Inner.N", "s.Inner.Bump()", "s.Mutex.<sync.Mutex>Unlock()", "s.Inner.Bump"} {
		found := false
		for _, g := range got {
			found = found || g == want
		}
		if !found {
			t.Errorf("no %s among %q", want, got)
		}
	}
	targets := map[string]string{}
	for _, set := range m.Methods {
		if set.T != "S" {
			continue
		}
		for _, e := range set.Methods {
			targets[e.Name] = e.Target.K + " " + e.Target.Path + " " + e.Target.Recv + "." + e.Target.Name
		}
	}
	for name, want := range map[string]string{
		"Lock":   "method sync Mutex.Lock",
		"Unlock": "method sync Mutex.Unlock",
		"Bump":   "method  Inner.Bump",
	} {
		if targets[name] != want {
			t.Errorf("method set of S: %s runs %q, want %q", name, targets[name], want)
		}
	}
}

// A field promoted through an embedded struct of another package is a field of a struct of
// another package: foreign, as a direct one is.
func TestAFieldPromotedFromAStructOfAnotherPackageStaysForeign(t *testing.T) {
	m := loadSource(t, `package p

import "net/url"

type Link struct {
	*url.URL
}

func Host(l *Link) string {
	return l.Host
}
`)
	if len(m.Types) != 1 || len(m.Types[0].Fields) != 1 || !m.Types[0].Fields[0].Embedded {
		t.Fatalf("Link with an embedded *url.URL not declared: types %v, refusals %v", m.Types, m.Refusals)
	}
	if len(m.Refusals) != 1 || !strings.Contains(m.Refusals[0].What, "l.Host") {
		t.Fatalf("want one refusal, of l.Host, got %v", m.Refusals)
	}
}
