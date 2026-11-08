package main

import (
	"bufio"
	"bytes"
	"go/ast"
	"go/build"
	"go/constant"
	"go/token"
	"go/types"
	"os"
	"path/filepath"
	"strings"
)

// What a whole-program analysis needs beyond the bodies cir/1 holds: every call a construct
// outside cir/1 makes, named from go/types; every function the module declares but does not
// read; the method set of each named type; the //go:linkname directives; the import path.

// CallRef is one call, as cir/1's CallRef: func, method, iface, value, ref or lambda.
type CallRef struct {
	K       string
	Path    string
	Recv    string
	Name    string
	Iface   string
	Sig     string
	Consts  []ConstArg
	Methods []MethodSig
	Calls   []CallRef
	Pos     string
}

type ConstArg struct {
	Index int
	Value string
}

type MethodSig struct {
	Name string
	Sig  string
}

// Opaque is a function the module declares whose body cir/1 does not hold.
type Opaque struct {
	Name   string
	Recv   string
	Pos    string
	Reason string
	NoBody bool
	Calls  []CallRef
}

type MethodSet struct {
	T       string
	Methods []MethodEntry
}

type MethodEntry struct {
	Name   string
	Sig    string
	Target CallRef
}

type Linkname struct {
	Name   string
	Target string
}

// importPathEnv overrides the import path the reader derives, for a directory it cannot place.
const importPathEnv = "CODESPEC_READ_GO_IMPORT_PATH"

// importPath is the import path of the package in dir: relative to GOROOT/src for the standard
// library, the module path joined with the directory below the go.mod above it otherwise, and ""
// when neither applies (no go.mod before the repository root).
func importPath(dir string) string {
	if p := os.Getenv(importPathEnv); p != "" {
		return p
	}
	abs, err := filepath.Abs(dir)
	if err != nil {
		return ""
	}
	if rel, err := filepath.Rel(filepath.Join(build.Default.GOROOT, "src"), abs); err == nil && rel != "." && !strings.HasPrefix(rel, "..") {
		return filepath.ToSlash(rel)
	}
	for d := abs; ; d = filepath.Dir(d) {
		if data, err := os.ReadFile(filepath.Join(d, "go.mod")); err == nil {
			mp := modulePath(data)
			if mp == "" {
				return ""
			}
			rel, _ := filepath.Rel(d, abs)
			if rel == "." {
				return mp
			}
			return mp + "/" + filepath.ToSlash(rel)
		}
		if _, err := os.Stat(filepath.Join(d, ".git")); err == nil || filepath.Dir(d) == d {
			return ""
		}
	}
}

func modulePath(gomod []byte) string {
	sc := bufio.NewScanner(bytes.NewReader(gomod))
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) >= 2 && f[0] == "module" {
			return strings.Trim(f[1], `"`)
		}
	}
	return ""
}

// pathOf is the import path a call names pkg by: "" for this package and the universe.
func (f *front) pathOf(pkg *types.Package) string {
	if pkg == nil || pkg == f.pkg {
		return ""
	}
	return pkg.Path()
}

func qualifier(p *types.Package) string { return p.Path() }

// sigOf spells a function type without parameter names, every named type by its full path:
// func([]byte) (int, error).
func sigOf(t types.Type) string {
	s, ok := t.Underlying().(*types.Signature)
	if !ok {
		return types.TypeString(t, qualifier)
	}
	var b strings.Builder
	b.WriteString("func(")
	n := s.Params().Len()
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(", ")
		}
		pt := s.Params().At(i).Type()
		if s.Variadic() && i == n-1 {
			if sl, ok := pt.(*types.Slice); ok {
				b.WriteString("...")
				pt = sl.Elem()
			}
		}
		b.WriteString(types.TypeString(pt, qualifier))
	}
	b.WriteString(")")
	switch r := s.Results(); r.Len() {
	case 0:
	case 1:
		b.WriteString(" " + types.TypeString(r.At(0).Type(), qualifier))
	default:
		b.WriteString(" (")
		for i := 0; i < r.Len(); i++ {
			if i > 0 {
				b.WriteString(", ")
			}
			b.WriteString(types.TypeString(r.At(i).Type(), qualifier))
		}
		b.WriteString(")")
	}
	return b.String()
}

