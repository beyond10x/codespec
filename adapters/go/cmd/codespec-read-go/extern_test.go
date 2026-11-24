package main

import (
	"strings"
	"testing"
)

// A call of a function of another package with cir/1 types is an extern call, declared once from
// the callee's signature, a variadic callee (path.Join) declared variadic; a nil argument typed from its
// parameter (os.WriteFile); one with a parameter type cir/1 has no type for (complex128) stays
// foreign with a refusal.
func TestStandardLibraryCallsReadAsExternsUnlessForeignTyped(t *testing.T) {
	m := loadSource(t, `package p

import (
	"os"
	"path"
	"strconv"
	"strings"
)

func Shout(s string) string {
	return strings.ToUpper(s) + strings.ToUpper(s)
}

func Port() (int, error) {
	return strconv.Atoi(os.Getenv("PORT"))
}

func Under(s string) string {
	return path.Join("root", s)
}

func Save(s string) error {
	return os.WriteFile(s, nil, 0o600)
}

func Complex() string {
	return strconv.FormatComplex(1i, 'g', -1, 128)
}
`)
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	externs, _ := doc["externs"].([]any)
	var got []string
	for _, x := range externs {
		n := x.(node)
		got = append(got, n["path"].(string)+"."+n["name"].(string))
	}
	want := []string{"os.Getenv", "os.WriteFile", "path.Join", "strconv.Atoi", "strings.ToUpper"}
	if len(got) != len(want) {
		t.Fatalf("externs %v, want %v", got, want)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("externs %v, want %v", got, want)
		}
	}
	if len(m.Refusals) != 1 || !strings.Contains(m.Refusals[0].What, "strconv.FormatComplex") {
		t.Fatalf("want only strconv.FormatComplex (complex128) refused, got %v", m.Refusals)
	}
}
