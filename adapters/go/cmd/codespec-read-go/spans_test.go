package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

// update rewrites the expected span documents under testdata/spans from the reader's output.
var update = flag.Bool("update", false, "rewrite testdata/spans/*.spans.json")

// asReader is set in the environment of `codespec read`: the test binary then runs as the reader
// it calls, `codespec-read-go`, so the strict read needs no separate build of the reader.
const asReader = "CODESPEC_READ_GO_TEST_AS_READER"

func TestMain(m *testing.M) {
	if os.Getenv(asReader) == "1" {
		main()
		os.Exit(0)
	}
	os.Exit(m.Run())
}

// root is the repository root, four directories above this package.
func root(t *testing.T) string {
	t.Helper()
	r, err := filepath.Abs("../../../..")
	if err != nil {
		t.Fatal(err)
	}
	return r
}

// codespec is the codespec binary: $CODESPEC, else target/debug/codespec of the repository. The
// test is skipped when neither is set nor built; `task examples-go` sets $CODESPEC.
func codespec(t *testing.T) string {
	t.Helper()
	if p := os.Getenv("CODESPEC"); p != "" {
		return p
	}
	p := filepath.Join(root(t), "target", "debug", "codespec")
	if _, err := os.Stat(p); err != nil {
		t.Skipf("no codespec binary at %s (set CODESPEC or build b10x-codespec-cli)", p)
	}
	return p
}

// strictRead runs `codespec read --spans` over dir with this test binary as the Go reader and
// returns the cir/1 module and the span document it wrote; it fails unless the span document
// passed strict reading.
func strictRead(t *testing.T, dir string) (cir, spans []byte) {
	t.Helper()
	bin := codespec(t)
	adapters := t.TempDir()
	self, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(self, filepath.Join(adapters, "codespec-read-go")); err != nil {
		t.Fatal(err)
	}
	out := t.TempDir()
	cirPath, spansPath := filepath.Join(out, "read.cir.json"), filepath.Join(out, "read.spans.json")
	cmd := exec.Command(bin, "read", "--lang", "go", "--adapters", adapters, "--out", cirPath, "--spans", spansPath, dir)
	cmd.Env = append(os.Environ(), asReader+"=1")
	if b, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("codespec read --spans %s: %v\n%s", dir, err, b)
	}
	if cir, err = os.ReadFile(cirPath); err != nil {
		t.Fatal(err)
	}
	if spans, err = os.ReadFile(spansPath); err != nil {
		t.Fatal(err)
	}
	return cir, spans
}

// spansOf is the reader's --spans object for dir, decoded.
func spansOf(t *testing.T, dir string) map[string]any {
	t.Helper()
	var b bytes.Buffer
	if err := writeSpans(&b, dir); err != nil {
		t.Fatal(err)
	}
	var v map[string]any
	if err := json.Unmarshal(b.Bytes(), &v); err != nil {
		t.Fatalf("--spans output is not JSON: %v", err)
	}
	return v
}

// sameNames fails unless the text of every name span of v is the name of the node its `at`
// names in v's cir/1 module: strict reading takes an `at` of the right kind on the wrong node,
// and in Go a name is spelled as cir/1 names it.
func sameNames(t *testing.T, v map[string]any) {
	t.Helper()
	for _, s := range v["spans"].([]any) {
		s := s.(map[string]any)
		if s["role"] != "name" {
			continue
		}
		at := s["at"].(string)
		var cur any = v["cir"]
		segs := strings.Split(at, ".")
		for k, seg := range segs {
			switch c := cur.(type) {
			case map[string]any:
				cur = c[seg]
			case []any:
				i, err := strconv.Atoi(seg)
				if err != nil || i >= len(c) {
					t.Fatalf("%s: no element %s", at, seg)
				}
				cur = c[i]
			}
			// cq names a struct literal's field by its value; the field's name is `.name`.
			if lf, ok := cur.(map[string]any); ok && k > 0 && segs[k-1] == "fields" && lf["value"] != nil {
				if k+1 < len(segs) && segs[k+1] == "name" {
					continue
				}
				cur = lf["value"]
			}
		}
		name := cur
		if n, ok := cur.(map[string]any); ok {
			name = n["name"]
		}
		if name != s["text"] {
			t.Errorf("name span %s spells %q, the node there is named %v", at, s["text"], name)
		}
	}
}

