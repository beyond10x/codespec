package main

import (
	"go/ast"
	"go/constant"
	"go/token"
	"go/types"
	"strings"
)

// The routes a package registers, as cir/1 `http` entries: net/http's HandleFunc and Handle (the
// package's functions and a *http.ServeMux's methods) and echo's route methods on an *echo.Echo or
// an *echo.Group. An entry names its handler by key and, when another package declares it, that
// package's import path. A registration the reader cannot turn into an entry is a refusal at the
// call, `what` "http route": the reader reads one package and runs no code, so it follows only a
// constant path, a group made once by Group with a constant prefix in the same function, and a
// handler that names a function or a method.

// Entry is one route: the function a framework starts, as cir/1's Entry.
type Entry struct {
	Kind   string
	Func   string
	Method string
	On     string
	Path   string
}

const echoPath = "github.com/labstack/echo/v4"

// echoRoutes maps echo's route methods to the HTTP method each registers; "" is any method. Add
// (a method argument) and Match (a list of them) are read apart.
var echoRoutes = map[string]string{
	"GET": "GET", "POST": "POST", "PUT": "PUT", "DELETE": "DELETE", "PATCH": "PATCH",
	"HEAD": "HEAD", "OPTIONS": "OPTIONS", "CONNECT": "CONNECT", "TRACE": "TRACE", "Any": "",
}

const routeWhat = "http route"

// routes reads the registrations of every function body of the package, in source order. It runs
// after the declarations are read: a handler of this package must be a function of `funcs`.
func (f *front) routes(files []*ast.File) {
	read := map[string]bool{}
	for _, fn := range f.mod.Funcs {
		key := fn.Name
		if fn.Recv != nil {
			key = recvName(fn.Recv.T) + "." + fn.Name
		}
		read[key] = true
	}
	for _, file := range files {
		for _, d := range file.Decls {
			fd, ok := d.(*ast.FuncDecl)
			if !ok || fd.Body == nil {
				continue
			}
			var stack []ast.Node
			ast.Inspect(fd.Body, func(n ast.Node) bool {
				if n == nil {
					stack = stack[:len(stack)-1]
					return true
				}
				if call, ok := n.(*ast.CallExpr); ok {
					f.route(fd, call, inLoop(stack), read)
				}
				stack = append(stack, n)
				return true
			})
		}
	}
}

func inLoop(stack []ast.Node) bool {
	for _, n := range stack {
		switch n.(type) {
		case *ast.ForStmt, *ast.RangeStmt:
			return true
		}
	}
	return false
}

// route records the entry call registers, or refuses it; a call that registers no route is
// passed over.
func (f *front) route(fd *ast.FuncDecl, call *ast.CallExpr, loop bool, read map[string]bool) {
	sel, ok := ast.Unparen(call.Fun).(*ast.SelectorExpr)
	if !ok {
		return
	}
	fn, ok := f.info.Uses[sel.Sel].(*types.Func)
	if !ok || fn.Pkg() == nil {
		return
	}
	recv := fn.Type().(*types.Signature).Recv()
	var echo, handle bool
	switch {
	case fn.Pkg().Path() == "net/http" && (fn.Name() == "HandleFunc" || fn.Name() == "Handle") &&
		(recv == nil || isNamed(recv.Type(), "net/http", "ServeMux")):
		handle = fn.Name() == "Handle"
	case fn.Pkg().Path() == echoPath && recv != nil && isEchoRouter(recv.Type()):
		if _, ok := echoRoutes[fn.Name()]; !ok && fn.Name() != "Add" && fn.Name() != "Match" {
			return
		}
		echo = true
	default:
		return
	}
	// Add and Match take the method or methods first.
	args := call.Args
	if echo && (fn.Name() == "Add" || fn.Name() == "Match") {
		if len(args) < 3 {
			return
		}
		args = args[1:]
	}
	if len(args) < 2 {
		return
	}
	refuse := func(reason string) { f.refuse(call, routeWhat, reason) }
	if loop {
		refuse("registered in a loop: the reader does not run the loop")
		return
	}
	var methods []string
	var prefix string
	if echo {
		p, why := f.groupPrefix(fd, sel.X, map[*types.Var]bool{})
		if why != "" {
			refuse(why)
			return
		}
		prefix = p
		switch fn.Name() {
		case "Add":
			m, ok := f.constString(call.Args[0])
			if !ok {
				refuse("the method is not a constant")
				return
			}
			methods = []string{m}
		case "Match":
			ms, ok := f.constStrings(call.Args[0])
			if !ok {
				refuse("the methods are not a list of constants")
				return
			}
			methods = ms
		default:
			methods = []string{echoRoutes[fn.Name()]}
		}
	}
	pattern, ok := f.constString(args[0])
	if !ok {
		refuse("the path is not a constant")
		return
	}
	path := prefix + pattern
	if !echo {
		m, rest, why := splitPattern(pattern)
		if why != "" {
			refuse(why)
			return
		}
		methods, path = []string{m}, rest
	}
	var key, pkg, why string
	if handle {
		key, pkg, why = f.handlerValue(args[1])
	} else {
		key, pkg, why = f.handlerFunc(args[1])
	}
	if why != "" {
		refuse(why)
		return
	}
	if pkg == "" && !read[key] {
		refuse("the handler " + key + " is not a function the reader read")
		return
	}
	for _, method := range methods {
		f.mod.Entries = append(f.mod.Entries, Entry{Kind: "http", Func: key, Method: method, On: path, Path: pkg})
	}
}

