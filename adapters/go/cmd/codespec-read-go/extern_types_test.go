package main

import (
	"encoding/json"
	"strings"
	"testing"
)

// A type of another package is an opaque extern type, so a struct holding one is declared; a
// method called on a value of one is an extern method call, declared once with its receiver; a
// field of a struct of another package stays foreign.
func TestTypesOfAnotherPackageReadAsExternTypesAndTheirMethodsAsExternMethods(t *testing.T) {
	m := loadSource(t, `package p

import (
	"net/url"
	"strings"
	"sync"
)

type Tally struct {
	mu    sync.Mutex
	words *strings.Builder
	n     int
}

func (t *Tally) Add(w string) int {
	t.mu.Lock()
	t.words.WriteString(w)
	t.n++
	t.mu.Unlock()
	return t.words.Len()
}

func Show(u *url.URL) string {
	return u.String()
}

func Host(u *url.URL) string {
	return u.Host
}
`)
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	src := string(b)
	if len(m.Types) != 1 || m.Types[0].Name != "Tally" {
		t.Fatalf("Tally not declared: types %v, refusals %v", m.Types, m.Refusals)
	}
	if f := m.Types[0].Fields[0].T; f.K != "extern" || f.Path != "sync" || f.Name != "Mutex" {
		t.Fatalf("field mu is %+v, want extern sync.Mutex", f)
	}
	externs, _ := doc["externs"].([]any)
	var got []string
	for _, x := range externs {
		n := x.(node)
		key := n["path"].(string) + "."
		if r, ok := n["recv"].(node); ok {
			for r["k"] == "ptr" {
				r = r["elem"].(node)
			}
			key += r["name"].(string) + "."
		}
		got = append(got, key+n["name"].(string))
	}
	want := []string{
		"net/url.URL.String",
		"strings.Builder.Len",
		"strings.Builder.WriteString",
		"sync.Mutex.Lock",
		"sync.Mutex.Unlock",
	}
	if strings.Join(got, " ") != strings.Join(want, " ") {
		t.Fatalf("externs %v, want %v", got, want)
	}
	if !strings.Contains(src, `"k":"extern_method"`) || !strings.Contains(src, `"type":"Mutex"`) {
		t.Fatalf("no extern_method call on sync.Mutex in %s", src)
	}
	if len(m.Refusals) != 1 || !strings.Contains(m.Refusals[0].What, "u.Host") {
		t.Fatalf("want one refusal, of the field u.Host of a struct of another package, got %v", m.Refusals)
	}
}
