package main

import (
	"encoding/json"
	"reflect"
	"strconv"
	"strings"
	"testing"
)

// withoutPos is m as JSON with every "pos" field removed, at any depth.
func withoutPos(t *testing.T, m *Module) any {
	t.Helper()
	b, err := json.Marshal(m)
	if err != nil {
		t.Fatal(err)
	}
	var v any
	if err := json.Unmarshal(b, &v); err != nil {
		t.Fatal(err)
	}
	var strip func(v any)
	strip = func(v any) {
		switch v := v.(type) {
		case map[string]any:
			delete(v, "pos")
			for _, x := range v {
				strip(x)
			}
		case []any:
			for _, x := range v {
				strip(x)
			}
		}
	}
	strip(v)
	return v
}

// sameButPos fails unless the two sources read to the same module but for statement positions,
// both with no refusal.
func sameButPos(t *testing.T, initForm, twoStatements string) *Module {
	t.Helper()
	got := loadSource(t, initForm)
	want := loadSource(t, twoStatements)
	if len(got.Refusals) != 0 {
		t.Fatalf("init form refused: %v", got.Refusals)
	}
	if len(want.Refusals) != 0 {
		t.Fatalf("two-statement form refused: %v", want.Refusals)
	}
	g, w := withoutPos(t, got), withoutPos(t, want)
	if !reflect.DeepEqual(g, w) {
		gb, _ := json.MarshalIndent(g, "", " ")
		wb, _ := json.MarshalIndent(w, "", " ")
		t.Fatalf("init form reads\n%s\ntwo-statement form reads\n%s", gb, wb)
	}
	return got
}

const ifInitPrelude = `package p

import "errors"

var ErrNegative = errors.New("negative")

func F() error {
	return ErrNegative
}

func G() error {
	return nil
}
`

func TestAnErrorCheckInTheIfInitReadsAsTheTwoStatementForm(t *testing.T) {
	sameButPos(t, ifInitPrelude+`
func Run() (int, error) {
	if err := F(); err != nil {
		return 0, err
	}
	return 1, nil
}
`, ifInitPrelude+`
func Run() (int, error) {
	err := F()
	if err != nil {
		return 0, err
	}
	return 1, nil
}
`)
}

func TestTwoErrorChecksInIfInitsReadTheSecondAsAnAssignment(t *testing.T) {
	m := sameButPos(t, ifInitPrelude+`
func Run() (int, error) {
	if err := F(); err != nil {
		return 0, err
	}
	if err := G(); err != nil {
		return 1, err
	}
	return 2, nil
}
`, ifInitPrelude+`
func Run() (int, error) {
	err := F()
	if err != nil {
		return 0, err
	}
	err = G()
	if err != nil {
		return 1, err
	}
	return 2, nil
}
`)
	var run *Func
	for i := range m.Funcs {
		if m.Funcs[i].Name == "Run" {
			run = &m.Funcs[i]
		}
	}
	if run == nil || len(run.Body) != 5 || run.Body[0].K != "let" || run.Body[2].K != "assign" {
		t.Fatalf("body %+v", run)
	}
}