// constStrings is the list a composite literal of string constants holds, `[]string{"GET", …}`.
func (f *front) constStrings(e ast.Expr) ([]string, bool) {
	lit, ok := ast.Unparen(e).(*ast.CompositeLit)
	if !ok {
		return nil, false
	}
	var out []string
	for _, x := range lit.Elts {
		s, ok := f.constString(x)
		if !ok {
			return nil, false
		}
		out = append(out, s)
	}
	return out, true
}

// splitPattern splits a net/http pattern, `[METHOD ][HOST]/[PATH]`, into its method ("" for any)
// and its path; a pattern naming a host is refused, the entry has no place for it.
func splitPattern(p string) (method, path, why string) {
	if i := strings.IndexAny(p, " \t"); i >= 0 {
		method, path = p[:i], strings.TrimLeft(p[i:], " \t")
	} else {
		path = p
	}
	if !strings.HasPrefix(path, "/") {
		return "", "", "the pattern names a host or no path: an entry holds a path only"
	}
	return method, path, ""
}

func (f *front) constString(e ast.Expr) (string, bool) {
	tv, ok := f.info.Types[e]
	if !ok || tv.Value == nil || tv.Value.Kind() != constant.String {
		return "", false
	}
	return constant.StringVal(tv.Value), true
}

// isNamed reports whether t, or what it points to, is the named type path.name.
func isNamed(t types.Type, path, name string) bool {
	if p, ok := t.(*types.Pointer); ok {
		t = p.Elem()
	}
	n, ok := t.(*types.Named)
	return ok && n.Obj().Pkg() != nil && n.Obj().Pkg().Path() == path && n.Obj().Name() == name
}

func isEchoRouter(t types.Type) bool {
	return isNamed(t, echoPath, "Echo") || isNamed(t, echoPath, "Group")
}

// groupPrefix is the prefix routes registered on x get: none on an *echo.Echo; on an *echo.Group
// the prefixes of the Group calls that made it, joined. x is followed through a call of Group or a
// local variable of fd assigned once, by such a call; seen holds the variables being followed.
func (f *front) groupPrefix(fd *ast.FuncDecl, x ast.Expr, seen map[*types.Var]bool) (string, string) {
	x = ast.Unparen(x)
	switch t := f.info.TypeOf(x); {
	case isNamed(t, echoPath, "Echo"):
		return "", ""
	case !isNamed(t, echoPath, "Group"):
		// A struct embedding a group or the router: the reader does not follow what it holds.
		return "", "the route's receiver is not an *echo.Echo or *echo.Group: its prefix is not known"
	}
	switch x := x.(type) {
	case *ast.CallExpr:
		return f.groupCall(fd, x, seen)
	case *ast.Ident:
		v, ok := f.info.Uses[x].(*types.Var)
		if !ok {
			return "", "the group is not a variable the reader can follow: its prefix is not known"
		}
		if v.Parent() == f.pkg.Scope() {
			return "", "the group is a package-level variable: its prefix is not known"
		}
		if isParam(f.info, fd, v) {
			return "", "the group is a parameter of the function: its prefix is not known"
		}
		if seen[v] {
			return "", "the group is made from itself: its prefix is not known"
		}
		seen[v] = true
		values, taken := assignments(f.info, fd, v)
		switch {
		case taken:
			return "", "the group's address is taken: its prefix is not known"
		case len(values) > 1:
			return "", "the group is assigned more than once: its prefix is not known"
		case len(values) == 0 || values[0] == nil:
			return "", "the group is not made by Group in the function: its prefix is not known"
		}
		call, ok := ast.Unparen(values[0]).(*ast.CallExpr)
		if !ok {
			return "", "the group is not made by Group in the function: its prefix is not known"
		}
		return f.groupCall(fd, call, seen)
	}
	return "", "the group is not a variable the reader can follow: its prefix is not known"
}

