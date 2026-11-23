#+feature dynamic-literals
package codespec_read_odin

import "core:encoding/json"
import "core:fmt"
import "core:odin/ast"
import "core:strconv"
import "core:unicode/utf8"

Expr :: struct {node: Object, ty: string}

refuse :: proc(r: ^Reader, message: string) {
	if r.refusal == "" { r.refusal = message }
}

bad_expr :: proc(r: ^Reader, reason: string) -> Expr {
	refuse(r, reason)
	return {}
}

typed :: proc(node: Object, ty: string) -> Expr {
	result := node
	result["t"] = type_object(ty)
	return Expr{result, ty}
}

local_var :: proc(n, t: string) -> Expr { return typed(Object{"k" = "var", "name" = n}, t) }

lower_expr :: proc(r: ^Reader, e: ^ast.Expr, env: ^Types, expect := "") -> Expr {
	if e == nil { return bad_expr(r, "expected an expression") }
	v: Expr
	#partial switch x in e.derived_expr {
	case ^ast.Paren_Expr: v = lower_expr(r, x.expr, env, expect)
	case ^ast.Ident:
		ty, bound := env.values[x.name]
		if bound {
			v = local_var(x.name, ty)
		} else if x.name == "true" || x.name == "false" {
			v = typed(Object{"k" = "lit", "value" = x.name}, "bool")
		} else {
			return bad_expr(r, "unknown name in scalar expression")
		}
	case ^ast.Basic_Lit:
		if x.tok.kind == .Integer {
			v = int_literal(r, x.tok.text, false, expect)
		} else if x.tok.kind == .String {
			s, _, ok := strconv.unquote_string(x.tok.text)
			if !ok || !utf8.valid_string(s) { return bad_expr(r, "string literal outside the UTF-8 subset") }
			lit, err := json.unparse(s)
			if err != nil { return bad_expr(r, "invalid string literal") }
			v = typed(Object{"k" = "lit", "value" = lit}, "string")
		} else {
			return bad_expr(r, "literal outside the scalar subset")
		}
	case ^ast.Unary_Expr:
		op := x.op.text
		lit, literal := x.expr.derived_expr.(^ast.Basic_Lit)
		if op == "-" && literal && lit.tok.kind == .Integer {
			v = int_literal(r, lit.tok.text, true, expect)
		} else {
			arg := lower_expr(r, x.expr, env, expect)
			if !((op == "+" || op == "-") && signed(arg.ty) || op == "!" && arg.ty == "bool") {
				return bad_expr(r, "unsupported unary operand or operator")
			}
			v = typed(Object{"k" = "unop", "op" = op, "x" = arg.node}, arg.ty)
		}
	case ^ast.Binary_Expr:
		op := x.op.text
		arithmetic := op == "+" || op == "-" || op == "*" || op == "/" || op == "%"
		logical := op == "&&" || op == "||"
		equality := op == "==" || op == "!="
		ordered := op == "<" || op == "<=" || op == ">" || op == ">="
		want := expect if arithmetic else ""
		left := lower_expr(r, x.left, env, want)
		right := lower_expr(r, x.right, env, left.ty)
		if !(arithmetic && signed(left.ty) || logical && left.ty == "bool" ||
		     equality || ordered && (signed(left.ty) || left.ty == "string")) {
			return bad_expr(r, "unsupported binary operand or operator")
		}
		if op == "/" && !safe_division(left, right, env) {
			return bad_expr(r, "Odin signed division can trap on the minimum integer divided by -1")
		}
		t := left.ty if arithmetic else "bool"
		v = typed(Object{"k" = "binop", "op" = op, "x" = left.node, "y" = right.node}, t)
	case ^ast.Call_Expr:
		name := ident(x.expr)
		idx, ok := r.lookup[name]
		if name in env.values || x.ellipsis.text != "" { return bad_expr(r, "expected a known local procedure call") }
		if !ok && (signed(name) || name == "bool" || name == "string") &&
		   len(x.args) == 1 && literal_operand(x.args[0], env) {
			v = lower_expr(r, x.args[0], env, name)
		} else if ok {
			f := r.functions[idx]
			if len(x.args) != len(f.params) || len(f.results) > 1 { return bad_expr(r, "call outside the scalar subset") }
			args := Array{}
			for a, i in x.args { append(&args, lower_expr(r, a, env, f.params[i].ty).node) }
			t := f.results[0] if len(f.results) == 1 else "unit"
			v = typed(Object{"k" = "call", "name" = f.id, "args" = args}, t)
		} else {
			return bad_expr(r, "expected a known local call or a scalar literal cast")
		}
	case: return bad_expr(r, "expression outside the typed Odin subset")
	}
	if expect != "" && v.ty != expect { return bad_expr(r, "expression type differs from its operand or declared result") }
	return v
}