// An if whose init the 0.12.0 rule left foreign is hoisted too, each declared name renamed to a
// fresh one where its spelling occurs elsewhere in the function: the hand-written hoisted form.
func TestAnIfInitTheEarlierRuleLeftForeignIsHoistedWithFreshNames(t *testing.T) {
	for _, c := range []struct{ name, init, hoisted string }{
		{"used outside", `
func Run() error {
	err := F()
	if err := G(); err != nil {
		return err
	}
	return err
}
`, `
func Run() error {
	err := F()
	err_1 := G()
	if err_1 != nil {
		return err_1
	}
	return err
}
`},
		{"a parameter", `
func Run(err error) (int, error) {
	if err := F(); err != nil {
		return 0, err
	}
	return 1, err
}
`, `
func Run(err error) (int, error) {
	err_1 := F()
	if err_1 != nil {
		return 0, err_1
	}
	return 1, err
}
`},
		{"not a call, with an else", `
func Run() int {
	if x := 1; x > 0 {
		return x
	} else {
		return 0
	}
}
`, `
func Run() int {
	x := 1
	if x > 0 {
		return x
	} else {
		return 0
	}
}
`},
		{"an else if", `
func Run(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	} else if err := G(); err != nil {
		return 1, err
	}
	return 2, nil
}
`, `
func Run(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	} else {
		err := G()
		if err != nil {
			return 1, err
		}
	}
	return 2, nil
}
`},
		{"nested in an if declaring the same name", `
func Run() error {
	if err := F(); err != nil {
		if err := G(); err != nil {
			return err
		}
		return err
	}
	return nil
}
`, `
func Run() error {
	err := F()
	if err != nil {
		err_1 := G()
		if err_1 != nil {
			return err_1
		}
		return err
	}
	return nil
}
`},
		{"the init's call reads the name", `
func Run() error {
	if ErrNegative := Check(ErrNegative); ErrNegative != nil {
		return ErrNegative
	}
	return nil
}

func Check(e error) error {
	return e
}
`, `
func Run() error {
	ErrNegative_1 := Check(ErrNegative)
	if ErrNegative_1 != nil {
		return ErrNegative_1
	}
	return nil
}

func Check(e error) error {
	return e
}
`},
		{"a comma-ok index", `
func Run(m map[string]int, k string) int {
	if v, ok := m[k]; ok {
		return v
	}
	return 0
}
`, `
func Run(m map[string]int, k string) int {
	v, ok := m[k]
	if ok {
		return v
	}
	return 0
}
`},
		{"an else and two names", `
func Run() int {
	if n, err := Two(); err != nil {
		return 0
	} else {
		return n
	}
}

func Two() (int, error) {
	return 1, nil
}
`, `
func Run() int {
	n, err := Two()
	if err != nil {
		return 0
	} else {
		return n
	}
}

func Two() (int, error) {
	return 1, nil
}
`},
		{"an assignment init", `
func Run() error {
	var err error
	if err = F(); err != nil {
		return err
	}
	return nil
}
`, `
func Run() error {
	var err error
	err = F()
	if err != nil {
		return err
	}
	return nil
}
`},
		{"a blank and a name reused with another type", `
func Run() int {
	if _, x := Pair(); x != nil {
		return 0
	}
	if x := Count(); x > 0 {
		return x
	}
	return 2
}

func Pair() (int, error) {
	return 1, nil
}

func Count() int {
	return 1
}
`, `
func Run() int {
	_, x_1 := Pair()
	if x_1 != nil {
		return 0
	}
	x_2 := Count()
	if x_2 > 0 {
		return x_2
	}
	return 2
}

func Pair() (int, error) {
	return 1, nil
}

func Count() int {
	return 1
}
`},
		{"a send init", `
func Run(c chan int) int {
	if c <- 1; len(c) > 0 {
		return 1
	}
	return 0
}
`, `
func Run(c chan int) int {
	c <- 1
	if len(c) > 0 {
		return 1
	}
	return 0
}
`},
		{"a switch init", `
func Run(n int) int {
	switch m := n * 2; {
	case m > 4:
		return m
	}
	return 0
}
`, `
func Run(n int) int {
	m := n * 2
	switch {
	case m > 4:
		return m
	}
	return 0
}
`},
		{"a closure capturing a shared name", `
func Run() error {
	var get func() error
	if err := F(); err != nil {
		get = func() error { return err }
	}
	if err := G(); err != nil {
		return err
	}
	return get()
}
`, `
func Run() error {
	var get func() error
	err_1 := F()
	if err_1 != nil {
		get = func() error { return err_1 }
	}
	err_2 := G()
	if err_2 != nil {
		return err_2
	}
	return get()
}
`},
		{"inside a literal whose name is used outside", `
func Run() error {
	check := func() error {
		err := F()
		if err := G(); err != nil {
			return err
		}
		return err
	}
	return check()
}
`, `
func Run() error {
	check := func() error {
		err := F()
		err_1 := G()
		if err_1 != nil {
			return err_1
		}
		return err
	}
	return check()
}
`},
	} {
		t.Run(c.name, func(t *testing.T) {
			sameButPos(t, ifInitPrelude+c.init, ifInitPrelude+c.hoisted)
		})
	}
}

