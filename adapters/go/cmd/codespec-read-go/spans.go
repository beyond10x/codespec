package main

// `codespec-read-go --spans <dir>`: where the nodes of the module read stand in the package's
// files, the `codespec-spans/1` lists of docs/code-edit-language.md § 6.2 (format and strict
// reading: crates/edit).
//
// The frontend notes on its own model (ir.go) the identifier of each name where it builds the
// node: Func and Opaque their declaration and name, a struct field its name, an Expr of a named
// kind its NameTok, a struct literal one token per field. After the module is encoded, a walk
// over the same model gives each node its path in the cir/1 document, which is cq's index path
// below `funcs.<i>.body`: the encoder writes every list in the model's order.
//
//	what                                   span
//	a function of funcs or opaque          decl: its doc comment, else `func`, to its end; name: its identifier
//	a field of a struct declaration        name: its identifier; an embedded field is lowered
//	f(…), x.m(…), p.F(…), x.M(…) of p      name: the identifier after the last dot
//	f, x.m as a function value             name: f, m
//	x.f                                    name: f; the hidden hops of a promoted field are implicit
//	T{f: v}                                each field's name: f; a positional literal's are implicit
//
// A node the reader made from no token of its own is `lowered`, and an edit needing it is
// refused. Offsets are bytes of the file as read, a byte-order mark counted; `line` and
// `end_line` count `\n`.

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"go/ast"
	"go/token"
	"go/types"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strconv"
)

// made is the reason of a named node the reader noted no token for.
const made = "lowered: made by the reader from no token of its own"

type spansOutput struct {
	Cir       node           `json:"cir"`
	Files     []fileOut      `json:"files"`
	Spans     []spanOut      `json:"spans"`
	Unspanned []unspannedOut `json:"unspanned"`
	Refs      []any          `json:"refs"`
}

type fileOut struct {
	Path      string `json:"path"`
	Sha256    string `json:"sha256"`
	Len       int    `json:"len"`
	Generated bool   `json:"generated"`
}

type spanOut struct {
	At      string  `json:"at"`
	Role    string  `json:"role"`
	File    int     `json:"file"`
	Start   int     `json:"start"`
	End     int     `json:"end"`
	Line    int     `json:"line"`
	EndLine int     `json:"end_line"`
	Text    *string `json:"text,omitempty"`
}

type unspannedOut struct {
	At     string `json:"at"`
	Role   string `json:"role"`
	Reason string `json:"reason"`
}

// writeSpans reads the package in dir and writes the object --spans prints:
// `{"cir", "files", "spans", "unspanned", "refs"}`, cir the module a plain read prints.
func writeSpans(w io.Writer, dir string) error {
	m, err := load(dir)
	if err != nil {
		return err
	}
	doc, err := encode(m)
	if err != nil {
		return err
	}
	out, err := spanLists(m, dir)
	if err != nil {
		return err
	}
	out.Cir = doc
	enc := json.NewEncoder(w)
	enc.SetEscapeHTML(false)
	return enc.Encode(out)
}

// declRange is a function declaration from its doc comment, else its `func`, to its end.
func declRange(d *ast.FuncDecl) (token.Pos, token.Pos) {
	start := d.Pos()
	if d.Doc != nil {
		start = d.Doc.Pos()
	}
	return start, d.End()
}

// fieldTok is the identifier of the field v of the struct ts declares. An embedded field is
// named by its type, and a field of the struct another type declares (`type T Base`) is spelled
// there: neither is lowered to a token of its own.
func fieldTok(ts *ast.TypeSpec, v *types.Var) Tok {
	switch {
	case v.Embedded():
		return Tok{Why: "lowered: an embedded field is named by its type"}
	case v.Pos() < ts.Pos() || v.Pos() >= ts.End():
		return Tok{Why: "lowered: the field is declared by another type"}
	}
	return Tok{Pos: v.Pos(), End: v.Pos() + token.Pos(len(v.Name()))}
}

// spanWalk gives the nodes of a module their paths and their tokens' places in the files read.
type spanWalk struct {
	fset  *token.FileSet
	index map[string]int // a file's name in fset to its index in files
	texts [][]byte
	// The offset of every byte after a `\n`, per file.
	starts [][]int
	out    *spansOutput
}

