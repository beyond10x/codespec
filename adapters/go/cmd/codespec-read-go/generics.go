package main

import (
	"fmt"
	"go/ast"
	"go/types"
)

// Type parameters. A generic function or type is declared with its type parameters, each
// constrained by an interface cir/1 declares (any, the package's own, or another package's); a
// method of a generic type binds them through its receiver's type arguments. Every call of a
// generic function and every use of a generic type carries its type arguments, the inferred ones
// from Info.Instances written out. A constraint with a type set (`~int | ~string`, `comparable`)
// has no cir/1 form and is refused with that reason.

// tparams reads list; why is the reason one of its constraints has no cir/1 form, or "".
func (f *front) tparams(list *types.TypeParamList) (out []TypeParam, why string) {
	for i := 0; i < list.Len(); i++ {
		tp := list.At(i)
		c, ok := f.constraint(tp.Constraint())
		if !ok {
			return nil, fmt.Sprintf("constraint of type parameter %s: %s has no cir/1 type",
				tp.Obj().Name(), types.TypeString(tp.Constraint(), types.RelativeTo(f.pkg)))
		}
		out = append(out, TypeParam{Name: tp.Obj().Name(), Constraint: c})
	}
	return out, ""
}

// constraint is the cir/1 type of the constraint c: any, or a named interface that is a method set
// only, the package's own (named) or another package's (extern).
func (f *front) constraint(c types.Type) (Type, bool) {
	if it, ok := types.Unalias(c).(*types.Interface); ok {
		return T("any"), it.Empty()
	}
	n, ok := types.Unalias(c).(*types.Named)
	if !ok || n.Obj().Pkg() == nil || n.TypeArgs().Len() > 0 {
		return Type{}, false
	}
	if it, ok := n.Underlying().(*types.Interface); !ok || !it.IsMethodSet() {
		return Type{}, false
	}
	t := f.typ(n)
	return t, t.K == "named" || t.K == "extern"
}

// typeArgs reads the type arguments of an instantiation; false when one has no cir/1 type.
func (f *front) typeArgs(list *types.TypeList) ([]Type, bool) {
	var out []Type
	for i := 0; i < list.Len(); i++ {
		t := f.typ(list.At(i))
		if hasForeign(t) {
			return nil, false
		}
		out = append(out, t)
	}
	return out, true
}

// instance is the instantiation id stands for, a use of a generic function, with its type
// arguments read; ok false when it is not one.
func (f *front) instance(id *ast.Ident) (targs []Type, generic, ok bool) {
	in, found := f.info.Instances[id]
	if !found {
		return nil, false, true
	}
	targs, ok = f.typeArgs(in.TypeArgs)
	return targs, true, ok
}

// calleeIdent is the identifier a call's function expression names through an explicit
// instantiation (`f[int]`, `f[int, string]`, `pkg.F[int]`), or the expression itself.
func calleeIdent(fun ast.Expr) ast.Expr {
	switch ix := fun.(type) {
	case *ast.IndexExpr:
		return ix.X
	case *ast.IndexListExpr:
		return ix.X
	}
	return fun
}

// paramCall reads x.Name(args) where x is a value of a type parameter: a call through the
// interface that constrains it.
func (f *front) paramCall(tp *types.TypeParam, name string, x Expr, args []Expr, t *Type) (Expr, bool) {
	n, ok := types.Unalias(tp.Constraint()).(*types.Named)
	if !ok {
		return Expr{}, false
	}
	return f.ifaceCall(n, name, x, args, t)
}
