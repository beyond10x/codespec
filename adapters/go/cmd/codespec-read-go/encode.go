package main

import "fmt"

// encode turns the frontend's module into a cir/1 document. Nodes are maps: encoding/json writes
// their keys sorted and the codespec CLI re-encodes the module canonically after validating it,
// so field order here carries no meaning.

type node = map[string]any

func encode(m *Module) (node, error) {
	e := &enc{}
	doc := node{
		"format": "cir/1",
		"name":   m.Name,
		"reader": node{"language": "go", "name": "codespec-read-go", "version": version},
		"types":  e.list(len(m.Types), func(i int) any { return e.typeDecl(m.Types[i]) }),
		"funcs":  e.list(len(m.Funcs), func(i int) any { return e.fn(m.Funcs[i]) }),
		"layout": nonNil(m.Layout),
	}
	if m.Doc != "" {
		doc["doc"] = m.Doc
	}
	if len(m.Consts) > 0 {
		doc["consts"] = e.list(len(m.Consts), func(i int) any {
			c := m.Consts[i]
			return node{"name": c.Name, "t": e.typ(c.T), "value": e.expr(c.Value), "pos": c.Pos}
		})
	}
	if len(m.Errors) > 0 {
		doc["errors"] = e.list(len(m.Errors), func(i int) any {
			v := m.Errors[i]
			return node{"name": v.Name, "message": v.Message, "pos": v.Pos}
		})
	}
	if len(m.Refusals) > 0 {
		doc["refusals"] = e.list(len(m.Refusals), func(i int) any {
			r := m.Refusals[i]
			return node{"pos": r.Pos, "what": r.What, "reason": r.Reason}
		})
	}
	return doc, e.err
}

type enc struct{ err error }

func (e *enc) fail(format string, a ...any) {
	if e.err == nil {
		e.err = fmt.Errorf(format, a...)
	}
}

func nonNil(s []string) []string {
	if s == nil {
		return []string{}
	}
	return s
}

func (e *enc) list(n int, f func(int) any) []any {
	out := make([]any, 0, n)
	for i := 0; i < n; i++ {
		out = append(out, f(i))
	}
	return out
}

func (e *enc) typ(t Type) node {
	switch t.K {
	case "int":
		w := "platform"
		if t.Bits != 0 {
			w = fmt.Sprint(t.Bits)
		}
		return node{"k": "int", "width": w, "signed": true}
	case "bool", "string", "error", "any", "unit":
		return node{"k": t.K}
	case "named":
		return node{"k": "named", "name": t.Name}
	case "ptr", "slice":
		return node{"k": t.K, "elem": e.typ(*t.Elem)}
	case "map":
		return node{"k": "map", "key": e.typ(*t.Key), "elem": e.typ(*t.Elem)}
	case "chan":
		return node{"k": "chan", "elem": e.typ(*t.Elem), "dir": t.Dir}
	case "tuple":
		return node{"k": "tuple", "elems": e.list(len(t.Elems), func(i int) any { return e.typ(t.Elems[i]) })}
	}
	e.fail("type %s %q has no cir/1 form", t.K, t.Name)
	return node{"k": "any"}
}

func (e *enc) typeDecl(td TypeDecl) node {
	n := node{"k": td.K, "name": td.Name, "exported": td.Exported, "pos": td.Pos}
	if td.Doc != "" {
		n["doc"] = td.Doc
	}
	switch td.K {
	case "struct":
		n["fields"] = e.list(len(td.Fields), func(i int) any {
			return node{"name": td.Fields[i].Name, "t": e.typ(td.Fields[i].T)}
		})
	case "newtype":
		n["under"] = e.typ(*td.Under)
	case "enum":
		n["under"] = e.typ(*td.Under)
		n["variants"] = e.list(len(td.Variants), func(i int) any {
			return node{"name": td.Variants[i].Name, "value": e.expr(td.Variants[i].Value)}
		})
	}
	return n
}

