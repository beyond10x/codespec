#+feature dynamic-literals
// Read one scalar Odin package using the native parser. Signatures are collected
// before any bodies, so calls can refer to later definitions and other files.
// The conformance runner uses --signatures to call the procedures as spelled.
package codespec_read_odin

import "core:encoding/json"
import "core:fmt"
import "core:odin/ast"
import "core:odin/parser"
import "core:odin/tokenizer"
import "core:os"
import "core:slice"
import "core:strings"
import "core:unicode/utf8"

VERSION :: #config(CODESPEC_VERSION, "0.14.0")
FEATURES :: [?]string{
	"type.int", "type.bool", "type.string",
	"expr.lit", "expr.var", "expr.call", "expr.binop", "expr.unop",
	"stmt.return", "stmt.if", "stmt.let", "stmt.assign",
	"op.add", "op.sub", "op.mul", "op.div", "op.rem", "op.eq", "op.ne",
	"op.lt", "op.le", "op.gt", "op.ge", "op.land", "op.lor",
	"unop.not", "unop.plus", "unop.neg",
}

Object :: json.Object
Array :: json.Array
Types :: struct { values: map[string]string, not_minus_one: map[string]bool }

Param :: struct {name, ty: string}
Function :: struct {
	source_name, id: string,
	exported: bool,
	params: [dynamic]Param,
	results: [dynamic]string,
	proc_lit: ^ast.Proc_Lit,
	file: ^ast.File,
	pos: string,
}
Reader :: struct {
	pkg: string,
	signatures_only: bool,
	functions: [dynamic]Function,
	lookup: map[string]int,
	refusals: Array,
	refusal: string,
}

fail :: proc(message: string) -> ! {
	fmt.eprintln("codespec-read-odin:", message)
	os.exit(1)
}

emit :: proc(v: json.Value) {
	text, err := json.unparse(v, {sort_maps_by_key = true})
	if err != nil { fail("could not encode JSON") }
	fmt.println(text)
}

main :: proc() {
	if len(os.args) == 2 && os.args[1] == "--version" {
		fmt.println("codespec-read-odin", VERSION)
		return
	}
	if len(os.args) == 2 && os.args[1] == "--describe" {
		features := Array{}
		for f in FEATURES { append(&features, f) }
		emit(Object{
			"format" = "codespec-adapter/1", "lang" = "odin", "role" = "reader",
			"cir" = "cir/1", "version" = VERSION, "features" = features,
		})
		return
	}
	signatures := len(os.args) == 3 && os.args[1] == "--signatures"
	if !signatures && len(os.args) != 2 { fail("usage: codespec-read-odin [--signatures] SOURCE | --describe") }
	path := os.args[len(os.args)-1]
	paths := [dynamic]string{}
	input, stat_err := os.lstat(path, context.allocator)
	if stat_err != nil { fail(fmt.aprintf("%s: %v", path, stat_err)) }
	if input.type == .Directory {
		infos, err := os.read_all_directory_by_path(path, context.allocator)
		if err != nil { fail(fmt.aprintf("%s: %v", path, err)) }
		for info in infos {
			if strings.has_suffix(info.name, ".odin") {
				name, join_err := os.join_path({path, info.name}, context.allocator)
				if join_err != nil { fail(fmt.aprintf("%s: %v", path, join_err)) }
				append(&paths, name)
			}
		}
		slice.sort(paths[:])
	} else {
		append(&paths, path)
	}
	if len(paths) == 0 { fail(fmt.aprintf("%s: no .odin files", path)) }

	reader := Reader{signatures_only = signatures}
	total_bytes := i64(0)
	for name in paths {
		info, err := os.lstat(name, context.allocator)
		if err != nil || info.type != .Regular { fail(fmt.aprintf("%s: expected a regular Odin source file", name)) }
		if strings.has_suffix(info.name, ".odin") {
			base := strings.trim_suffix(info.name, ".odin")
			if at := strings.last_index_byte(base, '_'); at >= 0 {
				suffix := base[at+1:]
				os_kind, _ := parser.get_build_os_from_string(suffix)
				if os_kind != .Unknown || parser.get_build_arch_from_string(suffix) != .Unknown {
					fail(fmt.aprintf("%s: target-specific source file", info.name))
				}
			}
		}
		total_bytes += info.size
		if total_bytes > 1024*1024 { fail("package source exceeds 1 MiB") }
		data, read_err := os.read_entire_file(name, context.allocator)
		if read_err != nil { fail(fmt.aprintf("%s: %v", name, read_err)) }
		collect_source(&reader, info.name, string(data))
	}
	if signatures {
		emit(signature_table(&reader))
	} else {
		emit(module_document(&reader))
	}
}