// A goto keeps every if with an init of its function foreign, as before.
func TestIfInitAGotoKeepsEveryIfWithAnInitForeign(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run(m map[string]int) int {
	if v, ok := m["a"]; ok {
		return v
	}
	goto L
L:
	return 0
}
`)
	for _, r := range m.Refusals {
		if r.Pos == ifInitLine(2) && r.Reason == "if with an init statement" {
			return
		}
	}
	t.Fatalf("refusals %v, want one at %s for the if with an init statement", m.Refusals, ifInitLine(2))
}

// A wrapped error is not the propagation form, but the hoist does not depend on the body: the if
// reads as the two-statement form does, its return the errorf intrinsic.
func TestAWrappedErrorInAnIfInitIsHoistedAndItsReturnReadAsToday(t *testing.T) {
	prelude := strings.Replace(ifInitPrelude, `import "errors"`, "import (\n\t\"errors\"\n\t\"fmt\"\n)", 1)
	m := sameButPos(t, prelude+`
func Run() (int, error) {
	if err := F(); err != nil {
		return 0, fmt.Errorf("wrap: %w", err)
	}
	return 1, nil
}
`, prelude+`
func Run() (int, error) {
	err := F()
	if err != nil {
		return 0, fmt.Errorf("wrap: %w", err)
	}
	return 1, nil
}
`)
	for _, fn := range m.Funcs {
		if fn.Name == "Run" {
			ret := fn.Body[1].Then[0]
			if ret.K != "return" || len(ret.Results) != 2 || ret.Results[1].K != "intrinsic" || ret.Results[1].Name != "errorf" {
				t.Fatalf("return %+v", ret)
			}
		}
	}
}

// ifInitRun returns the function Run of m.
func ifInitRun(t *testing.T, m *Module) *Func {
	t.Helper()
	for i := range m.Funcs {
		if m.Funcs[i].Name == "Run" {
			return &m.Funcs[i]
		}
	}
	t.Fatalf("no func Run in %+v", m.Funcs)
	return nil
}

// ifInitAssignsTo lists every assign statement anywhere in body whose left side is the
// variable name.
func ifInitAssignsTo(body []Stmt, name string) []Stmt {
	var found []Stmt
	var walk func(ss []Stmt)
	walk = func(ss []Stmt) {
		for _, s := range ss {
			if s.K == "assign" {
				for _, l := range s.Lhs {
					if l.K == "var" && l.Name == name {
						found = append(found, s)
					}
				}
			}
			walk(s.Then)
			walk(s.Else)
			walk(s.Body)
			for _, c := range s.Cases {
				walk(c.Body)
			}
		}
	}
	walk(body)
	return found
}

// ifInitLine is the line in a source of ifInitPrelude+fn of the line-th line of fn,
// counted as the existing tests count it.
func ifInitLine(line int) string {
	return "p.go:" + strconv.Itoa(strings.Count(ifInitPrelude, "\n")+1+line)
}

// The second init reads as `err = G()`, an assignment to the variable the first init declared.
// In Go each init declares a fresh variable, so p still points at F's error after the second if;
// after the hoist the assignment overwrites the variable p points at, and Run returns G's result
// (nil) where Go returns ErrNegative. The read is fully inside cir/1, with no refusal to flag it.
func TestIfInitAnAddressTakenInitVariableIsNotReassignedByALaterInit(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	var p *error
	if err := F(); err != nil {
		p = &err
	}
	if err := G(); err != nil {
		return err
	}
	if p != nil {
		return *p
	}
	return nil
}
`)
	run := ifInitRun(t, m)
	if got := ifInitAssignsTo(run.Body, "err"); len(got) != 0 {
		b, _ := json.MarshalIndent(got, "", " ")
		t.Fatalf("the second init writes the variable whose address the first if took (refusals %v):\n%s", m.Refusals, b)
	}
}