func (e *enc) fn(f Func) node {
	n := node{
		"name":     f.Name,
		"exported": f.Exported,
		"params":   e.list(len(f.Params), func(i int) any { return node{"name": f.Params[i].Name, "t": e.typ(f.Params[i].T)} }),
		"results":  e.list(len(f.Results), func(i int) any { return e.typ(f.Results[i]) }),
		"body":     e.stmts(f.Body),
		"pos":      f.Pos,
	}
	if f.Recv != nil {
		n["recv"] = node{"name": f.Recv.Name, "t": e.typ(f.Recv.T)}
	}
	if f.Doc != "" {
		n["doc"] = f.Doc
	}
	return n
}

func (e *enc) stmts(b []Stmt) []any {
	return e.list(len(b), func(i int) any { return e.stmt(b[i]) })
}

func (e *enc) exprs(xs []Expr) []any {
	return e.list(len(xs), func(i int) any { return e.expr(xs[i]) })
}

func (e *enc) stmt(s Stmt) node {
	n := node{"k": s.K, "pos": s.Pos}
	switch s.K {
	case "let":
		n["names"], n["rhs"] = s.Names, e.exprs(s.Rhs)
	case "var":
		n["name"], n["t"] = s.Names[0], e.typ(*s.T)
	case "assign":
		n["lhs"], n["rhs"] = e.exprs(s.Lhs), e.exprs(s.Rhs)
		if s.Op != "" {
			n["op"] = s.Op
		}
	case "incr", "decr", "expr":
		n["x"] = e.expr(*s.X)
	case "if":
		n["cond"], n["then"] = e.expr(*s.Cond), e.stmts(s.Then)
		if len(s.Else) > 0 {
			n["else"] = e.stmts(s.Else)
		}
	case "for_range":
		n["key"], n["val"], n["over"], n["body"] = s.Key, s.Val, e.expr(*s.Over), e.stmts(s.Body)
	case "for":
		if s.Init != nil {
			n["init"] = e.stmt(*s.Init)
		}
		if s.Cond != nil {
			n["cond"] = e.expr(*s.Cond)
		}
		if s.Post != nil {
			n["post"] = e.stmt(*s.Post)
		}
		n["body"] = e.stmts(s.Body)
	case "break", "continue":
	case "switch":
		if s.Tag != nil {
			n["tag"] = e.expr(*s.Tag)
		}
		n["cases"] = e.list(len(s.Cases), func(i int) any {
			c := s.Cases[i]
			return node{"values": e.exprs(c.Values), "body": e.stmts(c.Body)}
		})
	case "return":
		n["results"] = e.exprs(s.Results)
	case "send":
		n["chan"], n["x"] = e.expr(*s.Chan), e.expr(*s.X)
	case "foreign":
		n["lang"], n["src"], n["reason"] = s.Lang, s.Src, s.Reason
	default:
		e.fail("statement %s at %s has no cir/1 form", s.K, s.Pos)
	}
	return n
}

func (e *enc) expr(x Expr) node {
	if x.T == nil {
		e.fail("expression %s %q has no type", x.K, x.Name)
		return node{"k": x.K}
	}
	n := node{"k": x.K, "t": e.typ(*x.T)}
	switch x.K {
	case "lit":
		n["value"] = x.Value
	case "nil", "make", "zero":
	case "var", "const", "errvar":
		n["name"] = x.Name
	case "field":
		n["x"], n["name"] = e.expr(*x.X), x.Name
	case "index":
		n["x"], n["index"] = e.expr(*x.X), e.expr(*x.Y)
	case "call", "intrinsic":
		n["name"], n["args"] = x.Name, e.exprs(x.Args)
	case "method":
		n["x"], n["name"], n["args"] = e.expr(*x.X), x.Name, e.exprs(x.Args)
	case "binop":
		n["op"], n["x"], n["y"] = x.Op, e.expr(*x.X), e.expr(*x.Y)
	case "unop":
		n["op"], n["x"] = x.Op, e.expr(*x.X)
	case "struct":
		n["name"] = x.Name
		n["fields"] = e.list(len(x.FieldNames), func(i int) any {
			return node{"name": x.FieldNames[i], "value": e.expr(x.Args[i])}
		})
	case "conv":
		n["name"], n["x"] = x.Name, e.expr(*x.X)
	case "to_error":
		n["x"] = e.expr(*x.X)
	default:
		e.fail("expression %s has no cir/1 form", x.K)
	}
	return n
}
