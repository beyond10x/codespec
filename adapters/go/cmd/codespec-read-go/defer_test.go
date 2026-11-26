package main

import (
	"strings"
	"testing"
)

// `defer c.mu.Unlock()` reads as a `defer` statement holding the call, its receiver read as any
// other expression, with no refusal (docs/query-language.md § 8).
func TestADeferOfACallReadsAsADeferStatementWithNoRefusal(t *testing.T) {
	m := loadSource(t, `package p

import "sync"

type Counter struct {
	mu sync.Mutex
	n  int
}

func (c *Counter) Add(k int) int {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.n += k
	return c.n
}

func done() {}

func Count(n int) int {
	defer done()
	return n
}
`)
	if len(m.Refusals) > 0 {
		t.Fatalf("refusals %v", m.Refusals)
	}
	doc := encoded(t, m)
	add := findFunc(doc, "Add", "funcs") // a read method's recv is its parameter, not a key
	if add == nil {
		t.Fatalf("Counter.Add not read: %v", doc["funcs"])
	}
	d := obj(list(add["body"])[1])
	if d["k"] != "defer" || d["pos"] != "p.go:12" {
		t.Fatalf("statement 1 of Add: %v", d)
	}
	call := obj(d["call"])
	if call["k"] != "extern_method" || call["path"] != "sync" || call["type"] != "Mutex" || call["name"] != "Unlock" {
		t.Fatalf("deferred call %v", call)
	}
	recv := obj(call["recv"])
	if recv["k"] != "field" || recv["name"] != "mu" || obj(recv["x"])["name"] != "c" {
		t.Fatalf("deferred call's receiver %v", recv)
	}
	count := findFunc(doc, "Count", "funcs")
	if d := obj(list(count["body"])[0]); d["k"] != "defer" || obj(d["call"])["k"] != "call" || obj(d["call"])["name"] != "done" {
		t.Fatalf("statement 0 of Count: %v", d)
	}
}

// A deferred call holding an expression of a type cir/1 has no form for is kept foreign, naming
// the type, as any other statement holding one: the document still encodes.
func TestADeferredCallHoldingATypeOutsideCir1IsKeptForeign(t *testing.T) {
	m := loadSource(t, `package p

import "sync"

var state struct {
	mu sync.Mutex
	n  int
}

func Count(n int) int {
	defer state.mu.Unlock()
	return n
}
`)
	doc := encoded(t, m)
	count := findFunc(doc, "Count", "funcs")
	if count == nil {
		t.Fatalf("Count not read: %v", doc["funcs"])
	}
	d := obj(list(count["body"])[0])
	reason, _ := d["reason"].(string)
	if d["k"] != "foreign" || !strings.Contains(reason, "type outside cir/1: struct{") {
		t.Fatalf("statement 0 of Count: %v", d)
	}
}

// A call ref of a method or through an interface names the local its receiver is rooted at,
// through fields, indexes, `*` and `&` (`recv_var`); a receiver rooted at a package variable or
// at a call's result names none.
func TestAMethodAndAnInterfaceCallRefNameTheLocalTheirReceiverIsRootedAt(t *testing.T) {
	m := loadSource(t, `package p

import (
	"io"
	"sync"
	"unsafe"
)

type Counter struct {
	mu   sync.Mutex
	subs []*Counter
}

var global sync.Mutex

func (c *Counter) get() *Counter { return c }

func (c *Counter) Raw(w io.Writer, p []byte, u unsafe.Pointer) {
	c.mu.Lock()
	defer c.mu.Unlock()
	w.Write(p)
	(*c).subs[0].mu.Lock()
	global.Lock()
	c.get().mu.Unlock()
}
`)
	doc := encoded(t, m)
	raw := findFunc(doc, "Counter.Raw", "opaque")
	if raw == nil {
		t.Fatalf("Counter.Raw is not opaque: %v", doc["opaque"])
	}
	var got []string
	for _, c := range list(raw["calls"]) {
		o := obj(c)
		if o["k"] != "method" && o["k"] != "iface" {
			continue
		}
		v, _ := o["recv_var"].(string)
		got = append(got, callKeys([]any{c})[0]+" @"+v)
	}
	want := strings.Join([]string{
		"method:sync Mutex Lock @c",
		"method:sync Mutex Unlock @c",
		"iface:io.Writer Write @w",
		"method:sync Mutex Lock @c",
		"method:sync Mutex Lock @",
		"method:sync Mutex Unlock @",
		"method:Counter get @c",
	}, "\n")
	if strings.Join(got, "\n") != want {
		t.Fatalf("calls\n%s\nwant\n%s", strings.Join(got, "\n"), want)
	}
}
