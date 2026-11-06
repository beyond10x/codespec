package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func adversaryLoad(t *testing.T, src string) *Module {
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

// adversaryDanglingLabels returns the foreign statements anywhere in body whose Go source names
// label while no statement in the output carries that label any more.
func adversaryForeignNaming(body []Stmt, label string) []string {
	var naming []string
	declared := false
	var walk func(ss []Stmt)
	walk = func(ss []Stmt) {
		for _, s := range ss {
			if s.K == "foreign" {
				if strings.Contains(s.Src, label+":") {
					declared = true
				}
				if strings.Contains(s.Src, "goto "+label) || strings.Contains(s.Src, "break "+label) || strings.Contains(s.Src, "continue "+label) {
					naming = append(naming, s.Src)
				}
				continue
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
	if declared {
		return nil
	}
	return naming
}

// A `goto L` outside the labeled loop names the label too. labeled() promises that a loop whose
// label names anything but its own break or continue stays foreign; ownBranches only walks the
// loop body, so the loop is read as a plain loop and the foreign `goto L` names a label the
// output no longer has.
func TestAdversaryGotoFromOutsideTheLoopKeepsTheLabeledLoopForeign(t *testing.T) {
	m := adversaryLoad(t, `package p

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
	if dangling := adversaryForeignNaming(body, "L"); len(dangling) > 0 {
		t.Fatalf("foreign source names label L, which the output dropped: %q; body kinds %v",
			dangling, adversaryKinds(body))
	}
}

// A `break L` read (and marked) inside a switch clause, where a later clause turns the whole
// switch foreign: the mark survives, so the labeled loop is read as a plain loop while the
// foreign switch's source still says `break L`.
func TestAdversaryBranchInsideASwitchThatTurnsForeignKeepsTheLabeledLoopForeign(t *testing.T) {
	m := adversaryLoad(t, `package p

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
	if dangling := adversaryForeignNaming(body, "L"); len(dangling) > 0 {
		t.Fatalf("foreign source names label L, which the output dropped: %q; body kinds %v",
			dangling, adversaryKinds(body))
	}
}

func adversaryKinds(ss []Stmt) []string {
	var out []string
	for _, s := range ss {
		out = append(out, s.K)
	}
	return out
}