func methodSigs(it *types.Interface) []MethodSig {
	var out []MethodSig
	for i := 0; i < it.NumMethods(); i++ {
		m := it.Method(i)
		out = append(out, MethodSig{Name: m.Name(), Sig: sigOf(m.Type())})
	}
	return out
}

// target is the call of fn as a method or function, without arguments.
func (f *front) target(fn *types.Func, pos string) CallRef {
	fn = fn.Origin()
	sig := fn.Type().(*types.Signature)
	if r := sig.Recv(); r != nil {
		rt := r.Type()
		if p, ok := rt.(*types.Pointer); ok {
			rt = p.Elem()
		}
		rt = types.Unalias(rt)
		if it, ok := rt.Underlying().(*types.Interface); ok {
			return CallRef{K: "iface", Iface: types.TypeString(rt, qualifier), Name: fn.Name(), Methods: methodSigs(it), Pos: pos}
		}
		if n, ok := rt.(*types.Named); ok {
			return CallRef{K: "method", Path: f.pathOf(n.Obj().Pkg()), Recv: n.Obj().Name(), Name: fn.Name(), Pos: pos}
		}
		return CallRef{K: "value", Sig: sigOf(sig), Pos: pos}
	}
	return CallRef{K: "func", Path: f.pathOf(fn.Pkg()), Name: fn.Name(), Pos: pos}
}

// ref is fn used as a value of type t: a function or method value, or a method expression.
func (f *front) ref(fn *types.Func, t types.Type, pos string) CallRef {
	c := f.target(fn, pos)
	sig := sigOf(t)
	switch c.K {
	case "iface":
		// A method value of an interface is a closure that dispatches when called.
		return CallRef{K: "lambda", Sig: sig, Calls: []CallRef{c}, Pos: pos}
	case "method":
		return CallRef{K: "ref", Path: c.Path, Recv: c.Recv, Name: c.Name, Sig: sig, Pos: pos}
	case "func":
		return CallRef{K: "ref", Path: c.Path, Name: c.Name, Sig: sig, Pos: pos}
	}
	return CallRef{K: "lambda", Sig: sig, Calls: []CallRef{c}, Pos: pos}
}

// callsOf lists every call n makes, in source order, a call before its receiver and arguments.
// A function literal is a lambda listing its own calls.
func (f *front) callsOf(n ast.Node) []CallRef {
	out := []CallRef{}
	f.collect(n, &out)
	return onceNew(out)
}

// newOf records, for a module with an import path, that a value of type t is made at pos: each
// named non-interface type declared at package level that t is or holds by value in a field or
// an array element, as a `new` entry. An interface call reaches only the types made.
func (f *front) newOf(t types.Type, pos string, out *[]CallRef) {
	if f.mod.Path == "" || t == nil {
		return
	}
	seen := map[*types.Named]bool{}
	var walk func(t types.Type)
	walk = func(t types.Type) {
		switch u := types.Unalias(t).(type) {
		case *types.Named:
			o := u.Origin()
			if seen[o] {
				return
			}
			seen[o] = true
			obj := o.Obj()
			if _, iface := o.Underlying().(*types.Interface); !iface && obj.Pkg() != nil && obj.Parent() == obj.Pkg().Scope() {
				*out = append(*out, CallRef{K: "new", Path: f.pathOf(obj.Pkg()), Name: obj.Name(), Pos: pos})
			}
			walk(u.Underlying())
		case *types.Struct:
			for i := 0; i < u.NumFields(); i++ {
				walk(u.Field(i).Type())
			}
		case *types.Array:
			walk(u.Elem())
		}
	}
	walk(t)
}

// onceNew keeps the first `new` entry of each type.
func onceNew(cs []CallRef) []CallRef {
	seen := map[string]bool{}
	out := cs[:0]
	for _, c := range cs {
		if c.K == "new" {
			k := c.Path + " " + c.Name
			if seen[k] {
				continue
			}
			seen[k] = true
		}
		out = append(out, c)
	}
	return out
}