// spanLists is the files, spans, unspanned entries and refs of the module m read from dir.
func spanLists(m *Module, dir string) (*spansOutput, error) {
	w := &spanWalk{fset: m.Fset, index: map[string]int{}, out: &spansOutput{
		Files: []fileOut{}, Spans: []spanOut{}, Unspanned: []unspannedOut{}, Refs: []any{},
	}}
	for i, s := range m.Sources {
		text, err := os.ReadFile(s.Path)
		if err != nil {
			return nil, err
		}
		rel, err := filepath.Rel(dir, s.Path)
		if err != nil {
			return nil, err
		}
		sum := sha256.Sum256(text)
		w.out.Files = append(w.out.Files, fileOut{
			Path: filepath.ToSlash(rel), Sha256: hex.EncodeToString(sum[:]), Len: len(text), Generated: s.Generated,
		})
		var starts []int
		for k, b := range text {
			if b == '\n' {
				starts = append(starts, k+1)
			}
		}
		w.index[s.Path] = i
		w.texts = append(w.texts, text)
		w.starts = append(w.starts, starts)
	}
	for i, fn := range m.Funcs {
		at := "funcs." + strconv.Itoa(i)
		w.decl(at, fn.DeclPos, fn.DeclEnd)
		w.name(at, fn.NameTok)
		w.stmts(fn.Body, at+".body.")
	}
	for i, o := range m.Opaque {
		at := "opaque." + strconv.Itoa(i)
		w.decl(at, o.DeclPos, o.DeclEnd)
		w.name(at, o.NameTok)
	}
	for i, td := range m.Types {
		for j, fl := range td.Fields {
			w.name("types."+strconv.Itoa(i)+".fields."+strconv.Itoa(j), fl.NameTok)
		}
	}
	w.noOverlap()
	return w.out, nil
}

// place is the file and byte offsets of the token p..e, when it stands in a file read.
func (w *spanWalk) place(p, e token.Pos) (file, start, end int, ok bool) {
	if !p.IsValid() || e <= p {
		return 0, 0, 0, false
	}
	tf := w.fset.File(p)
	if tf == nil {
		return 0, 0, 0, false
	}
	file, ok = w.index[tf.Name()]
	if !ok || int(e) > tf.Base()+tf.Size() {
		return 0, 0, 0, false
	}
	return file, tf.Offset(p), tf.Offset(e), true
}

// line is the 1-based line of byte off of file.
func (w *spanWalk) line(file, off int) int {
	s := w.starts[file]
	return sort.Search(len(s), func(k int) bool { return s[k] > off }) + 1
}

func (w *spanWalk) span(at, role string, file, start, end int) {
	s := spanOut{At: at, Role: role, File: file, Start: start, End: end, Line: w.line(file, start), EndLine: w.line(file, end-1)}
	if role == "name" {
		text := string(w.texts[file][start:end])
		s.Text = &text
	}
	w.out.Spans = append(w.out.Spans, s)
}

func (w *spanWalk) unspanned(at, role, reason string) {
	w.out.Unspanned = append(w.out.Unspanned, unspannedOut{At: at, Role: role, Reason: reason})
}

func (w *spanWalk) decl(at string, p, e token.Pos) {
	if file, start, end, ok := w.place(p, e); ok {
		w.span(at, "decl", file, start, end)
		return
	}
	w.unspanned(at, "decl", "lowered: its declaration stands in no file read")
}

func (w *spanWalk) name(at string, t Tok) {
	if file, start, end, ok := w.place(t.Pos, t.End); ok {
		w.span(at, "name", file, start, end)
		return
	}
	switch {
	case t.Why != "":
		w.unspanned(at, "name", t.Why)
	case t.Pos.IsValid():
		w.unspanned(at, "name", "lowered: its identifier stands in no file read")
	default:
		w.unspanned(at, "name", made)
	}
}

