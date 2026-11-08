package main

import (
	"fmt"
	"go/ast"
	"go/types"
	"reflect"
	"sort"
)

// Interface types. The package's own is declared with its whole method set; another package's is
// declared once a call goes through it. A method call through a value of a named interface type
// is an iface_call; a value given where an interface is expected is converted with to_iface.

// ifaceOutside is why cir/1 cannot declare the interface it, or "".
func (f *front) ifaceOutside(it *types.Interface) string {
	if !it.IsMethodSet() {
		return "interface with a type set (a constraint): outside cir/1"
	}
	for i := 0; i < it.NumMethods(); i++ {
		m := it.Method(i)
		sig := m.Type().(*types.Signature)
		if sig.Variadic() {
			return fmt.Sprintf("interface method %s is variadic: outside cir/1", m.Name())
		}
		check := func(what string, t types.Type) string {
			if hasForeign(f.typ(t)) {
				return fmt.Sprintf("interface method %s: %s of type %s has no cir/1 type", m.Name(), what, f.outsideName(t))
			}
			return ""
		}
		for j := 0; j < sig.Params().Len(); j++ {
			p := sig.Params().At(j)
			what := "parameter"
			if p.Name() != "" {
				what += " " + p.Name()
			}
			if why := check(what, p.Type()); why != "" {
				return why
			}
		}
		for j := 0; j < sig.Results().Len(); j++ {
			if why := check("result", sig.Results().At(j).Type()); why != "" {
				return why
			}
		}
	}
	return ""
}

// ifaceMethods is the method set of it, sorted by name.
func (f *front) ifaceMethods(it *types.Interface) []IfaceMethod {
	out := []IfaceMethod{}
	for i := 0; i < it.NumMethods(); i++ {
		m := it.Method(i)
		sig := m.Type().(*types.Signature)
		im := IfaceMethod{Name: m.Name(), Params: []Type{}, Results: []Type{}}
		for j := 0; j < sig.Params().Len(); j++ {
			im.Params = append(im.Params, f.typ(sig.Params().At(j).Type()))
		}
		for j := 0; j < sig.Results().Len(); j++ {
			im.Results = append(im.Results, f.typ(sig.Results().At(j).Type()))
		}
		out = append(out, im)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Name < out[j].Name })
	return out
}

// ifaceDecl declares the package's own interface ts.
func (f *front) ifaceDecl(ts *ast.TypeSpec, it *types.Interface, doc string) {
	f.mod.Interfaces = append(f.mod.Interfaces, Interface{
		Name: ts.Name.Name, Exported: ts.Name.IsExported(), Methods: f.ifaceMethods(it), Doc: doc, Pos: f.pos(ts.Pos()),
	})
	f.layout = append(f.layout, "type:"+ts.Name.Name)
}

// ifaceCall reads x.Name(args) where rt, the type of x, is a named interface type: the package's
// own, or another package's, which is declared as a candidate. The universe's error, an unnamed
// or generic interface, and another package's interface cir/1 cannot declare are not read here.
func (f *front) ifaceCall(rt types.Type, name string, x Expr, args []Expr, t *Type) (Expr, bool) {
	n, ok := types.Unalias(rt).(*types.Named)
	if !ok || n.Obj().Pkg() == nil || declaredInFunc(n.Obj()) || n.TypeArgs().Len() > 0 || t == nil || hasForeign(*t) {
		return Expr{}, false
	}
	it := n.Underlying().(*types.Interface)
	path := ""
	if n.Obj().Pkg() != f.pkg {
		if f.ifaceOutside(it) != "" {
			return Expr{}, false
		}
		path = n.Obj().Pkg().Path()
		d := Interface{Path: path, Name: n.Obj().Name(), Exported: true, Methods: f.ifaceMethods(it)}
		known := false
		for _, y := range f.mod.Interfaces {
			known = known || (y.Path == d.Path && y.Name == d.Name)
		}
		if !known {
			f.mod.Interfaces = append(f.mod.Interfaces, d)
		}
	} else if _, out := f.outside[n.Obj().Name()]; out {
		return Expr{}, false
	}
	return Expr{K: "iface_call", Path: path, Iface: n.Obj().Name(), Name: name, X: &x, Args: args, T: t}, true
}

// toIface converts e to the interface type want, the package's own (named) or another package's
// (extern): unchanged when it already has that type, or is nil, which takes it.
func toIface(e Expr, want Type) Expr {
	if e.T == nil || hasForeign(*e.T) || reflect.DeepEqual(*e.T, want) {
		return e
	}
	if e.K == "nil" {
		e.T = &want
		return e
	}
	return Expr{K: "to_iface", X: &e, T: &want}
}

// ifaceTarget is the cir/1 type of want when it is a named interface type of the package or of
// another package: what a value given where want is expected is converted to.
func (f *front) ifaceTarget(want types.Type) (Type, bool) {
	if want == nil || !types.IsInterface(want) {
		return Type{}, false
	}
	wt := f.typ(want)
	return wt, wt.K == "named" || wt.K == "extern"
}

// coerceArgs converts each argument given to a parameter of a named interface type of sig.
func (f *front) coerceArgs(args []Expr, sig *types.Signature) {
	if sig == nil || sig.Variadic() || sig.Params().Len() != len(args) {
		return
	}
	for i := range args {
		if wt, ok := f.ifaceTarget(sig.Params().At(i).Type()); ok {
			args[i] = toIface(args[i], wt)
		}
	}
}
