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
	// hoist holds the `if` statements of the function being read whose init is read in front of
	// them (planHoists).
	hoist hoists
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
	ensureGOROOT()
	conf := types.Config{Importer: importer.ForCompiler(fset, "source", nil)}
	pkg, err := conf.Check(name, fset, files, info)
	if err != nil {
		return nil, err
	}
	f := &front{fset: fset, info: info, pkg: pkg, conv: map[string]bool{}, unlabeled: map[*ast.BranchStmt]bool{},
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
		f.fnBody = d.Body
		params := map[string]bool{}
		if fn.Recv != nil {
			params[fn.Recv.Name] = true
		}
		for _, p := range fn.Params {
			params[p.Name] = true
		}
		f.hoist = f.planHoists(d.Body, params)
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
		if is, ok := s.(*ast.IfStmt); ok {
			if assign, ok := f.hoist[is]; ok {
				out = append(out, f.hoisted(is.Init.(*ast.AssignStmt), assign, results), f.ifStmt(is, results))
				continue
			}
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
		return f.ifStmt(s, results)
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
				return f.foreign(s, "expression outside cir/1")
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
		st := Stmt{K: "switch", Pos: pos}
		if s.Tag != nil {
			tag, ok := f.expr(s.Tag)
			if !ok {
				return f.foreign(s, "expression outside cir/1")
			}
			st.Tag = &tag
		}
		for _, cl := range s.Body.List {
			cc := cl.(*ast.CaseClause)
			vs, ok := f.exprs(cc.List)
			if !ok {
				return f.foreign(s, "expression outside cir/1")
			}
			st.Cases = append(st.Cases, Case{Values: vs, Body: f.inside("switch", cc.Body, results)})
		}
		return st
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
	case *ast.LabeledStmt:
		return f.labeled(s, results)
	case *ast.GoStmt:
		return f.foreign(s, "goroutine: concurrency is outside cir/1")
	case *ast.DeferStmt:
		return f.foreign(s, "defer is outside cir/1")
	}
	return f.foreign(s, fmt.Sprintf("statement %T is outside cir/1", s))
}

// ifStmt reads s but for its init statement, which the caller has read or refused.
func (f *front) ifStmt(s *ast.IfStmt, results []Type) Stmt {
	c, ok := f.expr(s.Cond)
	if !ok {
		return f.foreign(s, "expression outside cir/1")
	}
	st := Stmt{K: "if", Cond: &c, Then: f.stmts(s.Body.List, results), Pos: f.pos(s.Pos())}
	switch e := s.Else.(type) {
	case *ast.BlockStmt:
		st.Else = f.stmts(e.List, results)
	case *ast.IfStmt:
		st.Else = []Stmt{f.stmt(e, results)}
	}
	return st
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
	// The init and the condition must read inside cir/1: a foreign `if` keeps its init in its
	// source text, and a foreign init keeps `:=` in its source text even where it is read as an
	// assignment, so either would run or declare the init twice or wrongly.
	if _, ok := f.expr(s.Cond); !ok {
		return nil
	}
	if _, ok := f.exprs(init.Rhs); !ok {
		return nil
	}
	if _, ok := f.exprs(init.Lhs); !ok {
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
