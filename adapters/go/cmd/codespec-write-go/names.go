package main

import (
	"encoding/json"
	"go/types"
	"unicode"
	"unicode/utf8"
)

// goNames is the Go spelling of every function, method and struct field, fixed before anything is
// written. Two rules, both deterministic, change a cir/1 name:
//
//  1. Export. A function or method cir/1 marks exported whose name starts with a lower-case
//     letter (a Java public method, `id`) gets an upper-case first letter (`Id`), since Go exports
//     by spelling. If that name is already held (by a package-level name for a function, by
//     another method of the type for a method) it gets `_` appended until it is free. Names that
//     already agree with Go's rule, every name a Go or Rust reader emits, are unchanged.
//  2. Field and method of one name. Go rejects a field and a method of one name on one type (a
//     Java field `balance` beside its getter `balance()`). The method keeps its name, the
//     type's API; the field gets `_` appended until no field or method of the type holds it.
//  3. Predeclared names. A package-level declaration (function, type, enum variant, constant,
//     sentinel error) named like one of Go's predeclared identifiers (a Java static method
//     `make`, `len`, `append`) would hide it from the whole package, including the code the
//     writer adds itself (sorted_keys calls make, len and append). It gets `_` appended until
//     no package-level name holds it, and every reference follows. Every predeclared identifier
//     is lower case, so no exported name changes.
//
// Struct fields carry no export mark in cir/1 and keep their names but for rule 2.
type goNames struct {
	funcs   map[string]string // free function name -> Go name
	methods map[string]string // "<type>.<method>" -> Go name
	fields  map[string]string // "<type>.<field>" -> Go name
	pkg     map[string]string // type, variant, constant or error renamed by rule 3 -> Go name
}

// predeclared reports whether name is one of Go's predeclared identifiers.
func predeclared(name string) bool { return types.Universe.Lookup(name) != nil }

// decl is the Go name of a package-level type, enum variant, constant or sentinel error.
func (n *goNames) decl(name string) string {
	if g, ok := n.pkg[name]; ok {
		return g
	}
	return name
}

func exportName(name string, exported bool) string {
	r, n := utf8.DecodeRuneInString(name)
	if !exported || !unicode.IsLower(r) {
		return name
	}
	return string(unicode.ToUpper(r)) + name[n:]
}

// claim returns want, or want with `_` appended until taken does not hold it, and marks it taken.
func claim(taken map[string]bool, want string) string {
	for taken[want] {
		want += "_"
	}
	taken[want] = true
	return want
}

func newGoNames(m *Module) *goNames {
	n := &goNames{funcs: map[string]string{}, methods: map[string]string{}, fields: map[string]string{}, pkg: map[string]string{}}
	pkg := map[string]bool{}
	for _, td := range m.Types {
		pkg[td.Name] = true
		if td.K == "enum" {
			var vs []Variant
			if json.Unmarshal(td.Variants, &vs) == nil {
				for _, v := range vs {
					pkg[v.Name] = true
				}
			}
		}
	}
	for _, c := range m.Consts {
		pkg[c.Name] = true
	}
	for _, e := range m.Errors {
		pkg[e.Name] = true
	}
	for _, f := range m.Funcs {
		if f.Recv == nil {
			pkg[f.Name] = true
		}
	}
	// Rule 3: types, enum variants, constants and sentinel errors named like a predeclared
	// identifier, in declaration order.
	rename := func(name string) {
		if predeclared(name) {
			n.pkg[name] = claim(pkg, name)
		}
	}
	for _, td := range m.Types {
		rename(td.Name)
		if td.K == "enum" {
			var vs []Variant
			if json.Unmarshal(td.Variants, &vs) == nil {
				for _, v := range vs {
					rename(v.Name)
				}
			}
		}
	}
	for _, c := range m.Consts {
		rename(c.Name)
	}
	for _, e := range m.Errors {
		rename(e.Name)
	}
	methodsOf := map[string]map[string]bool{}
	for _, f := range m.Funcs {
		if f.Recv == nil {
			continue
		}
		t := recvTypeName(f.Recv.T)
		if methodsOf[t] == nil {
			methodsOf[t] = map[string]bool{}
		}
		methodsOf[t][f.Name] = true
	}
	goMethods := map[string]map[string]bool{}
	for _, f := range m.Funcs {
		name := exportName(f.Name, f.Exported)
		if f.Recv == nil {
			if name != f.Name || predeclared(name) {
				name = claim(pkg, name)
			}
			n.funcs[f.Name] = name
			continue
		}
		t := recvTypeName(f.Recv.T)
		if name != f.Name {
			name = claim(methodsOf[t], name)
		}
		n.methods[t+"."+f.Name] = name
		if goMethods[t] == nil {
			goMethods[t] = map[string]bool{}
		}
		goMethods[t][name] = true
	}
	for _, td := range m.Types {
		if td.K != "struct" {
			continue
		}
		taken := map[string]bool{}
		for name := range goMethods[td.Name] {
			taken[name] = true
		}
		for _, f := range td.Fields {
			taken[f.Name] = true
		}
		for _, f := range td.Fields {
			name := f.Name
			if goMethods[td.Name][name] {
				name = claim(taken, name)
			}
			n.fields[td.Name+"."+f.Name] = name
		}
	}
	return n
}

func (n *goNames) free(name string) string {
	if g, ok := n.funcs[name]; ok {
		return g
	}
	return name
}

func (n *goNames) method(typ, name string) string {
	if g, ok := n.methods[typ+"."+name]; ok {
		return g
	}
	return name
}

func (n *goNames) field(typ, name string) string {
	if g, ok := n.fields[typ+"."+name]; ok {
		return g
	}
	return name
}

func (n *goNames) funcOf(f *Func) string {
	if f.Recv == nil {
		return n.free(f.Name)
	}
	return n.method(recvTypeName(f.Recv.T), f.Name)
}

// typeNameOf is the declared type an expression's value is, through one pointer.
func typeNameOf(e *Expr) string {
	if e == nil || e.T == nil {
		return ""
	}
	return recvTypeName(*e.T)
}
