package main

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/parser"
	"go/printer"
	"go/token"
	"sort"
)

// sortedKeys writes cir/1's sorted_keys(m), the keys of a map in ascending order, as a function
// literal called on the map: the keys collected and sorted with sort.Slice. Integers compare
// numerically. Strings, and named types over a string, compare by UTF-16 code unit, as Java's
// String.compareTo does and cir/1 defines; Go's own string order is by byte, which differs for a
// character above U+FFFF against one in U+E000..U+FFFF. Any other key type is refused.
//
// The literal names nothing from the function around it but the map it is called on. It reaches
// the packages sort and unicode/utf16 under aliases no identifier of the module holds (see
// alias), and the predeclared identifiers it uses (make, len, append, rune, int, bool and those
// in the key and element types); a local or parameter of the function holding one of those
// names would shadow it, and the function is refused.
func (w *writer) sortedKeys(e Expr) ast.Expr {
	if len(e.Args) != 1 || e.Args[0].T == nil || e.Args[0].T.K != "map" {
		w.refuse("sorted_keys of other than one map")
		return ident("nil")
	}
	mt := *e.Args[0].T
	srt := w.alias("sort")
	var less string
	switch w.underKind(*mt.Key) {
	case "int":
		less = "return keys[i] < keys[j]"
	case "string":
		u := w.alias("unicode/utf16")
		less = fmt.Sprintf(`a, b := %[1]s.Encode([]rune(keys[i])), %[1]s.Encode([]rune(keys[j]))
		for n := 0; n < len(a) && n < len(b); n++ {
			if a[n] != b[n] {
				return a[n] < b[n]
			}
		}
		return len(a) < len(b)`, u)
	default:
		w.refuse("sorted_keys of a map keyed by %s: only integers and strings have an order", mt.Key.K)
		return ident("nil")
	}
	k, v := w.typeSrc(*mt.Key), w.typeSrc(*mt.Elem)
	src := fmt.Sprintf(`func(m map[%[1]s]%[2]s) []%[1]s {
	keys := make([]%[1]s, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	%[4]s.Slice(keys, func(i, j int) bool {
		%[3]s
	})
	return keys
}`, k, v, less, srt)
	fn, err := parser.ParseExpr(src)
	if err != nil {
		w.refuse("sorted_keys: %v", err)
		return ident("nil")
	}
	if name := w.shadowedIn(fn); name != "" {
		w.refuse("sorted_keys in a function whose local %s hides the predeclared %s the written sort uses", name, name)
		return ident("nil")
	}
	stripPositions(fn)
	return &ast.CallExpr{Fun: fn, Args: []ast.Expr{w.expr(e.Args[0])}}
}

// closureNames are the names the sort literal binds itself; every other identifier in it comes
// from outside.
var closureNames = map[string]bool{"m": true, "keys": true, "k": true, "i": true, "j": true, "a": true, "b": true, "n": true}

// shadowedIn is the first identifier the literal fn takes from outside it (a predeclared name or
// a type the module declares) that a parameter, receiver or local of the function being written
// holds, or "" when there is none. Aliases are fresh across the module and never collide.
func (w *writer) shadowedIn(fn ast.Expr) string {
	locals := map[string]bool{}
	if f := w.fn; f != nil {
		for _, p := range f.Params {
			locals[p.Name] = true
		}
		if f.Recv != nil {
			locals[f.Recv.Name] = true
		}
		collectNames(f.Body, locals)
	}
	var hit []string
	ast.Inspect(fn, func(n ast.Node) bool {
		switch x := n.(type) {
		case *ast.SelectorExpr:
			return false // only alias.Member: x.X is a fresh alias, x.Sel a package member
		case *ast.Ident:
			if !closureNames[x.Name] && locals[x.Name] {
				hit = append(hit, x.Name)
			}
		}
		return true
	})
	if len(hit) == 0 {
		return ""
	}
	sort.Strings(hit)
	return hit[0]
}

// alias is the import name of path for the code the writer adds itself (sorted_keys, sprintf,
// errorf, println, sentinel errors): the path's last element, or that with "_" appended until no
// name the module declares, binds or reads in cir/1 holds it, fixed for the file the first time it
// is asked. Without a clash it is the plain package name, so the import is the ordinary one.
func (w *writer) alias(path string) string {
	if a, ok := w.aliases[path]; ok {
		return a
	}
	all := map[string]bool{}
	for _, n := range w.moduleNames() {
		all[n] = true
	}
	for i := range w.m.Funcs {
		f := &w.m.Funcs[i]
		for _, p := range f.Params {
			all[p.Name] = true
		}
		if f.Recv != nil {
			all[f.Recv.Name] = true
		}
		boundNames(f.Body, all)
	}
	for _, a := range w.aliases {
		all[a] = true
	}
	base := path[lastSlash(path)+1:]
	a := base
	for all[a] || closureNames[a] {
		a += "_"
	}
	w.aliases[path] = a
	return a
}

func lastSlash(p string) int {
	for i := len(p) - 1; i >= 0; i-- {
		if p[i] == '/' {
			return i
		}
	}
	return -1
}

// underKind is the kind of t, or of the type a named enum or newtype stands on.
func (w *writer) underKind(t Type) string {
	for t.K == "named" {
		td := w.types[t.Name]
		if td == nil || td.Under == nil {
			return t.K
		}
		t = *td.Under
	}
	return t.K
}

// typeSrc is t as Go source.
func (w *writer) typeSrc(t Type) string {
	var b bytes.Buffer
	x := w.typ(t)
	// The text is parsed again as part of the helper: a name that is not an identifier would be
	// code by then, so it is refused here.
	if err := checkNode(x); err != nil {
		w.refuse("sorted_keys: %v", err)
		return "int"
	}
	if err := printer.Fprint(&b, token.NewFileSet(), x); err != nil {
		w.refuse("type: %v", err)
	}
	return b.String()
}
