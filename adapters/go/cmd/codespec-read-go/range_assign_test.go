package main

import (
	"strings"
	"testing"
)

// A cir/1 `for_range` declares its key and value (crates/model: `for key, val := range over`), so
// a range with `=` that assigns a variable declared before the loop cannot be one: read as one, the
// outer variable is never assigned. Such a range is kept foreign with a refusal at its line.
func requireRangeRefused(t *testing.T, src string, names ...string) {
	t.Helper()
	m := loadSource(t, src)
	for _, name := range names {
		body := funcNamed(t, m, name).Body
		if body[1].K != "foreign" || !strings.HasPrefix(body[1].Src, "for ") {
			t.Fatalf("%s: the range is read as %q, want foreign; body kinds %v", name, body[1].K, kinds(body))
		}
	}
	if got := len(m.Refusals); got != len(names) {
		t.Fatalf("refusals %d, want %d: %v", got, len(names), m.Refusals)
	}
	for _, r := range m.Refusals {
		if !strings.Contains(r.Reason, "declared before") {
			t.Fatalf("refusal %q does not name why", r.Reason)
		}
	}
}

func TestARangeThatAssignsVariablesDeclaredBeforeItIsRefusedAtItsLine(t *testing.T) {
	requireRangeRefused(t, `package p

func Last(xs []int) int {
	i := 0
	for i = range xs {
	}
	return i
}

func Value(xs []int) int {
	v := 0
	for _, v = range xs {
	}
	return v
}
`, "Last", "Value")
}

// A key that is not an identifier is refused the same way, never a panic of the reader.
func TestARangeThatAssignsAFieldIsRefusedAtItsLine(t *testing.T) {
	requireRangeRefused(t, `package p

type S struct{ f int }

func Field(xs []int) int {
	var s S
	for s.f = range xs {
	}
	return s.f
}
`, "Field")
}

// A range that assigns only `_` declares nothing and assigns nothing: it is a `for_range`.
func TestARangeThatAssignsOnlyBlanksIsRead(t *testing.T) {
	m := loadSource(t, `package p

func Blank(xs []int) int {
	n := 0
	for _ = range xs {
		n++
	}
	for _, _ = range xs {
		n++
	}
	return n
}
`)
	if ks := kinds(funcNamed(t, m, "Blank").Body); count(ks, "for_range") != 2 || count(ks, "foreign") != 0 || len(m.Refusals) != 0 {
		t.Fatalf("Blank: body kinds %v refusals %v, want two for_range and no refusal", ks, m.Refusals)
	}
}