ident :: proc(e: ^ast.Expr) -> string {
	if e != nil {
		if x, ok := e.derived_expr.(^ast.Ident); ok { return x.name }
	}
	return ""
}

signed :: proc(t: string) -> bool {
	return t == "int" || t == "i8" || t == "i16" || t == "i32" || t == "i64"
}

type_object :: proc(t: string) -> Object {
	if signed(t) {
		width := "platform" if t == "int" else t[1:]
		return Object{"k" = "int", "width" = width, "signed" = true}
	}
	return Object{"k" = t}
}

position :: proc(p: tokenizer.Pos) -> string { return fmt.aprintf("%s:%d", p.file, p.line) }

reject_decl :: proc(r: ^Reader, p: tokenizer.Pos, reason: string) -> ! {
	fmt.eprintf("codespec-read-odin: %s: %s\n", position(p), reason)
	os.exit(2 if r.signatures_only else 1)
}

// Use the native tokenizer to bound parser recursion before parsing. Spelling in
// comments and strings contributes no level or operator to these bounds.
check_source :: proc(r: ^Reader, label, src: string) {
	if !utf8.valid_string(src) { fail(fmt.aprintf("%s: invalid UTF-8", label)) }
	t: tokenizer.Tokenizer
	tokenizer.init(&t, src, label)
	depth, expression_tokens := 0, 0
	for {
		token := tokenizer.scan(&t)
		#partial switch token.kind {
		case .EOF: return
		case .Open_Paren, .Open_Bracket, .Open_Brace: depth += 1
		case .Close_Paren, .Close_Bracket, .Close_Brace: depth -= 1
		case .Semicolon, .Comma: expression_tokens = 0
		}
		expression_tokens += 1
		if depth > 128 || expression_tokens > 2048 {
			reject_decl(r, token.pos, "source exceeds the nesting or expression size limit")
		}
	}
}

