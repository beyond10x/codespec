package main

import (
	"fmt"
	"sort"
)

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
	if len(m.Vars) > 0 {
		doc["vars"] = e.list(len(m.Vars), func(i int) any {
			v := m.Vars[i]
			n := node{"name": v.Name, "t": e.typ(v.T), "pos": v.Pos}
			if v.Init != nil {
				n["init"] = e.expr(*v.Init)
			}
			if v.Foreign != nil {
				n["foreign"] = node{"lang": v.Foreign.Lang, "src": v.Foreign.Src, "reason": v.Foreign.Reason}
			}
			return n
		})
	}
	if ex := e.externs(m.Externs); len(ex) > 0 {
		doc["externs"] = ex
	}
	if is := e.interfaces(m.Interfaces); len(is) > 0 {
		doc["interfaces"] = is
	}
	if len(m.Refusals) > 0 {
		doc["refusals"] = e.list(len(m.Refusals), func(i int) any {
			r := m.Refusals[i]
			return node{"pos": r.Pos, "what": r.What, "reason": r.Reason}
		})
	}
	if m.Path != "" {
		doc["path"] = m.Path
	}
	if len(m.Opaque) > 0 {
		doc["opaque"] = e.list(len(m.Opaque), func(i int) any { return e.opaque(m.Opaque[i]) })
	}
	if len(m.Methods) > 0 {
		doc["methods"] = e.list(len(m.Methods), func(i int) any {
			ms := m.Methods[i]
			return node{"t": ms.T, "methods": e.list(len(ms.Methods), func(j int) any {
				x := ms.Methods[j]
				return node{"name": x.Name, "sig": x.Sig, "target": e.callRef(x.Target)}
			})}
		})
	}
	if len(m.Inits) > 0 {
		doc["inits"] = e.callRefs(m.Inits)
	}
	if len(m.Linknames) > 0 {
		doc["linknames"] = e.list(len(m.Linknames), func(i int) any {
			return node{"name": m.Linknames[i].Name, "target": m.Linknames[i].Target}
		})
	}
	return doc, e.err
}

// enc encodes one module; used records every extern a call names, as its key.
type enc struct {
	err  error
	used map[string]bool
}

// externs declares the candidates some encoded call names, sorted by path, then receiver type
// name ("" for a function), then name.
func (e *enc) externs(cands []Extern) []any {
	var keep []Extern
	for _, x := range cands {
		if e.used[x.key()] {
			keep = append(keep, x)
		}
	}
	sort.Slice(keep, func(i, j int) bool {
		if keep[i].Path != keep[j].Path {
			return keep[i].Path < keep[j].Path
		}
		if a, b := keep[i].recvTypeName(), keep[j].recvTypeName(); a != b {
			return a < b
		}
		return keep[i].Name < keep[j].Name
	})
	return e.list(len(keep), func(i int) any {
		x := keep[i]
		n := node{
			"path":    x.Path,
			"name":    x.Name,
			"params":  e.list(len(x.Params), func(j int) any { return e.typ(x.Params[j]) }),
			"results": e.list(len(x.Results), func(j int) any { return e.typ(x.Results[j]) }),
		}
		if x.Variadic {
			n["variadic"] = true
		}
		if x.Recv != nil {
			n["recv"] = e.typ(*x.Recv)
		}
		return e.tparams(n, x.Tparams)
	})
}

