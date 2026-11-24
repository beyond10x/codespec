package main

import (
	"strings"
	"testing"
)

// shapePrelude declares what the shape cases below use. Every name is made up.
const shapePrelude = `package p

import (
	"image"
	"io"
	"reflect"
	"slices"
	"time"
)

type Item struct{ n int }

func (i *Item) Get() int { return i.n }

func two() (int, error) { return 0, nil }

func take(a int, e error) int { return a }

func id[T any](x T) T { return x }

func use(x any) {}

var first, second = two()
`

// Each case is one function whose body holds one statement the reader keeps foreign; the
// statement's refusal names the shape of its first sub-expression with no cir/1 form.
var shapeCases = []struct{ shape, body string }{
	{"slice expression", "func A(xs []int) []int { return xs[1:] }"},
	{"field of a struct of another package", "func A(p image.Point) int { return p.X }"},
	{"conversion to int64", "func A(n int) int64 { return int64(n) }"},
	{"conversion to string", "func A(b []byte) string { return string(b) }"},
	{"conversion to []byte", "func A(s string) []byte { return []byte(s) }"},
	{"conversion to time.Duration", "func A(n int) time.Duration { return time.Duration(n) }"},
	{"constant of another package", "func A() time.Duration { return time.Second }"},
	{"variable of another package", "func A(err error) bool { return err == io.EOF }"},
	{"builtin make", "func A(n int) []int { return make([]int, n) }"},
	{"builtin panic", "func A() { panic(\"stop\") }"},
	{"builtin delete", "func A(m map[string]int) { delete(m, \"k\") }"},
	{"`_` as an operand", "func A(xs []int) { xs[0], _ = two() }"},
	{"call of a generic function whose constraint cir/1 lacks", "func A(xs []int) bool { return slices.Contains(xs, 1) }"},
	{"type assertion", "func A(x any) int { return x.(int) }"},
	{"literal of another package's struct", "func A() image.Point { return image.Point{X: 1} }"},
	{"literal of an anonymous struct type", "func A() int { return struct{ a int }{a: 1}.a }"},
	{"call through an interface cir/1 cannot declare", "func A(t reflect.Type) string { return t.Name() }"},
	{"call passed a multi-valued call", "func A() int { return take(two()) }"},
	{"array literal", "func A() [2]int { return [2]int{1, 2} }"},
	{"map literal with elements", "func A() map[string]int { return map[string]int{\"a\": 1} }"},
	{"method expression", "func A() func(*Item) int { return (*Item).Get }"},
	{"method value of an interface", "func A(w io.Writer) func([]byte) (int, error) { return w.Write }"},
	{"function literal of a variadic type", "func A() { use(func(xs ...int) {}) }"},
	{"generic function used as a value", "func A() func(int) int { return id[int] }"},
	{"use of a refused package variable", "func A() int { return first }"},
}

func TestARefusedExpressionNamesItsShape(t *testing.T) {
	var b strings.Builder
	b.WriteString(shapePrelude)
	lines := map[string]string{} // "p.go:<line>" of each case's function to the shape
	line := strings.Count(shapePrelude, "\n") + 1
	for i, c := range shapeCases {
		b.WriteString("\n")
		line++
		lines["p.go:"+itoa(line)] = c.shape
		b.WriteString(strings.Replace(c.body, "func A(", "func A"+itoa(i)+"(", 1) + "\n")
		line++
	}
	b.WriteString("\nvar tail = []int{1, 2, 3}[1:]\n")
	line++
	initLine := "p.go:" + itoa(line)

	m := loadSource(t, b.String())
	got := map[string]string{}
	for _, r := range m.Refusals {
		got[r.Pos] = r.Reason
	}
	for pos, shape := range lines {
		want := "expression outside cir/1: " + shape
		if got[pos] != want {
			t.Errorf("%s: reason %q, want %q", pos, got[pos], want)
		}
	}
	if want := "initializer outside cir/1: slice expression"; got[initLine] != want {
		t.Errorf("%s: reason %q, want %q", initLine, got[initLine], want)
	}
}

func itoa(n int) string {
	var d []byte
	for n > 0 || len(d) == 0 {
		d = append([]byte{byte('0' + n%10)}, d...)
		n /= 10
	}
	return string(d)
}