// groupCall is the prefix of the group call makes: echo's Group on an *echo.Echo or *echo.Group
// with a constant prefix.
func (f *front) groupCall(fd *ast.FuncDecl, call *ast.CallExpr, seen map[*types.Var]bool) (string, string) {
	sel, ok := ast.Unparen(call.Fun).(*ast.SelectorExpr)
	var fn *types.Func
	if ok {
		fn, _ = f.info.Uses[sel.Sel].(*types.Func)
	}
	if fn == nil || fn.Pkg() == nil || fn.Pkg().Path() != echoPath || fn.Name() != "Group" || len(call.Args) == 0 {
		return "", "the group is not made by Group in the function: its prefix is not known"
	}
	if recv := fn.Type().(*types.Signature).Recv(); recv == nil || !isEchoRouter(recv.Type()) {
		return "", "the group is not made by Group in the function: its prefix is not known"
	}
	prefix, ok := f.constString(call.Args[0])
	if !ok {
		return "", "the group's prefix is not a constant"
	}
	outer, why := f.groupPrefix(fd, sel.X, seen)
	if why != "" {
		return "", why
	}
	return outer + prefix, ""
}

// isParam reports whether v is a receiver or parameter of fd or of a function literal in it.
func isParam(info *types.Info, fd *ast.FuncDecl, v *types.Var) bool {
	found := false
	ast.Inspect(fd, func(n ast.Node) bool {
		ft, ok := n.(*ast.FuncType)
		if !found && ok {
			for _, fl := range []*ast.FieldList{ft.Params, ft.Results} {
				if fl == nil {
					continue
				}
				for _, field := range fl.List {
					for _, name := range field.Names {
						if info.Defs[name] == v {
							found = true
						}
					}
				}
			}
		}
		return !found
	})
	if fd.Recv != nil {
		for _, field := range fd.Recv.List {
			for _, name := range field.Names {
				if info.Defs[name] == v {
					return true
				}
			}
		}
	}
	return found
}

// assignments lists every value fd assigns to v (nil where the value is not one expression of
// its own, a call with several results), and whether fd takes v's address.
func assignments(info *types.Info, fd *ast.FuncDecl, v *types.Var) (values []ast.Expr, taken bool) {
	is := func(e ast.Expr) bool {
		id, ok := ast.Unparen(e).(*ast.Ident)
		return ok && (info.Defs[id] == v || info.Uses[id] == v)
	}
	ast.Inspect(fd.Body, func(n ast.Node) bool {
		switch n := n.(type) {
		case *ast.AssignStmt:
			for i, l := range n.Lhs {
				if !is(l) {
					continue
				}
				if len(n.Lhs) == len(n.Rhs) {
					values = append(values, n.Rhs[i])
				} else {
					values = append(values, nil)
				}
			}
		case *ast.ValueSpec:
			for i, name := range n.Names {
				if info.Defs[name] != v || len(n.Values) == 0 {
					continue
				}
				if len(n.Values) == len(n.Names) {
					values = append(values, n.Values[i])
				} else {
					values = append(values, nil)
				}
			}
		case *ast.UnaryExpr:
			if n.Op == token.AND && is(n.X) {
				taken = true
			}
		}
		return true
	})
	return values, taken
}

// handlerValue names the function a Handle registration runs: the ServeHTTP method of the
// value's named type, except for http.HandlerFunc, whose ServeHTTP only calls the function: f of
// a conversion http.HandlerFunc(f). Another named function type with its own ServeHTTP runs that
// ServeHTTP. A mounted router (*http.ServeMux, *echo.Echo) starts no function of its own: the
// routes registered on it are the entries.
func (f *front) handlerValue(e ast.Expr) (key, pkg, why string) {
	e = ast.Unparen(e)
	if call, ok := e.(*ast.CallExpr); ok && len(call.Args) == 1 {
		if tv, ok := f.info.Types[call.Fun]; ok && tv.IsType() && isNamed(tv.Type, "net/http", "HandlerFunc") {
			return f.handlerFunc(call.Args[0])
		}
	}
	t := f.info.TypeOf(e)
	if t == nil {
		return "", "", "the handler has no type"
	}
	if types.IsInterface(t) {
		if isVariable(f.info, e) {
			return "", "", "the handler is held in a variable of an interface type"
		}
		return "", "", "the handler is a value of an interface type: the function it runs is not known"
	}
	if isNamed(t, "net/http", "ServeMux") {
		return "", "", "the handler is a mounted ServeMux: the routes registered on it are the entries"
	}
	if isNamed(t, echoPath, "Echo") {
		return "", "", "the handler is a mounted echo router: the routes registered on it are the entries"
	}
	obj, _, _ := types.LookupFieldOrMethod(t, true, f.pkg, "ServeHTTP")
	fn, ok := obj.(*types.Func)
	if !ok {
		return "", "", "the handler's type has no ServeHTTP method"
	}
	if isNamed(fn.Type().(*types.Signature).Recv().Type(), "net/http", "HandlerFunc") {
		return f.handlerFunc(e)
	}
	return methodKey(f.pkg, fn)
}