// interfaces declares the package's own interfaces in source order, then those of other packages
// some encoded interface call goes through, sorted by path, then name.
func (e *enc) interfaces(all []Interface) []any {
	var keep, other []Interface
	for _, i := range all {
		if i.Path == "" {
			keep = append(keep, i)
		} else if e.used["iface:"+i.Path+"."+i.Name] {
			other = append(other, i)
		}
	}
	sort.Slice(other, func(a, b int) bool {
		if other[a].Path != other[b].Path {
			return other[a].Path < other[b].Path
		}
		return other[a].Name < other[b].Name
	})
	keep = append(keep, other...)
	types := func(ts []Type) []any { return e.list(len(ts), func(j int) any { return e.typ(ts[j]) }) }
	return e.list(len(keep), func(k int) any {
		i := keep[k]
		n := node{"name": i.Name, "exported": i.Exported, "methods": e.list(len(i.Methods), func(j int) any {
			m := i.Methods[j]
			return node{"name": m.Name, "params": types(m.Params), "results": types(m.Results)}
		})}
		for key, v := range map[string]string{"path": i.Path, "doc": i.Doc, "pos": i.Pos} {
			if v != "" {
				n[key] = v
			}
		}
		return n
	})
}

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
		return node{"k": "int", "width": w, "signed": !t.Unsigned}
	case "bool", "string", "error", "any", "unit":
		return node{"k": t.K}
	case "named":
		return e.targs(node{"k": "named", "name": t.Name}, t.Targs)
	case "extern":
		return e.targs(node{"k": "extern", "path": t.Path, "name": t.Name}, t.Targs)
	case "param":
		return node{"k": "param", "name": t.Name}
	case "ptr", "slice":
		return node{"k": t.K, "elem": e.typ(*t.Elem)}
	case "array":
		return node{"k": "array", "len": t.Len, "elem": e.typ(*t.Elem)}
	case "float":
		return node{"k": "float", "bits": fmt.Sprint(t.Bits)}
	case "map":
		return node{"k": "map", "key": e.typ(*t.Key), "elem": e.typ(*t.Elem)}
	case "chan":
		return node{"k": "chan", "elem": e.typ(*t.Elem), "dir": t.Dir}
	case "tuple":
		return node{"k": "tuple", "elems": e.list(len(t.Elems), func(i int) any { return e.typ(t.Elems[i]) })}
	case "func":
		return node{
			"k":       "func",
			"params":  e.list(len(t.Params), func(i int) any { return e.typ(t.Params[i]) }),
			"results": e.list(len(t.Results), func(i int) any { return e.typ(t.Results[i]) }),
		}
	}
	e.fail("type %s %q has no cir/1 form", t.K, t.Name)
	return node{"k": "any"}
}

// targs sets n's `targs` to ts, when there are any.
func (e *enc) targs(n node, ts []Type) node {
	if len(ts) > 0 {
		n["targs"] = e.list(len(ts), func(i int) any { return e.typ(ts[i]) })
	}
	return n
}

// tparams sets n's `tparams` to ps, when there are any.
func (e *enc) tparams(n node, ps []TypeParam) node {
	if len(ps) > 0 {
		n["tparams"] = e.list(len(ps), func(i int) any {
			return node{"name": ps[i].Name, "constraint": e.typ(ps[i].Constraint)}
		})
	}
	return n
}

