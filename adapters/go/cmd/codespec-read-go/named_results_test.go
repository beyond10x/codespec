package main

import (
	"strings"
	"testing"
)

const namedPrelude = `package p

import "errors"

var ErrEmpty = errors.New("empty")
`

// Without a defer nothing runs between a return and the caller, so named results are locals
// declared at the top with their zero values, and a bare return returns them: the unnamed form,
// written by hand.
func TestNamedResultsWithoutDeferReadAsTheUnnamedForm(t *testing.T) {
	sameButPos(t, namedPrelude+`
func Bare(s string) (n int, err error) {
	if s == "" {
		err = ErrEmpty
		return
	}
	n = len(s)
	return
}

func Blank(k int) (_ int, ok bool) {
	ok = k > 0
	return
}

func Literal() func() (v int) {
	return func() (v int) {
		v = 3
		return
	}
}
`, namedPrelude+`
func Bare(s string) (int, error) {
	var n int
	var err error
	if s == "" {
		err = ErrEmpty
		return n, err
	}
	n = len(s)
	return n, err
}

func Blank(k int) (int, bool) {
	var r_1 int
	var ok bool
	ok = k > 0
	return r_1, ok
}

func Literal() func() int {
	return func() int {
		var v int
		v = 3
		return v
	}
}
`)
}

// A return that names its values returns them, the named results declared but unread.
func TestNamedResultsWithExplicitReturnsDeclareThemAndReturnTheValues(t *testing.T) {
	m := loadSource(t, namedPrelude+`
func Explicit() (n int, err error) {
	return 1, nil
}
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	fn := funcNamed(t, m, "Explicit")
	b := fn.Body
	if len(fn.Results) != 2 || len(b) != 3 || b[0].K != "var" || b[0].Names[0] != "n" || b[1].K != "var" ||
		b[1].Names[0] != "err" || b[2].K != "return" || b[2].Results[0].Value != "1" || b[2].Results[1].K != "nil" {
		t.Fatalf("body %+v", b)
	}
}

// A defer can change named results after the return: such a function stays kept whole.
func TestNamedResultsWithADeferStayKeptWhole(t *testing.T) {
	m := loadSource(t, namedPrelude+`
func Deferred() (err error) {
	defer func() {
		if err != nil {
			err = ErrEmpty
		}
	}()
	return errors.New("x")
}
`)
	if len(m.Refusals) != 1 || !strings.HasPrefix(m.Refusals[0].Reason, "named results are outside cir/1") {
		t.Fatalf("refusals %v", m.Refusals)
	}
}
