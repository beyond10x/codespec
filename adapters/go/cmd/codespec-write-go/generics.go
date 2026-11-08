package main

import "go/ast"

// Type parameters: a generic type or function is written with its type parameter list, a
// type parameter as its name, and an instantiation with its type arguments in brackets.

// typeParams is the type parameter list `[T C, U D]`; nil without any.
func (w *writer) typeParams(ps []TypeParam) *ast.FieldList {
	if len(ps) == 0 {
		return nil
	}
	fl := &ast.FieldList{}
	for _, p := range ps {
		fl.List = append(fl.List, &ast.Field{Names: []*ast.Ident{ident(p.Name)}, Type: w.typ(p.Constraint)})
	}
	return fl
}

// instantiate is x with the type arguments targs: `x[A]`, `x[A, B]`; x itself without any.
func (w *writer) instantiate(x ast.Expr, targs []Type) ast.Expr {
	switch len(targs) {
	case 0:
		return x
	case 1:
		return &ast.IndexExpr{X: x, Index: w.typ(targs[0])}
	}
	ix := &ast.IndexListExpr{X: x}
	for _, t := range targs {
		ix.Indices = append(ix.Indices, w.typ(t))
	}
	return ix
}

// named is the type a struct literal or a conversion names: the declared type name, with the
// type arguments of t when t is an instance of it.
func (w *writer) named(name string, t *Type) ast.Expr {
	if t != nil && t.K == "named" && t.Name == name {
		return w.typ(*t)
	}
	return ident(w.names.decl(name))
}
