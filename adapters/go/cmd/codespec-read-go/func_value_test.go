package main

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// exprKinds lists the k of every expression in body, at any depth, lambda bodies included.
func exprKinds(body []Stmt) []string {
	var out []string
	var ex func(e *Expr)
	var st func(ss []Stmt)
	ex = func(e *Expr) {
		if e == nil {
			return
		}
		out = append(out, e.K)
		ex(e.X)
		ex(e.Y)
		for i := range e.Args {
			ex(&e.Args[i])
		}
		st(e.Body)
	}
	st = func(ss []Stmt) {
		for _, s := range ss {
			for _, e := range []*Expr{s.X, s.Cond, s.Over, s.Tag, s.Chan} {
				ex(e)
			}
			for _, l := range [][]Expr{s.Lhs, s.Rhs, s.Results} {
				for i := range l {
					ex(&l[i])
				}
			}
			st(s.Then)
			st(s.Else)
			st(s.Body)
		}
	}
	st(body)
	return out
}

func findExpr(body []Stmt, k string) *Expr {
	var found *Expr
	var ex func(e *Expr)
	ex = func(e *Expr) {
		if e == nil || found != nil {
			return
		}
		if e.K == k {
			found = e
			return
		}
		ex(e.X)
		for i := range e.Args {
			ex(&e.Args[i])
		}
	}
	for _, s := range body {
		for i := range s.Rhs {
			ex(&s.Rhs[i])
		}
		for i := range s.Results {
			ex(&s.Results[i])
		}
	}
	return found
}

// Each function-value case reads with no refusal, to the expressions its subject names: a
// function type, a module function and a method used as values, a function literal with its
// captures, a call of a function value held in a field.
func TestFunctionValueCasesReadWithoutRefusal(t *testing.T) {
	for _, c := range []struct {
		dir, fn string
		want    []string
	}{
		{"type-func", "Apply", []string{"call", "func_ref"}},
		{"expr-func-value", "Compose", []string{"conv", "func_ref", "call_value"}},
		{"expr-lambda", "Scale", []string{"lambda", "call_value"}},
		{"expr-call-value", "Allowed", []string{"struct", "func_ref", "call_value", "field"}},
	} {
		src, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "conformance", "cases", c.dir, "case.go"))
		if err != nil {
			t.Fatal(err)
		}
		m := loadSource(t, string(src))
		if len(m.Refusals) != 0 {
			t.Fatalf("%s: refusals %v", c.dir, m.Refusals)
		}
		var fn *Func
		for i := range m.Funcs {
			if m.Funcs[i].Name == c.fn {
				fn = &m.Funcs[i]
			}
		}
		if fn == nil {
			t.Fatalf("%s: no function %s", c.dir, c.fn)
		}
		got := strings.Join(exprKinds(fn.Body), " ")
		for _, k := range c.want {
			if !strings.Contains(" "+got+" ", " "+k+" ") {
				t.Fatalf("%s: %s has expressions %q, want %s among them", c.dir, c.fn, got, k)
			}
		}
		if _, err := encode(m); err != nil {
			t.Fatalf("%s: %v", c.dir, err)
		}
	}
}

// A function literal names the locals and parameters of its enclosing function that it uses,
// sorted, each once; not its own parameters or locals, not package-level names, not fields.
func TestALambdaCapturesTheOuterLocalsItUses(t *testing.T) {
	m := loadSource(t, `package p

type S struct{ n int }

func g(x int) int { return x }

func F(a int, b int, s S) int {
	c := 1
	h := func(x int) int {
		y := x + b
		c++
		return g(y) + a + a + s.n
	}
	return h(c)
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	l := findExpr(m.Funcs[1].Body, "lambda")
	if l == nil {
		t.Fatalf("no lambda in %+v", m.Funcs[1].Body)
	}
	if want := []string{"a", "b", "c", "s"}; !reflect.DeepEqual(l.Captures, want) {
		t.Fatalf("captures %v, want %v", l.Captures, want)
	}
}

// A named func, map or slice type is a newtype; an interface is declared as an interface.
func TestNamedFuncMapAndSliceTypesAreNewtypes(t *testing.T) {
	m := loadSource(t, `package p

type Handler func(string) error

type Index map[string]int

type Names []string

type Shape interface{ Area() int }

func Reset(i Index) Index {
	i = nil
	return i
}
`)
	if len(m.Funcs) != 1 || kinds(m.Funcs[0].Body)[0] != "assign" || m.Funcs[0].Body[0].Rhs[0].T.K != "named" {
		t.Fatalf("i = nil not read as an assignment of a nil Index: %+v", m.Funcs)
	}
	var got []string
	for _, td := range m.Types {
		got = append(got, td.Name+":"+td.K+":"+td.Under.K)
	}
	if want := []string{"Handler:newtype:func", "Index:newtype:map", "Names:newtype:slice"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("types %v, want %v", got, want)
	}
	if len(m.Refusals) != 0 || len(m.Interfaces) != 1 || m.Interfaces[0].Name != "Shape" {
		t.Fatalf("want Shape declared as an interface, nothing refused: interfaces %v, refusals %v", m.Interfaces, m.Refusals)
	}
}
