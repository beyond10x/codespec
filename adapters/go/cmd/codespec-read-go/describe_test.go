package main

import (
	"encoding/json"
	"path/filepath"
	"strings"
	"testing"
)

// claimed reports whether the reader's manifest covers id: an entry of `features` names it or
// its family (`stmt.*`), and no entry of `not` does.
func claimed(t *testing.T, id string) bool {
	t.Helper()
	var m struct {
		Features []string `json:"features"`
		Not      []string `json:"not"`
	}
	if err := json.Unmarshal([]byte(describe()), &m); err != nil {
		t.Fatal(err)
	}
	match := func(entries []string) bool {
		for _, e := range entries {
			if e == id || (strings.HasSuffix(e, ".*") && strings.HasPrefix(id, strings.TrimSuffix(e, "*"))) {
				return true
			}
		}
		return false
	}
	return match(m.Features) && !match(m.Not)
}

func loadCase(t *testing.T, name string) *Module {
	t.Helper()
	m, err := load(filepath.Join("..", "..", "..", "..", "conformance", "cases", name))
	if err != nil {
		t.Fatal(err)
	}
	return m
}

// A Go construct outside cir/1 (here `defer`) is kept verbatim as `stmt.foreign` with a refusal at
// its line, and the reader claims `stmt.foreign` on that evidence.
func TestADeferIsKeptForeignVerbatimAndTheReaderClaimsStmtForeign(t *testing.T) {
	m := loadCase(t, "stmt-foreign")
	var found *Stmt
	for _, f := range m.Funcs {
		for i, s := range f.Body {
			if s.K == "foreign" {
				found = &f.Body[i]
			}
		}
	}
	if found == nil {
		t.Fatal("stmt-foreign: no foreign statement read")
	}
	if found.Src != "defer done()" || found.Lang != "go" {
		t.Fatalf("foreign statement is not the verbatim defer: lang %q src %q", found.Lang, found.Src)
	}
	if len(m.Refusals) == 0 {
		t.Fatal("stmt-foreign: a foreign statement without a refusal")
	}
	if !claimed(t, "stmt.foreign") {
		t.Fatal("the manifest does not claim stmt.foreign")
	}
}

// The type-tuple case reads to a function of two results with no foreign statement, so the
// manifest describe() prints claims type.tuple; its `not` list is exactly the five features this
// reader does not read (README.md § Features left unclaimed).
func TestTheReaderClaimsTypeTupleAndLeavesOnlyTheFiveUnreadFeaturesUnclaimed(t *testing.T) {
	m := loadCase(t, "type-tuple")
	if len(m.Refusals) > 0 {
		t.Fatalf("type-tuple: refusals %v", m.Refusals)
	}
	if !claimed(t, "type.tuple") {
		t.Fatal("the manifest does not claim type.tuple")
	}
	var manifest struct {
		Not []string `json:"not"`
	}
	if err := json.Unmarshal([]byte(describe()), &manifest); err != nil {
		t.Fatal(err)
	}
	want := []string{"decl.union", "expr.zero", "intrinsic.sorted_keys", "stmt.lookup_else", "type.unit"}
	if strings.Join(manifest.Not, " ") != strings.Join(want, " ") {
		t.Fatalf("the manifest's not list is %v, want %v", manifest.Not, want)
	}
}

// A reader's claim covers what the reader itself prints, not what `codespec pass` makes of it
// (docs/conformance.md § 5): the stmt-lookup-else case reads to a comma-ok `let` and an `if`, and
// only the lookup-guard pass turns them into `lookup_else`, so the reader does not claim it.
func TestTheLookupElseCaseReadsWithoutLookupElseAndIsNotClaimed(t *testing.T) {
	m := loadCase(t, "stmt-lookup-else")
	for _, f := range m.Funcs {
		if n := count(kinds(f.Body), "lookup_else"); n > 0 {
			t.Fatalf("%s: the reader printed %d lookup_else; the claim rule in § 5 must change", f.Name, n)
		}
	}
	if claimed(t, "stmt.lookup_else") {
		t.Fatal("the manifest claims stmt.lookup_else, which only codespec pass produces")
	}
}