// The same in a loop: each iteration's `if err := G()` declares a fresh err in Go, so every
// pointer stored names a different variable. Read as an assignment to the err declared before the
// loop, every pointer names the one variable.
func TestIfInitAnInitInALoopIsNotAnAssignmentToOneVariableBeforeIt(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run(ps []*error) error {
	if err := F(); err != nil {
		return err
	}
	for i := 0; i < 3; i++ {
		if err := G(); err != nil {
			ps[i] = &err
		}
	}
	return nil
}
`)
	run := ifInitRun(t, m)
	if got := ifInitAssignsTo(run.Body, "err"); len(got) != 0 {
		b, _ := json.MarshalIndent(got, "", " ")
		t.Fatalf("each iteration's fresh err reads as one variable declared before the loop (refusals %v):\n%s", m.Refusals, b)
	}
}

// The condition is outside cir/1. The init is read in front as `let err = F()`, and then the
// whole if, init included, is kept foreign: the read now calls F twice and declares err twice,
// and the refusal names the condition, not the init (a slice expression).
func TestIfInitAnIfWhoseConditionIsOutsideCir1DoesNotAlsoHoistItsInit(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run(xs []error) error {
	if err := F(); xs[1:] != nil {
		return err
	}
	return nil
}
`)
	run := ifInitRun(t, m)
	want := ifInitLine(2)
	if len(m.Refusals) != 1 || m.Refusals[0].Pos != want || m.Refusals[0].Reason != "expression outside cir/1: slice expression" {
		t.Errorf("refusals %v, want one at %s for the slice expression", m.Refusals, want)
	}
	if len(run.Body) != 2 || run.Body[0].K != "foreign" {
		b, _ := json.MarshalIndent(run.Body, "", " ")
		t.Fatalf("want the if foreign followed by the return, got\n%s", b)
	}
}

// `goto L` jumps over the first if. In Go nothing is skipped but that if (its err is scoped to
// it); hoisted, the jump skips the `let err` and lands on `err = G()`, an assignment to a
// variable never declared on that path (Go itself refuses the two-statement spelling: goto
// jumps over a variable declaration).
func TestIfInitAGotoOverAHoistedDeclarationLeavesNoAssignmentToIt(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	goto L
	if err := F(); err != nil {
		return err
	}
L:
	G()
	if err := G(); err != nil {
		return err
	}
	return nil
}
`)
	run := ifInitRun(t, m)
	if got := ifInitAssignsTo(run.Body, "err"); len(got) != 0 {
		b, _ := json.MarshalIndent(run.Body, "", " ")
		t.Fatalf("an assignment to err that the goto reaches without its declaration:\n%s", b)
	}
}

// Hoists in nested blocks: an assignment inside a loop and a switch case to the err declared
// before them; declarations in sibling branches; a declaration in a loop body whose scope ends
// with the loop, so the init after the loop declares again.
func TestIfInitHoistsInNestedBlocksReadAsTheTwoStatementForm(t *testing.T) {
	sameButPos(t, ifInitPrelude+`
func Run(n int) (int, error) {
	if err := F(); err != nil {
		return 0, err
	}
	for i := 0; i < n; i++ {
		if err := G(); err != nil {
			return i, err
		}
	}
	switch n {
	case 1:
		if err := F(); err != nil {
			return 1, err
		}
	}
	return 2, nil
}

func Siblings(n int) error {
	if n > 0 {
		if err := F(); err != nil {
			return err
		}
	} else {
		if err := G(); err != nil {
			return err
		}
	}
	return nil
}