// isVariable reports whether e names a variable or a field of one.
func isVariable(info *types.Info, e ast.Expr) bool {
	switch x := e.(type) {
	case *ast.Ident:
		_, ok := info.Uses[x].(*types.Var)
		return ok
	case *ast.SelectorExpr:
		if sel := info.Selections[x]; sel != nil {
			return sel.Kind() == types.FieldVal
		}
		_, ok := info.Uses[x.Sel].(*types.Var)
		return ok
	}
	return false
}

// funcConversion is the operand of a conversion to a named function type (http.HandlerFunc(f),
// echo.HandlerFunc(f)).
func funcConversion(info *types.Info, e ast.Expr) (ast.Expr, bool) {
	call, ok := e.(*ast.CallExpr)
	if !ok || len(call.Args) != 1 {
		return nil, false
	}
	tv, ok := info.Types[call.Fun]
	if !ok || !tv.IsType() {
		return nil, false
	}
	if _, ok := tv.Type.Underlying().(*types.Signature); !ok {
		return nil, false
	}
	return ast.Unparen(call.Args[0]), true
}

// handlerFunc names the function a function value e is: a function, or a method value of a named
// type, by key and, when another package declares it, that package's import path.
func (f *front) handlerFunc(e ast.Expr) (key, pkg, why string) {
	e = ast.Unparen(e)
	if inner, ok := funcConversion(f.info, e); ok {
		e = inner
	}
	var obj types.Object
	switch x := e.(type) {
	case *ast.FuncLit:
		return "", "", "the handler is a function literal: no function key names it"
	case *ast.Ident:
		obj = f.info.Uses[x]
	case *ast.IndexExpr, *ast.IndexListExpr:
		return "", "", "the handler is an instance of a generic function: no function key names it"
	case *ast.SelectorExpr:
		if sel := f.info.Selections[x]; sel != nil {
			switch sel.Kind() {
			case types.MethodVal:
				fn := sel.Obj().(*types.Func)
				if types.IsInterface(sel.Recv()) {
					return "", "", "the handler is a method of an interface: the function it runs is not known"
				}
				return methodKey(f.pkg, fn)
			case types.FieldVal:
				return "", "", "the handler is held in a variable of a function type"
			default:
				return "", "", "the handler is a method expression: no function key names it"
			}
		}
		obj = f.info.Uses[x.Sel]
	default:
		return "", "", "the handler is the value of an expression: no function key names it"
	}
	switch o := obj.(type) {
	case *types.Func:
		if o.Type().(*types.Signature).TypeParams().Len() > 0 {
			return "", "", "the handler is an instance of a generic function: no function key names it"
		}
		return o.Name(), pkgOf(f.pkg, o.Pkg()), ""
	case *types.Var:
		if types.IsInterface(o.Type()) {
			return "", "", "the handler is held in a variable of an interface type"
		}
		return "", "", "the handler is held in a variable of a function type"
	}
	return "", "", "the handler is the value of an expression: no function key names it"
}

// methodKey is the key `T.m` of method fn, T the named type declaring it, and that type's package
// when it is not own.
func methodKey(own *types.Package, fn *types.Func) (key, pkg, why string) {
	t := fn.Type().(*types.Signature).Recv().Type()
	if p, ok := t.(*types.Pointer); ok {
		t = p.Elem()
	}
	n, ok := t.(*types.Named)
	if !ok {
		return "", "", "the handler is a method of an unnamed type: no function key names it"
	}
	if types.IsInterface(n) {
		return "", "", "the handler is a method of an interface: the function it runs is not known"
	}
	return n.Obj().Name() + "." + fn.Name(), pkgOf(own, n.Obj().Pkg()), ""
}

func pkgOf(own, p *types.Package) string {
	if p == nil || p == own {
		return ""
	}
	return p.Path()
}
