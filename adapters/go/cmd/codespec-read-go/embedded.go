package main

import (
	"go/ast"
	"go/types"
)

// embeddedName is the name cir/1 gives a field embedding t: the name of its `named` or `extern`
// type, through one pointer; "" for any other type. The model refuses an embedded field named
// otherwise (`Field::embedded_name`).
func embeddedName(t Type) string {
	if t.K == "ptr" && t.Elem != nil {
		t = *t.Elem
	}
	if t.K == "named" || t.K == "extern" {
		return t.Name
	}
	return ""
}

// holder is the type of the value the method or field of sel belongs to: the receiver's type, or
// with a promoted selection the type of the last embedded field it passes through, so a method
// promoted from an embedded interface is a method of that interface.
func holder(sel *types.Selection) types.Type {
	path := sel.Index()
	t := sel.Recv()
	for _, i := range path[:len(path)-1] {
		if p, ok := t.Underlying().(*types.Pointer); ok {
			t = p.Elem()
		}
		st, ok := t.Underlying().(*types.Struct)
		if !ok {
			return t
		}
		t = st.Field(i).Type()
	}
	return t
}

// receiver reads the operand of the selection e as the value the selected field or method
// belongs to: e.X, then one field access per embedded field the selection passes through, so
// `s.N` with N promoted from the embedded Inner reads as `s.Inner.N`. An embedded field of a
// struct of another package is not a field cir/1 reads.
func (f *front) receiver(e *ast.SelectorExpr, sel *types.Selection) (Expr, bool) {
	x, ok := f.expr(e.X)
	if !ok {
		return Expr{}, false
	}
	path := sel.Index()
	t := sel.Recv()
	for _, i := range path[:len(path)-1] {
		if p, ok := t.Underlying().(*types.Pointer); ok {
			t = p.Elem()
		}
		st, ok := t.Underlying().(*types.Struct)
		if !ok {
			return Expr{}, false
		}
		fl := st.Field(i)
		if fl.Pkg() != f.pkg {
			return Expr{}, false
		}
		ft := f.typ(fl.Type())
		inner := x
		x = Expr{K: "field", X: &inner, Name: fl.Name(), T: &ft}
		t = fl.Type()
	}
	return x, true
}