func AfterLoop(n int) error {
	for i := 0; i < n; i++ {
		if err := F(); err != nil {
			return err
		}
	}
	if err := G(); err != nil {
		return err
	}
	return nil
}
`, ifInitPrelude+`
func Run(n int) (int, error) {
	err := F()
	if err != nil {
		return 0, err
	}
	for i := 0; i < n; i++ {
		err = G()
		if err != nil {
			return i, err
		}
	}
	switch n {
	case 1:
		err = F()
		if err != nil {
			return 1, err
		}
	}
	return 2, nil
}

func Siblings(n int) error {
	if n > 0 {
		err := F()
		if err != nil {
			return err
		}
	} else {
		err := G()
		if err != nil {
			return err
		}
	}
	return nil
}

func AfterLoop(n int) error {
	for i := 0; i < n; i++ {
		err := F()
		if err != nil {
			return err
		}
	}
	err := G()
	if err != nil {
		return err
	}
	return nil
}
`)
}

// One name declared by two inits with different types is not one variable: each if is hoisted
// with a fresh name of its own (the table above reads the same shape with a blank).
func TestIfInitANameReusedWithAnotherTypeGetsAFreshNamePerIf(t *testing.T) {
	sameButPos(t, ifInitPrelude+`
func N() int {
	return 1
}

func Run() int {
	if x := F(); x != nil {
		return 0
	}
	if x := N(); x > 0 {
		return x
	}
	return 2
}
`, ifInitPrelude+`
func N() int {
	return 1
}

func Run() int {
	x_1 := F()
	if x_1 != nil {
		return 0
	}
	x_2 := N()
	if x_2 > 0 {
		return x_2
	}
	return 2
}
`)
}

// A closure that captures the first init's err would see the second init's value were the
// second read as an assignment to it: each if gets its own fresh name, so there is none.
func TestIfInitAClosureCapturingASharedNameIsNotReassigned(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	var get func() error
	if err := F(); err != nil {
		get = func() error { return err }
	}
	if err := G(); err != nil {
		return err
	}
	return get()
}
`)
	run := ifInitRun(t, m)
	if got := ifInitAssignsTo(run.Body, "err"); len(got) != 0 {
		b, _ := json.MarshalIndent(got, "", " ")
		t.Fatalf("the second init writes the variable the closure captured (refusals %v):\n%s", m.Refusals, b)
	}
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
}

// Taking the address of an init name is harmless when no other init shares the name: the one
// variable is the one Go declares.
func TestIfInitAnAddressTakenNameOfOneIfIsStillHoisted(t *testing.T) {
	sameButPos(t, ifInitPrelude+`
func Run() *error {
	if err := F(); err != nil {
		return &err
	}
	return nil
}
`, ifInitPrelude+`
func Run() *error {
	err := F()
	if err != nil {
		return &err
	}
	return nil
}
`)
}

