package main

import "testing"

// `err != nil` and `err == nil`: go/types leaves the nil untyped, and cir/1 has no untyped nil, so
// the nil takes the error's type and the module encodes.
func TestAnErrorComparedWithNilReadsWithTheNilTypedAsAnError(t *testing.T) {
	m := loadSource(t, `package p

import "errors"

var ErrNegative = errors.New("negative")

func Checked(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	}
	return n, nil
}

func Through(n int) (int, error) {
	v, err := Checked(n)
	if err != nil {
		return 0, err
	}
	return v, nil
}

func Ok(n int) bool {
	_, err := Checked(n)
	if err == nil {
		return true
	}
	return false
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	if _, err := encode(m); err != nil {
		t.Fatalf("encode: %v", err)
	}
	for _, name := range []string{"Through", "Ok"} {
		var fn *Func
		for i := range m.Funcs {
			if m.Funcs[i].Name == name {
				fn = &m.Funcs[i]
			}
		}
		if fn == nil {
			t.Fatalf("no func %s", name)
		}
		cond := fn.Body[1].Cond
		if cond == nil || cond.K != "binop" || cond.Y.K != "nil" {
			t.Fatalf("%s: condition %+v", name, cond)
		}
		if cond.Y.T == nil || cond.Y.T.K != "error" || cond.X.T == nil || cond.X.T.K != "error" {
			t.Fatalf("%s: nil typed %+v, compared with %+v", name, cond.Y.T, cond.X.T)
		}
	}
}

// A nil compared with a pointer, slice, map, interface or func value takes that value's type,
// whichever side it stands on, and never the untyped-nil kind.
func TestANilComparedWithANonErrorValueTakesThatValuesType(t *testing.T) {
	m := loadSource(t, `package p

type S struct{ A int }

type Shower interface{ Show() string }

func P(p *S) bool {
	if p != nil {
		return true
	}
	return false
}

func L(xs []int) bool {
	if nil == xs {
		return true
	}
	return false
}

func M(m map[string]int) bool {
	if m == nil {
		return true
	}
	return false
}

func I(s Shower) bool {
	if s != nil {
		return true
	}
	return false
}

func A(v any) bool {
	if nil != v {
		return true
	}
	return false
}
`)
	want := map[string]string{"P": "ptr", "L": "slice", "M": "map", "I": "named", "A": "any"}
	for i := range m.Funcs {
		fn := &m.Funcs[i]
		k, ok := want[fn.Name]
		if !ok {
			continue
		}
		cond := fn.Body[0].Cond
		if cond == nil || cond.K != "binop" {
			t.Fatalf("%s: condition %+v", fn.Name, cond)
		}
		for _, side := range []*Expr{cond.X, cond.Y} {
			if side.T == nil || side.T.K == "nil" {
				t.Fatalf("%s: %s side untyped: %+v", fn.Name, side.K, side.T)
			}
		}
		if cond.X.T.K != cond.Y.T.K {
			t.Fatalf("%s: nil typed %+v, compared with %+v (want kind like %s)", fn.Name, cond.Y.T, cond.X.T, k)
		}
	}
}
