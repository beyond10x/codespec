package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func loadSource(t *testing.T, src string) *Module {
	t.Helper()
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "p.go"), []byte(src), 0o644); err != nil {
		t.Fatal(err)
	}
	m, err := load(dir)
	if err != nil {
		t.Fatal(err)
	}
	return m
}

// kinds lists the k of every statement in body and in each block nested in it, depth first.
func kinds(body []Stmt) []string {
	var out []string
	var walk func(ss []Stmt)
	walk = func(ss []Stmt) {
		for _, s := range ss {
			out = append(out, s.K)
			walk(s.Then)
			walk(s.Else)
			walk(s.Body)
			for _, c := range s.Cases {
				walk(c.Body)
			}
		}
	}
	walk(body)
	return out
}

func count(ks []string, k string) int {
	n := 0
	for _, x := range ks {
		if x == k {
			n++
		}
	}
	return n
}

func TestALabeledBreakFromASwitchIsAPlainBreakOfTheLoop(t *testing.T) {
	m := loadSource(t, `package p

func U(n int, stop int) int {
	i := 0
loop:
	for i < n {
		switch i {
		case stop:
			break loop
		}
		i++
	}
	return i
}
`)
	ks := kinds(m.Funcs[0].Body)
	if len(m.Refusals) != 0 || count(ks, "foreign") != 0 {
		t.Fatalf("refusals %v, kinds %v", m.Refusals, ks)
	}
	want := []string{"let", "for", "switch", "break", "incr", "return"}
	if len(ks) != len(want) {
		t.Fatalf("kinds %v, want %v", ks, want)
	}
	for i := range want {
		if ks[i] != want[i] {
			t.Fatalf("kinds %v, want %v", ks, want)
		}
	}
}

func TestContinueAndBreakOfTheSameLabelAreReadAsPlainContinueAndBreak(t *testing.T) {
	m := loadSource(t, `package p

func M(xs []int) int {
	s := 0
outer:
	for _, x := range xs {
		switch {
		case x < 0:
			continue outer
		case x > 100:
			break outer
		}
		s += x
	}
	return s
}
`)
	ks := kinds(m.Funcs[0].Body)
	if len(m.Refusals) != 0 || count(ks, "foreign") != 0 {
		t.Fatalf("refusals %v, kinds %v", m.Refusals, ks)
	}
	if count(ks, "continue") != 1 || count(ks, "break") != 1 || count(ks, "for_range") != 1 {
		t.Fatalf("kinds %v: want one for_range, one continue, one break", ks)
	}
}

func TestABranchInsideASwitchThatTurnsForeignKeepsTheWholeLabeledLoopForeign(t *testing.T) {
	m := loadSource(t, `package p

func S(n int, s string) int {
	i := 0
L:
	for i < n {
		switch i {
		case 3:
			break L
		case len(s[1:]):
			i++
		}
		i++
	}
	return i
}
`)
	body := m.Funcs[0].Body
	ks := kinds(body)
	if len(ks) != 3 || ks[1] != "foreign" {
		t.Fatalf("kinds %v, want [let foreign return]", ks)
	}
	if dangling := adversaryForeignNaming(body, "L"); len(dangling) > 0 {
		t.Fatalf("foreign source names a label the output dropped: %q", dangling)
	}
	if len(m.Refusals) != 1 {
		t.Fatalf("refusals %v: want only the labeled statement's", m.Refusals)
	}
}

func TestABreakOfAnOuterLabelFromANestedLoopKeepsTheLabeledLoopForeign(t *testing.T) {
	m := loadSource(t, `package p

func N(n int) int {
	c := 0
L:
	for i := 0; i < n; i++ {
		for j := 0; j < n; j++ {
			if j == 2 {
				break L
			}
			c++
		}
	}
	return c
}
`)
	ks := kinds(m.Funcs[0].Body)
	if len(ks) != 3 || ks[1] != "foreign" || len(m.Refusals) != 1 {
		t.Fatalf("kinds %v, refusals %v", ks, m.Refusals)
	}
}

func TestAGotoFromOutsideTheLoopKeepsTheLabeledLoopForeignWithItsLabel(t *testing.T) {
	m := loadSource(t, `package p

func G(n int) int {
	i := 0
	if n > 100 {
		goto L
	}
	i = 1
L:
	for i < n {
		i++
	}
	return i
}
`)
	body := m.Funcs[0].Body
	ks := kinds(body)
	if count(ks, "for") != 0 || count(ks, "foreign") != 2 {
		t.Fatalf("kinds %v: want the goto and the labeled loop foreign", ks)
	}
	if !strings.HasPrefix(body[3].Src, "L:") {
		t.Fatalf("the labeled loop is not kept with its label: %q", body[3].Src)
	}
}