func (e *enc) typeDecl(td TypeDecl) node {
	n := e.tparams(node{"k": td.K, "name": td.Name, "exported": td.Exported, "pos": td.Pos}, td.Tparams)
	if td.Doc != "" {
		n["doc"] = td.Doc
	}
	switch td.K {
	case "struct":
		n["fields"] = e.list(len(td.Fields), func(i int) any {
			n := node{"name": td.Fields[i].Name, "t": e.typ(td.Fields[i].T)}
			if td.Fields[i].Embedded {
				n["embedded"] = true
			}
			return n
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

// param is a function's parameter; `variadic` only where it is.
func (e *enc) param(p Param) node {
	n := node{"name": p.Name, "t": e.typ(p.T)}
	if p.Variadic {
		n["variadic"] = true
	}
	return n
}

func (e *enc) fn(f Func) node {
	n := node{
		"name":     f.Name,
		"exported": f.Exported,
		"params":   e.list(len(f.Params), func(i int) any { return e.param(f.Params[i]) }),
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
	return e.tparams(n, f.Tparams)
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
		n["calls"] = e.callRefs(s.Calls)
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
	case "lit", "float":
		n["value"] = x.Value
	case "nil", "make", "zero":
	case "var", "const", "errvar", "global":
		n["name"] = x.Name
	case "field":
		n["x"], n["name"] = e.expr(*x.X), x.Name
	case "index":
		n["x"], n["index"] = e.expr(*x.X), e.expr(*x.Y)
	case "call", "intrinsic":
		n["name"], n["args"] = x.Name, e.exprs(x.Args)
		e.targs(n, x.Targs)
	case "method":
		n["x"], n["name"], n["args"] = e.expr(*x.X), x.Name, e.exprs(x.Args)
	case "extern":
		if e.used == nil {
			e.used = map[string]bool{}
		}
		e.used[x.Path+"."+x.Name] = true
		n["path"], n["name"], n["args"] = x.Path, x.Name, e.exprs(x.Args)
		e.targs(n, x.Targs)
	case "extern_method":
		if e.used == nil {
			e.used = map[string]bool{}
		}
		e.used[x.Path+"."+x.TypeName+"."+x.Name] = true
		n["path"], n["type"], n["name"] = x.Path, x.TypeName, x.Name
		n["recv"], n["args"] = e.expr(*x.X), e.exprs(x.Args)
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
	case "func_ref":
		n["name"] = x.Name
		if x.X != nil {
			n["recv"] = e.expr(*x.X)
		}
	case "lambda":
		n["params"] = e.list(len(x.Params), func(i int) any { return node{"name": x.Params[i].Name, "t": e.typ(x.Params[i].T)} })
		n["results"] = e.list(len(x.Results), func(i int) any { return e.typ(x.Results[i]) })
		n["captures"] = nonNil(x.Captures)
		n["body"] = e.stmts(x.Body)
	case "call_value":
		n["f"], n["args"] = e.expr(*x.X), e.exprs(x.Args)
	case "iface_call":
		if e.used == nil {
			e.used = map[string]bool{}
		}
		e.used["iface:"+x.Path+"."+x.Iface] = true
		if x.Path != "" {
			n["path"] = x.Path
		}
		n["iface"], n["name"], n["recv"], n["args"] = x.Iface, x.Name, e.expr(*x.X), e.exprs(x.Args)
	case "spread", "to_iface":
		n["x"] = e.expr(*x.X)
	default:
		e.fail("expression %s has no cir/1 form", x.K)
	}
	return n
}

func (e *enc) callRefs(cs []CallRef) []any {
	return e.list(len(cs), func(i int) any { return e.callRef(cs[i]) })
}

// callRef encodes one call as cir/1's CallRef; an empty field is omitted where cir/1 allows it.
func (e *enc) callRef(c CallRef) node {
	n := node{"k": c.K, "pos": c.Pos}
	opt := func(k, v string) {
		if v != "" {
			n[k] = v
		}
	}
	switch c.K {
	case "func":
		opt("path", c.Path)
		n["name"] = c.Name
		if len(c.Consts) > 0 {
			n["consts"] = e.list(len(c.Consts), func(i int) any {
				return node{"index": c.Consts[i].Index, "value": c.Consts[i].Value}
			})
		}
	case "method":
		opt("path", c.Path)
		n["recv"], n["name"] = c.Recv, c.Name
	case "iface":
		n["iface"], n["name"] = c.Iface, c.Name
		n["methods"] = e.list(len(c.Methods), func(i int) any {
			return node{"name": c.Methods[i].Name, "sig": c.Methods[i].Sig}
		})
	case "value":
		n["sig"] = c.Sig
	case "ref":
		opt("path", c.Path)
		opt("recv", c.Recv)
		n["name"], n["sig"] = c.Name, c.Sig
	case "lambda":
		n["sig"], n["calls"] = c.Sig, e.callRefs(c.Calls)
	case "new":
		opt("path", c.Path)
		n["name"] = c.Name
	default:
		e.fail("call %s at %s has no cir/1 form", c.K, c.Pos)
	}
	return n
}

func (e *enc) opaque(o Opaque) node {
	n := node{"name": o.Name, "pos": o.Pos, "reason": o.Reason, "calls": e.callRefs(o.Calls)}
	if o.Recv != "" {
		n["recv"] = o.Recv
	}
	if o.NoBody {
		n["no_body"] = true
	}
	return n
}
