package main

import (
	"strings"
	"testing"
)

const blankPrelude = `package p

import "io"

type Sink struct{}

func (Sink) Write(b []byte) (int, error) { return len(b), nil }

func pair() (int, error) { return 1, nil }
`

// `_ = x` evaluates x and drops it: cir/1 `let _ = x`, which the Go writer writes back as `_ = x`.
func TestABlankAssignmentReadsAsALetOfTheBlank(t *testing.T) {
	m := loadSource(t, blankPrelude+`
func Drop(x int) {
	_ = x
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	body := funcNamed(t, m, "Drop").Body
	if len(body) != 1 || body[0].K != "let" || len(body[0].Names) != 1 || body[0].Names[0] != "_" ||
		body[0].Rhs[0].K != "var" || body[0].Rhs[0].Name != "x" {
		t.Fatalf("body %+v", body)
	}
}

// An assignment with blank and named targets reads as a let of fresh names, `_` kept, followed
// by the assignment of each named target from its fresh name: the form written by hand.
func TestAnAssignmentWithABlankTargetReadsAsALetOfFreshNamesThenTheAssignment(t *testing.T) {
	sameButPos(t, blankPrelude+`
func Count(w io.Writer, b []byte) (int, error) {
	var n int
	var err error
	n, _ = pair()
	_, err = w.Write(b)
	return n, err
}
`, blankPrelude+`
func Count(w io.Writer, b []byte) (int, error) {
	var n int
	var err error
	n_1, _ := pair()
	n = n_1
	_, err_1 := w.Write(b)
	err = err_1
	return n, err
}
`)
}

// A blank target beside a target that is not an identifier stays foreign: the target's operands
// would be evaluated after the right side.
func TestABlankBesideAnIndexTargetStaysForeign(t *testing.T) {
	m := loadSource(t, blankPrelude+`
func Put(xs []int) {
	xs[0], _ = pair()
}
`)
	if len(m.Refusals) != 1 || !strings.HasPrefix(m.Refusals[0].Reason, "expression outside cir/1") {
		t.Fatalf("refusals %v", m.Refusals)
	}
}

// A package-level `var _ I = …` whose initializer has no run-time effect, and a type alias, declare
// nothing a body can name: they are omitted without a refusal. `var _ = f()` runs f at init and
// stays refused.
func TestCompileTimeOnlyDeclarationsAreOmittedWithoutARefusal(t *testing.T) {
	m := loadSource(t, blankPrelude+`
var _ io.Writer = Sink{}

var _ io.Writer = (*Sink)(nil)

var _ io.Writer = &Sink{}

var _ int

type Alias = Sink

func Use(a Alias) Sink { return a }
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	if len(m.Vars) != 0 {
		t.Fatalf("vars %+v", m.Vars)
	}
	for _, td := range m.Types {
		if td.Name == "Alias" {
			t.Fatalf("the alias is declared: %+v", td)
		}
	}
	for _, l := range m.Layout {
		if strings.Contains(l, "Alias") || strings.HasSuffix(l, ":_") {
			t.Fatalf("layout %v", m.Layout)
		}
	}
	use := funcNamed(t, m, "Use")
	if use.Params[0].T.K != "named" || use.Params[0].T.Name != "Sink" {
		t.Fatalf("the alias does not read as its target: %+v", use.Params[0])
	}

	m = loadSource(t, blankPrelude+`
var _ = run()

func run() int { return 1 }
`)
	if len(m.Refusals) != 1 || m.Refusals[0].What != "_" {
		t.Fatalf("refusals %v", m.Refusals)
	}
}