func (f *front) collect(n ast.Node, out *[]CallRef) {
	if n == nil {
		return
	}
	ast.Inspect(n, func(c ast.Node) bool {
		// A constant of a named type is a value of it: `const LevelDebug Level = 0`, or an untyped
		// constant passed where a named type is expected.
		if e, ok := c.(ast.Expr); ok {
			if tv, ok := f.info.Types[e]; ok && tv.Value != nil {
				f.newOf(tv.Type, f.pos(e.Pos()), out)
			}
		}
		switch c := c.(type) {
		case *ast.CompositeLit:
			f.newOf(f.info.TypeOf(c), f.pos(c.Pos()), out)
		case *ast.ValueSpec:
			// `var x T`: the zero value of T.
			for _, name := range c.Names {
				if obj := f.info.Defs[name]; obj != nil {
					f.newOf(obj.Type(), f.pos(name.Pos()), out)
				}
			}
		case *ast.FuncLit:
			*out = append(*out, CallRef{K: "lambda", Sig: sigOf(f.info.TypeOf(c)), Calls: f.callsOf(c.Body), Pos: f.pos(c.Pos())})
			return false
		case *ast.CallExpr:
			f.callExpr(c, out)
			return false
		case *ast.SelectorExpr:
			if sel := f.info.Selections[c]; sel != nil {
				if fn, ok := sel.Obj().(*types.Func); ok && sel.Kind() != types.FieldVal {
					*out = append(*out, f.ref(fn, f.info.TypeOf(c), f.pos(c.Pos())))
					f.collect(c.X, out)
					return false
				}
				return true
			}
			if fn, ok := f.info.Uses[c.Sel].(*types.Func); ok {
				*out = append(*out, f.ref(fn, fn.Type(), f.pos(c.Pos())))
				return false
			}
		case *ast.Ident:
			if fn, ok := f.info.Uses[c].(*types.Func); ok {
				*out = append(*out, f.ref(fn, fn.Type(), f.pos(c.Pos())))
			}
		}
		return true
	})
}

func (f *front) callExpr(c *ast.CallExpr, out *[]CallRef) {
	pos := f.pos(c.Pos())
	fun := ast.Unparen(c.Fun)
	args := func() {
		for _, a := range c.Args {
			f.collect(a, out)
		}
	}
	tv := f.info.Types[fun]
	if tv.IsType() {
		// A conversion makes a value of the type converted to.
		f.newOf(f.info.TypeOf(c), pos, out)
		args()
		return
	}
	if tv.IsBuiltin() {
		if id, ok := fun.(*ast.Ident); ok {
			switch t := f.info.TypeOf(c).(type) {
			case *types.Pointer:
				if id.Name == "new" {
					f.newOf(t.Elem(), pos, out)
				}
			case *types.Slice:
				if id.Name == "make" {
					f.newOf(t.Elem(), pos, out)
				}
			}
		}
		args()
		return
	}
	base := fun
	switch ix := fun.(type) {
	case *ast.IndexExpr:
		base = ast.Unparen(ix.X)
	case *ast.IndexListExpr:
		base = ast.Unparen(ix.X)
	}
	var fn *types.Func
	var recv ast.Expr
	switch b := base.(type) {
	case *ast.Ident:
		fn, _ = f.info.Uses[b].(*types.Func)
	case *ast.SelectorExpr:
		if sel := f.info.Selections[b]; sel != nil {
			if sel.Kind() != types.FieldVal {
				fn, _ = sel.Obj().(*types.Func)
				recv = b.X
			}
		} else {
			fn, _ = f.info.Uses[b.Sel].(*types.Func)
		}
	case *ast.FuncLit:
		// Called where it stands: its calls are the holder's.
		f.collect(b.Body, out)
		args()
		return
	}
	if fn != nil {
		cr := f.target(fn, pos)
		if cr.K == "func" {
			for i, a := range c.Args {
				if av := f.info.Types[a]; av.Value != nil && av.Value.Kind() == constant.Int {
					cr.Consts = append(cr.Consts, ConstArg{Index: i, Value: av.Value.ExactString()})
				}
			}
		}
		*out = append(*out, cr)
		if recv != nil {
			f.collect(recv, out)
		}
		args()
		return
	}
	*out = append(*out, CallRef{K: "value", Sig: sigOf(f.info.TypeOf(fun)), Pos: pos})
	f.collect(fun, out)
	args()
}

