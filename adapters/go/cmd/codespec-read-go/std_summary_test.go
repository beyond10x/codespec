package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"go/build"
	"go/importer"
	"go/token"
	"go/types"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"testing"
)

// summaryEntry is the part of a codespec-effects/1 entry the signature check reads.
type summaryEntry struct {
	Path    string   `json:"path"`
	Name    string   `json:"name"`
	Effects []string `json:"effects"`
}

// stdImports holds the standard library packages the tests of the summary imported from source:
// one import pass per test binary, shared by every test.
var stdImports struct {
	sync.Mutex
	imp  types.Importer
	pkgs map[string]*types.Package
}

// importStd imports the standard library package path, once per test binary.
func importStd(path string) (*types.Package, error) {
	stdImports.Lock()
	defer stdImports.Unlock()
	if stdImports.imp == nil {
		ensureGOROOT()
		stdImports.imp = importer.ForCompiler(token.NewFileSet(), "source", nil)
		stdImports.pkgs = map[string]*types.Package{}
	}
	if p, ok := stdImports.pkgs[path]; ok {
		return p, nil
	}
	p, err := stdImports.imp.Import(path)
	if err != nil {
		return nil, err
	}
	stdImports.pkgs[path] = p
	return p, nil
}

// lookupEntry is the function or method an entry names in the Go that runs the test: `F`, or
// `T.m` through the method set of `*T`; nil when there is none.
func lookupEntry(e summaryEntry) (*types.Func, error) {
	pkg, err := importStd(e.Path)
	if err != nil {
		return nil, err
	}
	var obj types.Object
	if typ, method, isMethod := strings.Cut(e.Name, "."); isMethod {
		if tn, ok := pkg.Scope().Lookup(typ).(*types.TypeName); ok {
			obj, _, _ = types.LookupFieldOrMethod(tn.Type(), true, pkg, method)
		}
	} else {
		obj = pkg.Scope().Lookup(e.Name)
	}
	fn, _ := obj.(*types.Func)
	return fn, nil
}

// readStdSummary is the entries of summaries/go-std.effects.json.
func readStdSummary(t *testing.T) []summaryEntry {
	t.Helper()
	b, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "summaries", "go-std.effects.json"))
	if err != nil {
		t.Fatal(err)
	}
	var doc struct {
		Externs []summaryEntry `json:"externs"`
	}
	if err := json.Unmarshal(b, &doc); err != nil {
		t.Fatal(err)
	}
	if len(doc.Externs) == 0 {
		t.Fatal("summaries/go-std.effects.json has no entries")
	}
	return doc.Externs
}

// goModMinor is the minor version of the `go` line of adapters/go/go.mod: the Go CI installs.
func goModMinor(t *testing.T) int {
	t.Helper()
	b, err := os.ReadFile(filepath.Join("..", "..", "go.mod"))
	if err != nil {
		t.Fatal(err)
	}
	for _, line := range strings.Split(string(b), "\n") {
		if v, ok := strings.CutPrefix(line, "go 1."); ok {
			minor, err := strconv.Atoi(strings.SplitN(v, ".", 2)[0])
			if err != nil {
				t.Fatalf("go.mod: %q: %v", line, err)
			}
			return minor
		}
	}
	t.Fatal("go.mod has no go line")
	return 0
}

// apiName spells one function or method line of a Go API list ($GOROOT/api/go1.N.txt) the way a
// summary names it, `F` or `T.m`, with its package; ok is false for every other line.
func apiName(line string) (pkg, name string, ok bool) {
	rest, found := strings.CutPrefix(line, "pkg ")
	if !found {
		return "", "", false
	}
	pkg, rest, found = strings.Cut(rest, ", ")
	if !found || strings.Contains(pkg, " ") {
		return "", "", false
	}
	switch {
	case strings.HasPrefix(rest, "func "):
		rest = rest[len("func "):]
		if i := strings.IndexAny(rest, "(["); i > 0 {
			return pkg, rest[:i], true
		}
	case strings.HasPrefix(rest, "method ("):
		recv, after, found := strings.Cut(rest[len("method ("):], ") ")
		if !found {
			return "", "", false
		}
		recv = strings.TrimPrefix(recv, "*")
		if i := strings.Index(recv, "["); i >= 0 {
			recv = recv[:i]
		}
		if i := strings.IndexAny(after, "(["); i > 0 {
			return pkg, recv + "." + after[:i], true
		}
	}
	return "", "", false
}