safe_division :: proc(left, right: Expr, env: ^Types) -> bool {
	if n, ok := right.node["value"].(string); ok && n != "-1" { return true }
	kind, _ := right.node["k"].(string)
	if n, ok := right.node["name"].(string); ok && kind == "var" && env.not_minus_one[n] {
		return true
	}
	if n, ok := left.node["value"].(string); ok {
		minimum := "-9223372036854775808"
		switch left.ty {
		case "i8": minimum = "-128"
		case "i16": minimum = "-32768"
		case "i32": minimum = "-2147483648"
		}
		return n != minimum
	}
	return false
}

// A branch known to exclude -1 makes signed division defined even for a
// minimum-valued dividend. Track this fact only while that binding is unchanged.
minus_one_test :: proc(e: ^ast.Expr) -> (name: string, equal: bool) {
	c, ok := e.derived_expr.(^ast.Binary_Expr)
	if !ok || (c.op.text != "==" && c.op.text != "!=") { return }
	for side in 0..<2 {
		a, b := c.left, c.right
		if side != 0 { a, b = b, a }
		u, negative := b.derived_expr.(^ast.Unary_Expr)
		if !negative || u.op.text != "-" { continue }
		n, literal := u.expr.derived_expr.(^ast.Basic_Lit)
		if literal && n.tok.kind == .Integer && n.tok.text == "1" { return ident(a), c.op.text == "==" }
	}
	return
}

returns_immediately :: proc(s: ^ast.Stmt) -> bool {
	if s == nil { return false }
	if b, ok := s.derived_stmt.(^ast.Block_Stmt); ok {
		if len(b.stmts) == 1 {
			_, returns := b.stmts[0].derived_stmt.(^ast.Return_Stmt)
			return returns
		}
	}
	return false
}

literal_operand :: proc(arg: ^ast.Expr, env: ^Types) -> bool {
	#partial switch x in arg.derived_expr {
	case ^ast.Basic_Lit: return x.tok.kind == .Integer || x.tok.kind == .String
	case ^ast.Ident: return (x.name == "true" || x.name == "false") && x.name not_in env.values
	case ^ast.Paren_Expr: return literal_operand(x.expr, env)
	case ^ast.Unary_Expr:
		lit, ok := x.expr.derived_expr.(^ast.Basic_Lit)
		return (x.op.text == "-" || x.op.text == "+") && ok && lit.tok.kind == .Integer
	}
	return false
}

// The standard integer parser accumulates into the target width. Accumulate
// against its bound first so an Odin literal cannot silently turn into another.
integer_value :: proc(text: string, negative: bool) -> (i64, bool) {
	base := u64(10)
	s := text
	if len(s) > 2 && s[0] == '0' {
		switch s[1] {
		case 'b': base = 2
		case 'o': base = 8
		case 'z': base = 12
		case 'x': base = 16
		case 'd': base = 10
		case: // The leading zero is an ordinary decimal digit.
		}
		if s[1] == 'b' || s[1] == 'o' || s[1] == 'z' || s[1] == 'x' || s[1] == 'd' { s = s[2:] }
	}
	limit := u64(max(i64))
	if negative { limit += 1 }
	n := u64(0)
	seen := false
	for c in s {
		if c == '_' { continue }
		d: u64
		switch {
		case c >= '0' && c <= '9': d = u64(c-'0')
		case c >= 'A' && c <= 'F': d = u64(c-'A'+10)
		case c >= 'a' && c <= 'f': d = u64(c-'a'+10)
		case: return 0, false
		}
		if d >= base || n > (limit-d)/base { return 0, false }
		n = n*base + d
		seen = true
	}
	return i64(-i128(n)) if negative else i64(n), seen
}

int_literal :: proc(r: ^Reader, text: string, negative: bool, expect: string) -> Expr {
	n, ok := integer_value(text, negative)
	if !ok { return bad_expr(r, "integer literal exceeds the signed 64-bit domain") }
	t := expect if signed(expect) else "int"
	if !fits(n, t) { return bad_expr(r, "integer literal outside its declared width") }
	return typed(Object{"k" = "lit", "value" = fmt.aprint(n)}, t)
}

fits :: proc(n: i64, t: string) -> bool {
	switch t {
	case "i8": return n >= -128 && n <= 127
	case "i16": return n >= -32_768 && n <= 32_767
	case "i32": return n >= -2_147_483_648 && n <= 2_147_483_647
	case: return true
	}
}

