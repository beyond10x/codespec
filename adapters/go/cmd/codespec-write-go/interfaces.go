package main

import (
	"go/ast"
	"go/token"
)

// ifaceDecl writes the module's own interface: `type Name interface { M(params) results }`.
func (w *writer) ifaceDecl(in *Interface) []piece {
	w.pos = in.Pos
	fl := &ast.FieldList{}
	for _, m := range in.Methods {
		fl.List = append(fl.List, &ast.Field{Names: []*ast.Ident{ident(m.Name)}, Type: w.funcType(nil, m.Params, m.Results)})
	}
	spec := &ast.TypeSpec{Name: ident(w.names.decl(in.Name)), Type: &ast.InterfaceType{Methods: fl}}
	return []piece{{doc: in.Doc, node: &ast.GenDecl{Tok: token.TYPE, Specs: []ast.Spec{spec}}}}
}

// isIface tells whether name is one of the module's own interfaces.
func (w *writer) isIface(name string) bool {
	for _, in := range w.m.Interfaces {
		if in.Path == "" && in.Name == name {
			return true
		}
	}
	return false
}
