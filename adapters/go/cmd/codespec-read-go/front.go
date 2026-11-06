package main

import (
	"fmt"
	"go/ast"
	"go/importer"
	"go/parser"
	"go/printer"
	"go/token"
	"go/types"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// The Go frontend: one package directory to one Module. Every construct it does not translate is
// kept as a `foreign` statement carrying its Go source, and listed as a refusal.

type front struct {
	fset   *token.FileSet
	info   *types.Info
	pkg    *types.Package
	mod    *Module
	conv   map[string]bool // named types the code converts arbitrary values to
	layout []string
}

func load(dir string) (*Module, error) {
	fset := token.NewFileSet()
	pkgs, err := parser.ParseDir(fset, dir, nil, parser.ParseComments)
	if err != nil {
		return nil, err
	}
	if len(pkgs) != 1 {
		return nil, fmt.Errorf("%s: want exactly one package, found %d", dir, len(pkgs))
	}
	var name string
	var files []*ast.File
	for n, p := range pkgs {
		name = n
		names := make([]string, 0, len(p.Files))
		for fn := range p.Files {
			names = append(names, fn)
		}
		sort.Strings(names)
		for _, fn := range names {
			files = append(files, p.Files[fn])
		}
	}
	info := &types.Info{
		Defs: map[*ast.Ident]types.Object{}, Uses: map[*ast.Ident]types.Object{},
		Types: map[ast.Expr]types.TypeAndValue{}, Selections: map[*ast.SelectorExpr]*types.Selection{},
	}
	conf := types.Config{Importer: importer.ForCompiler(fset, "source", nil)}
	pkg, err := conf.Check(name, fset, files, info)
	if err != nil {
		return nil, err
	}
	f := &front{fset: fset, info: info, pkg: pkg, conv: map[string]bool{},
		mod: &Module{Name: name}}
	for _, file := range files {
		if file.Doc != nil {
			f.mod.Doc = strings.TrimSpace(file.Doc.Text())
		}
	}
	f.scanConversions(files)
	for _, file := range files {
		for _, d := range file.Decls {
			f.decl(d)
		}
	}
	f.mod.Layout = f.layout
	return f.mod, nil
}

func (f *front) pos(p token.Pos) string {
	ps := f.fset.Position(p)
	return fmt.Sprintf("%s:%d", filepath.Base(ps.Filename), ps.Line)
}

func (f *front) src(n ast.Node) string {
	var b strings.Builder
	printer.Fprint(&b, f.fset, n)
	return b.String()
}

func (f *front) refuse(n ast.Node, what, reason string) {
	f.mod.Refusals = append(f.mod.Refusals, Refusal{Pos: f.pos(n.Pos()), What: what, Reason: reason})
}

// scanConversions records every named type some code converts a non-constant value to. A string
// type with constants and no such conversion is closed: its values are exactly its constants.
func (f *front) scanConversions(files []*ast.File) {
	for _, file := range files {
		ast.Inspect(file, func(n ast.Node) bool {
			call, ok := n.(*ast.CallExpr)
			if !ok || len(call.Args) != 1 {
				return true
			}
			if tv, ok := f.info.Types[call.Fun]; ok && tv.IsType() {
				if nt, ok := tv.Type.(*types.Named); ok {
					if av := f.info.Types[call.Args[0]]; av.Value == nil {
						f.conv[nt.Obj().Name()] = true
					}
				}
			}
			return true
		})
	}
}

func (f *front) typ(t types.Type) Type {
	switch t := t.(type) {
	case *types.Named:
		if t.Obj().Pkg() == nil && t.Obj().Name() == "error" {
			return T("error")
		}
		if t.Obj().Pkg() == f.pkg {
			return Type{K: "named", Name: t.Obj().Name()}
		}
		return Type{K: "foreign", Name: t.String()}
	case *types.Alias:
		return f.typ(types.Unalias(t))
	case *types.Basic:
		switch {
		case t.Kind() == types.String || t.Kind() == types.UntypedString:
			return T("string")
		case t.Kind() == types.Bool || t.Kind() == types.UntypedBool:
			return T("bool")
		case t.Kind() == types.Int || t.Kind() == types.UntypedInt:
			return Type{K: "int"} // Go int: platform width
		case t.Kind() == types.Int64:
			return Type{K: "int", Bits: 64}
		case t.Kind() == types.Int32:
			return Type{K: "int", Bits: 32}
		case t.Kind() == types.UntypedNil:
			return T("nil")
		}
		return Type{K: "foreign", Name: t.String()}
	case *types.Pointer:
		e := f.typ(t.Elem())
		return Type{K: "ptr", Elem: &e}
	case *types.Slice:
		e := f.typ(t.Elem())
		return Type{K: "slice", Elem: &e}
	case *types.Map:
		k, e := f.typ(t.Key()), f.typ(t.Elem())
		return Type{K: "map", Key: &k, Elem: &e}
	case *types.Chan:
		e := f.typ(t.Elem())
		dir := map[types.ChanDir]string{types.SendRecv: "both", types.SendOnly: "send", types.RecvOnly: "recv"}[t.Dir()]
		return Type{K: "chan", Elem: &e, Dir: dir}
	case *types.Interface:
		if t.Empty() {
			return T("any")
		}
		if types.Identical(t, types.Universe.Lookup("error").Type().Underlying()) {
			return T("error")
		}
	case *types.Tuple:
		tt := Type{K: "tuple"}
		for i := 0; i < t.Len(); i++ {
			tt.Elems = append(tt.Elems, f.typ(t.At(i).Type()))
		}
		return tt
	}
	return Type{K: "foreign", Name: t.String()}
}

func (f *front) typeOf(e ast.Expr) *Type {
	t := f.info.TypeOf(e)
	if t == nil {
		return nil
	}
	tt := f.typ(t)
	return &tt
}

func docText(g *ast.CommentGroup) string {
	if g == nil {
		return ""
	}
	return strings.TrimSpace(g.Text())
}

func (f *front) decl(d ast.Decl) {
	switch d := d.(type) {
	case *ast.GenDecl:
		switch d.Tok {
		case token.TYPE:
			for _, sp := range d.Specs {
				f.typeDecl(sp.(*ast.TypeSpec), d.Doc)
			}
		case token.CONST:
			f.constDecl(d)
		case token.VAR:
			for _, sp := range d.Specs {
				f.varDecl(sp.(*ast.ValueSpec))
			}
		case token.IMPORT:
		}
	case *ast.FuncDecl:
		f.funcDecl(d)
	}
}

func (f *front) typeDecl(ts *ast.TypeSpec, doc *ast.CommentGroup) {
	obj := f.info.Defs[ts.Name].(*types.TypeName)
	td := TypeDecl{Name: ts.Name.Name, Exported: ts.Name.IsExported(), Doc: docText(doc), Pos: f.pos(ts.Pos())}
	if ts.Doc != nil {
		td.Doc = docText(ts.Doc)
	}
	switch u := obj.Type().Underlying().(type) {
	case *types.Struct:
		td.K = "struct"
		for i := 0; i < u.NumFields(); i++ {
			if u.Field(i).Embedded() {
				f.refuse(ts, ts.Name.Name, "struct with an embedded field: field and method promotion are outside cir/1")
				return
			}
			td.Fields = append(td.Fields, Field{Name: u.Field(i).Name(), T: f.typ(u.Field(i).Type())})
		}
	case *types.Basic:
		td.K = "newtype"
		ut := f.typ(u)
		td.Under = &ut
	default:
		f.refuse(ts, ts.Name.Name, "type declaration other than struct or basic newtype")
		return
	}
	f.mod.Types = append(f.mod.Types, td)
	f.layout = append(f.layout, "type:"+td.Name)
}

func (f *front) findType(name string) *TypeDecl {
	for i := range f.mod.Types {
		if f.mod.Types[i].Name == name {
			return &f.mod.Types[i]
		}
	}
	return nil
}

func (f *front) constDecl(d *ast.GenDecl) {
	for _, sp := range d.Specs {
		vs := sp.(*ast.ValueSpec)
		for i, n := range vs.Names {
			c := f.info.Defs[n].(*types.Const)
			val := Expr{K: "lit", Value: c.Val().ExactString(), T: ptrT(f.typ(c.Type().Underlying()))}
			if i < len(vs.Values) {
				if bl, ok := vs.Values[i].(*ast.BasicLit); ok {
					val.Value = bl.Value
				}
			}
			if nt, ok := c.Type().(*types.Named); ok && nt.Obj().Pkg() == f.pkg {
				if td := f.findType(nt.Obj().Name()); td != nil && td.K != "struct" && !f.conv[td.Name] {
					td.K = "enum"
					td.Variants = append(td.Variants, Variant{Name: n.Name, Value: val})
					continue
				}
			}
			f.mod.Consts = append(f.mod.Consts, ConstDecl{Name: n.Name, T: f.typ(c.Type()), Value: val, Pos: f.pos(n.Pos())})
			f.layout = append(f.layout, "const:"+n.Name)
		}
	}
}

func ptrT(t Type) *Type { return &t }

func (f *front) varDecl(vs *ast.ValueSpec) {
	for i, n := range vs.Names {
		if i < len(vs.Values) {
			if call, ok := vs.Values[i].(*ast.CallExpr); ok && f.isPkgFunc(call.Fun, "errors", "New") && len(call.Args) == 1 {
				if bl, ok := call.Args[0].(*ast.BasicLit); ok {
					msg, _ := strconv.Unquote(bl.Value)
					f.mod.Errors = append(f.mod.Errors, ErrorVar{Name: n.Name, Message: msg, Pos: f.pos(n.Pos())})
					f.layout = append(f.layout, "error:"+n.Name)
					continue
				}
			}
		}
		f.refuse(vs, n.Name, "package-level variable other than errors.New(<literal>)")
	}
}

func (f *front) isPkgFunc(e ast.Expr, pkg, name string) bool {
	sel, ok := e.(*ast.SelectorExpr)
	if !ok {
		return false
	}
	id, ok := sel.X.(*ast.Ident)
	if !ok {
		return false
	}
	pn, ok := f.info.Uses[id].(*types.PkgName)
	return ok && pn.Imported().Path() == pkg && sel.Sel.Name == name
}

func (f *front) funcDecl(d *ast.FuncDecl) {
	fn := Func{Name: d.Name.Name, Exported: d.Name.IsExported(), Pos: f.pos(d.Pos()), Doc: docText(d.Doc)}
	sig := f.info.Defs[d.Name].Type().(*types.Signature)
	if d.Recv != nil {
		r := sig.Recv()
		name := r.Name()
		if name == "" {
			name = "_"
		}
		fn.Recv = &Param{Name: name, T: f.typ(r.Type())}
	}
	for i := 0; i < sig.Params().Len(); i++ {
		p := sig.Params().At(i)
		fn.Params = append(fn.Params, Param{Name: p.Name(), T: f.typ(p.Type())})
	}
	for i := 0; i < sig.Results().Len(); i++ {
		fn.Results = append(fn.Results, f.typ(sig.Results().At(i).Type()))
	}
	if hasNamedResults(d.Type) {
		fn.Body = []Stmt{f.namedResultsBody(d)}
	} else {
		fn.Body = f.stmts(d.Body.List, fn.Results)
	}
	f.mod.Funcs = append(f.mod.Funcs, fn)
	key := "func:" + fn.Name
	if fn.Recv != nil {
		key = "func:" + recvName(fn.Recv.T) + "." + fn.Name
	}
	f.layout = append(f.layout, key)
}

func recvName(t Type) string {
	if t.K == "ptr" {
		return t.Elem.Name
	}
	return t.Name
}

func (f *front) foreign(s ast.Stmt, reason string) Stmt {
	f.refuse(s, firstLine(f.src(s)), reason)
	return Stmt{K: "foreign", Lang: "go", Src: f.src(s), Reason: reason, Pos: f.pos(s.Pos())}
}

func hasNamedResults(ft *ast.FuncType) bool {
	if ft.Results == nil {
		return false
	}
	for _, r := range ft.Results.List {
		if len(r.Names) > 0 {
			return true
		}
	}
	return false
}

// namedResultsBody keeps a function with named results as one foreign statement: its body,
// unchanged, inside a closure that declares the same named results and is returned at once, so
// bare returns and writes to the results keep their meaning under an unnamed signature.
func (f *front) namedResultsBody(d *ast.FuncDecl) Stmt {
	reason := "named results are outside cir/1: the body is kept in a closure that declares them"
	sig := f.src(&ast.FuncType{Params: &ast.FieldList{}, Results: d.Type.Results})
	src := "return " + sig + " " + f.src(d.Body) + "()"
	f.refuse(d.Body, d.Name.Name, reason)
	return Stmt{K: "foreign", Lang: "go", Src: src, Reason: reason, Pos: f.pos(d.Body.Pos())}
}

func firstLine(s string) string {
	if i := strings.IndexByte(s, '\n'); i >= 0 {
		return s[:i] + " …"
	}
	return s
}

func (f *front) stmts(list []ast.Stmt, results []Type) []Stmt {
	var out []Stmt
	for _, s := range list {
		out = append(out, f.stmt(s, results))
	}
	return out
}

func (f *front) stmt(s ast.Stmt, results []Type) Stmt {
	pos := f.pos(s.Pos())
	switch s := s.(type) {
	case *ast.AssignStmt:
		lhs, ok := f.exprs(s.Lhs)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		rhs, ok := f.exprs(s.Rhs)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		if s.Tok == token.DEFINE {
			var names []string
			for _, l := range s.Lhs {
				names = append(names, l.(*ast.Ident).Name)
			}
			return Stmt{K: "let", Names: names, Rhs: rhs, Pos: pos}
		}
		op := ""
		if s.Tok != token.ASSIGN {
			op = s.Tok.String()
		}
		for i := range rhs {
			if i < len(s.Lhs) {
				rhs[i] = f.coerce(rhs[i], f.info.TypeOf(s.Lhs[i]))
			}
		}
		return Stmt{K: "assign", Op: op, Lhs: lhs, Rhs: rhs, Pos: pos}
	case *ast.IncDecStmt:
		x, ok := f.expr(s.X)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		k := "incr"
		if s.Tok == token.DEC {
			k = "decr"
		}
		return Stmt{K: k, X: &x, Pos: pos}
	case *ast.DeclStmt:
		gd, ok := s.Decl.(*ast.GenDecl)
		if ok && gd.Tok == token.VAR && len(gd.Specs) == 1 {
			vs := gd.Specs[0].(*ast.ValueSpec)
			if len(vs.Names) == 1 && len(vs.Values) == 0 {
				t := f.typ(f.info.TypeOf(vs.Names[0]))
				return Stmt{K: "var", Names: []string{vs.Names[0].Name}, T: &t, Pos: pos}
			}
		}
		return f.foreign(s, "declaration other than `var x T`")
	case *ast.IfStmt:
		if s.Init != nil {
			return f.foreign(s, "if with an init statement")
		}
		c, ok := f.expr(s.Cond)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		st := Stmt{K: "if", Cond: &c, Then: f.stmts(s.Body.List, results), Pos: pos}
		switch e := s.Else.(type) {
		case *ast.BlockStmt:
			st.Else = f.stmts(e.List, results)
		case *ast.IfStmt:
			st.Else = []Stmt{f.stmt(e, results)}
		}
		return st
	case *ast.RangeStmt:
		over, ok := f.expr(s.X)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		key, val := "_", "_"
		if s.Key != nil {
			key = s.Key.(*ast.Ident).Name
		}
		if s.Value != nil {
			val = s.Value.(*ast.Ident).Name
		}
		return Stmt{K: "for_range", Key: key, Val: val, Over: &over, Body: f.stmts(s.Body.List, results), Pos: pos}
	case *ast.ReturnStmt:
		rs, ok := f.exprs(s.Results)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		for i := range rs {
			if i < len(results) && rs[i].K == "nil" {
				rt := results[i]
				rs[i].T = &rt
			}
			if i < len(results) && results[i].K == "error" {
				rs[i] = f.coerce(rs[i], types.Universe.Lookup("error").Type())
			}
		}
		return Stmt{K: "return", Results: rs, Pos: pos}
	case *ast.ExprStmt:
		x, ok := f.expr(s.X)
		if !ok {
			return f.foreign(s, "expression outside cir/1")
		}
		return Stmt{K: "expr", X: &x, Pos: pos}
	case *ast.SendStmt:
		ch, ok1 := f.expr(s.Chan)
		x, ok2 := f.expr(s.Value)
		if !ok1 || !ok2 {
			return f.foreign(s, "expression outside cir/1")
		}
		return Stmt{K: "send", Chan: &ch, X: &x, Pos: pos}
	case *ast.GoStmt:
		return f.foreign(s, "goroutine: concurrency is outside cir/1")
	case *ast.DeferStmt:
		return f.foreign(s, "defer is outside cir/1")
	}
	return f.foreign(s, fmt.Sprintf("statement %T is outside cir/1", s))
}

// coerce wraps a value of an error struct type that is used where an `error` is expected.
func (f *front) coerce(e Expr, want types.Type) Expr {
	if want == nil {
		return e
	}
	if _, ok := want.Underlying().(*types.Interface); !ok || e.T == nil || e.T.K != "named" {
		return e
	}
	return Expr{K: "to_error", X: &e, T: ptrT(T("error"))}
}

func (f *front) exprs(es []ast.Expr) ([]Expr, bool) {
	var out []Expr
	for _, e := range es {
		x, ok := f.expr(e)
		if !ok {
			return nil, false
		}
		out = append(out, x)
	}
	return out, true
}

func (f *front) expr(e ast.Expr) (Expr, bool) {
	t := f.typeOf(e)
	switch e := e.(type) {
	case *ast.ParenExpr:
		return f.expr(e.X)
	case *ast.BasicLit:
		return Expr{K: "lit", Value: e.Value, T: t}, true
	case *ast.Ident:
		switch o := f.info.Uses[e].(type) {
		case *types.Nil:
			return Expr{K: "nil", T: t}, true
		case *types.Const:
			if o.Pkg() == nil {
				return Expr{K: "lit", Value: e.Name, T: t}, true
			}
			return Expr{K: "const", Name: e.Name, T: t}, true
		case *types.Var:
			if o.Parent() == f.pkg.Scope() {
				return Expr{K: "errvar", Name: e.Name, T: t}, true
			}
			return Expr{K: "var", Name: e.Name, T: t}, true
		}
		if f.info.Defs[e] != nil {
			return Expr{K: "var", Name: e.Name, T: t}, true
		}
		return Expr{}, false
	case *ast.SelectorExpr:
		x, ok := f.expr(e.X)
		if !ok {
			return Expr{}, false
		}
		return Expr{K: "field", X: &x, Name: e.Sel.Name, T: t}, true
	case *ast.IndexExpr:
		x, ok1 := f.expr(e.X)
		y, ok2 := f.expr(e.Index)
		if !ok1 || !ok2 {
			return Expr{}, false
		}
		return Expr{K: "index", X: &x, Y: &y, T: t}, true
	case *ast.StarExpr:
		x, ok := f.expr(e.X)
		if !ok {
			return Expr{}, false
		}
		return Expr{K: "unop", Op: "*", X: &x, T: t}, true
	case *ast.UnaryExpr:
		x, ok := f.expr(e.X)
		if !ok {
			return Expr{}, false
		}
		return Expr{K: "unop", Op: e.Op.String(), X: &x, T: t}, true
	case *ast.BinaryExpr:
		x, ok1 := f.expr(e.X)
		y, ok2 := f.expr(e.Y)
		if !ok1 || !ok2 {
			return Expr{}, false
		}
		return Expr{K: "binop", Op: e.Op.String(), X: &x, Y: &y, T: t}, true
	case *ast.CompositeLit:
		if t != nil && (t.K == "map" || t.K == "slice") && len(e.Elts) == 0 {
			return Expr{K: "make", T: t}, true
		}
		if t == nil || t.K != "named" {
			return Expr{}, false
		}
		out := Expr{K: "struct", Name: t.Name, T: t}
		for _, el := range e.Elts {
			kv, ok := el.(*ast.KeyValueExpr)
			if !ok {
				return Expr{}, false
			}
			v, ok := f.expr(kv.Value)
			if !ok {
				return Expr{}, false
			}
			out.FieldNames = append(out.FieldNames, kv.Key.(*ast.Ident).Name)
			out.Args = append(out.Args, v)
		}
		return out, true
	case *ast.CallExpr:
		return f.call(e, t)
	}
	return Expr{}, false
}

func (f *front) call(e *ast.CallExpr, t *Type) (Expr, bool) {
	if e.Ellipsis.IsValid() {
		return Expr{}, false // a spread call: cir/1 has no spread argument
	}
	args, ok := f.exprs(e.Args)
	if !ok {
		return Expr{}, false
	}
	if tv, ok := f.info.Types[e.Fun]; ok && tv.IsType() {
		if len(args) != 1 || t == nil || t.K != "named" {
			return Expr{}, false
		}
		return Expr{K: "conv", Name: t.Name, X: &args[0], T: t}, true
	}
	for _, in := range []struct{ pkg, name, as string }{
		{"fmt", "Sprintf", "sprintf"}, {"fmt", "Errorf", "errorf"}, {"fmt", "Println", "println"},
	} {
		if f.isPkgFunc(e.Fun, in.pkg, in.name) {
			return Expr{K: "intrinsic", Name: in.as, Args: args, T: t}, true
		}
	}
	switch fun := e.Fun.(type) {
	case *ast.Ident:
		switch f.info.Uses[fun].(type) {
		case *types.Builtin:
			if fun.Name == "append" || fun.Name == "len" {
				return Expr{K: "intrinsic", Name: fun.Name, Args: args, T: t}, true
			}
			return Expr{}, false
		case *types.Func:
			return Expr{K: "call", Name: fun.Name, Args: args, T: t}, true
		}
	case *ast.SelectorExpr:
		if sel := f.info.Selections[fun]; sel != nil && sel.Kind() == types.MethodVal {
			x, ok := f.expr(fun.X)
			if !ok {
				return Expr{}, false
			}
			return Expr{K: "method", X: &x, Name: fun.Sel.Name, Args: args, T: t}, true
		}
	}
	return Expr{}, false
}
