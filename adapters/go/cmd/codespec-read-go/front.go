package main

import (
	"fmt"
	"go/ast"
	"go/build"
	"go/importer"
	"go/parser"
	"go/printer"
	"go/token"
	"go/types"
	"io/fs"
	"path/filepath"
	"reflect"
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
	// breaks holds the enclosing statements an unlabeled `break` would leave, innermost last:
	// "loop" or "switch". cir/1 `break` leaves a loop only.
	breaks []string
	// loops holds the label of each enclosing loop, innermost last; "" for an unlabeled loop.
	loops []string
	// label is the label of the loop statement being read, consumed when its body is entered.
	label string
	// unlabeled holds every labeled branch read as an unlabeled cir/1 break or continue.
	unlabeled map[*ast.BranchStmt]bool
	// fnBody is the body of the function being read: the scope of its labels.
	fnBody *ast.BlockStmt
	// hoist holds the `if` statements of the function or literal body being read whose init is
	// read in front of them (planHoists).
	hoist hoists
	// outside holds the named types of the package that have no cir/1 declaration, with the reason.
	outside map[string]string
	// globals holds the package-level variables read as module vars; sentinels those read as sentinel
	// errors. A package-level variable in neither is refused, and so is every expression naming it.
	globals, sentinels map[*types.Var]bool
	// sig is the signature of the function or literal being read: the types its returns convert to.
	sig *types.Signature
	// why is the shape of the first sub-expression of the statement being read that has no cir/1
	// form, in reading order (no); "" while every expression read so far has one.
	why string
	// spelled maps every identifier spelling of the package-level declaration being read to the
	// objects it names there (nil for none); fresh holds the names freshName gave in it; rename
	// holds the local variables read under a fresh name (name).
	spelled map[string]map[types.Object]bool
	fresh   map[string]bool
	rename  map[types.Object]string
	// gotoBody is the body gotoHas says holds a goto or not (canHoist).
	gotoBody *ast.BlockStmt
	gotoHas  bool
	// named holds the named results of the function or literal being read, as the locals a bare
	// return returns (resultVars); nil when it has none.
	named []Expr
}

// beginDecl starts reading the package-level declaration n: no fresh name given, none renamed.
func (f *front) beginDecl(n ast.Node) {
	f.spelled, f.fresh, f.rename = map[string]map[types.Object]bool{}, map[string]bool{}, map[types.Object]string{}
	ast.Inspect(n, func(c ast.Node) bool {
		if id, ok := c.(*ast.Ident); ok {
			o := f.info.Defs[id]
			if o == nil {
				o = f.info.Uses[id]
			}
			if f.spelled[id.Name] == nil {
				f.spelled[id.Name] = map[types.Object]bool{}
			}
			f.spelled[id.Name][o] = true
		}
		return true
	})
}

// freshName is `<base>_<n>` for the smallest n ≥ 1 that is spelled nowhere in the declaration
// being read and not given before in it: a name of a local the reader introduces, which can
// neither shadow nor be shadowed by a name the source uses.
func (f *front) freshName(base string) string {
	for n := 1; ; n++ {
		c := base + "_" + strconv.Itoa(n)
		if len(f.spelled[c]) == 0 && !f.fresh[c] {
			f.fresh[c] = true
			return c
		}
	}
}

// collides reports whether the name of the local obj is spelled anywhere in the declaration being
// read for something other than obj.
func (f *front) collides(name string, obj types.Object) bool {
	for o := range f.spelled[name] {
		if o != obj {
			return true
		}
	}
	return false
}

// name is the cir/1 name of the identifier id: the fresh name its variable is read under, or its
// own.
func (f *front) name(id *ast.Ident) string {
	o := f.info.Defs[id]
	if o == nil {
		o = f.info.Uses[id]
	}
	if r, ok := f.rename[o]; ok && o != nil {
		return r
	}
	return id.Name
}

// whole reads the statements the Go statement s lowers to; when any of them is outside cir/1,
// s is kept foreign whole for that one's reason, and what reading the others declared or refused
// is taken back.
func (f *front) whole(s ast.Stmt, read func() []Stmt) []Stmt {
	refused, externs, ifaces := len(f.mod.Refusals), len(f.mod.Externs), len(f.mod.Interfaces)
	outer := f.why
	f.why = ""
	defer func() { f.why = outer }()
	parts := read()
	for _, p := range parts {
		reason := ""
		if p.K == "foreign" {
			reason = p.Reason
		} else if n := stmtForeignType(p); n != "" {
			reason = "type outside cir/1: " + n
		}
		if reason != "" {
			f.mod.Refusals = f.mod.Refusals[:refused]
			f.mod.Externs = f.mod.Externs[:externs]
			f.mod.Interfaces = f.mod.Interfaces[:ifaces]
			return []Stmt{f.foreign(s, reason)}
		}
	}
	return parts
}

// constLit is the constant expression e as the literal of the value go/types computed for it.
func (f *front) constLit(e ast.Expr, t *Type) (Expr, bool) {
	val := f.info.Types[e].Value
	if val == nil {
		return f.no("constant without a value")
	}
	if bits := floatBits(f.info.TypeOf(e)); bits != 0 {
		v, ok := floatSpelling(val, bits)
		if !ok {
			return f.no("constant that is not a finite float")
		}
		return Expr{K: "float", Value: v, T: t}, true
	}
	return Expr{K: "lit", Value: val.ExactString(), T: t}, true
}

// declStmt reads a declaration inside a function. A `var` spec without values is one `var` per
// name; one with values a `let` of its names, which is what `:=` reads as, each value converted to
// the declared type as an assignment converts it and kept only when it then has that type. A
// `const` is no statement: each use reads as the literal of its value (expr). A `type` is outside
// cir/1.
func (f *front) declStmt(s *ast.DeclStmt, results []Type) []Stmt {
	gd, ok := s.Decl.(*ast.GenDecl)
	if !ok {
		return []Stmt{f.stmt(s, results)}
	}
	switch gd.Tok {
	case token.CONST:
		return nil
	case token.TYPE:
		return []Stmt{f.foreign(s, "type declaration inside a function: outside cir/1")}
	}
	pos := f.pos(s.Pos())
	return f.whole(s, func() []Stmt {
		var out []Stmt
		for _, sp := range gd.Specs {
			vs := sp.(*ast.ValueSpec)
			names := make([]string, len(vs.Names))
			for i, n := range vs.Names {
				names[i] = f.name(n)
			}
			if len(vs.Values) == 0 {
				for i, n := range vs.Names {
					t := f.typ(f.info.TypeOf(n))
					out = append(out, Stmt{K: "var", Names: []string{names[i]}, T: &t, Pos: pos})
				}
				continue
			}
			rhs, ok := f.exprs(vs.Values)
			if !ok {
				return []Stmt{f.foreign(s, f.exprReason())}
			}
			if vs.Type != nil {
				other := func() []Stmt {
					return []Stmt{f.foreign(s, "declaration whose value has another type than the declared one")}
				}
				if len(rhs) == len(vs.Names) {
					for i := range rhs {
						want := f.info.TypeOf(vs.Names[i])
						if rhs[i].K == "nil" && rhs[i].T != nil && rhs[i].T.K == "nil" {
							rhs[i].T = ptrT(f.typ(want))
						}
						rhs[i] = f.coerce(rhs[i], want)
						if rhs[i].T == nil || !reflect.DeepEqual(*rhs[i].T, f.typ(want)) {
							return other()
						}
					}
				} else {
					tup, _ := f.info.TypeOf(vs.Values[0]).(*types.Tuple)
					for i, n := range vs.Names {
						if tup == nil || i >= tup.Len() || !types.Identical(tup.At(i).Type(), f.info.TypeOf(n)) {
							return other()
						}
					}
				}
			}
			out = append(out, Stmt{K: "let", Names: names, Rhs: rhs, Pos: pos})
		}
		return out
	})
}

func isBlank(e ast.Expr) bool {
	id, ok := ast.Unparen(e).(*ast.Ident)
	return ok && id.Name == "_"
}

// blankAssign reads an assignment with a blank target: `_ = e` as `let _ = e`, and `a, _ = f()`
// as `let a_1, _ = f()` followed by `a = a_1`, each named target from its fresh name. Every named
// target must be an identifier: the operands of an index or field target would be evaluated after
// the right side. It reports false for any other statement.
func (f *front) blankAssign(s ast.Stmt, results []Type) ([]Stmt, bool) {
	as, ok := s.(*ast.AssignStmt)
	if !ok || as.Tok != token.ASSIGN {
		return nil, false
	}
	blanks := 0
	for _, l := range as.Lhs {
		if isBlank(l) {
			blanks++
		} else if _, ok := ast.Unparen(l).(*ast.Ident); !ok {
			return nil, false
		}
	}
	if blanks == 0 {
		return nil, false
	}
	pos := f.pos(as.Pos())
	return f.whole(as, func() []Stmt {
		rhs, ok := f.exprs(as.Rhs)
		if !ok {
			return []Stmt{f.foreign(as, f.exprReason())}
		}
		names := make([]string, len(as.Lhs))
		var assigns []Stmt
		for i, l := range as.Lhs {
			if isBlank(l) {
				names[i] = "_"
				continue
			}
			id := ast.Unparen(l).(*ast.Ident)
			target, ok := f.expr(id)
			if !ok {
				return []Stmt{f.foreign(as, f.exprReason())}
			}
			names[i] = f.freshName(id.Name)
			var vt types.Type
			if len(as.Rhs) == len(as.Lhs) {
				vt = f.info.TypeOf(as.Rhs[i])
				if rhs[i].K == "nil" && rhs[i].T != nil && rhs[i].T.K == "nil" {
					rhs[i].T, vt = target.T, f.info.TypeOf(l)
				}
			} else if tup, ok := f.info.TypeOf(as.Rhs[0]).(*types.Tuple); ok && i < tup.Len() {
				vt = tup.At(i).Type()
			}
			if vt == nil {
				return []Stmt{f.foreign(as, "assignment with a blank target outside cir/1")}
			}
			v := Expr{K: "var", Name: names[i], T: ptrT(f.typ(vt))}
			assigns = append(assigns, Stmt{K: "assign", Lhs: []Expr{target}, Rhs: []Expr{f.coerce(v, f.info.TypeOf(l))}, Pos: pos})
		}
		return append([]Stmt{{K: "let", Names: names, Rhs: rhs, Pos: pos}}, assigns...)
	}), true
}

