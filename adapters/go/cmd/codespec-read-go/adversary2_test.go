package main

import (
	"path/filepath"
	"strings"
	"testing"
)

func adversary2Load(t *testing.T, name string) *Module {
	t.Helper()
	m, err := load(filepath.Join("testdata", "adversary2", name))
	if err != nil {
		t.Fatal(err)
	}
	return m
}

// Go's f(g()) passes each result of a multi-valued g as one argument of f. The reader writes it
// as a call of f with one argument of tuple type and no refusal: `add` has two parameters and the
// call one argument, and cir/1 says nothing that spreads a tuple over parameters (both writers
// refuse it: "a call of add with 1 arguments for 2 parameters"). Now that the reader claims
// type.tuple, a tuple in argument position is either written as the arguments Go passes or kept
// foreign with a refusal.
func TestAdversary2ACallPassedAMultiValuedCallIsReadAsItsArgumentsOrRefused(t *testing.T) {
	m := adversary2Load(t, "tuplearg")
	for _, name := range []string{"Spread", "Variadic"} {
		s := funcNamed(t, m, name).Body[0]
		if s.K == "foreign" {
			continue
		}
		if s.K != "return" || len(s.Results) != 1 || s.Results[0].K != "call" {
			t.Fatalf("%s: read as %q, want a return of a call or a foreign statement", name, s.K)
		}
		call := s.Results[0]
		for _, a := range call.Args {
			if a.T != nil && a.T.K == "tuple" {
				t.Errorf("%s: the call of %s has %d argument(s), one of them of type tuple, and the reader refuses nothing (refusals %v): Go passes each result as its own argument",
					name, call.Name, len(call.Args), m.Refusals)
			}
		}
	}
}

// The unit's page: "A range that assigns only `_` is read." `for (_) = range xs` assigns only
// `_` (go/types unwraps the parentheses), and Go runs it as `for range xs`.
func TestAdversary2ARangeAssigningOnlyAParenthesisedBlankIsRead(t *testing.T) {
	m := adversary2Load(t, "rangeforms")
	ks := kinds(funcNamed(t, m, "ParenBlank").Body)
	if count(ks, "for_range") != 1 || count(ks, "foreign") != 0 {
		t.Fatalf("ParenBlank: body kinds %v refusals %v, want one for_range and no foreign", ks, m.Refusals)
	}
}

// A labelled range that assigns a variable declared before it is refused (good), but the refusal
// names the label as the cause: its only branch is its own `break L`, which labeled() reads, so
// the reason a reader of the refusal list gets is not the reason the statement was refused.
func TestAdversary2ALabelledRangeThatAssignsAnOuterVariableIsRefusedForThatReason(t *testing.T) {
	m := adversary2Load(t, "rangeforms")
	if k := funcNamed(t, m, "Labelled").Body[1].K; k != "foreign" {
		t.Fatalf("Labelled: the labelled range is read as %q, want foreign", k)
	}
	for _, r := range m.Refusals {
		if strings.HasPrefix(r.What, "L:") && !strings.Contains(r.Reason, "declared before") {
			t.Fatalf("Labelled: refused at %s for %q, want the range-assigns reason", r.Pos, r.Reason)
		}
	}
}

// The manifest, not the claimNot variable, leaves each of the six unclaimed: the unit's test
// compares claimNot with a literal, which stays green when describe() stops passing it on.
func TestAdversary2TheManifestLeavesEachOfTheSixUnclaimed(t *testing.T) {
	for _, id := range []string{"decl.union", "expr.zero", "intrinsic.sorted_keys", "stmt.lookup_else", "stmt.raise", "type.unit"} {
		if claimed(t, id) {
			t.Errorf("the manifest claims %s", id)
		}
	}
	if !claimed(t, "type.tuple") {
		t.Error("the manifest does not claim type.tuple")
	}
}

// The type-tuple case's `x, y := split(a, b)` binds a call of type tuple (int, int): the unit's
// test checks only that the case has no refusal, which stays green when the call is typed int.
func TestAdversary2TheTypeTupleCaseBindsACallOfATupleOfTwoInts(t *testing.T) {
	m := loadCase(t, "type-tuple")
	s := funcNamed(t, m, "Flip").Body[0]
	if s.K != "let" || len(s.Names) != 2 || len(s.Rhs) != 1 {
		t.Fatalf("Flip: first statement %q names %v rhs %d, want a let of two names and one call", s.K, s.Names, len(s.Rhs))
	}
	tt := s.Rhs[0].T
	if tt == nil || tt.K != "tuple" || len(tt.Elems) != 2 || tt.Elems[0].K != "int" || tt.Elems[1].K != "int" {
		t.Fatalf("Flip: the call is typed %+v, want tuple (int, int)", tt)
	}
}
