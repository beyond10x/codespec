package main

import (
	"encoding/json"
	"go/types"
	"strings"
	"testing"
)

// A method entry `T.m` names a call only when m is declared on T: the reader names a call of a
// promoted method by the type that declares it (externMethod reads the receiver of fn, so
// rw.Flush() on a *bufio.ReadWriter is bufio.Writer.Flush). An entry for a promoted method
// resolves through types.LookupFieldOrMethod and no document can ever name it.
func TestAdversaryEveryMethodEntryIsDeclaredOnItsOwnType(t *testing.T) {
	var promoted []string
	for _, e := range readStdSummary(t) {
		typ, _, isMethod := strings.Cut(e.Name, ".")
		if !isMethod {
			continue
		}
		fn, err := lookupEntry(e)
		if err != nil {
			t.Fatal(err)
		}
		if fn == nil {
			continue
		}
		recv := fn.Type().(*types.Signature).Recv().Type()
		if p, ok := recv.(*types.Pointer); ok {
			recv = p.Elem()
		}
		named, ok := recv.(*types.Named)
		if !ok || named.Origin().Obj().Name() != typ {
			promoted = append(promoted, e.Path+"."+e.Name+" (declared on "+recv.String()+")")
		}
	}
	if len(promoted) > 0 {
		t.Errorf("%d entries name a promoted method no call is named by:\n%s", len(promoted), strings.Join(promoted, "\n"))
	}
}

// An entry may take a function value (a callback, an iter.Seq): crates/effects counts a function
// of the set given to a call outside the set as called by the caller (crate doc of
// b10x-codespec-effects; crates/effects/tests/effects.rs::
// a_function_passed_to_a_call_outside_the_set_counts_as_called_by_the_caller). What that rule
// needs from the reader is checked here: a named function or method value passed to an entry
// that takes a function value reads as a `func_ref` in the arguments of the extern call, or as a
// `ref` call ref listed beside the call on a `foreign` statement or an opaque function. The
// sample holds one call per shape: a package function, a method of an extern type, a generic
// function, a conversion to a named function type, a method value, and a callback of a signature
// cir/1 cannot type. Every entry that takes a function value is listed in the log.
func TestAdversaryAFunctionPassedToASummarisedEntryReadsAsAFunctionValueTheEngineCounts(t *testing.T) {
	var taking []string
	for _, e := range readStdSummary(t) {
		fn, err := lookupEntry(e)
		if err != nil {
			t.Fatal(err)
		}
		if fn == nil {
			continue
		}
		params := fn.Type().(*types.Signature).Params()
		for i := 0; i < params.Len(); i++ {
			pt := params.At(i).Type()
			if tp, ok := pt.(*types.TypeParam); ok {
				pt = tp.Constraint()
			}
			if _, ok := pt.Underlying().(*types.Signature); ok {
				taking = append(taking, e.Path+"."+e.Name)
				break
			}
		}
	}
	t.Logf("%d entries take a function value: %s", len(taking), strings.Join(taking, ", "))

	m := loadSource(t, `package p

import (
	"io/fs"
	"net/http"
	"path/filepath"
	"slices"
	"sort"
	"strings"
	"sync"
	"time"
)

type T struct{ n int }

func (x *T) Init() { x.n = 1 }

func stamp() {}

func before(a, b int) bool { return a < b }

func visit(path string, d fs.DirEntry, err error) error { return err }

func cmpInts(a, b int) int { return a - b }

func upper(r rune) rune { return r }

func serve(w http.ResponseWriter, r *http.Request) {}

func Once(o *sync.Once) { o.Do(stamp) }

func Method(o *sync.Once, x *T) { o.Do(x.Init) }

func Sort(x []int) { sort.Slice(x, before) }

func Walk(root string) error { return filepath.WalkDir(root, visit) }

func Generic(x []int) { slices.SortFunc(x, cmpInts) }

func Map(s string) string { return strings.Map(upper, s) }

func Later() *time.Timer { return time.AfterFunc(time.Second, stamp) }

func Handle() { http.Handle("/", http.HandlerFunc(serve)) }

func HandleFunc() { http.HandleFunc("/", serve) }
`)
	doc := encoded(t, m)
	for _, c := range []struct{ fn, passed, callee string }{
		{"Once", "stamp", `"name":"Do","path":"sync"`},
		{"Method", "Init", `"name":"Do","path":"sync"`},
		{"Sort", "before", `"name":"Slice","path":"sort"`},
		{"Walk", "visit", `"name":"WalkDir","path":"path/filepath"`},
		{"Generic", "cmpInts", `"name":"SortFunc","path":"slices"`},
		{"Map", "upper", `"name":"Map","path":"strings"`},
		{"Later", "stamp", `"name":"AfterFunc","path":"time"`},
		{"Handle", "serve", `"name":"Handle","path":"net/http"`},
		{"HandleFunc", "serve", `"name":"HandleFunc","path":"net/http"`},
	} {
		f := findFunc(doc, c.fn, "funcs")
		if f == nil {
			f = findFunc(doc, c.fn, "opaque")
		}
		if f == nil {
			t.Errorf("%s: not read", c.fn)
			continue
		}
		b, err := json.Marshal(f)
		if err != nil {
			t.Fatal(err)
		}
		text := string(b)
		value := strings.Contains(text, `"k":"func_ref","name":"`+c.passed+`"`) ||
			strings.Contains(text, `"k":"ref","name":"`+c.passed+`"`)
		if !strings.Contains(text, c.callee) || !value {
			t.Errorf("%s: %s passed to %s reads as neither a func_ref argument nor a ref beside the call:\n%s", c.fn, c.passed, c.callee, text)
		}
	}
}
