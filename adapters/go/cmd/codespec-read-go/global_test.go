package main

import (
	"encoding/json"
	"strings"
	"testing"
)

// Package-level variables are module vars: a typed zero value, an initializer cir/1 expresses, or
// an initializer kept as foreign Go with a refusal. A function reads and assigns them as globals;
// errors.New(<literal>) stays a sentinel error, a variable of an extern type (sync.Mutex) is a var,
// and a variable of a type outside cir/1 (uintptr) is refused for its type.
func TestPackageVariablesReadAsModuleVarsAndGlobals(t *testing.T) {
	m := loadSource(t, `package p

import (
	"errors"
	"sync"
)

var ErrGone = errors.New("gone")

var (
	hits     int
	greeting = "hello"
	limit    int64 = 3
	names    = []string{"a"}
	mu       sync.Mutex
	addr     uintptr
)

func Greet(name string) string {
	hits = hits + 1
	return greeting + name
}

func Over() bool {
	return limit > 2
}

func Fail() error {
	return ErrGone
}
`)
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	b, _ := json.Marshal(doc)
	s := string(b)
	for _, want := range []string{
		`"vars":[{"name":"hits","pos":"p.go:11","t":{"k":"int","signed":true,"width":"platform"}}`,
		`{"init":{"k":"lit","t":{"k":"string"},"value":"\"hello\""},"name":"greeting"`,
		`{"init":{"k":"lit","t":{"k":"int","signed":true,"width":"64"},"value":"3"},"name":"limit"`,
		`{"foreign":{"lang":"go","reason":"initializer outside cir/1","src":"[]string{\"a\"}"},"name":"names"`,
		`"lhs":[{"k":"global","name":"hits"`,
		`{"k":"global","name":"greeting","t":{"k":"string"}}`,
		`{"k":"errvar","name":"ErrGone"`,
	} {
		if !strings.Contains(s, want) {
			t.Errorf("missing %s in\n%s", want, s)
		}
	}
	if !strings.Contains(s, `{"name":"mu","pos":"p.go:15","t":{"k":"extern","name":"Mutex","path":"sync"}}`) {
		t.Errorf("mu, of the extern type sync.Mutex, is not a var in\n%s", s)
	}
	if strings.Contains(s, `"name":"addr"`) {
		t.Errorf("addr, of a type outside cir/1, read as a var")
	}
	var reasons []string
	for _, r := range m.Refusals {
		reasons = append(reasons, r.What+": "+r.Reason)
		if strings.Contains(r.Reason, "package-level variable") {
			t.Errorf("refusal names the old cause: %s: %s", r.What, r.Reason)
		}
	}
	got := strings.Join(reasons, "\n")
	for _, want := range []string{
		"names: initializer outside cir/1",
		"addr: variable of type uintptr has no cir/1 type",
	} {
		if !strings.Contains(got, want) {
			t.Errorf("missing refusal %q in\n%s", want, got)
		}
	}
	for _, f := range []string{"Greet", "Over"} {
		for _, st := range funcNamed(t, m, f).Body {
			if st.K == "foreign" {
				t.Errorf("%s: foreign statement %q: %s", f, st.Src, st.Reason)
			}
		}
	}
	if !has(m.Layout, "var:hits") || !has(m.Layout, "var:names") || !has(m.Layout, "var:mu") || has(m.Layout, "var:addr") {
		t.Errorf("layout %v", m.Layout)
	}
}

// A constant of an extern type over uintptr (syscall.Errno) has no cir/1 literal: it is refused, and
// the package still encodes (internal/syscall/unix and os failed whole before).
func TestAConstantOverUintptrIsRefusedAndThePackageEncodes(t *testing.T) {
	m := loadSource(t, `package p

import "syscall"

const Again = syscall.EAGAIN
`)
	if _, err := encode(m); err != nil {
		t.Fatal(err)
	}
	if len(m.Consts) != 0 {
		t.Errorf("consts %v", m.Consts)
	}
}