// For both Go examples, `codespec read --spans` reads the span document strictly and writes the
// cir/1 module byte for byte as the checked-in read; the reader's object lists the files.
func TestSpansOfTheGoExamplesReadStrictlyAndTheirCirIsTheCheckedInRead(t *testing.T) {
	for _, ex := range []string{"series", "orders"} {
		t.Run(ex, func(t *testing.T) {
			dir := filepath.Join(root(t), "examples", ex, "source")
			v := spansOf(t, dir)
			for _, k := range []string{"cir", "files", "spans", "unspanned", "refs"} {
				if _, ok := v[k]; !ok {
					t.Fatalf("--spans output has no %q member", k)
				}
			}
			if len(v) != 5 {
				t.Fatalf("--spans output has %d members, want 5", len(v))
			}
			sameNames(t, v)
			files, _ := v["files"].([]any)
			if len(files) != 1 || files[0].(map[string]any)["path"] != ex+".go" {
				t.Fatalf("files %v, want one record of %s.go", files, ex)
			}
			cir, _ := strictRead(t, dir)
			want, err := os.ReadFile(filepath.Join(root(t), "examples", ex, ex+".read.cir.json"))
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(cir, want) {
				t.Fatalf("cir/1 read with --spans differs from examples/%s/%s.read.cir.json", ex, ex)
			}
		})
	}
}

// spanCase is a package read with spans: its files, and the name spans (`<at> <text>`) and
// unspanned entries (`<at> <role> <reason>`) it must hold among others.
type spanCase struct {
	name      string
	files     map[string]string
	names     []string
	unspanned []string
	generated []string // files marked generated
}

var spanCases = []spanCase{
	{
		name:  "one_line",
		files: map[string]string{"p.go": "package p\n\nfunc A() int { return 1 }; func B() int { return A() }\n"},
		names: []string{"funcs.0 A", "funcs.1 B", "funcs.1.body.0.results.0 A"},
	},
	{
		name: "doc_method",
		files: map[string]string{"p.go": `package p

type Box struct {
	n int
}

// Size returns the count.
// It reads n.
func (b *Box) Size() int { return b.n }

func Each(b *Box) func() int {
	return func() int { return b.Size() }
}

func Fast(x int) int
`},
		names: []string{
			"types.0.fields.0 n", "funcs.0 Size", "funcs.0.body.0.results.0 n",
			"funcs.1 Each", "funcs.1.body.0.results.0.body.0.results.0 Size", "opaque.0 Fast",
		},
	},
	{
		name:  "crlf",
		files: map[string]string{"p.go": "package p\r\n\r\n// Load gives one.\r\nfunc Load() int {\r\n\treturn 1\r\n}\r\n\r\nfunc Use() int {\r\n\treturn Load() + Load()\r\n}\r\n"},
		names: []string{"funcs.0 Load", "funcs.1 Use", "funcs.1.body.0.results.0.x Load", "funcs.1.body.0.results.0.y Load"},
	},
	{
		name:  "bom",
		files: map[string]string{"p.go": "\ufeffpackage p\n\nfunc Load() int { return 1 }\n\nfunc Use() int { return Load() }\n"},
		names: []string{"funcs.0 Load", "funcs.1 Use", "funcs.1.body.0.results.0 Load"},
	},
	{
		name: "multibyte",
		files: map[string]string{"p.go": `package p

type Maß struct {
	größe int
}

func Größe(m Maß) int { return m.größe }

func Über() int { return Größe(Maß{größe: 2}) }
`},
		names: []string{
			"types.0.fields.0 größe", "funcs.0 Größe", "funcs.0.body.0.results.0 größe",
			"funcs.1 Über", "funcs.1.body.0.results.0 Größe", "funcs.1.body.0.results.0.args.0.fields.0.name größe",
		},
	},
	{
		name: "literals",
		files: map[string]string{"p.go": `package p

type Point struct {
	x, y int
}

func Positional() Point { return Point{1, 2} }

func Keyed() Point { return Point{y: 2, x: 1} }
`},
		names: []string{
			"types.0.fields.0 x", "types.0.fields.1 y",
			"funcs.1.body.0.results.0.fields.0.name y", "funcs.1.body.0.results.0.fields.1.name x",
		},
		unspanned: []string{
			"funcs.0.body.0.results.0.fields.0.name name implicit",
			"funcs.0.body.0.results.0.fields.1.name name implicit",
		},
	},
	{
		name: "promoted",
		files: map[string]string{"p.go": `package p

type Base struct {
	n int
}

type Outer struct {
	Base
	m int
}

func Get(o Outer) int { return o.n }
`},
		names: []string{"types.1.fields.1 m", "funcs.0.body.0.results.0 n"},
		unspanned: []string{
			"types.1.fields.0 name lowered: an embedded field is named by its type",
			"funcs.0.body.0.results.0.x name implicit",
		},
	},
	{
		name: "generated",
		files: map[string]string{
			"gen.go":  "// Code generated by hand. DO NOT EDIT.\n\npackage p\n\nfunc Made() int { return 1 }\n",
			"main.go": "package p\n\nfunc Use() int { return Made() }\n",
		},
		names:     []string{"funcs.0 Made", "funcs.1 Use", "funcs.1.body.0.results.0 Made"},
		generated: []string{"gen.go"},
	},
	{
		name: "blank",
		files: map[string]string{"p.go": `package p

func load() int { return 1 }

func Run() {
	_ = load()
}
`},
		names: []string{"funcs.1.body.0.rhs.0 load"},
	},
	{
		name: "local_decl",
		files: map[string]string{"p.go": `package p

func load() int { return 1 }

func Run() int {
	var v = load()
	const k = 2
	return v + k
}
`},
		names: []string{"funcs.1.body.0.rhs.0 load"},
	},
	{
		name: "small_forms",
		files: map[string]string{"p.go": `package p

import "sync"

type Node struct {
	val  int
	next *Node
}

func Link(n *Node) *Node { return &Node{val: 1, next: n} }

func Fresh() *Node { return new(Node) }

func Zero() *sync.WaitGroup {
	w := sync.WaitGroup{}
	return &w
}
`},
		names: []string{
			"funcs.0.body.0.results.0.x.fields.0.name val", "funcs.0.body.0.results.0.x.fields.1.name next",
		},
	},
	{
		name: "named_results",
		files: map[string]string{"p.go": `package p

func half(n int) int { return n / 2 }

// Split gives the halves of n.
func Split(n int) (q, r int) {
	q = half(n)
	r = n - q
	return
}
`},
		names: []string{"funcs.1 Split", "funcs.1.body.2.rhs.0 half"},
	},
	{
		name: "if_init",
		files: map[string]string{"p.go": `package p

import "errors"

func load() (int, error) { return 1, errors.New("no") }

func count() int { return 2 }

func Run() int {
	if v, err := load(); err != nil {
		return v
	}
	switch c := count(); c {
	case 1:
		return c
	}
	return 0
}
`},
		names: []string{"funcs.2.body.0.rhs.0 load", "funcs.2.body.2.rhs.0 count"},
	},
}

