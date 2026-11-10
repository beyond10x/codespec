package main

import (
	"testing"
)

// sortedKeysCalls lists every sorted_keys intrinsic in body, depth first through results,
// right-hand sides and nested blocks.
func sortedKeysCalls(body []Stmt) []Expr {
	var out []Expr
	var expr func(e *Expr)
	expr = func(e *Expr) {
		if e == nil {
			return
		}
		if e.K == "intrinsic" && e.Name == "sorted_keys" {
			out = append(out, *e)
		}
		expr(e.X)
		for i := range e.Args {
			expr(&e.Args[i])
		}
	}
	var walk func(ss []Stmt)
	walk = func(ss []Stmt) {
		for _, s := range ss {
			for i := range s.Results {
				expr(&s.Results[i])
			}
			for i := range s.Rhs {
				expr(&s.Rhs[i])
			}
			walk(s.Then)
			walk(s.Else)
			walk(s.Body)
		}
	}
	walk(body)
	return out
}

// `slices.Sorted(maps.Keys(m))` (Go 1.23) is the keys of m in ascending order. Over integer keys
// that is cir/1's sorted_keys, a named key type included. Over string keys it is not: Go orders
// strings by byte, sorted_keys by UTF-16 code unit, so that call is read as before. Over a named
// map type it is read as before too: sorted_keys takes a map, and the Go writer refuses it of a
// named type. Its neighbours (collect, then sort) are read as before: no sorted_keys.
func TestSlicesSortedOfMapsKeysOverIntegerKeysIsSortedKeys(t *testing.T) {
	m := loadSource(t, `package p

import (
	"maps"
	"slices"
)

type ID int

type Index map[ID]bool

func Ints(m map[int]string) []int { return slices.Sorted(maps.Keys(m)) }

func IDs(m map[ID]bool) []ID { return slices.Sorted(maps.Keys(m)) }

func Indexed(m Index) []ID { return slices.Sorted(maps.Keys(m)) }

func Names(m map[string]int) []string { return slices.Sorted(maps.Keys(m)) }

func Collect(m map[int]int) []int {
	k := slices.Collect(maps.Keys(m))
	slices.Sort(k)
	return k
}
`)
	for _, name := range []string{"Ints", "IDs"} {
		body := funcNamed(t, m, name).Body
		got := sortedKeysCalls(body)
		if len(got) != 1 || len(body) != 1 || body[0].K != "return" {
			t.Fatalf("%s: %d sorted_keys, body kinds %v, want a return of one sorted_keys", name, len(got), kinds(body))
		}
		sk := got[0]
		if len(sk.Args) != 1 || sk.Args[0].K != "var" || sk.Args[0].Name != "m" || sk.T == nil || sk.T.K != "slice" {
			t.Fatalf("%s: sorted_keys %+v, want one argument, the variable m, and a slice type", name, sk)
		}
	}
	for _, name := range []string{"Indexed", "Names", "Collect"} {
		if got := sortedKeysCalls(funcNamed(t, m, name).Body); len(got) != 0 {
			t.Fatalf("%s: read as sorted_keys %+v", name, got)
		}
	}
}
