package main

import (
	"strings"
	"testing"
)

const localDeclPrelude = `package p

type Fault struct{ code int }

func (f Fault) Error() string { return "fault" }

func pair() (int, string) { return 1, "a" }
`

// Each local declaration reads as the statements it means, written by hand: `var x = e` as
// `x := e`, `var x T = e` as `x := e` where e already has type T, `var a, b T` as one `var` per
// name, a group as its specs in order, a local constant as its literal at each use.
func TestLocalDeclarationsReadAsTheirHandWrittenForm(t *testing.T) {
	sameButPos(t, localDeclPrelude+`
func Init() int {
	var x = 1
	return x
}

func Typed(n int64) int64 {
	var x int64 = n
	return x
}

func Two() int {
	var a, b int
	return a + b
}

func Group() (int, string) {
	var (
		a    int
		b, c = pair()
	)
	return a + b, c
}

func Multi() (int, string) {
	var a, b = pair()
	return a, b
}

func Constant() int {
	const two = 2
	const name = "n"
	return two * len(name)
}

func Half() float64 {
	const half = 0.5
	return half
}
`, localDeclPrelude+`
func Init() int {
	x := 1
	return x
}

func Typed(n int64) int64 {
	x := n
	return x
}

func Two() int {
	var a int
	var b int
	return a + b
}

func Group() (int, string) {
	var a int
	b, c := pair()
	return a + b, c
}

func Multi() (int, string) {
	a, b := pair()
	return a, b
}

func Constant() int {
	return 2 * len("n")
}

func Half() float64 {
	return 0.5
}
`)
}

// `var e error = Fault{}` converts the struct to error, as `e := error(Fault{})` would.
func TestATypedLocalVarConvertsItsValueToTheDeclaredType(t *testing.T) {
	m := loadSource(t, localDeclPrelude+`
func Fails() error {
	var e error = Fault{code: 1}
	return e
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	body := funcNamed(t, m, "Fails").Body
	if body[0].K != "let" || body[0].Rhs[0].K != "to_error" {
		t.Fatalf("body %+v", body)
	}
}

// A type declared inside a function stays refused, with its own reason.
func TestALocalTypeDeclarationStaysRefused(t *testing.T) {
	m := loadSource(t, localDeclPrelude+`
func Local() int {
	type pt struct{ x int }
	return 1
}
`)
	if len(m.Refusals) != 1 || !strings.Contains(m.Refusals[0].Reason, "type declaration inside a function") {
		t.Fatalf("refusals %v", m.Refusals)
	}
}