// Each named case reads with the name spans and unspanned entries it names, its expected span
// document under testdata/spans, and passes strict reading.
func TestSpansOfNamedCases(t *testing.T) {
	for _, c := range spanCases {
		t.Run(c.name, func(t *testing.T) {
			dir := t.TempDir()
			for name, src := range c.files {
				if err := os.WriteFile(filepath.Join(dir, name), []byte(src), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			v := spansOf(t, dir)
			sameNames(t, v)
			if refusals, ok := v["cir"].(map[string]any)["refusals"]; ok && c.name != "doc_method" {
				t.Fatalf("refused: %v", refusals)
			}
			have := map[string]bool{}
			for _, s := range v["spans"].([]any) {
				s := s.(map[string]any)
				if s["role"] == "name" {
					have[fmt.Sprintf("%s %s", s["at"], s["text"])] = true
				}
			}
			for _, u := range v["unspanned"].([]any) {
				u := u.(map[string]any)
				have[fmt.Sprintf("%s %s %s", u["at"], u["role"], u["reason"])] = true
			}
			for _, w := range append(append([]string{}, c.names...), c.unspanned...) {
				if !have[w] {
					t.Errorf("no %q among the spans and unspanned entries", w)
				}
			}
			for _, f := range v["files"].([]any) {
				f := f.(map[string]any)
				want := false
				for _, g := range c.generated {
					want = want || f["path"] == g
				}
				if f["generated"] != want {
					t.Errorf("file %v generated %v, want %v", f["path"], f["generated"], want)
				}
			}
			delete(v, "cir")
			var got bytes.Buffer
			enc := json.NewEncoder(&got)
			enc.SetEscapeHTML(false)
			enc.SetIndent("", "  ")
			if err := enc.Encode(v); err != nil {
				t.Fatal(err)
			}
			path := filepath.Join("testdata", "spans", c.name+".spans.json")
			if *update {
				if err := os.WriteFile(path, got.Bytes(), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			want, err := os.ReadFile(path)
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(got.Bytes(), want) {
				t.Errorf("span document differs from %s:\n%s", path, got.String())
			}
			strictRead(t, dir)
		})
	}
}

// A named result read as a local is a `var` statement, which takes no name span; the function's
// decl span covers the whole declaration, its doc comment to its closing brace.
func TestSpansOfNamedResultsCoverTheWholeDeclaration(t *testing.T) {
	src := "package p\n\n// Split gives the halves of n.\nfunc Split(n int) (q, r int) {\n\tq = n / 2\n\tr = n - q\n\treturn\n}\n"
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "p.go"), []byte(src), 0o644); err != nil {
		t.Fatal(err)
	}
	v := spansOf(t, dir)
	for _, s := range v["spans"].([]any) {
		s := s.(map[string]any)
		at := s["at"].(string)
		if strings.Contains(at, ".body.") {
			t.Errorf("span at %s: the body holds no named node", at)
		}
		if at == "funcs.0" && s["role"] == "decl" {
			if s["start"] != float64(strings.Index(src, "// Split")) || s["end"] != float64(len(src)-1) {
				t.Errorf("decl %v..%v, want the doc comment to the closing brace", s["start"], s["end"])
			}
		}
	}
	if u := v["unspanned"].([]any); len(u) != 0 {
		t.Errorf("unspanned %v, want none", u)
	}
}