// no fails the read of an expression whose shape has no cir/1 form, and keeps shape as the reason
// unless an earlier sub-expression of the same statement already failed.
func (f *front) no(shape string) (Expr, bool) {
	if f.why == "" {
		f.why = shape
	}
	return Expr{}, false
}

// exprReason is the reason of a statement or initializer kept foreign for an expression: the shape
// no recorded, once.
func (f *front) exprReason() string {
	why := f.why
	f.why = ""
	if why == "" {
		return "expression outside cir/1"
	}
	return "expression outside cir/1: " + why
}

// goType spells t as the reader's messages do, a type of another package by its package name.
func (f *front) goType(t types.Type) string {
	return types.TypeString(t, func(p *types.Package) string {
		if p == f.pkg {
			return ""
		}
		return p.Name()
	})
}

func load(dir string) (*Module, error) {
	fset := token.NewFileSet()
	// The files the toolchain would build for this platform: no test files, build constraints
	// applied. A directory with no buildable file falls back to every non-test file.
	keep := func(fi fs.FileInfo) bool { return !strings.HasSuffix(fi.Name(), "_test.go") }
	// cgo off: a file needing cgo is not Go source cir/1 can read, and with cgo off the standard
	// library builds its pure-Go paths (net's own resolver).
	ctx := build.Default
	ctx.CgoEnabled = false
	if bp, err := ctx.ImportDir(dir, 0); err == nil && len(bp.GoFiles) > 0 {
		chosen := map[string]bool{}
		for _, f := range bp.GoFiles {
			chosen[f] = true
		}
		keep = func(fi fs.FileInfo) bool { return chosen[fi.Name()] }
	}
	pkgs, err := parser.ParseDir(fset, dir, keep, parser.ParseComments)
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
		Instances: map[*ast.Ident]types.Instance{},
	}
	ensureGOROOT()
	conf := types.Config{Importer: importer.ForCompiler(fset, "source", nil)}
	path := importPath(dir)
	checkAs := name
	if path != "" {
		checkAs = path
	}
	pkg, err := conf.Check(checkAs, fset, files, info)
	if err != nil {
		return nil, err
	}
	f := &front{fset: fset, info: info, pkg: pkg, conv: map[string]bool{}, unlabeled: map[*ast.BranchStmt]bool{},
		mod: &Module{Name: name, Path: path}}
	for _, file := range files {
		if file.Doc != nil {
			f.mod.Doc = strings.TrimSpace(file.Doc.Text())
		}
	}
	f.scanConversions(files)
	f.scanOutside(files)
	f.scanVars(files)
	for _, file := range files {
		for _, d := range file.Decls {
			f.decl(d)
		}
	}
	// Method sets and linknames serve a set of modules linked by import path; a module without one
	// is never in such a set.
	if path != "" {
		f.methodSets()
		f.linknames(files)
		f.inits(files)
	}
	f.routes(files)
	f.mod.Fset = fset
	for _, file := range files {
		f.mod.Sources = append(f.mod.Sources, Source{Path: fset.File(file.FileStart).Name(), Generated: ast.IsGenerated(file)})
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

// declaredInFunc reports whether the type name obj is declared inside a function rather than at
// the top level of its package.
func declaredInFunc(obj *types.TypeName) bool {
	return obj.Pkg() != nil && obj.Parent() != nil && obj.Parent() != obj.Pkg().Scope()
}

func (f *front) typ(t types.Type) Type {
	switch t := t.(type) {
	case *types.Named:
		if t.Obj().Pkg() == nil && t.Obj().Name() == "error" {
			return T("error")
		}
		if declaredInFunc(t.Obj()) {
			// A type declared inside a function has no cir/1 declaration to name.
			return Type{K: "foreign", Name: t.Obj().Name() + " (declared inside a function)"}
		}
		// An instantiated generic type carries its type arguments.
		targs, ok := f.typeArgs(t.TypeArgs())
		if t.Obj().Pkg() == f.pkg {
			if _, out := f.outside[t.Obj().Name()]; out {
				return Type{K: "foreign", Name: t.Obj().Name()}
			}
			if !ok {
				return Type{K: "foreign", Name: t.String()}
			}
			return Type{K: "named", Name: t.Obj().Name(), Targs: targs}
		}
		if t.Obj().Pkg() != nil && ok {
			return Type{K: "extern", Path: t.Obj().Pkg().Path(), Name: t.Obj().Name(), Targs: targs}
		}
		return Type{K: "foreign", Name: t.String()}
	case *types.TypeParam:
		return Type{K: "param", Name: t.Obj().Name()}
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
		case t.Kind() == types.Int16:
			return Type{K: "int", Bits: 16}
		case t.Kind() == types.Int8:
			return Type{K: "int", Bits: 8}
		case t.Kind() == types.Uint:
			return Type{K: "int", Unsigned: true}
		case t.Kind() == types.Uint64:
			return Type{K: "int", Bits: 64, Unsigned: true}
		case t.Kind() == types.Uint32:
			return Type{K: "int", Bits: 32, Unsigned: true}
		case t.Kind() == types.Uint16:
			return Type{K: "int", Bits: 16, Unsigned: true}
		case t.Kind() == types.Uint8:
			return Type{K: "int", Bits: 8, Unsigned: true}
		case t.Kind() == types.Float64 || t.Kind() == types.UntypedFloat:
			return Type{K: "float", Bits: 64}
		case t.Kind() == types.Float32:
			return Type{K: "float", Bits: 32}
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
	case *types.Array:
		e := f.typ(t.Elem())
		return Type{K: "array", Len: t.Len(), Elem: &e}
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
	case *types.Struct:
		// struct{} holds no value but itself: cir/1 unit.
		if t.NumFields() == 0 {
			return T("unit")
		}
	case *types.Tuple:
		tt := Type{K: "tuple"}
		for i := 0; i < t.Len(); i++ {
			tt.Elems = append(tt.Elems, f.typ(t.At(i).Type()))
		}
		return tt
	case *types.Signature:
		// A function value: no receiver, type parameters or variadic last parameter.
		if t.Recv() != nil || t.TypeParams().Len() > 0 || t.Variadic() {
			break
		}
		ft := Type{K: "func", Params: []Type{}, Results: []Type{}}
		for i := 0; i < t.Params().Len(); i++ {
			ft.Params = append(ft.Params, f.typ(t.Params().At(i).Type()))
		}
		for i := 0; i < t.Results().Len(); i++ {
			ft.Results = append(ft.Results, f.typ(t.Results().At(i).Type()))
		}
		return ft
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
	if ts.Assign.IsValid() {
		// A type alias declares no type: every use of it reads as the type it names (typ). It is
		// omitted without a refusal (README.md § Omitted).
		return
	}
	if why, out := f.outside[ts.Name.Name]; out {
		f.refuse(ts, ts.Name.Name, why)
		return
	}
	obj := f.info.Defs[ts.Name].(*types.TypeName)
	td := TypeDecl{Name: ts.Name.Name, Exported: ts.Name.IsExported(), Doc: docText(doc), Pos: f.pos(ts.Pos())}
	if n, ok := obj.Type().(*types.Named); ok {
		td.Tparams, _ = f.tparams(n.TypeParams())
	}
	if ts.Doc != nil {
		td.Doc = docText(ts.Doc)
	}
	switch u := obj.Type().Underlying().(type) {
	case *types.Struct:
		td.K = "struct"
		for i := 0; i < u.NumFields(); i++ {
			td.Fields = append(td.Fields, Field{Name: u.Field(i).Name(), T: f.typ(u.Field(i).Type()), Embedded: u.Field(i).Embedded(), NameTok: fieldTok(ts, u.Field(i))})
		}
	case *types.Basic, *types.Signature, *types.Map, *types.Slice:
		td.K = "newtype"
		ut := f.typ(u)
		td.Under = &ut
	case *types.Interface:
		f.ifaceDecl(ts, u, td.Doc)
		return
	default:
		f.refuse(ts, ts.Name.Name, "type declaration other than struct or newtype over a basic, func, map or slice type")
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
			// The literal is typed by the underlying type: a constant of an extern type over uintptr
			// has no cir/1 literal.
			if hasForeign(f.typ(c.Type())) || hasForeign(f.typ(c.Type().Underlying())) {
				f.refuse(vs, n.Name, fmt.Sprintf("constant of type %s has no cir/1 type", c.Type()))
				continue
			}
			val := Expr{K: "lit", Value: c.Val().ExactString(), T: ptrT(f.typ(c.Type().Underlying()))}
			if bits := floatBits(c.Type()); bits != 0 {
				v, ok := floatSpelling(c.Val(), bits)
				if !ok {
					f.refuse(vs, n.Name, fmt.Sprintf("constant %s is not a finite %s", c.Val().ExactString(), c.Type()))
					continue
				}
				val = Expr{K: "float", Value: v, T: val.T}
			} else if i < len(vs.Values) {
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

// sentinelMessage is the message of `errors.New(<literal>)`, the initializer of a sentinel error.
func (f *front) sentinelMessage(e ast.Expr) (string, bool) {
	call, ok := e.(*ast.CallExpr)
	if !ok || !f.isPkgFunc(call.Fun, "errors", "New") || len(call.Args) != 1 {
		return "", false
	}
	bl, ok := call.Args[0].(*ast.BasicLit)
	if !ok {
		return "", false
	}
	msg, _ := strconv.Unquote(bl.Value)
	return msg, true
}

// varOutside is why the package-level variable n of vs is refused, or "": a variable of a type
// outside cir/1, the blank variable, and one of several set from one multi-value expression.
func (f *front) varOutside(vs *ast.ValueSpec, n *ast.Ident) string {
	v := f.info.Defs[n].(*types.Var)
	if t := f.typ(v.Type()); hasForeign(t) {
		return fmt.Sprintf("variable of type %s has no cir/1 type", v.Type())
	}
	if n.Name == "_" {
		return "blank variable: cir/1 declares named variables only"
	}
	if len(vs.Values) > 0 && len(vs.Values) != len(vs.Names) {
		return "variables set together from one multi-value expression"
	}
	return ""
}

// scanVars sorts every package-level variable, before any body is read, into a sentinel error
// (`errors.New(<literal>)`), a module var, or neither (refused by varDecl).
func (f *front) scanVars(files []*ast.File) {
	f.globals, f.sentinels = map[*types.Var]bool{}, map[*types.Var]bool{}
	for _, file := range files {
		for _, d := range file.Decls {
			gd, ok := d.(*ast.GenDecl)
			if !ok || gd.Tok != token.VAR {
				continue
			}
			for _, sp := range gd.Specs {
				vs := sp.(*ast.ValueSpec)
				for i, n := range vs.Names {
					v := f.info.Defs[n].(*types.Var)
					if len(vs.Values) == len(vs.Names) {
						if _, ok := f.sentinelMessage(vs.Values[i]); ok {
							f.sentinels[v] = true
							continue
						}
					}
					if f.varOutside(vs, n) == "" {
						f.globals[v] = true
					}
				}
			}
		}
	}
}

// compileTimeOnly reports whether the i-th variable of the package-level vs is the blank variable
// with no initializer or one that has no run-time effect: `var _ I = (*T)(nil)`, `var _ I = T{}`,
// an assertion the compiler checks. It declares nothing a body can name and does nothing when
// the package is initialized, so it is omitted without a refusal (README.md § Omitted).
func (f *front) compileTimeOnly(vs *ast.ValueSpec, i int) bool {
	if vs.Names[i].Name != "_" {
		return false
	}
	if len(vs.Values) == 0 {
		return true
	}
	return len(vs.Values) == len(vs.Names) && f.effectFree(vs.Values[i])
}

// effectFree reports whether evaluating e can have no effect and cannot fail: a constant, nil, a
// function literal, a conversion of such a value, and a composite literal of such values or its
// address. Any call, receive, index or field read is not.
func (f *front) effectFree(e ast.Expr) bool {
	if tv, ok := f.info.Types[e]; ok && tv.Value != nil {
		return true
	}
	switch e := ast.Unparen(e).(type) {
	case *ast.Ident:
		_, isNil := f.info.Uses[e].(*types.Nil)
		return isNil
	case *ast.BasicLit, *ast.FuncLit:
		return true
	case *ast.UnaryExpr:
		return e.Op == token.AND && f.effectFree(e.X)
	case *ast.CompositeLit:
		for _, el := range e.Elts {
			if kv, ok := el.(*ast.KeyValueExpr); ok {
				if id, ok := kv.Key.(*ast.Ident); !ok || !isField(f.info.Uses[id]) {
					if !f.effectFree(kv.Key) {
						return false
					}
				}
				el = kv.Value
			}
			if !f.effectFree(el) {
				return false
			}
		}
		return true
	case *ast.CallExpr:
		if tv, ok := f.info.Types[e.Fun]; ok && tv.IsType() && len(e.Args) == 1 {
			return f.effectFree(e.Args[0])
		}
	}
	return false
}

func isField(o types.Object) bool {
	v, ok := o.(*types.Var)
	return ok && v.IsField()
}

func (f *front) varDecl(vs *ast.ValueSpec) {
	f.beginDecl(vs)
	for i, n := range vs.Names {
		v := f.info.Defs[n].(*types.Var)
		if f.compileTimeOnly(vs, i) {
			continue
		}
		if f.sentinels[v] {
			msg, _ := f.sentinelMessage(vs.Values[i])
			f.mod.Errors = append(f.mod.Errors, ErrorVar{Name: n.Name, Message: msg, Pos: f.pos(n.Pos())})
			f.layout = append(f.layout, "error:"+n.Name)
			continue
		}
		if !f.globals[v] {
			f.refuse(vs, n.Name, f.varOutside(vs, n))
			continue
		}
		d := VarDecl{Name: n.Name, T: f.typ(v.Type()), Pos: f.pos(n.Pos())}
		if i < len(vs.Values) {
			f.why = ""
			x, ok := f.expr(vs.Values[i])
			if ok && x.K == "nil" {
				x.T = ptrT(d.T)
			}
			if ok && exprForeignType(x) == "" {
				f.why = ""
				x = f.coerce(x, v.Type())
				d.Init = &x
			} else {
				reason := "initializer outside cir/1"
				if !ok {
					reason = "initializer " + strings.TrimPrefix(f.exprReason(), "expression ")
				}
				d.Foreign = &Stmt{K: "foreign", Lang: "go", Src: f.src(vs.Values[i]), Reason: reason}
				f.refuse(vs.Values[i], n.Name, reason)
			}
		}
		f.mod.Vars = append(f.mod.Vars, d)
		f.layout = append(f.layout, "var:"+n.Name)
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
	f.beginDecl(d)
	fn := Func{Name: d.Name.Name, Exported: d.Name.IsExported(), Pos: f.pos(d.Pos()), Doc: docText(d.Doc), NameTok: idTok(d.Name)}
	fn.DeclPos, fn.DeclEnd = declRange(d)
	sig := f.info.Defs[d.Name].Type().(*types.Signature)
	if d.Body == nil {
		f.refuse(d, d.Name.Name, "function without a Go body (assembly or linkname)")
		f.opaque(d, "function without a Go body (assembly or linkname)")
		return
	}
	if why := f.signatureOutside(sig); why != "" {
		f.refuse(d, d.Name.Name, why)
		f.opaque(d, why)
		return
	}
	fn.Tparams, _ = f.tparams(sig.TypeParams())
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
		// Strict reading refuses an empty name; an unnamed parameter is `_`, as for a receiver
		// and a function literal's parameter.
		name := p.Name()
		if name == "" {
			name = "_"
		}
		fn.Params = append(fn.Params, Param{Name: name, T: f.typ(p.Type()), Variadic: sig.Variadic() && i == sig.Params().Len()-1})
	}
	for i := 0; i < sig.Results().Len(); i++ {
		fn.Results = append(fn.Results, f.typ(sig.Results().At(i).Type()))
	}
	if hasNamedResults(d.Type) && hasDefer(d.Body) {
		fn.Body = []Stmt{f.namedResultsBody(d)}
	} else {
		f.fnBody, f.sig = d.Body, sig
		params := map[string]bool{}
		if fn.Recv != nil {
			params[fn.Recv.Name] = true
		}
		for _, p := range fn.Params {
			params[p.Name] = true
		}
		var decls []Stmt
		f.named = nil
		if hasNamedResults(d.Type) {
			decls, f.named = f.resultVars(sig.Results(), params)
		}
		f.hoist = f.planHoists(d.Body, params)
		fn.Body = append(decls, f.stmts(d.Body.List, fn.Results)...)
		f.named = nil
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

// foreign keeps s verbatim as a foreign statement with a refusal. A labeled branch inside s
// that was already read as an unlabeled break or continue is unmarked: its source survives in
// the foreign text, so the loop it names must keep its label.
func (f *front) foreign(s ast.Stmt, reason string) Stmt {
	ast.Inspect(s, func(n ast.Node) bool {
		if b, ok := n.(*ast.BranchStmt); ok {
			delete(f.unlabeled, b)
		}
		return true
	})
	f.refuse(s, firstLine(f.src(s)), reason)
	return Stmt{K: "foreign", Lang: "go", Src: f.src(s), Reason: reason, Pos: f.pos(s.Pos()), Calls: f.callsOf(s)}
}

// rangeName is the name a range clause binds as its key or value, parentheses removed: `_` when
// there is none, "" when it is not an identifier (`for s.f = range xs`).
func rangeName(e ast.Expr) string {
	if e == nil {
		return "_"
	}
	if id, ok := ast.Unparen(e).(*ast.Ident); ok {
		return id.Name
	}
	return ""
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

// hasDefer reports whether body holds a defer outside any function literal in it: only then does
// code run between a return and the caller, and read the named results there.
func hasDefer(body *ast.BlockStmt) bool {
	found := false
	ast.Inspect(body, func(n ast.Node) bool {
		switch n.(type) {
		case *ast.FuncLit:
			return false
		case *ast.DeferStmt:
			found = true
		}
		return !found
	})
	return found
}

// resultVars declares the named results res as locals with their zero values, `var n T` each, a
// blank result under a fresh name `r_<n>`, and returns the declarations and the values a bare
// return returns. Each name is added to params.
func (f *front) resultVars(res *types.Tuple, params map[string]bool) ([]Stmt, []Expr) {
	var decls []Stmt
	var vals []Expr
	for i := 0; i < res.Len(); i++ {
		r := res.At(i)
		name := r.Name()
		if name == "" || name == "_" {
			name = f.freshName("r")
		} else {
			params[name] = true
		}
		t := f.typ(r.Type())
		decls = append(decls, Stmt{K: "var", Names: []string{name}, T: &t, Pos: f.pos(r.Pos())})
		vals = append(vals, Expr{K: "var", Name: name, T: ptrT(t)})
	}
	return decls, vals
}

// namedResultsBody keeps a function with named results as one foreign statement: its body,
// unchanged, inside a closure that declares the same named results and is returned at once, so
// bare returns and writes to the results keep their meaning under an unnamed signature.
func (f *front) namedResultsBody(d *ast.FuncDecl) Stmt {
	reason := "named results are outside cir/1: the body is kept in a closure that declares them"
	sig := f.src(&ast.FuncType{Params: &ast.FieldList{}, Results: d.Type.Results})
	src := "return " + sig + " " + f.src(d.Body) + "()"
	f.refuse(d.Body, d.Name.Name, reason)
	return Stmt{K: "foreign", Lang: "go", Src: src, Reason: reason, Pos: f.pos(d.Body.Pos()), Calls: f.callsOf(d.Body)}
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
		if is, ok := s.(*ast.IfStmt); ok {
			if assign, ok := f.hoist[is]; ok {
				out = append(out, f.hoisted(is.Init.(*ast.AssignStmt), assign, results), f.ifStmt(is, results))
				continue
			}
		}
		if parts, ok := f.blankAssign(s, results); ok {
			out = append(out, parts...)
			continue
		}
		if ds, ok := s.(*ast.DeclStmt); ok {
			out = append(out, f.declStmt(ds, results)...)
			continue
		}
		if is, ok := s.(*ast.IfStmt); ok && is.Init != nil && f.canHoist() {
			out = append(out, f.hoistInit(is, is.Init, results)...)
			continue
		}
		if ss, ok := s.(*ast.SwitchStmt); ok && ss.Init != nil && f.canHoist() {
			out = append(out, f.hoistInit(ss, ss.Init, results)...)
			continue
		}
		out = append(out, f.stmt(s, results))
	}
	return out
}

// hoisted reads the init statement of a hoisted `if` as the statement in front of it: as written
// (a declaration), or as an assignment to the names an earlier hoisted init declared.
func (f *front) hoisted(init *ast.AssignStmt, assign bool, results []Type) Stmt {
	if !assign {
		return f.stmt(init, results)
	}
	return f.stmt(&ast.AssignStmt{Lhs: init.Lhs, TokPos: init.TokPos, Tok: token.ASSIGN, Rhs: init.Rhs}, results)
}

// inside reads the statements of a loop or switch body, with that construct as the target of an
// unlabeled break.
func (f *front) inside(target string, list []ast.Stmt, results []Type) []Stmt {
	f.breaks = append(f.breaks, target)
	if target == "loop" {
		f.loops = append(f.loops, f.label)
		f.label = ""
	}
	out := f.stmts(list, results)
	if target == "loop" {
		f.loops = f.loops[:len(f.loops)-1]
	}
	f.breaks = f.breaks[:len(f.breaks)-1]
	return out
}

// branchesTo returns every branch naming label in a function body, outside function literals
// (a label is not visible inside one).
func branchesTo(label string, fnBody *ast.BlockStmt) []*ast.BranchStmt {
	var found []*ast.BranchStmt
	if fnBody == nil {
		return found
	}
	ast.Inspect(fnBody, func(c ast.Node) bool {
		switch c := c.(type) {
		case *ast.FuncLit:
			return false
		case *ast.BranchStmt:
			if c.Label != nil && c.Label.Name == label {
				found = append(found, c)
			}
		}
		return true
	})
	return found
}

// ownBranches returns every branch to label inside body, and whether every branch naming label
// anywhere in the function is one of them and a break or continue outside any loop nested in
// body: one that leaves or continues the labeled loop itself, which is what an unlabeled cir/1
// break or continue in that loop does. A `goto L`, or any branch to L outside the loop, keeps
// the labeled statement foreign.
func ownBranches(label string, fnBody, body *ast.BlockStmt) ([]*ast.BranchStmt, bool) {
	var found []*ast.BranchStmt
	ok := true
	var walk func(n ast.Node, nested bool)
	walk = func(n ast.Node, nested bool) {
		ast.Inspect(n, func(c ast.Node) bool {
			switch c := c.(type) {
			case *ast.ForStmt:
				walk(c.Body, true)
				return false
			case *ast.RangeStmt:
				walk(c.Body, true)
				return false
			case *ast.FuncLit:
				return false
			case *ast.BranchStmt:
				if c.Label != nil && c.Label.Name == label {
					found = append(found, c)
					if nested || (c.Tok != token.BREAK && c.Tok != token.CONTINUE) {
						ok = false
					}
				}
			}
			return true
		})
	}
	walk(body, false)
	inside := map[*ast.BranchStmt]bool{}
	for _, b := range found {
		inside[b] = true
	}
	for _, b := range branchesTo(label, fnBody) {
		if !inside[b] {
			ok = false
		}
	}
	return found, ok
}

// labeled reads a loop whose label only its own break and continue name as that loop, each of
// those branches as an unlabeled cir/1 break or continue. When the label names anything else,
// or a branch to it ends up inside a foreign statement (whose source would name a label the
// output no longer has), the whole labeled statement is foreign.
func (f *front) labeled(s *ast.LabeledStmt, results []Type) Stmt {
	reason := "labeled statement other than a loop whose label only its own break or continue names"
	var body *ast.BlockStmt
	switch l := s.Stmt.(type) {
	case *ast.ForStmt:
		body = l.Body
	case *ast.RangeStmt:
		body = l.Body
	}
	if body == nil {
		return f.foreign(s, reason)
	}
	branches, ok := ownBranches(s.Label.Name, f.fnBody, body)
	if !ok {
		return f.foreign(s, reason)
	}
	refused := len(f.mod.Refusals)
	f.label = s.Label.Name
	st := f.stmt(s.Stmt, results)
	f.label = ""
	if st.K == "foreign" {
		// the loop itself is outside cir/1: the labeled statement is refused for that reason
		reason = st.Reason
	}
	for _, b := range branches {
		if !f.unlabeled[b] {
			st.K = "foreign"
		}
	}
	if st.K == "foreign" {
		f.mod.Refusals = f.mod.Refusals[:refused]
		return f.foreign(s, reason)
	}
	return st
}

// stmt reads s; a statement holding an expression of a type cir/1 has no form for is kept foreign.
func (f *front) stmt(s ast.Stmt, results []Type) Stmt {
	refused := len(f.mod.Refusals)
	// A statement read inside an expression (a function literal's body) keeps its own shape.
	outer := f.why
	f.why = ""
	defer func() { f.why = outer }()
	st := f.stmtRaw(s, results)
	if n := stmtForeignType(st); n != "" {
		st = f.foreign(s, "type outside cir/1: "+n)
	}
	// A function literal's body is read before the statement holding it is known to be kept;
	// when it is foreign, the refusals of that body go with it and the statement's own stays.
	if st.K == "foreign" && len(f.mod.Refusals) > refused+1 && holdsFuncLit(s) {
		last := f.mod.Refusals[len(f.mod.Refusals)-1]
		f.mod.Refusals = append(f.mod.Refusals[:refused], last)
	}
	return st
}

func holdsFuncLit(n ast.Node) bool {
	found := false
	ast.Inspect(n, func(c ast.Node) bool {
		if _, ok := c.(*ast.FuncLit); ok {
			found = true
		}
		return !found
	})
	return found
}

func (f *front) stmtRaw(s ast.Stmt, results []Type) Stmt {
	pos := f.pos(s.Pos())
	switch s := s.(type) {
	case *ast.AssignStmt:
		lhs, ok := f.exprs(s.Lhs)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		rhs, ok := f.exprs(s.Rhs)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		if s.Tok == token.DEFINE {
			var names []string
			for _, l := range s.Lhs {
				names = append(names, f.name(l.(*ast.Ident)))
			}
			return Stmt{K: "let", Names: names, Rhs: rhs, Pos: pos}
		}
		op := ""
		if s.Tok != token.ASSIGN {
			op = s.Tok.String()
		}
		for i := range rhs {
			// go/types leaves the nil of `m = nil` untyped; it takes the type of its target.
			if i < len(lhs) && rhs[i].K == "nil" && rhs[i].T != nil && rhs[i].T.K == "nil" {
				rhs[i].T = lhs[i].T
			}
			if i < len(s.Lhs) {
				rhs[i] = f.coerce(rhs[i], f.info.TypeOf(s.Lhs[i]))
			}
		}
		return Stmt{K: "assign", Op: op, Lhs: lhs, Rhs: rhs, Pos: pos}
	case *ast.IncDecStmt:
		x, ok := f.expr(s.X)
		if !ok {
			return f.foreign(s, f.exprReason())
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
		return f.ifStmt(s, results)
	case *ast.RangeStmt:
		over, ok := f.expr(s.X)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		key, val := rangeName(s.Key), rangeName(s.Value)
		// for_range declares its key and value; `=` assigns variables declared before the loop
		// (or a field, an element: no name at all), which a for_range would shadow and never
		// assign. Only `_` means the same either way.
		if s.Tok == token.ASSIGN && (key != "_" || val != "_") {
			return f.foreign(s, "range with = assigns variables declared before the loop; cir/1 for_range declares its key and value")
		}
		return Stmt{K: "for_range", Key: key, Val: val, Over: &over, Body: f.inside("loop", s.Body.List, results), Pos: pos}
	case *ast.ForStmt:
		st := Stmt{K: "for", Pos: pos}
		if s.Init != nil {
			init := f.stmt(s.Init, results)
			if init.K != "let" && init.K != "assign" {
				return f.foreign(s, "for with an init statement other than := or =")
			}
			st.Init = &init
		}
		if s.Cond != nil {
			c, ok := f.expr(s.Cond)
			if !ok {
				return f.foreign(s, f.exprReason())
			}
			st.Cond = &c
		}
		if s.Post != nil {
			post := f.stmt(s.Post, results)
			if post.K != "assign" && post.K != "incr" && post.K != "decr" {
				return f.foreign(s, "for with a post statement other than =, ++ or --")
			}
			st.Post = &post
		}
		st.Body = f.inside("loop", s.Body.List, results)
		return st
	case *ast.BranchStmt:
		if s.Label != nil {
			inner := len(f.loops) > 0 && f.loops[len(f.loops)-1] == s.Label.Name
			if !inner || (s.Tok != token.BREAK && s.Tok != token.CONTINUE) {
				return f.foreign(s, "labeled branch other than a break or continue of the innermost loop")
			}
			f.unlabeled[s] = true
			if s.Tok == token.BREAK {
				return Stmt{K: "break", Pos: pos}
			}
			return Stmt{K: "continue", Pos: pos}
		}
		switch s.Tok {
		case token.BREAK:
			if len(f.breaks) == 0 || f.breaks[len(f.breaks)-1] != "loop" {
				return f.foreign(s, "break out of a switch: cir/1 break leaves a loop")
			}
			return Stmt{K: "break", Pos: pos}
		case token.CONTINUE:
			return Stmt{K: "continue", Pos: pos}
		}
		return f.foreign(s, s.Tok.String()+" is outside cir/1")
	case *ast.SwitchStmt:
		if s.Init != nil {
			return f.foreign(s, "switch with an init statement")
		}
		return f.switchStmt(s, results)
	case *ast.ReturnStmt:
		rs, ok := f.exprs(s.Results)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		if len(s.Results) == 0 && f.named != nil {
			rs = append([]Expr{}, f.named...) // a bare return returns the named results
		}
		for i := range rs {
			if i < len(results) && rs[i].K == "nil" {
				rt := results[i]
				rs[i].T = &rt
			}
			if i < len(results) && results[i].K == "error" {
				rs[i] = f.coerce(rs[i], types.Universe.Lookup("error").Type())
			} else if f.sig != nil && len(rs) == f.sig.Results().Len() {
				if wt, ok := f.ifaceTarget(f.sig.Results().At(i).Type()); ok {
					rs[i] = toIface(rs[i], wt)
				}
			}
		}
		return Stmt{K: "return", Results: rs, Pos: pos}
	case *ast.ExprStmt:
		x, ok := f.expr(s.X)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		return Stmt{K: "expr", X: &x, Pos: pos}
	case *ast.SendStmt:
		ch, ok1 := f.expr(s.Chan)
		x, ok2 := f.expr(s.Value)
		if !ok1 || !ok2 {
			return f.foreign(s, f.exprReason())
		}
		return Stmt{K: "send", Chan: &ch, X: &x, Pos: pos}
	case *ast.LabeledStmt:
		return f.labeled(s, results)
	case *ast.GoStmt:
		return f.foreign(s, "goroutine: concurrency is outside cir/1")
	case *ast.DeferStmt:
		return f.foreign(s, "defer is outside cir/1")
	}
	return f.foreign(s, fmt.Sprintf("statement %T is outside cir/1", s))
}

// switchStmt reads s but for its init statement, which the caller has read or refused.
func (f *front) switchStmt(s *ast.SwitchStmt, results []Type) Stmt {
	st := Stmt{K: "switch", Pos: f.pos(s.Pos())}
	if s.Tag != nil {
		tag, ok := f.expr(s.Tag)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		st.Tag = &tag
	}
	for _, cl := range s.Body.List {
		cc := cl.(*ast.CaseClause)
		vs, ok := f.exprs(cc.List)
		if !ok {
			return f.foreign(s, f.exprReason())
		}
		st.Cases = append(st.Cases, Case{Values: vs, Body: f.inside("switch", cc.Body, results)})
	}
	return st
}

// canHoist reports whether the function or literal body being read has no goto: a goto could
// jump over a hoisted declaration to a use of it.
func (f *front) canHoist() bool {
	if f.gotoBody != f.fnBody {
		f.gotoBody, f.gotoHas = f.fnBody, f.fnBody != nil && hasGoto(f.fnBody)
	}
	return f.fnBody != nil && !f.gotoHas
}

// hoistInit reads the `if` or `switch` s with the init statement init as the init in front of
// it, for every init the planned 0.12.0 hoist (planHoists) left: cir/1 has no block statement to
// keep the init's names scoped to s. A name the init declares keeps its spelling unless it
// collides, spelled elsewhere in the declaration being read for something else; it is then read
// under a fresh name, in the init and in every use inside s, so it neither shadows an outer
// variable nor is shadowed. When the init or s is outside cir/1, s is kept foreign whole.
func (f *front) hoistInit(s, init ast.Stmt, results []Type) []Stmt {
	var renamed []types.Object
	if as, ok := init.(*ast.AssignStmt); ok && as.Tok == token.DEFINE {
		for _, l := range as.Lhs {
			id, ok := l.(*ast.Ident)
			if !ok || id.Name == "_" {
				continue
			}
			if o := f.info.Defs[id]; o != nil && f.collides(id.Name, o) {
				f.rename[o] = f.freshName(id.Name)
				renamed = append(renamed, o)
			}
		}
	}
	parts := f.whole(s, func() []Stmt {
		first := f.stmt(init, results)
		if first.K == "foreign" {
			return []Stmt{first}
		}
		switch s := s.(type) {
		case *ast.IfStmt:
			return []Stmt{first, f.ifStmt(s, results)}
		case *ast.SwitchStmt:
			return []Stmt{first, f.switchStmt(s, results)}
		}
		return []Stmt{first, f.foreign(s, fmt.Sprintf("statement %T is outside cir/1", s))}
	})
	if len(parts) == 1 && parts[0].K == "foreign" {
		for _, o := range renamed {
			delete(f.rename, o)
		}
	}
	return parts
}

// ifStmt reads s but for its init statement, which the caller has read or refused.
func (f *front) ifStmt(s *ast.IfStmt, results []Type) Stmt {
	c, ok := f.expr(s.Cond)
	if !ok {
		return f.foreign(s, f.exprReason())
	}
	st := Stmt{K: "if", Cond: &c, Then: f.stmts(s.Body.List, results), Pos: f.pos(s.Pos())}
	switch e := s.Else.(type) {
	case *ast.BlockStmt:
		st.Else = f.stmts(e.List, results)
	case *ast.IfStmt:
		if e.Init != nil && f.canHoist() {
			st.Else = f.hoistInit(e, e.Init, results) // else if init; c: the else list is the init and the if
		} else {
			st.Else = []Stmt{f.stmt(e, results)}
		}
	}
	return st
}

// coerce wraps a value of an error struct type that is used where an `error` is expected, and
// converts a value used where a named interface is expected (interfaces.go).
func (f *front) coerce(e Expr, want types.Type) Expr {
	if _, param := want.(*types.TypeParam); want == nil || param {
		return e // a value of a type parameter's type is the type argument's own
	}
	if wt, ok := f.ifaceTarget(want); ok {
		return toIface(e, wt)
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
		if bits := floatBits(f.info.TypeOf(e)); bits != 0 {
			v, ok := floatSpelling(f.info.Types[e].Value, bits)
			return Expr{K: "float", Value: v, T: t}, ok
		}
		return Expr{K: "lit", Value: e.Value, T: t}, true
	case *ast.Ident:
		switch o := f.info.Uses[e].(type) {
		case *types.Nil:
			return Expr{K: "nil", T: t}, true
		case *types.Const:
			if o.Pkg() == nil {
				return Expr{K: "lit", Value: e.Name, T: t}, true
			}
			if o.Parent() != o.Pkg().Scope() {
				return f.constLit(e, t) // a constant declared inside a function
			}
			return Expr{K: "const", Name: e.Name, T: t}, true
		case *types.Var:
			if o.Parent() == f.pkg.Scope() {
				switch {
				case f.sentinels[o]:
					return Expr{K: "errvar", Name: e.Name, T: t}, true
				case f.globals[o]:
					return Expr{K: "global", Name: e.Name, T: t}, true
				case t != nil && hasForeign(*t):
					// refused for its type: the statement holding it is foreign for that type
					return Expr{K: "global", Name: e.Name, T: t}, true
				}
				return f.no("use of a refused package variable")
			}
			return Expr{K: "var", Name: f.name(e), T: t}, true
		case *types.Func:
			// A function of the package used as a value; an instance of a generic one has no
			// cir/1 form (func_ref carries no type arguments).
			if _, generic := f.info.Instances[e]; generic {
				return f.no("generic function used as a value")
			}
			if o.Pkg() != f.pkg || t == nil || t.K != "func" {
				return f.no("function value of " + f.funcTypeShape(o.Type()))
			}
			return Expr{K: "func_ref", Name: e.Name, T: t, NameTok: idTok(e)}, true
		case *types.TypeName:
			return f.no("type as an operand")
		case *types.Builtin:
			return f.no("builtin " + e.Name + " used as a value")
		}
		if f.info.Defs[e] != nil {
			return Expr{K: "var", Name: f.name(e), T: t}, true
		}
		if e.Name == "_" {
			return f.no("`_` as an operand")
		}
		return f.no("name " + e.Name)
	case *ast.SelectorExpr:
		if sel := f.info.Selections[e]; sel != nil && sel.Kind() == types.FieldVal && sel.Obj().Pkg() != f.pkg {
			return f.no("field of a struct of another package") // the extern type is opaque
		}
		if sel := f.info.Selections[e]; sel != nil && sel.Kind() != types.FieldVal {
			// A method value x.M of a method the package declares, through the embedded fields it
			// is promoted from; a method expression T.M is outside cir/1.
			// A method value of an interface dispatches when called: no function to name.
			switch {
			case sel.Kind() != types.MethodVal:
				return f.no("method expression")
			case types.IsInterface(holder(sel)):
				return f.no("method value of an interface")
			case sel.Obj().Pkg() != f.pkg:
				return f.no("method value of another package's type")
			case t == nil || t.K != "func":
				return f.no("method value of " + f.funcTypeShape(f.info.TypeOf(e)))
			}
			x, ok := f.receiver(e, sel)
			if !ok {
				return Expr{}, false
			}
			return Expr{K: "func_ref", Name: e.Sel.Name, X: &x, T: t, NameTok: idTok(e.Sel)}, true
		}
		if sel := f.info.Selections[e]; sel != nil {
			x, ok := f.receiver(e, sel)
			if !ok {
				return Expr{}, false
			}
			return Expr{K: "field", X: &x, Name: e.Sel.Name, T: t, NameTok: idTok(e.Sel)}, true
		}
		if id, ok := e.X.(*ast.Ident); ok {
			if _, ok := f.info.Uses[id].(*types.PkgName); ok {
				// a qualified name of another package, not called
				switch f.info.Uses[e.Sel].(type) {
				case *types.Const:
					if t != nil && (t.K == "int" || t.K == "float" || t.K == "string" || t.K == "bool") {
						return f.constLit(e, t) // math.MaxInt32
					}
					return f.no("constant of another package")
				case *types.Var:
					return f.no("variable of another package")
				case *types.Func:
					return f.no("function of another package used as a value")
				}
				return f.no("name of another package")
			}
		}
		x, ok := f.expr(e.X)
		if !ok {
			return Expr{}, false
		}
		return Expr{K: "field", X: &x, Name: e.Sel.Name, T: t, NameTok: idTok(e.Sel)}, true
	case *ast.FuncLit:
		return f.lambda(e, t)
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
		// go/types leaves the nil of `err != nil` untyped; cir/1 has no untyped nil, so it takes
		// the type of the operand it is compared with.
		untyped := func(e Expr) bool { return e.K == "nil" && e.T != nil && e.T.K == "nil" }
		if untyped(y) && !untyped(x) {
			y.T = x.T
		} else if untyped(x) && !untyped(y) {
			x.T = y.T
		}
		return Expr{K: "binop", Op: e.Op.String(), X: &x, Y: &y, T: t}, true
	case *ast.CompositeLit:
		return f.compositeLit(e, t)
	case *ast.CallExpr:
		return f.call(e, t)
	case *ast.SliceExpr:
		return f.no("slice expression")
	case *ast.TypeAssertExpr:
		return f.no("type assertion")
	case *ast.IndexListExpr:
		return f.no("generic function used as a value")
	case *ast.ArrayType, *ast.MapType, *ast.ChanType, *ast.FuncType, *ast.InterfaceType, *ast.StructType:
		return f.no("type as an operand")
	}
	return f.no(fmt.Sprintf("%T", e))
}

// compositeLit reads a composite literal. An element `{…}` of a literal whose elements are
// pointers, `[]*T{{…}}`, is `&T{…}`.
func (f *front) compositeLit(e *ast.CompositeLit, t *Type) (Expr, bool) {
	gt := f.info.TypeOf(e)
	if p, ok := gt.(*types.Pointer); ok && e.Type == nil && t != nil && t.K == "ptr" {
		x, ok := f.literalOf(e, t.Elem, p.Elem())
		if !ok {
			return Expr{}, false
		}
		return Expr{K: "unop", Op: "&", X: &x, T: t}, true
	}
	return f.literalOf(e, t, gt)
}

// literalOf reads the composite literal e of the type gt, t in cir/1. An empty one of a map or
// slice is `make`, of another package's struct its zero value. A slice literal with elements is
// `append` of them to the empty slice. A struct literal names its fields, a positional one in
// declaration order. An untyped nil takes the type of its field or element.
func (f *front) literalOf(e *ast.CompositeLit, t *Type, gt types.Type) (Expr, bool) {
	if t != nil && t.K == "unit" && len(e.Elts) == 0 {
		return Expr{K: "zero", T: t}, true // struct{}{}
	}
	if t != nil && (t.K == "map" || t.K == "slice") && len(e.Elts) == 0 {
		return Expr{K: "make", T: t}, true
	}
	if _, ok := gt.Underlying().(*types.Struct); ok && t != nil && t.K == "extern" && len(e.Elts) == 0 {
		return Expr{K: "zero", T: t}, true // sync.WaitGroup{}
	}
	if st, ok := gt.Underlying().(*types.Slice); ok && t != nil && t.K == "slice" {
		args := []Expr{{K: "make", T: t}}
		for _, el := range e.Elts {
			if _, ok := el.(*ast.KeyValueExpr); ok {
				return f.no("slice literal with keyed elements")
			}
			v, ok := f.expr(el)
			if !ok {
				return Expr{}, false
			}
			args = append(args, f.given(v, st.Elem()))
		}
		return Expr{K: "intrinsic", Name: "append", Args: args, T: t}, true
	}
	if t == nil || t.K != "named" {
		return f.no(f.literalShape(e))
	}
	st, ok := gt.Underlying().(*types.Struct)
	if !ok {
		return f.no("literal of a named map, slice or array type")
	}
	out := Expr{K: "struct", Name: t.Name, T: t}
	for i, el := range e.Elts {
		val, fv, tok := el, (*types.Var)(nil), Tok{Why: "implicit"}
		if kv, ok := el.(*ast.KeyValueExpr); ok {
			val = kv.Value
			fv, _ = f.info.Uses[kv.Key.(*ast.Ident)].(*types.Var)
			tok = idTok(kv.Key.(*ast.Ident))
		} else if i < st.NumFields() {
			fv = st.Field(i)
		}
		if fv == nil {
			return f.no("struct literal of an unknown field")
		}
		v, ok := f.expr(val)
		if !ok {
			return Expr{}, false
		}
		out.FieldNames = append(out.FieldNames, fv.Name())
		out.FieldToks = append(out.FieldToks, tok)
		out.Args = append(out.Args, f.given(v, fv.Type()))
	}
	return out, true
}

// given is the value v given where a value of the type want is expected: an untyped nil takes
// that type, and a value given to a named interface is converted to it.
func (f *front) given(v Expr, want types.Type) Expr {
	f.typeNil(&v, want)
	if wt, ok := f.ifaceTarget(want); ok {
		v = toIface(v, wt)
	}
	return v
}

// typeNil gives the untyped nil x the type want.
func (f *front) typeNil(x *Expr, want types.Type) {
	if x.K == "nil" && x.T != nil && x.T.K == "nil" {
		t := f.typ(want)
		x.T = &t
	}
}

// typeNilArgs gives each untyped nil among the arguments of a call of sig the type of its
// parameter, of the variadic parameter's element past its last fixed one.
func (f *front) typeNilArgs(args []Expr, sig *types.Signature, spread bool) {
	ps := sig.Params()
	for i := range args {
		switch {
		case sig.Variadic() && !spread && i >= ps.Len()-1:
			if s, ok := ps.At(ps.Len() - 1).Type().Underlying().(*types.Slice); ok {
				f.typeNil(&args[i], s.Elem())
			}
		case i < ps.Len():
			f.typeNil(&args[i], ps.At(i).Type())
		}
	}
}

// makeLit reads `make(map[K]V)`, `make(map[K]V, n)`, `make([]T, 0)` and `make([]T, 0, n)` as
// `make`, the empty map or slice, when n is a size hint that can be dropped: a constant, or `len`
// of a variable or of a field of one that is not reached through a pointer. Any other make is not
// this form.
func (f *front) makeLit(e *ast.CallExpr, t *Type) (Expr, bool) {
	if t == nil || (t.K != "map" && t.K != "slice") || len(e.Args) == 0 {
		return Expr{}, false
	}
	hints := e.Args[1:]
	if t.K == "slice" {
		if len(hints) == 0 {
			return Expr{}, false
		}
		if v := f.info.Types[hints[0]].Value; v == nil || v.ExactString() != "0" {
			return Expr{}, false
		}
		hints = hints[1:]
	}
	for _, n := range hints {
		if !f.sizeHint(n) {
			return Expr{}, false
		}
	}
	return Expr{K: "make", T: t}, true
}

// sizeHint reports whether n is a constant or `len` of a variable or of a field of one not reached
// through a pointer: a value whose evaluation neither panics nor has an effect.
func (f *front) sizeHint(n ast.Expr) bool {
	if f.info.Types[n].Value != nil {
		return true
	}
	c, ok := ast.Unparen(n).(*ast.CallExpr)
	if !ok || len(c.Args) != 1 || c.Ellipsis.IsValid() {
		return false
	}
	if id, ok := ast.Unparen(c.Fun).(*ast.Ident); !ok || f.info.Uses[id] != types.Universe.Lookup("len") {
		return false
	}
	switch a := ast.Unparen(c.Args[0]).(type) {
	case *ast.Ident:
		_, ok := f.info.Uses[a].(*types.Var)
		return ok
	case *ast.SelectorExpr:
		sel := f.info.Selections[a]
		x, ok := ast.Unparen(a.X).(*ast.Ident)
		if !ok || sel == nil || sel.Kind() != types.FieldVal || sel.Indirect() {
			return false
		}
		_, isVar := f.info.Uses[x].(*types.Var)
		_, isPtr := f.info.TypeOf(x).Underlying().(*types.Pointer)
		return isVar && !isPtr
	}
	return false
}

// newLit reads `new(T)` of a struct T as `&T{}`, of another package's struct as the address of its
// zero value. Of any other type it is not this form: cir/1 has no address of a zero int.
func (f *front) newLit(e *ast.CallExpr, t *Type) (Expr, bool) {
	p, ok := f.info.TypeOf(e).(*types.Pointer)
	if !ok || t == nil || t.K != "ptr" || t.Elem == nil {
		return Expr{}, false
	}
	if _, ok := p.Elem().Underlying().(*types.Struct); !ok {
		return Expr{}, false
	}
	var x Expr
	switch t.Elem.K {
	case "named":
		x = Expr{K: "struct", Name: t.Elem.Name, T: t.Elem}
	case "extern":
		x = Expr{K: "zero", T: t.Elem}
	default:
		return Expr{}, false
	}
	return Expr{K: "unop", Op: "&", X: &x, T: t}, true
}

// literalShape names what a composite literal that is not a struct literal of the package is.
func (f *front) literalShape(e *ast.CompositeLit) string {
	t := f.info.TypeOf(e)
	if t == nil {
		return "composite literal"
	}
	switch t.Underlying().(type) {
	case *types.Slice:
		return "slice literal with elements"
	case *types.Map:
		return "map literal with elements"
	case *types.Array:
		return "array literal"
	case *types.Struct:
		if n, ok := types.Unalias(t).(*types.Named); ok && n.Obj().Pkg() != f.pkg {
			return "literal of another package's struct"
		}
		if _, ok := types.Unalias(t).(*types.Named); !ok {
			return "literal of an anonymous struct type"
		}
		return "literal of a struct type outside cir/1"
	}
	return "composite literal of type " + f.goType(t)
}

// funcTypeShape names what makes the function type t have no cir/1 type.
func (f *front) funcTypeShape(t types.Type) string {
	if sig, ok := t.Underlying().(*types.Signature); ok && sig.Variadic() {
		return "a variadic type"
	}
	return "a type outside cir/1"
}

// externCall reads pkg.Name(args), a call of a function of another package, as an extern call,
// declaring the extern from the callee's own signature; a variadic callee is declared variadic, a
// generic one with its type parameters, the call with its type arguments. A callee with a
// parameter or result type, or a constraint, cir/1 has no type for is outside cir/1.
func (f *front) externCall(pn *types.PkgName, sel *ast.Ident, args []Expr, t *Type) (Expr, bool) {
	fn, ok := f.info.Uses[sel].(*types.Func)
	if !ok {
		return f.no("call of a function value of another package")
	}
	if t == nil {
		return f.no("call of another package's function with a type outside cir/1")
	}
	fn = fn.Origin()
	sig := fn.Type().(*types.Signature)
	tps, why := f.tparams(sig.TypeParams())
	if why != "" {
		return f.no("call of a generic function whose constraint cir/1 lacks")
	}
	targs, _, ok := f.instance(sel)
	if !ok {
		return f.no("call of a generic function with a type argument outside cir/1")
	}
	x := Extern{Path: pn.Imported().Path(), Name: fn.Name(), Variadic: sig.Variadic(), Tparams: tps}
	for i := 0; i < sig.Params().Len(); i++ {
		p := f.typ(sig.Params().At(i).Type())
		if hasForeign(p) {
			return f.no("call of another package's function with a type outside cir/1")
		}
		x.Params = append(x.Params, p)
	}
	for i := 0; i < sig.Results().Len(); i++ {
		r := f.typ(sig.Results().At(i).Type())
		if hasForeign(r) {
			return f.no("call of another package's function with a type outside cir/1")
		}
		x.Results = append(x.Results, r)
	}
	if hasForeign(*t) {
		return f.no("call of another package's function with a type outside cir/1")
	}
	f.extern(x)
	return Expr{K: "extern", Path: x.Path, Name: x.Name, Targs: targs, Args: args, T: t, NameTok: idTok(sel)}, true
}

// extern declares x once; the encoder sorts the declarations.
func (f *front) extern(x Extern) {
	for _, y := range f.mod.Externs {
		if y.key() == x.key() {
			return
		}
	}
	f.mod.Externs = append(f.mod.Externs, x)
}

// hasForeign tells whether t, or a type inside it, has no cir/1 form.
func hasForeign(t Type) bool {
	if t.K == "foreign" || t.K == "nil" {
		return true
	}
	if t.Key != nil && hasForeign(*t.Key) {
		return true
	}
	if t.Elem != nil && hasForeign(*t.Elem) {
		return true
	}
	for _, e := range t.Elems {
		if hasForeign(e) {
			return true
		}
	}
	for _, e := range append(append(append([]Type{}, t.Params...), t.Results...), t.Targs...) {
		if hasForeign(e) {
			return true
		}
	}
	return false
}

func (f *front) call(e *ast.CallExpr, t *Type) (Expr, bool) {
	// f(g()) passes each result of a multi-valued g as its own argument; cir/1 has no argument
	// that spreads a tuple over parameters, so the call is outside it.
	if len(e.Args) == 1 {
		if _, multi := f.info.TypeOf(e.Args[0]).(*types.Tuple); multi {
			return f.no("call passed a multi-valued call")
		}
	}
	if id, ok := ast.Unparen(e.Fun).(*ast.Ident); ok {
		if _, ok := f.info.Uses[id].(*types.Builtin); ok && id.Name != "append" && id.Name != "len" {
			if id.Name == "make" {
				if x, ok := f.makeLit(e, t); ok {
					return x, true
				}
			}
			if id.Name == "new" {
				if x, ok := f.newLit(e, t); ok {
					return x, true
				}
			}
			return f.no("builtin " + id.Name)
		}
	}
	if sk, ok := f.sortedKeys(e, t); ok {
		return sk, true
	}
	args, ok := f.exprs(e.Args)
	if !ok {
		return Expr{}, false
	}
	// A spread call f(xs...): its last argument, a slice, is the variadic parameter itself.
	spread := e.Ellipsis.IsValid()
	if spread {
		last := e.Args[len(e.Args)-1]
		if _, ok := f.info.TypeOf(last).Underlying().(*types.Slice); !ok {
			return f.no("spread of a string") // append(b, s...) of a string: no slice to pass
		}
		x := args[len(args)-1]
		args[len(args)-1] = Expr{K: "spread", X: &x, T: x.T}
	}
	if tv, ok := f.info.Types[e.Fun]; ok && tv.IsType() {
		if wt, ok := f.ifaceTarget(tv.Type); ok && len(args) == 1 {
			return toIface(args[0], wt), true
		}
		if len(args) != 1 || t == nil || t.K != "named" {
			return f.no("conversion to " + f.goType(tv.Type))
		}
		return Expr{K: "conv", Name: t.Name, X: &args[0], T: t}, true
	}
	if ft := f.info.TypeOf(e.Fun); ft != nil {
		if sig, ok := ft.Underlying().(*types.Signature); ok {
			f.typeNilArgs(args, sig, spread)
		}
	}
	// The fmt intrinsics take their arguments one by one; called with a spread, each is the
	// extern call it is in Go.
	for _, in := range []struct{ pkg, name, as string }{
		{"fmt", "Sprintf", "sprintf"}, {"fmt", "Errorf", "errorf"}, {"fmt", "Println", "println"},
	} {
		if !spread && f.isPkgFunc(e.Fun, in.pkg, in.name) {
			return Expr{K: "intrinsic", Name: in.as, Args: args, T: t}, true
		}
	}
	if ft := f.info.TypeOf(e.Fun); ft != nil {
		if sig, ok := ft.Underlying().(*types.Signature); ok {
			f.coerceArgs(args, sig)
		}
	}
	// An explicit instantiation f[T](…) calls f with those type arguments.
	callee := e.Fun
	if base := calleeIdent(e.Fun); base != e.Fun {
		id, _ := base.(*ast.Ident)
		if s, ok := base.(*ast.SelectorExpr); ok {
			id = s.Sel
		}
		if _, inst := f.info.Instances[id]; id != nil && inst {
			callee = base
		}
	}
	switch fun := callee.(type) {
	case *ast.Ident:
		switch f.info.Uses[fun].(type) {
		case *types.Builtin:
			if fun.Name == "append" || (fun.Name == "len" && !spread) {
				return Expr{K: "intrinsic", Name: fun.Name, Args: args, T: t}, true
			}
			return f.no("builtin " + fun.Name + " with a spread")
		case *types.Func:
			targs, _, ok := f.instance(fun)
			if !ok {
				return f.no("call of a generic function with a type argument outside cir/1")
			}
			return Expr{K: "call", Name: fun.Name, Targs: targs, Args: args, T: t, NameTok: idTok(fun)}, true
		}
	case *ast.SelectorExpr:
		if x, ok := fun.X.(*ast.Ident); ok {
			if pn, ok := f.info.Uses[x].(*types.PkgName); ok {
				return f.externCall(pn, fun.Sel, args, t)
			}
		}
		if sel := f.info.Selections[fun]; sel != nil && sel.Kind() == types.MethodVal {
			x, ok := f.receiver(fun, sel)
			if !ok {
				return Expr{}, false
			}
			if tp, ok := holder(sel).(*types.TypeParam); ok {
				return f.paramCall(tp, fun.Sel.Name, x, args, t)
			}
			if n, ok := types.Unalias(holder(sel)).(*types.Named); ok && n.Obj().Pkg() != nil && types.IsInterface(n) {
				return f.ifaceCall(n, fun.Sel.Name, x, args, t)
			}
			if fn, ok := sel.Obj().(*types.Func); ok && fn.Pkg() != nil && fn.Pkg() != f.pkg {
				m, ok := f.externMethod(fn, x, args, t)
				m.NameTok = idTok(fun.Sel)
				return m, ok
			}
			return Expr{K: "method", X: &x, Name: fun.Sel.Name, Args: args, T: t, NameTok: idTok(fun.Sel)}, true
		}
	}
	// A call of a function value: a local, a parameter, a field, a function literal. A function
	// type is never variadic in cir/1, so a spread call of a value is outside it.
	if ft := f.info.TypeOf(e.Fun); ft != nil && !spread {
		if _, ok := ft.Underlying().(*types.Signature); ok {
			fn, ok := f.expr(e.Fun)
			if !ok {
				return Expr{}, false
			}
			return Expr{K: "call_value", X: &fn, Args: args, T: t}, true
		}
	}
	if spread {
		return f.no("spread call of a function value")
	}
	return f.no(fmt.Sprintf("call of %T", e.Fun))
}

// sortedKeys reads `slices.Sorted(maps.Keys(m))`, the keys of m in ascending order, as cir/1's
// sorted_keys when m is a map (not a named type over one) whose keys are integers. Over string
// keys it is not sorted_keys: Go orders strings by byte, sorted_keys by UTF-16 code unit. Any
// other shape is not this intrinsic.
func (f *front) sortedKeys(e *ast.CallExpr, t *Type) (Expr, bool) {
	if !f.isPkgFunc(e.Fun, "slices", "Sorted") || len(e.Args) != 1 || e.Ellipsis.IsValid() {
		return Expr{}, false
	}
	keys, ok := ast.Unparen(e.Args[0]).(*ast.CallExpr)
	if !ok || !f.isPkgFunc(keys.Fun, "maps", "Keys") || len(keys.Args) != 1 {
		return Expr{}, false
	}
	mt, ok := f.info.TypeOf(keys.Args[0]).Underlying().(*types.Map)
	if !ok {
		return Expr{}, false
	}
	if b, ok := mt.Key().Underlying().(*types.Basic); !ok || b.Info()&types.IsInteger == 0 {
		return Expr{}, false
	}
	m, ok := f.expr(keys.Args[0])
	if !ok || m.T == nil || m.T.K != "map" {
		return Expr{}, false // sorted_keys takes a map, not a named type over one
	}
	return Expr{K: "intrinsic", Name: "sorted_keys", Args: []Expr{m}, T: t}, true
}

// cir/1 has no block statement, so an `if` whose init statement declares names cannot keep them
// scoped to the `if`. The reader hoists the init in front of the `if` where that keeps the
// meaning: the init is `<names> := <call>` and it and the condition read inside cir/1, the `if`
// has no `else` and is a statement of a block (not an `else if`), the function has no goto, no
// name is a parameter or the receiver, and no name occurs anywhere in the function body outside
// the `if` (the call of its init counting as outside) except inside another such `if` that
// declares it with the same type, neither containing the other, where the name is never captured
// by a function literal or its address taken (escapingNames). Walking the body
// in source order, one whose names are all undeclared reads as `let` and declares them in its
// block; one whose names are all declared in scope by an earlier one reads as an assignment; one
// with some of each stays foreign, and the plan is made again without it.
//
// "The function" is a declared function or a function literal: a literal's body is planned on its
// own when the literal is read (lambda), with the literal's parameters, so the names a hoisted init
// declares there stay scoped to the literal. The plan of the enclosing body never holds an `if`
// of a literal (blocks does not enter one), and every name used inside a literal counts as
// escaping for it. The init and condition are read inside cir/1 and with no type outside it
// (probe), so neither the hoisted init nor the `if` turns foreign afterwards.

// ifInit is one `if` whose init statement may be hoisted.
type ifInit struct {
	s     *ast.IfStmt
	init  *ast.AssignStmt
	names map[string]types.Type // the non-blank names the init declares
	blank bool                  // the init also has a blank name
}

// hoists holds, for the function being read, every `if` read as its init followed by the `if`,
// and whether the init reads as an assignment (true) or as a declaration (false).
type hoists map[*ast.IfStmt]bool

// blocks walks the statement lists the reader reads as statement lists, in source order: list
// itself and, nested in its statements, the bodies of if, else, for, range and switch cases.
// enter is called before each list, leave after it, and each for every statement of a list.
func blocks(list []ast.Stmt, enter, leave func(), each func(ast.Stmt)) {
	var within func(s ast.Stmt)
	var walk func(list []ast.Stmt)
	walk = func(list []ast.Stmt) {
		enter()
		for _, s := range list {
			each(s)
			within(s)
		}
		leave()
	}
	within = func(s ast.Stmt) {
		switch s := s.(type) {
		case *ast.IfStmt:
			walk(s.Body.List)
			switch e := s.Else.(type) {
			case *ast.BlockStmt:
				walk(e.List)
			case *ast.IfStmt:
				within(e)
			}
		case *ast.ForStmt:
			walk(s.Body.List)
		case *ast.RangeStmt:
			walk(s.Body.List)
		case *ast.SwitchStmt:
			for _, cl := range s.Body.List {
				walk(cl.(*ast.CaseClause).Body)
			}
		case *ast.LabeledStmt:
			within(s.Stmt)
		}
	}
	walk(list)
}

// planHoists decides which `if` statements of body are hoisted. params holds the names of the
// function's parameters and receiver.
func (f *front) planHoists(body *ast.BlockStmt, params map[string]bool) hoists {
	if hasGoto(body) {
		// a goto can jump over a hoisted declaration to a later assignment to it
		return hoists{}
	}
	escaped := f.escapingNames(body)
	var cands []*ifInit
	nop := func() {}
	blocks(body.List, nop, nop, func(s ast.Stmt) {
		if is, ok := s.(*ast.IfStmt); ok {
			if c := f.ifInitShape(is, params); c != nil {
				cands = append(cands, c)
			}
		}
	})
	live := map[*ast.IfStmt]*ifInit{}
	for _, c := range cands {
		live[c.s] = c
	}
	for {
		removed := false
		for _, c := range cands {
			if live[c.s] != nil && !namesKeptInside(c, live, body, escaped) {
				delete(live, c.s)
				removed = true
			}
		}
		if removed {
			continue
		}
		plan, mixed := assignOrDeclare(body, live)
		if mixed == nil {
			return plan
		}
		delete(live, mixed)
	}
}

// ifInitShape returns s as a hoisting candidate, or nil when its shape alone rules it out.
func (f *front) ifInitShape(s *ast.IfStmt, params map[string]bool) *ifInit {
	init, ok := s.Init.(*ast.AssignStmt)
	if !ok || s.Else != nil || init.Tok != token.DEFINE || len(init.Rhs) != 1 {
		return nil
	}
	if _, ok := init.Rhs[0].(*ast.CallExpr); !ok {
		return nil
	}
	// The init and the condition must read inside cir/1, with no type outside it: a foreign `if`
	// keeps its init in its source text, and a foreign init keeps `:=` in its source text even
	// where it is read as an assignment, so either would run or declare the init twice or wrongly.
	probed := []ast.Expr{s.Cond}
	probed = append(probed, init.Rhs...)
	probed = append(probed, init.Lhs...)
	if !f.probe(probed) {
		return nil
	}
	c := &ifInit{s: s, init: init, names: map[string]types.Type{}}
	for _, l := range init.Lhs {
		id, ok := l.(*ast.Ident)
		if !ok {
			return nil
		}
		if id.Name == "_" {
			c.blank = true
			continue
		}
		if params[id.Name] {
			return nil
		}
		obj := f.info.Defs[id]
		if obj == nil {
			return nil
		}
		c.names[id.Name] = obj.Type()
	}
	if len(c.names) == 0 {
		return nil
	}
	return c
}

// probe reports whether every expression of es reads inside cir/1 with no type outside it, and
// leaves the reader as it found it: the refusals, extern and interface declarations and branch
// marks the read adds (a function literal's body is read with it) are taken back, so the read of
// the body that follows adds each once, and only for what it keeps.
func (f *front) probe(es []ast.Expr) bool {
	refusals, externs, ifaces, why := len(f.mod.Refusals), len(f.mod.Externs), len(f.mod.Interfaces), f.why
	unlabeled := make(map[*ast.BranchStmt]bool, len(f.unlabeled))
	for b, v := range f.unlabeled {
		unlabeled[b] = v
	}
	defer func() {
		f.mod.Refusals = f.mod.Refusals[:refusals]
		f.mod.Externs = f.mod.Externs[:externs]
		f.mod.Interfaces = f.mod.Interfaces[:ifaces]
		f.unlabeled = unlabeled
		f.why = why
	}()
	for _, e := range es {
		x, ok := f.expr(e)
		if !ok || exprForeignType(x) != "" {
			return false
		}
	}
	return true
}

// holds reports whether pos lies in c's `if` and outside the call of its init, the one place in
// the `if` where an init name still means what it means outside it.
func (c *ifInit) holds(pos token.Pos) bool {
	call := c.init.Rhs[0]
	return pos >= c.s.Pos() && pos < c.s.End() && !(pos >= call.Pos() && pos < call.End())
}

func (c *ifInit) contains(d *ifInit) bool {
	return d.s.Pos() >= c.s.Pos() && d.s.End() <= c.s.End()
}

// hasGoto reports whether body holds a goto outside function literals (a label is not visible
// inside one).
func hasGoto(body *ast.BlockStmt) bool {
	found := false
	ast.Inspect(body, func(n ast.Node) bool {
		switch n := n.(type) {
		case *ast.FuncLit:
			return false
		case *ast.BranchStmt:
			if n.Tok == token.GOTO {
				found = true
			}
		}
		return !found
	})
	return found
}

// escapingNames returns, by spelling, every name in body whose variable may be reached other
// than by its name: used inside a function literal, the operand of `&` or of a slice expression,
// or the receiver of a pointer method called on an addressable value. Hoisted inits that share
// such a name would make one variable of what Go keeps as one per `if`, and a pointer or closure
// taken from one would see the next one's value.
func (f *front) escapingNames(body *ast.BlockStmt) map[string]bool {
	escaped := map[string]bool{}
	root := func(e ast.Expr) {
		for {
			switch x := e.(type) {
			case *ast.ParenExpr:
				e = x.X
			case *ast.SelectorExpr:
				e = x.X
			case *ast.IndexExpr:
				e = x.X
			case *ast.Ident:
				escaped[x.Name] = true
				return
			default:
				return
			}
		}
	}
	ast.Inspect(body, func(n ast.Node) bool {
		switch n := n.(type) {
		case *ast.FuncLit:
			ast.Inspect(n.Body, func(m ast.Node) bool {
				if id, ok := m.(*ast.Ident); ok {
					escaped[id.Name] = true
				}
				return true
			})
			return false
		case *ast.UnaryExpr:
			if n.Op == token.AND {
				root(n.X)
			}
		case *ast.SliceExpr:
			root(n.X)
		case *ast.SelectorExpr:
			if sel := f.info.Selections[n]; sel != nil && sel.Kind() == types.MethodVal {
				if _, ptrRecv := sel.Obj().Type().(*types.Signature).Recv().Type().(*types.Pointer); ptrRecv {
					if _, ptrX := f.info.TypeOf(n.X).Underlying().(*types.Pointer); !ptrX {
						root(n.X)
					}
				}
			}
		}
		return true
	})
	return escaped
}

// namesKeptInside reports whether every identifier in body spelled like one of c's names lies in
// c, or in another live candidate disjoint from c that declares that name with the same type and
// that name is not in escaped. The check goes by spelling, so a use of an outer variable, constant
// or function of that name counts; only the name of a field or method selector is skipped.
func namesKeptInside(c *ifInit, live map[*ast.IfStmt]*ifInit, body *ast.BlockStmt, escaped map[string]bool) bool {
	ok := true
	var visit func(n ast.Node) bool
	visit = func(n ast.Node) bool {
		if !ok {
			return false
		}
		switch n := n.(type) {
		case *ast.SelectorExpr:
			ast.Inspect(n.X, visit)
			return false
		case *ast.Ident:
			t, named := c.names[n.Name]
			if !named || c.holds(n.Pos()) {
				return true
			}
			if escaped[n.Name] {
				ok = false
				return false
			}
			for _, d := range live {
				if d == c || c.contains(d) || d.contains(c) || !d.holds(n.Pos()) {
					continue
				}
				if dt, ok := d.names[n.Name]; ok && types.Identical(dt, t) {
					return true
				}
			}
			ok = false
		}
		return true
	}
	ast.Inspect(body, visit)
	return ok
}

// assignOrDeclare walks body in source order with a scope per block. A live candidate none of
// whose names is declared in scope reads as a declaration, and declares them in its block; one
// all of whose names are, and that has no blank name, reads as an assignment. It returns the plan,
// or the first candidate that is neither.
func assignOrDeclare(body *ast.BlockStmt, live map[*ast.IfStmt]*ifInit) (hoists, *ast.IfStmt) {
	plan := hoists{}
	var mixed *ast.IfStmt
	var scopes []map[string]bool
	declared := func(n string) bool {
		for i := len(scopes) - 1; i >= 0; i-- {
			if scopes[i][n] {
				return true
			}
		}
		return false
	}
	blocks(body.List,
		func() { scopes = append(scopes, map[string]bool{}) },
		func() { scopes = scopes[:len(scopes)-1] },
		func(s ast.Stmt) {
			is, ok := s.(*ast.IfStmt)
			c := live[is]
			if !ok || c == nil || mixed != nil {
				return
			}
			in := 0
			for n := range c.names {
				if declared(n) {
					in++
				}
			}
			switch {
			case in == 0:
				plan[is] = false
				for n := range c.names {
					scopes[len(scopes)-1][n] = true
				}
			case in == len(c.names) && !c.blank:
				plan[is] = true
			default:
				mixed = is
			}
		})
	return plan, mixed
}

// lambda reads a function literal: its parameters and results, the outer locals it uses, and its
// body, in which no enclosing loop or label is visible. A literal with named results and a defer
// is outside cir/1.
func (f *front) lambda(e *ast.FuncLit, t *Type) (Expr, bool) {
	if hasNamedResults(e.Type) && hasDefer(e.Body) {
		return f.no("function literal with named results and a defer")
	}
	if t == nil || t.K != "func" || hasForeign(*t) {
		return f.no("function literal of " + f.funcTypeShape(f.info.TypeOf(e)))
	}
	sig := f.info.TypeOf(e).(*types.Signature)
	x := Expr{K: "lambda", T: t, Params: []Param{}, Results: t.Results, Captures: f.captures(e)}
	for i := 0; i < sig.Params().Len(); i++ {
		name := sig.Params().At(i).Name()
		if name == "" {
			name = "_"
		}
		x.Params = append(x.Params, Param{Name: name, T: t.Params[i]})
	}
	breaks, loops, label, fnBody, outer, hoist, named := f.breaks, f.loops, f.label, f.fnBody, f.sig, f.hoist, f.named
	f.breaks, f.loops, f.label, f.fnBody, f.sig, f.named = nil, nil, "", e.Body, sig, nil
	// The literal's body is a function body of its own: its `if` inits are planned over it, with
	// its parameters, and the names they declare are scoped to it.
	params := map[string]bool{}
	for _, p := range x.Params {
		params[p.Name] = true
	}
	var decls []Stmt
	if hasNamedResults(e.Type) {
		decls, f.named = f.resultVars(sig.Results(), params)
	}
	f.hoist = f.planHoists(e.Body, params)
	x.Body = append(decls, f.stmts(e.Body.List, t.Results)...)
	f.breaks, f.loops, f.label, f.fnBody, f.sig, f.hoist, f.named = breaks, loops, label, fnBody, outer, hoist, named
	return x, true
}

// captures names, sorted and each once, every local, parameter or receiver declared outside the
// literal that its body uses.
func (f *front) captures(lit *ast.FuncLit) []string {
	seen := map[string]bool{}
	ast.Inspect(lit.Body, func(n ast.Node) bool {
		id, ok := n.(*ast.Ident)
		if !ok {
			return true
		}
		v, ok := f.info.Uses[id].(*types.Var)
		if !ok || v.IsField() || v.Pkg() != f.pkg || v.Parent() == f.pkg.Scope() {
			return true
		}
		if v.Pos() < lit.Pos() || v.Pos() >= lit.End() {
			name := v.Name()
			if r, ok := f.rename[v]; ok {
				name = r
			}
			seen[name] = true
		}
		return true
	})
	out := []string{}
	for n := range seen {
		out = append(out, n)
	}
	sort.Strings(out)
	return out
}