// The second init is outside cir/1 (a slice expression), so that if stays foreign with its init in
// its text; err then occurs outside the first if too, whose err is read under a fresh name.
func TestIfInitAnInitOutsideCir1KeepsItsIfForeignAndRenamesTheOther(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run(errs []error) error {
	if err := F(); err != nil {
		return err
	}
	if err := errs[1:][0]; err != nil {
		return err
	}
	return nil
}
`)
	if len(m.Refusals) != 1 || m.Refusals[0].Pos != ifInitLine(5) || m.Refusals[0].Reason != "expression outside cir/1: slice expression" {
		t.Fatalf("refusals %v, want one at %s for the slice expression", m.Refusals, ifInitLine(5))
	}
	run := ifInitRun(t, m)
	if run.Body[0].K != "let" || run.Body[0].Names[0] != "err_1" || run.Body[2].K != "foreign" {
		b, _ := json.MarshalIndent(run.Body, "", " ")
		t.Fatalf("body:\n%s", b)
	}
}

// ifInitLambda returns the first lambda in body, searching statements and their expressions.
func ifInitLambda(body []Stmt) *Expr {
	var inExpr func(e *Expr) *Expr
	inExpr = func(e *Expr) *Expr {
		if e == nil {
			return nil
		}
		if e.K == "lambda" {
			return e
		}
		for _, p := range []*Expr{e.X, e.Y} {
			if l := inExpr(p); l != nil {
				return l
			}
		}
		for i := range e.Args {
			if l := inExpr(&e.Args[i]); l != nil {
				return l
			}
		}
		return nil
	}
	var inStmts func(ss []Stmt) *Expr
	inStmts = func(ss []Stmt) *Expr {
		for i := range ss {
			s := &ss[i]
			for _, p := range []*Expr{s.X, s.Cond} {
				if l := inExpr(p); l != nil {
					return l
				}
			}
			for _, list := range [][]Expr{s.Rhs, s.Results} {
				for j := range list {
					if l := inExpr(&list[j]); l != nil {
						return l
					}
				}
			}
			for _, b := range [][]Stmt{s.Then, s.Else, s.Body} {
				if l := inStmts(b); l != nil {
					return l
				}
			}
		}
		return nil
	}
	return inStmts(body)
}

// An if with an init inside a function literal is hoisted within the literal's body, as the
// two-statement form inside the literal reads; the literal's own err is not a capture.
func TestIfInitInsideAFunctionLiteralIsHoistedWithinTheLiteral(t *testing.T) {
	m := sameButPos(t, ifInitPrelude+`
func Run() error {
	check := func() error {
		if err := F(); err != nil {
			return err
		}
		return nil
	}
	return check()
}
`, ifInitPrelude+`
func Run() error {
	check := func() error {
		err := F()
		if err != nil {
			return err
		}
		return nil
	}
	return check()
}
`)
	l := ifInitLambda(ifInitRun(t, m).Body)
	if l == nil || len(l.Body) != 3 || l.Body[0].K != "let" || l.Body[1].K != "if" || len(l.Captures) != 0 {
		t.Fatalf("lambda %+v", l)
	}
}

// The name an init inside a literal declares is scoped to the literal: hoisting it there leaves
// the enclosing function's own err, which Run returns, untouched and uncaptured.
func TestIfInitInsideAFunctionLiteralDoesNotTouchTheEnclosingName(t *testing.T) {
	m := sameButPos(t, ifInitPrelude+`
func Run() error {
	err := F()
	check := func() error {
		if err := G(); err != nil {
			return err
		}
		return nil
	}
	if check() != nil {
		return nil
	}
	return err
}
`, ifInitPrelude+`
func Run() error {
	err := F()
	check := func() error {
		err := G()
		if err != nil {
			return err
		}
		return nil
	}
	if check() != nil {
		return nil
	}
	return err
}
`)
	run := ifInitRun(t, m)
	if got := ifInitAssignsTo(run.Body, "err"); len(got) != 0 {
		t.Fatalf("an assignment to err: %+v", got)
	}
	if l := ifInitLambda(run.Body); l == nil || len(l.Captures) != 0 || l.Body[0].K != "let" {
		t.Fatalf("lambda %+v", l)
	}
}

// A literal inside a hoisted if captures the hoisted name: one variable, as Go has, read as the
// two-statement form reads.
func TestIfInitALiteralCapturingAHoistedNameCapturesIt(t *testing.T) {
	m := sameButPos(t, ifInitPrelude+`
func Run() error {
	if err := F(); err != nil {
		get := func() error { return err }
		return get()
	}
	return nil
}
`, ifInitPrelude+`
func Run() error {
	err := F()
	if err != nil {
		get := func() error { return err }
		return get()
	}
	return nil
}
`)
	run := ifInitRun(t, m)
	if run.Body[0].K != "let" {
		t.Fatalf("body %+v", run.Body)
	}
	if l := ifInitLambda(run.Body); l == nil || !reflect.DeepEqual(l.Captures, []string{"err"}) {
		t.Fatalf("lambda %+v", l)
	}
}

// Planning reads the condition and init of each candidate before the body is read; that read
// leaves nothing behind: a literal in the condition refuses its goroutine once, not twice.
func TestIfInitPlanningLeavesNoRefusalOfItsOwn(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	if err := G(); func() bool { go G(); return err != nil }() {
		return err
	}
	return nil
}
`)
	if len(m.Refusals) != 1 || m.Refusals[0].Reason != "goroutine: concurrency is outside cir/1" {
		t.Fatalf("refusals %v, want the goroutine's alone", m.Refusals)
	}
	if run := ifInitRun(t, m); run.Body[0].K != "let" {
		t.Fatalf("body %+v", run.Body)
	}
}

