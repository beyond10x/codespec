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

// Each function keeps its if foreign, with the refusal at the if's line.
func TestAnIfInitWhoseHoistWouldChangeTheMeaningStaysForeign(t *testing.T) {
	for _, c := range []struct {
		name, src string
		line      int // the line of the if in src, counted from the first line of the function
	}{
		{"used outside", `
func Run() error {
	err := F()
	if err := G(); err != nil {
		return err
	}
	return err
}
`, 3},
		{"a parameter", `
func Run(err error) (int, error) {
	if err := F(); err != nil {
		return 0, err
	}
	return 1, err
}
`, 2},
		{"not a call, with an else", `
func Run() int {
	if x := 1; x > 0 {
		return x
	} else {
		return 0
	}
}
`, 2},
		{"an else if", `
func Run(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	} else if err := G(); err != nil {
		return 1, err
	}
	return 2, nil
}
`, 4},
		// the outer if is hoisted; the inner one would overwrite the err the outer body returns
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
`, 3},
		// the call reads the package-level ErrNegative that the init then shadows
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
`, 2},
	} {
		t.Run(c.name, func(t *testing.T) {
			src := ifInitPrelude + c.src
			m := loadSource(t, src)
			// the function starts on the line after the prelude's last line
			ifLine := strings.Count(ifInitPrelude, "\n") + 1 + c.line
			want := "p.go:" + strconv.Itoa(ifLine)
			if len(m.Refusals) != 1 || m.Refusals[0].Pos != want || m.Refusals[0].Reason != "if with an init statement" {
				t.Fatalf("refusals %v, want one at %s for the if with an init statement", m.Refusals, want)
			}
			var run *Func
			for i := range m.Funcs {
				if m.Funcs[i].Name == "Run" {
					run = &m.Funcs[i]
				}
			}
			if run == nil || count(kinds(run.Body), "foreign") != 1 {
				t.Fatalf("body %+v", run)
			}
		})
	}
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
// and the refusal names the condition, not the init.
func TestIfInitAnIfWhoseConditionIsOutsideCir1DoesNotAlsoHoistItsInit(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	if err := F(); func() bool { return err != nil }() {
		return err
	}
	return nil
}
`)
	run := ifInitRun(t, m)
	want := ifInitLine(2)
	if len(m.Refusals) != 1 || m.Refusals[0].Pos != want || m.Refusals[0].Reason != "if with an init statement" {
		t.Errorf("refusals %v, want one at %s for the if with an init statement", m.Refusals, want)
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

// One name declared by two inits with different types is not one variable: both ifs stay foreign.
func TestIfInitANameReusedWithAnotherTypeStaysForeign(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
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
`)
	want := []string{ifInitLine(6), ifInitLine(9)}
	if len(m.Refusals) != 2 || m.Refusals[0].Pos != want[0] || m.Refusals[1].Pos != want[1] ||
		m.Refusals[0].Reason != "if with an init statement" || m.Refusals[1].Reason != "if with an init statement" {
		t.Fatalf("refusals %v, want one at each of %v for the if with an init statement", m.Refusals, want)
	}
}

// A closure that captures the first init's err would see the second init's value were the
// second read as an assignment to it: neither is hoisted.
func TestIfInitAClosureCapturingASharedNameKeepsBothForeign(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	var get func() error
	if err := F(); err != nil {
		get = func() error { return err }
	}
	if err := G(); err != nil {
		return err
	}
	_ = get
	return nil
}
`)
	run := ifInitRun(t, m)
	if got := ifInitAssignsTo(run.Body, "err"); len(got) != 0 {
		b, _ := json.MarshalIndent(got, "", " ")
		t.Fatalf("the second init writes the variable the closure captured (refusals %v):\n%s", m.Refusals, b)
	}
	for _, want := range []string{ifInitLine(3), ifInitLine(6)} {
		found := false
		for _, r := range m.Refusals {
			found = found || (r.Pos == want && r.Reason == "if with an init statement")
		}
		if !found {
			t.Errorf("refusals %v, want one at %s for the if with an init statement", m.Refusals, want)
		}
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

// The second init's call is outside cir/1 (errors.Join), so that if stays foreign with its
// init in its text; err then occurs outside the first if too, which stays foreign as well.
func TestIfInitAnInitOutsideCir1KeepsEveryIfSharingItsNameForeign(t *testing.T) {
	m := loadSource(t, ifInitPrelude+`
func Run() error {
	if err := F(); err != nil {
		return err
	}
	if err := errors.Join(F()); err != nil {
		return err
	}
	return nil
}
`)
	want := []string{ifInitLine(2), ifInitLine(5)}
	if len(m.Refusals) != 2 || m.Refusals[0].Pos != want[0] || m.Refusals[1].Pos != want[1] ||
		m.Refusals[0].Reason != "if with an init statement" || m.Refusals[1].Reason != "if with an init statement" {
		t.Fatalf("refusals %v, want one at each of %v for the if with an init statement", m.Refusals, want)
	}
}