// addedAfter maps `<path> <name>` to the API list that added it, for every list of this Go newer
// than go1.<minor>.
func addedAfter(t *testing.T, minor int) map[string]string {
	t.Helper()
	ensureGOROOT()
	out := map[string]string{}
	for n := minor + 1; ; n++ {
		file := fmt.Sprintf("go1.%d.txt", n)
		f, err := os.Open(filepath.Join(build.Default.GOROOT, "api", file))
		if os.IsNotExist(err) {
			return out
		}
		if err != nil {
			t.Fatal(err)
		}
		sc := bufio.NewScanner(f)
		sc.Buffer(make([]byte, 1<<20), 1<<20)
		for sc.Scan() {
			if pkg, name, ok := apiName(sc.Text()); ok {
				if _, seen := out[pkg+" "+name]; !seen {
					out[pkg+" "+name] = file
				}
			}
		}
		f.Close()
		if err := sc.Err(); err != nil {
			t.Fatal(err)
		}
	}
}

// summaryProblems resolves every entry with go/types against the standard library of the Go
// that runs the test, each package imported once, and against the API lists added after the Go
// go.mod names; it returns one line per entry that fails either.
func summaryProblems(t *testing.T, entries []summaryEntry) []string {
	t.Helper()
	minor := goModMinor(t)
	later := addedAfter(t, minor)
	var problems []string
	for _, e := range entries {
		what := e.Path + "." + e.Name
		if file, ok := later[e.Path+" "+e.Name]; ok {
			problems = append(problems, fmt.Sprintf("%s: added in %s, after the go 1.%d of go.mod", what, file, minor))
			continue
		}
		fn, err := lookupEntry(e)
		if err != nil {
			problems = append(problems, fmt.Sprintf("%s: %v", what, err))
			continue
		}
		if fn == nil || !fn.Exported() {
			problems = append(problems, fmt.Sprintf("%s: no such exported function or method in this Go", what))
		}
	}
	return problems
}

// Every entry of summaries/go-std.effects.json names a function or method of the standard
// library of the Go that runs this test, and none was added after the Go go.mod names (the Go CI
// installs): an entry for a function the pinned Go lacks fails here, so a Go change is noticed.
func TestEveryEntryOfTheGoStandardLibrarySummaryResolvesInThePinnedGo(t *testing.T) {
	for _, p := range summaryProblems(t, readStdSummary(t)) {
		t.Error(p)
	}
}

// The check bites: a function this Go lacks, a method its type lacks and a function added after
// go.mod's Go are each named; a function the Go has is not.
func TestTheSummarySignatureCheckNamesAnEntryThePinnedGoLacks(t *testing.T) {
	problems := summaryProblems(t, []summaryEntry{
		{Path: "strings", Name: "ToUpper"},
		{Path: "strings", Name: "NoSuchFunction"},
		{Path: "strings", Name: "Builder.NoSuchMethod"},
		{Path: "strings", Name: "Lines"},
	})
	text := strings.Join(problems, "\n")
	if len(problems) != 3 {
		t.Fatalf("want 3 problems, got:\n%s", text)
	}
	for _, name := range []string{"strings.NoSuchFunction:", "strings.Builder.NoSuchMethod:", "strings.Lines:"} {
		if !strings.Contains(text, name) {
			t.Errorf("%s not named in:\n%s", name, text)
		}
	}
	if strings.Contains(text, "strings.ToUpper") {
		t.Errorf("strings.ToUpper named in:\n%s", text)
	}
}
