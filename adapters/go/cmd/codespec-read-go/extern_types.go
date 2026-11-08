package main

import "go/types"

// Types of another package are opaque `extern` types (typ), and a method called on a value of one
// is an extern method call, declared in Externs with its receiver.

// externMethod reads x.Name(args), where fn is a method of a type of another package, as an extern
// method call, declaring the method from its own signature: a method of a generic type with the
// type's parameters as its receiver's type arguments. A receiver that is not an extern type or a
// pointer to one, or a method with a parameter or result type cir/1 has no type for, is outside
// cir/1; a variadic method is declared variadic.
func (f *front) externMethod(fn *types.Func, x Expr, args []Expr, t *Type) (Expr, bool) {
	fn = fn.Origin()
	sig := fn.Type().(*types.Signature)
	if t == nil || sig.Recv() == nil || sig.TypeParams().Len() > 0 {
		return Expr{}, false
	}
	recv := f.typ(sig.Recv().Type())
	base := recv
	if base.K == "ptr" {
		base = *base.Elem
	}
	if base.K != "extern" || base.Path != fn.Pkg().Path() {
		return Expr{}, false
	}
	d := Extern{Path: base.Path, Name: fn.Name(), Recv: &recv, Variadic: sig.Variadic()}
	for i := 0; i < sig.Params().Len(); i++ {
		p := f.typ(sig.Params().At(i).Type())
		if hasForeign(p) {
			return Expr{}, false
		}
		d.Params = append(d.Params, p)
	}
	for i := 0; i < sig.Results().Len(); i++ {
		r := f.typ(sig.Results().At(i).Type())
		if hasForeign(r) {
			return Expr{}, false
		}
		d.Results = append(d.Results, r)
	}
	if hasForeign(*t) {
		return Expr{}, false
	}
	f.extern(d)
	return Expr{K: "extern_method", Path: d.Path, TypeName: base.Name, Name: d.Name, X: &x, Args: args, T: t}, true
}

// recvTypeName is the name of the extern type a method's receiver belongs to; "" for a function.
func (x Extern) recvTypeName() string {
	if x.Recv == nil {
		return ""
	}
	r := *x.Recv
	if r.K == "ptr" {
		r = *r.Elem
	}
	return r.Name
}

// key is "path.name" for a function and "path.Type.name" for a method: what a call names.
func (x Extern) key() string {
	if t := x.recvTypeName(); t != "" {
		return x.Path + "." + t + "." + x.Name
	}
	return x.Path + "." + x.Name
}