collect_source :: proc(r: ^Reader, label, src: string) {
	check_source(r, label, src)
	file := new(ast.File)
	file.fullpath, file.src = label, src
	p := parser.default_parser()
	if !parser.parse_file(&p, file) || file.syntax_error_count > 0 { fail(fmt.aprintf("%s: Odin parse failed", label)) }
	if len(file.tags) != 0 { reject_decl(r, file.tags[0].pos, "conditional and package file tags are outside the subset") }
	if r.pkg != "" && r.pkg != file.pkg_name { fail(fmt.aprintf("%s: more than one package", label)) }
	r.pkg = file.pkg_name
	for s in file.decls {
		v, ok := s.derived_stmt.(^ast.Value_Decl)
		if !ok && r.signatures_only {
			// Imports are compiled by the native runner with the original body.
			if _, imported := s.derived_stmt.(^ast.Import_Decl); imported { continue }
			if _, imported := s.derived_stmt.(^ast.Foreign_Import_Decl); imported { continue }
		}
		if !ok || len(v.names) != 1 || len(v.values) != 1 || v.is_mutable || v.is_using || v.type != nil {
			reject_decl(r, s.pos, "expected a procedure declaration")
		}
		pl, is_proc := v.values[0].derived_expr.(^ast.Proc_Lit)
		if !is_proc || pl.body == nil || pl.type.generic || pl.type.diverging || pl.type.calling_convention != nil ||
		   len(pl.where_clauses) != 0 || pl.tags != {} || pl.type.tags != {} {
			reject_decl(r, s.pos, "unsupported procedure declaration")
		}
		name := ident(v.names[0])
		if name == "" || name == "_" { reject_decl(r, s.pos, "procedure name is not an identifier") }
		exported := true
		for attr in v.attributes {
			if len(attr.elems) != 1 || ident(attr.elems[0]) != "private" || !exported {
				reject_decl(r, attr.pos, "only @(private) procedure attributes are supported")
			}
			exported = false
		}
		id := name
		if exported && (name[0] < 'A' || name[0] > 'Z') { id = strings.to_pascal_case(name) }
		if id == "" { reject_decl(r, s.pos, "procedure has no cir/1 name") }
		for f in r.functions {
			if f.id == id || f.source_name == name { reject_decl(r, s.pos, "duplicate procedure or cir/1 name collision") }
		}
		f := Function{source_name = name, id = id, exported = exported, file = file, proc_lit = pl, pos = position(s.pos)}
		if pl.type.params != nil {
			for field in pl.type.params.list {
				t := field_type(r, field)
				if len(field.names) == 0 { reject_decl(r, field.pos, "expected a named parameter") }
				for arg in field.names {
					n := ident(arg)
					if n == "" { reject_decl(r, arg.pos, "unsupported parameter") }
					for prev in f.params {
						if prev.name == n { reject_decl(r, arg.pos, "duplicate parameter") }
					}
					append(&f.params, Param{n, t})
				}
			}
		}
		if pl.type.results != nil {
			for field in pl.type.results.list {
				t := field_type(r, field)
				// Unnamed fields get a synthetic identifier at the type's position.
				// An explicit name always appears before its colon and type.
				for name in field.names {
					if name.pos.offset != field.type.pos.offset {
						reject_decl(r, field.pos, "named result outside the subset")
					}
				}
				append(&f.results, t)
			}
		}
		r.lookup[name] = len(r.functions)
		append(&r.functions, f)
	}
}

field_type :: proc(r: ^Reader, f: ^ast.Field) -> string {
	t := ident(f.type)
	if f.default_value != nil || f.flags != {} || f.tag.text != "" || !(signed(t) || t == "string" || t == "bool") {
		reject_decl(r, f.pos, "expected a scalar parameter or result type")
	}
	return t
}

signature_table :: proc(r: ^Reader) -> Object {
	functions := Array{}
	for f in r.functions {
		if !f.exported { continue }
		params, results := Array{}, Array{}
		for p in f.params { append(&params, p.ty) }
		for t in f.results { append(&results, t) }
		append(&functions, Object{"name" = f.id, "source" = f.source_name, "params" = params, "results" = results})
	}
	return Object{"package" = r.pkg, "functions" = functions}
}

module_document :: proc(r: ^Reader) -> Object {
	functions, layout := Array{}, Array{}
	for &f in r.functions {
		params, results := Array{}, Array{}
		env := Types{}
		for p in f.params {
			env.values[p.name] = p.ty
			append(&params, Object{"name" = p.name, "t" = type_object(p.ty)})
		}
		for t in f.results { append(&results, type_object(t)) }
		body := lower_block(r, &f, f.proc_lit.body, &env)
		append(&functions, Object{
			"name" = f.id, "exported" = f.exported, "params" = params,
			"results" = results, "body" = body, "pos" = f.pos,
		})
		append(&layout, fmt.aprintf("func:%s", f.id))
	}
	return Object{
		"format" = "cir/1", "name" = r.pkg,
		"reader" = Object{"language" = "odin", "name" = "codespec-read-odin", "version" = VERSION},
		"types" = Array{}, "funcs" = functions, "layout" = layout, "refusals" = r.refusals,
	}
}
