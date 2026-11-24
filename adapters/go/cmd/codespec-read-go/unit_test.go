package main

import (
	"encoding/json"
	"strings"
	"testing"
)

// struct{} reads as unit wherever a type holds it, and struct{}{} as the zero value of unit, so a
// struct, method or function that only held struct{} reads, and so does a call through
// context.Context, whose Done method returns <-chan struct{}.
func TestEmptyStructReadsAsUnit(t *testing.T) {
	m := loadSource(t, `package p

import "context"

type Stopper struct{ done chan struct{} }

func (s *Stopper) Stop() { s.done <- struct{}{} }

type Set struct{ m map[string]struct{} }

func (s *Set) Add(k string) { s.m[k] = struct{}{} }

func (s *Set) Has(k string) bool {
	_, ok := s.m[k]
	return ok
}

func Wait(ch chan struct{}) { <-ch }

func Closed() <-chan struct{} { return nil }

func Check(ctx context.Context) error { return ctx.Err() }
`)
	if len(m.Refusals) != 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	b, _ := json.Marshal(m)
	doc := string(b)
	for _, want := range []string{
		`{"name":"done","t":{"k":"chan","dir":"both","elem":{"k":"unit"}}}`,
		`{"name":"m","t":{"k":"map","key":{"k":"string"},"elem":{"k":"unit"}}}`,
		`{"k":"zero","t":{"k":"unit"}}`,
		`"results":[{"k":"chan","dir":"recv","elem":{"k":"unit"}}]`,
		`"name":"Err","path":"context","iface":"Context"`,
	} {
		if !strings.Contains(doc, want) {
			t.Errorf("no %s in\n%s", want, doc)
		}
	}
}

// A struct type with fields and no name is still outside cir/1.
func TestAnAnonymousStructWithFieldsStaysOutside(t *testing.T) {
	m := loadSource(t, `package p

type Holder struct{ cfg struct{ a int } }
`)
	if len(m.Refusals) != 1 || !strings.Contains(m.Refusals[0].Reason, "struct{a int} has no cir/1 type") {
		t.Fatalf("refusals %v", m.Refusals)
	}
}