clone_env :: proc(env: ^Types) -> ^Types {
	copy := new(Types)
	for k,v in env.values { copy.values[k] = v }
	for k,v in env.not_minus_one { copy.not_minus_one[k] = v }
	return copy
}

lower_block :: proc(r: ^Reader, f: ^Function, s: ^ast.Stmt, env: ^Types) -> Array {
	body := Array{}
	b, ok := s.derived_stmt.(^ast.Block_Stmt)
	if !ok || b.label != nil {
		append(&body, take_stmt(r, f, s, env))
		return body
	}
	for item in b.stmts {
		if _, empty := item.derived_stmt.(^ast.Empty_Stmt); empty { continue }
		append(&body, take_stmt(r, f, item, env))
	}
	return body
}

take_stmt :: proc(r: ^Reader, f: ^Function, s: ^ast.Stmt, env: ^Types) -> Object {
	prior := r.refusal
	r.refusal = ""
	st := lower_stmt(r, f, s, env)
	reason := r.refusal
	r.refusal = prior
	if reason != "" {
		env.not_minus_one = {}
		p := position(s.pos)
		src := f.file.src[s.pos.offset:s.end.offset]
		append(&r.refusals, Object{"pos" = p, "what" = src, "reason" = reason})
		return Object{"k" = "foreign", "lang" = "odin", "src" = src, "pos" = p, "reason" = reason}
	}
	return st
}

lower_stmt :: proc(r: ^Reader, f: ^Function, s: ^ast.Stmt, env: ^Types) -> Object {
	p := position(s.pos)
	#partial switch x in s.derived_stmt {
	case ^ast.Return_Stmt:
		if len(x.results) == len(f.results) {
			values := Array{}
			for expr, i in x.results { append(&values, lower_expr(r, expr, env, f.results[i]).node) }
			return Object{"k" = "return", "pos" = p, "results" = values}
		}
	case ^ast.Value_Decl:
		if len(x.names) == 1 && len(x.values) == 1 && x.is_mutable && !x.is_using && len(x.attributes) == 0 {
			n := ident(x.names[0])
			t := ident(x.type)
			if n != "" && n != "_" && n not_in env.values && (x.type == nil || signed(t) || t == "bool" || t == "string") {
				v := lower_expr(r, x.values[0], env, t)
				if r.refusal == "" { env.values[n] = v.ty }
				return Object{"k" = "let", "pos" = p, "names" = Array{n}, "rhs" = Array{v.node}}
			}
		}
	case ^ast.Assign_Stmt:
		if len(x.lhs) == 1 && len(x.rhs) == 1 && x.op.text == "=" {
			n := ident(x.lhs[0])
			if t, found := env.values[n]; found {
				v := lower_expr(r, x.rhs[0], env, t)
				env.not_minus_one[n] = false
				return Object{"k" = "assign", "pos" = p, "lhs" = Array{local_var(n,t).node}, "rhs" = Array{v.node}}
			}
		}
	case ^ast.If_Stmt:
		if x.init == nil && x.label == nil {
			test := lower_expr(r, x.cond, env, "bool")
			if r.refusal != "" { return {} }
			yes, no := clone_env(env), clone_env(env)
			n, equal := minus_one_test(x.cond)
			if n != "" && signed(env.values[n]) {
				if equal { no.not_minus_one[n] = true } else { yes.not_minus_one[n] = true }
			}
			st := Object{"k" = "if", "cond" = test.node, "then" = lower_block(r, f, x.body, yes), "pos" = p}
			if x.else_stmt != nil { st["else"] = lower_block(r, f, x.else_stmt, no) }
			// A branch may reassign a previously guarded divisor. Retain a
			// fact after the if only when every path reaching here carries it.
			env.not_minus_one = {}
			for key,v in yes.not_minus_one {
				if v && (no.not_minus_one[key] || returns_immediately(x.else_stmt)) {
					env.not_minus_one[key] = true
				}
			}
			if returns_immediately(x.body) {
				for key,v in no.not_minus_one { if v { env.not_minus_one[key] = true } }
			}
			return st
		}
	case ^ast.Expr_Stmt:
		if _, ok := x.expr.derived_expr.(^ast.Call_Expr); ok {
			v := lower_expr(r, x.expr, env)
			return Object{"k" = "expr", "x" = v.node, "pos" = p}
		}
	}
	refuse(r, "statement outside the typed Odin subset")
	return {}
}