// opaque records a function declaration whose body cir/1 does not hold. A second declaration
// with the same key (Go allows several `init`) adds nothing: nothing calls it by name.
func (f *front) opaque(d *ast.FuncDecl, reason string) {
	o := Opaque{Name: d.Name.Name, Pos: f.pos(d.Pos()), Reason: reason, NoBody: d.Body == nil, Calls: []CallRef{}}
	if d.Name.Name == "_" {
		return
	}
	if fn, ok := f.info.Defs[d.Name].(*types.Func); ok {
		if r := fn.Type().(*types.Signature).Recv(); r != nil {
			rt := r.Type()
			if p, ok := rt.(*types.Pointer); ok {
				rt = p.Elem()
			}
			if n, ok := types.Unalias(rt).(*types.Named); ok {
				o.Recv = n.Obj().Name()
			}
		}
	}
	key := o.Name
	if o.Recv != "" {
		key = o.Recv + "." + o.Name
	}
	for _, fn := range f.mod.Funcs {
		k := fn.Name
		if fn.Recv != nil {
			k = recvName(fn.Recv.T) + "." + fn.Name
		}
		if k == key {
			return
		}
	}
	for _, x := range f.mod.Opaque {
		k := x.Name
		if x.Recv != "" {
			k = x.Recv + "." + x.Name
		}
		if k == key {
			return
		}
	}
	if d.Body != nil {
		o.Calls = f.callsOf(d.Body)
	}
	f.mod.Opaque = append(f.mod.Opaque, o)
}

// methodSets records the method set of every named type the package declares at its top level,
// interfaces excepted, each method with the call that runs when it is called (a promoted method
// runs the embedded type's).
func (f *front) methodSets() {
	scope := f.pkg.Scope()
	for _, name := range scope.Names() {
		tn, ok := scope.Lookup(name).(*types.TypeName)
		if !ok || tn.IsAlias() {
			continue
		}
		named, ok := tn.Type().(*types.Named)
		if !ok {
			continue
		}
		if _, ok := named.Underlying().(*types.Interface); ok {
			continue
		}
		ms := types.NewMethodSet(types.NewPointer(named))
		if ms.Len() == 0 {
			continue
		}
		set := MethodSet{T: name}
		for i := 0; i < ms.Len(); i++ {
			fn, ok := ms.At(i).Obj().(*types.Func)
			if !ok {
				continue
			}
			set.Methods = append(set.Methods, MethodEntry{Name: fn.Name(), Sig: sigOf(fn.Type()), Target: f.target(fn, f.pos(fn.Pos()))})
		}
		f.mod.Methods = append(f.mod.Methods, set)
	}
}

// linknames records every `//go:linkname local target` naming a function of the package.
func (f *front) linknames(files []*ast.File) {
	seen := map[string]bool{}
	for _, file := range files {
		for _, g := range file.Comments {
			for _, c := range g.List {
				fs := strings.Fields(c.Text)
				if len(fs) != 3 || fs[0] != "//go:linkname" || seen[fs[1]] {
					continue
				}
				if _, ok := f.pkg.Scope().Lookup(fs[1]).(*types.Func); !ok {
					continue
				}
				seen[fs[1]] = true
				f.mod.Linknames = append(f.mod.Linknames, Linkname{Name: fs[1], Target: fs[2]})
			}
		}
	}
}

// inits records every call the package-level variable initializers make and every function they
// use as a value: `var connectFunc = syscall.Connect` makes syscall.Connect a candidate wherever
// a value of its signature is called.
func (f *front) inits(files []*ast.File) {
	for _, file := range files {
		for _, d := range file.Decls {
			g, ok := d.(*ast.GenDecl)
			if !ok || g.Tok != token.VAR {
				continue
			}
			for _, sp := range g.Specs {
				f.collect(sp, &f.mod.Inits)
			}
		}
	}
	f.mod.Inits = onceNew(f.mod.Inits)
}
