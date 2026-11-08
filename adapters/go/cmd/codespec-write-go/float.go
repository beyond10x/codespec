package main

import (
	"go/ast"
	"go/token"
	"strings"
)

// floatLit writes a cir/1 float literal so that Go reads a float constant: a value spelled
// without a point or an exponent (`2`, `1000`) gets `.0`. A negative value is a negation of the
// literal.
func (w *writer) floatLit(value string) ast.Expr {
	digits := strings.TrimPrefix(value, "-")
	if !strings.ContainsAny(digits, ".e") {
		digits += ".0"
	}
	var x ast.Expr = &ast.BasicLit{Kind: token.FLOAT, Value: digits}
	if strings.HasPrefix(value, "-") {
		x = &ast.UnaryExpr{Op: token.SUB, X: x}
	}
	return x
}