// Planning declares no extern: an extern call only in the text of a foreign if is not declared.
// The init reads, the condition (a slice expression) does not: the if is foreign, and the extern its
// init declared is taken back.
func TestIfInitPlanningDeclaresNoExtern(t *testing.T) {
	prelude := strings.Replace(ifInitPrelude, `import "errors"`, "import (\n\t\"errors\"\n\t\"os\"\n)", 1)
	m := loadSource(t, prelude+`
func Run(xs []int) error {
	err := F()
	if err := os.Remove("x"); xs[1:] != nil {
		return err
	}
	return err
}
`)
	for _, x := range m.Externs {
		if x.Name == "Remove" {
			t.Fatalf("os.Remove declared though only foreign text calls it; refusals %v", m.Refusals)
		}
	}
}

// An init of a type outside cir/1 is not hoisted: the if stays foreign with its init in its
// text, never a foreign declaration in front of an if that names it.
func TestIfInitAnInitOfATypeOutsideCir1StaysForeign(t *testing.T) {
	m := loadSource(t, `package p

import "unsafe"

type g struct{ mu unsafe.Pointer }

var cur *g

func pick() *g { return cur }

func Run() int {
	if gp := pick(); gp != nil {
		return 1
	}
	return 0
}
`)
	if _, err := encode(m); err != nil {
		t.Fatalf("encode: %v", err)
	}
	run := ifInitRun(t, m)
	if len(run.Body) != 2 || run.Body[0].K != "foreign" || !strings.HasPrefix(run.Body[0].Src, "if gp := pick()") {
		b, _ := json.MarshalIndent(run.Body, "", " ")
		t.Fatalf("body, refusals %v:\n%s", m.Refusals, b)
	}
}

// Reading a literal plans its body and then restores the enclosing function's plan: the ifs of
// Run after the literal are still hoisted, the second as an assignment to the first's err, and the
// literal's own e stays a declaration inside it.
func TestIfInitAfterAFunctionLiteralTheEnclosingPlanIsRestored(t *testing.T) {
	m := sameButPos(t, ifInitPrelude+`
func Run() error {
	if err := F(); err != nil {
		return err
	}
	check := func() error {
		if e := G(); e != nil {
			return e
		}
		return nil
	}
	if err := G(); err != nil {
		return err
	}
	return check()
}
`, ifInitPrelude+`
func Run() error {
	err := F()
	if err != nil {
		return err
	}
	check := func() error {
		e := G()
		if e != nil {
			return e
		}
		return nil
	}
	err = G()
	if err != nil {
		return err
	}
	return check()
}
`)
	run := ifInitRun(t, m)
	if len(run.Body) != 6 || run.Body[0].K != "let" || run.Body[3].K != "assign" || run.Body[4].K != "if" {
		b, _ := json.MarshalIndent(run.Body, "", " ")
		t.Fatalf("body:\n%s", b)
	}
	if l := ifInitLambda(run.Body); l == nil || len(l.Body) != 3 || l.Body[0].K != "let" {
		t.Fatalf("lambda %+v", l)
	}
}