// noOverlap keeps one name span per token: a node the reader copied carries the token of the
// node it copied, and the later of the two, in walk order, is lowered, as an edit of one token
// would change both.
func (w *spanWalk) noOverlap() {
	var names []int
	for i, s := range w.out.Spans {
		if s.Role == "name" {
			names = append(names, i)
		}
	}
	sort.SliceStable(names, func(a, b int) bool {
		x, y := w.out.Spans[names[a]], w.out.Spans[names[b]]
		if x.File != y.File {
			return x.File < y.File
		}
		if x.Start != y.Start {
			return x.Start < y.Start
		}
		return names[a] < names[b]
	})
	drop := map[int]string{}
	last := -1
	for _, i := range names {
		if last >= 0 && w.out.Spans[last].File == w.out.Spans[i].File && w.out.Spans[i].Start < w.out.Spans[last].End {
			drop[i] = w.out.Spans[last].At
			continue
		}
		last = i
	}
	if len(drop) == 0 {
		return
	}
	kept := w.out.Spans[:0:0]
	for i, s := range w.out.Spans {
		if other, ok := drop[i]; ok {
			w.unspanned(s.At, "name", "lowered: its token is the name of "+other+" too")
			continue
		}
		kept = append(kept, s)
	}
	w.out.Spans = kept
}

func (w *spanWalk) stmts(ss []Stmt, base string) {
	for k, s := range ss {
		w.stmt(s, base+strconv.Itoa(k))
	}
}

func (w *spanWalk) exprs(xs []Expr, base string) {
	for k, x := range xs {
		w.expr(x, base+strconv.Itoa(k))
	}
}

// stmt walks the statement s at path p as encode writes it.
func (w *spanWalk) stmt(s Stmt, p string) {
	switch s.K {
	case "let":
		w.exprs(s.Rhs, p+".rhs.")
	case "assign":
		w.exprs(s.Lhs, p+".lhs.")
		w.exprs(s.Rhs, p+".rhs.")
	case "incr", "decr", "expr":
		w.expr(*s.X, p+".x")
	case "if":
		w.expr(*s.Cond, p+".cond")
		w.stmts(s.Then, p+".then.")
		w.stmts(s.Else, p+".else.")
	case "for_range":
		w.expr(*s.Over, p+".over")
		w.stmts(s.Body, p+".body.")
	case "for":
		if s.Init != nil {
			w.stmt(*s.Init, p+".init")
		}
		if s.Cond != nil {
			w.expr(*s.Cond, p+".cond")
		}
		if s.Post != nil {
			w.stmt(*s.Post, p+".post")
		}
		w.stmts(s.Body, p+".body.")
	case "switch":
		if s.Tag != nil {
			w.expr(*s.Tag, p+".tag")
		}
		for c, cs := range s.Cases {
			cp := p + ".cases." + strconv.Itoa(c)
			w.exprs(cs.Values, cp+".values.")
			w.stmts(cs.Body, cp+".body.")
		}
	case "return":
		w.exprs(s.Results, p+".results.")
	case "send":
		w.expr(*s.Chan, p+".chan")
		w.expr(*s.X, p+".x")
	case "defer":
		w.expr(*s.Call, p+".call")
	}
}

// expr walks the expression x at path p as encode writes it: its own name, then its operands.
func (w *spanWalk) expr(x Expr, p string) {
	switch x.K {
	case "call", "method", "extern", "extern_method", "func_ref", "field":
		w.name(p, x.NameTok)
	case "struct":
		for j := range x.FieldNames {
			t := Tok{}
			if j < len(x.FieldToks) {
				t = x.FieldToks[j]
			}
			w.name(p+".fields."+strconv.Itoa(j)+".name", t)
		}
	}
	switch x.K {
	case "field", "unop", "conv", "to_error", "spread", "to_iface":
		w.expr(*x.X, p+".x")
	case "index":
		w.expr(*x.X, p+".x")
		w.expr(*x.Y, p+".index")
	case "call", "intrinsic", "extern":
		w.exprs(x.Args, p+".args.")
	case "method":
		w.expr(*x.X, p+".x")
		w.exprs(x.Args, p+".args.")
	case "extern_method", "iface_call":
		w.expr(*x.X, p+".recv")
		w.exprs(x.Args, p+".args.")
	case "binop":
		w.expr(*x.X, p+".x")
		w.expr(*x.Y, p+".y")
	case "struct":
		for j := range x.Args {
			w.expr(x.Args[j], p+".fields."+strconv.Itoa(j))
		}
	case "func_ref":
		if x.X != nil {
			w.expr(*x.X, p+".recv")
		}
	case "call_value":
		w.expr(*x.X, p+".f")
		w.exprs(x.Args, p+".args.")
	case "lambda":
		w.stmts(x.Body, p+".body.")
	}
}
