package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Both variadic cases read with no refusal: the variadic function's last parameter is marked, a
// variadic extern is declared variadic, and a spread argument is a `spread` expression.
func TestVariadicCasesReadWithoutRefusal(t *testing.T) {
	for _, c := range []struct {
		dir, fn string
		want    []string
	}{
		{"decl-variadic", "Sums", []string{"call"}},
		{"decl-variadic", "Joined", []string{"extern"}},
		{"expr-spread", "Twice", []string{"intrinsic", "spread", "call"}},
		{"expr-spread", "Joined", []string{"extern", "spread"}},
	} {
		src, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "conformance", "cases", c.dir, "case.go"))
		if err != nil {
			t.Fatal(err)
		}
		m := loadSource(t, string(src))
		if len(m.Refusals) != 0 {
			t.Fatalf("%s: refusals %v", c.dir, m.Refusals)
		}
		var sum, fn *Func
		for i := range m.Funcs {
			switch m.Funcs[i].Name {
			case "sum":
				sum = &m.Funcs[i]
			case c.fn:
				fn = &m.Funcs[i]
			}
		}
		if sum == nil || fn == nil {
			t.Fatalf("%s: no sum or %s in %+v", c.dir, c.fn, m.Funcs)
		}
		if p := sum.Params[0]; !p.Variadic || p.T.K != "slice" {
			t.Fatalf("%s: sum's parameter %+v is not a variadic slice", c.dir, p)
		}
		got := strings.Join(exprKinds(fn.Body), " ")
		for _, k := range c.want {
			if !strings.Contains(" "+got+" ", " "+k+" ") {
				t.Fatalf("%s: %s has expressions %q, want %s among them", c.dir, c.fn, got, k)
			}
		}
		if _, err := encode(m); err != nil {
			t.Fatalf("%s: %v", c.dir, err)
		}
		var join *Extern
		for i := range m.Externs {
			if m.Externs[i].key() == "path.Join" {
				join = &m.Externs[i]
			}
		}
		if join == nil || !join.Variadic {
			t.Fatalf("%s: path.Join not declared variadic: %+v", c.dir, m.Externs)
		}
	}
}

// Variadic functions of other packages are extern calls, a method of another package's type
// among them; fmt.Sprintf without a spread stays the sprintf intrinsic, with one it is an extern
// call; append of a string spread onto a byte slice stays outside cir/1.
func TestVariadicExternsAreExternCalls(t *testing.T) {
	m := loadSource(t, `package p

import (
	"errors"
	"fmt"
	"log/slog"
	"strings"
)

func Log(l *slog.Logger, msg string, n int) {
	slog.Info(msg, "n", n)
	l.Info(msg)
}

func Join(a error, b error) error {
	return errors.Join(a, b)
}

func Format(f string, args []any) string {
	return fmt.Sprintf(f, args...) + fmt.Sprintf(f, 1)
}

func Replace(s string, pairs []string) string {
	return strings.NewReplacer(pairs...).Replace(s)
}

func Bytes(b []byte, s string) []byte {
	return append(b, s...)
}
`)
	var refused []string
	for _, r := range m.Refusals {
		refused = append(refused, r.What+": "+r.Reason)
	}
	if len(refused) != 1 || !strings.Contains(refused[0], "append(b, s...)") {
		t.Fatalf("want only append(b, s...) refused, got %q", refused)
	}
	var keys []string
	for _, x := range m.Externs {
		if x.Variadic {
			keys = append(keys, x.key())
		}
	}
	for _, want := range []string{"log/slog.Info", "log/slog.Logger.Info", "errors.Join", "fmt.Sprintf", "strings.NewReplacer"} {
		if !has(keys, want) {
			t.Fatalf("%s not declared variadic among %q", want, keys)
		}
	}
	got := strings.Join(exprKinds(m.Funcs[2].Body), " ")
	if want := "binop extern var spread var intrinsic var lit"; got != want {
		t.Fatalf("Format reads to %q, want %q", got, want)
	}
	if _, err := encode(m); err != nil {
		t.Fatal(err)
	}
}
