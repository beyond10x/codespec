package main

import (
	"fmt"
	"go/ast"
	"go/types"
)

// A package of a real repository holds declarations cir/1 cannot express: a struct with a field
// of another package's type, a function taking a func value, a method on such a struct. Each is
// refused on its own, with a reason, and the rest of the package is read; one such declaration
// never fails the module.

// scanOutside records every named type of the package that has no cir/1 declaration, with the
// reason, to a fixed point: a struct with a field of such a type has none either. typ then gives
// a reference to one of them the foreign type, so every signature and statement using it is
// refused in turn.
func (f *front) scanOutside(files []*ast.File) {
	f.outside = map[string]string{}
	var specs []*ast.TypeSpec
	for _, file := range files {
		for _, d := range file.Decls {
			if g, ok := d.(*ast.GenDecl); ok {
				for _, sp := range g.Specs {
					if ts, ok := sp.(*ast.TypeSpec); ok {
						specs = append(specs, ts)
					}
				}
			}
		}
	}
	for changed := true; changed; {
		changed = false
		for _, ts := range specs {
			if _, done := f.outside[ts.Name.Name]; done {
				continue
			}
			if why := f.typeOutside(ts); why != "" {
				f.outside[ts.Name.Name] = why
				changed = true
			}
		}
	}
}

// typeOutside is why the declared type ts has no cir/1 declaration, or "".
func (f *front) typeOutside(ts *ast.TypeSpec) string {
	obj, ok := f.info.Defs[ts.Name].(*types.TypeName)
	if !ok {
		return "type declaration outside cir/1"
	}
	if ts.Assign.IsValid() {
		return "type alias: outside cir/1"
	}
	if ts.TypeParams != nil {
		if it, ok := obj.Type().Underlying().(*types.Interface); ok {
			if why := f.ifaceOutside(it); why != "" {
				return why
			}
			return "interface with type parameters: outside cir/1"
		}
		if n, ok := obj.Type().(*types.Named); ok {
			if _, why := f.tparams(n.TypeParams()); why != "" {
				return why
			}
		}
	}
	switch u := obj.Type().Underlying().(type) {
	case *types.Struct:
		for i := 0; i < u.NumFields(); i++ {
			fl := u.Field(i)
			t := f.typ(fl.Type())
			if hasForeign(t) {
				return fmt.Sprintf("field %s of type %s has no cir/1 type", fl.Name(), f.outsideName(fl.Type()))
			}
			if fl.Embedded() && embeddedName(t) != fl.Name() {
				return fmt.Sprintf("embedded field %s is not named after its type %s (embedded through an alias, or a predeclared type): outside cir/1", fl.Name(), unaliased(fl.Type()))
			}
		}
	case *types.Basic, *types.Signature, *types.Map, *types.Slice:
		if hasForeign(f.typ(u)) {
			return fmt.Sprintf("underlying type %s has no cir/1 type", f.outsideName(u))
		}
	case *types.Interface:
		return f.ifaceOutside(u)
	default:
		return "type declaration other than struct or newtype over a basic, func, map or slice type"
	}
	return ""
}

// signatureOutside is why a function's receiver, parameters or results have no cir/1 type, or "".
func (f *front) signatureOutside(sig *types.Signature) string {
	if _, why := f.tparams(sig.TypeParams()); why != "" {
		return why
	}
	// A method of a generic type names the type's parameters in its receiver, each once.
	if rp := sig.RecvTypeParams(); rp != nil {
		for i := 0; i < rp.Len(); i++ {
			if rp.At(i).Obj().Name() == "_" {
				return "receiver type parameter named _: outside cir/1"
			}
		}
	}
	check := func(what string, t types.Type) string {
		if hasForeign(f.typ(t)) {
			return fmt.Sprintf("%s of type %s has no cir/1 type", what, f.outsideName(t))
		}
		return ""
	}
	if r := sig.Recv(); r != nil {
		if why := check("receiver", r.Type()); why != "" {
			return why
		}
	}
	for i := 0; i < sig.Params().Len(); i++ {
		if why := check("parameter "+sig.Params().At(i).Name(), sig.Params().At(i).Type()); why != "" {
			return why
		}
	}
	for i := 0; i < sig.Results().Len(); i++ {
		if why := check("result", sig.Results().At(i).Type()); why != "" {
			return why
		}
	}
	return ""
}

// foreignName names the first part of t with no cir/1 form; never "" for a type hasForeign holds,
// since "" means "none" to the callers (a pointer to a foreign struct has no name of its own).
func foreignName(t Type) string {
	if t.K == "foreign" || t.K == "nil" {
		if t.Name != "" {
			return t.Name
		}
		return t.K
	}
	for _, p := range []*Type{t.Key, t.Elem} {
		if p != nil && hasForeign(*p) {
			return foreignName(*p)
		}
	}
	for _, e := range t.Elems {
		if hasForeign(e) {
			return foreignName(e)
		}
	}
	for _, e := range append(append([]Type{}, t.Params...), t.Results...) {
		if hasForeign(e) {
			return foreignName(e)
		}
	}
	return t.K
}

// outsideName is the Go spelling of what has no cir/1 form in the Go type t: t itself, or for a
// function type the first parameter or result type that has none, so a reason names the type
// that is missing rather than the function type that holds it.
func (f *front) outsideName(t types.Type) string {
	if ft := f.typ(t); ft.K == "func" {
		return foreignName(ft)
	}
	return t.String()
}

// stmtForeignType is the first type without a cir/1 form among the expressions and the declared
// type a statement holds directly (nested blocks are checked as their own statements), or "".
func stmtForeignType(s Stmt) string {
	if s.K == "foreign" {
		return ""
	}
	if s.T != nil && hasForeign(*s.T) {
		return foreignName(*s.T)
	}
	var exprs []Expr
	for _, p := range []*Expr{s.X, s.Chan, s.Cond, s.Over, s.Map, s.Index, s.Tag} {
		if p != nil {
			exprs = append(exprs, *p)
		}
	}
	exprs = append(exprs, s.Lhs...)
	exprs = append(exprs, s.Rhs...)
	exprs = append(exprs, s.Results...)
	for _, c := range s.Cases {
		exprs = append(exprs, c.Values...)
	}
	for _, e := range exprs {
		if n := exprForeignType(e); n != "" {
			return n
		}
	}
	return ""
}

func exprForeignType(e Expr) string {
	if e.T != nil && hasForeign(*e.T) {
		return foreignName(*e.T)
	}
	for _, p := range []*Expr{e.X, e.Y} {
		if p != nil {
			if n := exprForeignType(*p); n != "" {
				return n
			}
		}
	}
	for _, a := range e.Args {
		if n := exprForeignType(a); n != "" {
			return n
		}
	}
	return ""
}

// unaliased spells t with an alias, at the top or under one pointer, resolved to the type it names.
func unaliased(t types.Type) string {
	if p, ok := types.Unalias(t).(*types.Pointer); ok {
		return "*" + types.Unalias(p.Elem()).String()
	}
	return types.Unalias(t).String()
}
