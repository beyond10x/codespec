package main

import (
	"strings"
	"testing"
)

// A call of a function of another package with cir/1 types is an extern call, declared once from
// the callee's signature; a variadic callee (path.Join), and one with a parameter of another package's type,
// stay foreign with a refusal each.
func TestStandardLibraryCallsReadAsExternsUnlessVariadicOrForeignTyped(t *testing.T) {
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
	want := []string{"os.Getenv", "strconv.Atoi", "strings.ToUpper"}
	if len(got) != len(want) {
		t.Fatalf("externs %v, want %v", got, want)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("externs %v, want %v", got, want)
		}
	}
	if len(m.Refusals) != 2 || !strings.Contains(m.Refusals[0].What, "path.Join") || !strings.Contains(m.Refusals[1].What, "os.WriteFile") {
		t.Fatalf("want refusals of path.Join (variadic) and os.WriteFile (fs.FileMode), got %v", m.Refusals)
	}
}
