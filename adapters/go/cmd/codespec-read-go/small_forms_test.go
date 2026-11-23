package main

import (
	"encoding/json"
	"strings"
	"testing"
)

const smallPrelude = `package p

import (
	"io"
	"math"
	"sync"
)

type Point struct {
	x, y int
	tag  string
}

type Node struct {
	val  int
	next *Node
}

type Holder struct {
	wg  sync.WaitGroup
	out io.Writer
}

var _ = math.MaxInt8
`

// Each small form reads as the form of the same meaning cir/1 already has, written by hand.
func TestSmallLiteralAndBuiltinFormsReadAsTheirHandWrittenForm(t *testing.T) {
	sameButPos(t, smallPrelude+`
func Positional() Point { return Point{1, 2, "a"} }

func Slice(a, b int) []int { return []int{a, b, 3} }

func Pointers() []*Node { return []*Node{{val: 1}, &Node{val: 2}} }

func Maps(xs []int) (map[string]int, map[int][]int) {
	return make(map[string]int), make(map[int][]int, len(xs))
}

func Empty(xs []int) ([]int, []string) {
	return make([]int, 0), make([]string, 0, 8)
}

func Fresh() *Node { return new(Node) }

func Limit() int { return math.MaxInt32 }
`, smallPrelude+`
func Positional() Point { return Point{x: 1, y: 2, tag: "a"} }

func Slice(a, b int) []int { return append([]int{}, a, b, 3) }

func Pointers() []*Node { return append([]*Node{}, &Node{val: 1}, &Node{val: 2}) }

func Maps(xs []int) (map[string]int, map[int][]int) {
	return map[string]int{}, map[int][]int{}
}

func Empty(xs []int) ([]int, []string) {
	return []int{}, []string{}
}

func Fresh() *Node { return &Node{} }

func Limit() int { return 2147483647 }
`)
}

// An untyped nil takes the type of the parameter, field or element it is given to; an empty
// literal of another package's struct is that type's zero value.
func TestNilArgumentsAndZeroValuesAreTyped(t *testing.T) {
	m := loadSource(t, smallPrelude+`
func Write(w io.Writer) (int, error) { return w.Write(nil) }

func Link() Node { return Node{val: 1, next: nil} }

func Nils() []*Node { return []*Node{nil} }

func Holders() Holder { return Holder{wg: sync.WaitGroup{}} }
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	for _, c := range []struct{ fn, want string }{
		{"Write", `{"k":"nil","t":{"k":"slice","elem":{"k":"int","bits":8,"unsigned":true}}}`},
		{"Link", `{"k":"nil","t":{"k":"ptr","elem":{"k":"named","name":"Node"}}}`},
		{"Nils", `{"k":"nil","t":{"k":"ptr","elem":{"k":"named","name":"Node"}}}`},
		{"Holders", `{"k":"zero","t":{"k":"extern","name":"WaitGroup","path":"sync"}}`},
	} {
		b, _ := json.Marshal(funcNamed(t, m, c.fn).Body)
		if !strings.Contains(string(b), c.want) {
			t.Errorf("%s: no %s in %s", c.fn, c.want, b)
		}
	}
}

// What wave 1 leaves refused keeps its shape: a length operand, a channel, a constant of another
// package's named type, a keyed slice literal, new of a type that is not a struct.
func TestSmallFormsCir1CannotExpressStayRefused(t *testing.T) {
	m := loadSource(t, `package p

import "time"

func Sized(n int) []int { return make([]int, n) }

func Channel() chan int { return make(chan int) }

func Wait() time.Duration { return time.Second }

func Keyed() []int { return []int{2: 1} }

func Counter() *int { return new(int) }
`)
	want := map[string]string{
		"p.go:5":  "expression outside cir/1: builtin make",
		"p.go:7":  "expression outside cir/1: builtin make",
		"p.go:9":  "expression outside cir/1: constant of another package",
		"p.go:11": "expression outside cir/1: slice literal with keyed elements",
		"p.go:13": "expression outside cir/1: builtin new",
	}
	if len(m.Refusals) != len(want) {
		t.Fatalf("refusals %v", m.Refusals)
	}
	for _, r := range m.Refusals {
		if want[r.Pos] != r.Reason {
			t.Errorf("%s: reason %q, want %q", r.Pos, r.Reason, want[r.Pos])
		}
	}
}
