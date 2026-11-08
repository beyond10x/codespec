package main

import (
	"encoding/json"
	"go/build"
	"path/filepath"
	"strings"
	"testing"
)

// The encoded document, read back into plain maps.
func encoded(t *testing.T, m *Module) map[string]any {
	t.Helper()
	doc, err := encode(m)
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	var out map[string]any
	if err := json.Unmarshal(b, &out); err != nil {
		t.Fatal(err)
	}
	return out
}

func list(v any) []any {
	l, _ := v.([]any)
	return l
}

func obj(v any) map[string]any {
	o, _ := v.(map[string]any)
	return o
}

// callKeys spells each call as k:path.recv.name, sig for a value call.
func callKeys(calls []any) []string {
	var out []string
	for _, c := range calls {
		o := obj(c)
		s := o["k"].(string) + ":"
		for _, f := range []string{"path", "recv", "iface", "name", "sig"} {
			if v, ok := o[f].(string); ok && v != "" {
				s += v + " "
			}
		}
		out = append(out, strings.TrimSpace(s))
	}
	return out
}

func findFunc(doc map[string]any, key string, in string) map[string]any {
	for _, f := range list(doc[in]) {
		o := obj(f)
		name := o["name"].(string)
		if r, ok := o["recv"].(string); ok {
			name = r + "." + name
		}
		if name == key {
			return o
		}
	}
	return nil
}

// A statement cir/1 cannot express still lists the calls it makes, and a function whose
// signature cir/1 cannot express is in `opaque` with its position, reason and calls.
func TestAForeignStatementAndARefusedFunctionListTheirCalls(t *testing.T) {
	m := loadSource(t, `package p

import (
	"os"
	"unsafe"
)

func Home() string {
	var x int
	return os.Getenv(string(rune(uintptr(unsafe.Pointer(&x)))))
}

func Raw(p unsafe.Pointer) string {
	return os.Getenv("HOME")
}
`)
	doc := encoded(t, m)
	home := findFunc(doc, "Home", "funcs")
	if home == nil {
		t.Fatalf("Home not read: %v", doc["funcs"])
	}
	var calls []string
	for _, s := range list(home["body"]) {
		if obj(s)["k"] == "foreign" && strings.Contains(obj(s)["src"].(string), "os.Getenv") {
			calls = callKeys(list(obj(s)["calls"]))
		}
	}
	if strings.Join(calls, ",") != "func:os Getenv" {
		t.Fatalf("foreign statement calls %v, want [func:os Getenv]", calls)
	}
	raw := findFunc(doc, "Raw", "opaque")
	if raw == nil {
		t.Fatalf("Raw is not opaque: %v", doc["opaque"])
	}
	if raw["pos"] != "p.go:13" || !strings.Contains(raw["reason"].(string), "unsafe.Pointer") {
		t.Fatalf("Raw at %v, reason %v", raw["pos"], raw["reason"])
	}
	if got := callKeys(list(raw["calls"])); strings.Join(got, ",") != "func:os Getenv" {
		t.Fatalf("Raw calls %v", got)
	}
	if obj(list(raw["calls"])[0])["pos"] != "p.go:14" {
		t.Fatalf("call position %v", list(raw["calls"])[0])
	}
	if doc["path"] != nil || doc["methods"] != nil {
		t.Fatalf("a package without an import path has path %v, method sets %v", doc["path"], doc["methods"])
	}
}

// Calls through an interface and of a function value, a function and a method used as values,
// a function literal (whose calls are its own and the holder's) and a constant argument are
// each named from go/types. A package with an import path lists the method set of each type.
func TestEveryKindOfCallIsNamed(t *testing.T) {
	t.Setenv(importPathEnv, "example.com/p")
	m := loadSource(t, `package p

import (
	"io"
	"syscall"
	"unsafe"
)

type T struct{ n int }

func (t *T) Write(b []byte) (int, error) { return len(b), nil }

func run(f func() error) error { return f() }

var hook = run

func Use(w io.Writer, p unsafe.Pointer) {
	w.Write(nil)
	t := &T{}
	g := t.Write
	h := run
	_ = h(func() error { _, err := w.Write(nil); return err })
	_, _ = g(nil)
	syscall.Syscall(syscall.SYS_READ, 0, 0, 0)
}
`)
	doc := encoded(t, m)
	use := findFunc(doc, "Use", "opaque")
	if use == nil {
		t.Fatalf("Use is not opaque: %v", doc["opaque"])
	}
	got := strings.Join(callKeys(list(use["calls"])), "\n")
	want := strings.Join([]string{
		"iface:io.Writer Write",
		"new:T",
		"ref:T Write func([]byte) (int, error)",
		"ref:run func(func() error) error",
		"value:func(func() error) error",
		"lambda:func() error",
		"value:func([]byte) (int, error)",
		"func:syscall Syscall",
	}, "\n")
	if got != want {
		t.Fatalf("calls\n%s\nwant\n%s", got, want)
	}
	calls := list(use["calls"])
	if ms := list(obj(calls[0])["methods"]); len(ms) != 1 || obj(ms[0])["sig"] != "func([]byte) (int, error)" {
		t.Fatalf("io.Writer methods %v", ms)
	}
	if inner := callKeys(list(obj(calls[5])["calls"])); strings.Join(inner, ",") != "iface:io.Writer Write" {
		t.Fatalf("lambda calls %v", inner)
	}
	if c := list(obj(calls[7])["consts"]); len(c) != 4 || obj(c[0])["value"] != "0" || obj(c[0])["index"] != float64(0) {
		t.Fatalf("Syscall consts %v", c)
	}
	if got := callKeys(list(doc["inits"])); strings.Join(got, ",") != "ref:run func(func() error) error" {
		t.Fatalf("inits %v", got)
	}
	sets := list(doc["methods"])
	if len(sets) != 1 || obj(sets[0])["t"] != "T" {
		t.Fatalf("method sets %v", sets)
	}
	entry := obj(list(obj(sets[0])["methods"])[0])
	if entry["name"] != "Write" || entry["sig"] != "func([]byte) (int, error)" || obj(entry["target"])["recv"] != "T" {
		t.Fatalf("method entry %v", entry)
	}
}

// A package of the standard library carries its import path, a function without a body is
// opaque and marked so, and a //go:linkname directive is recorded.
func TestAStandardPackageHasItsPathItsBodyLessFunctionsAndItsLinknames(t *testing.T) {
	ensureGOROOT()
	m, err := load(filepath.Join(build.Default.GOROOT, "src", "internal", "poll"))
	if err != nil {
		t.Fatal(err)
	}
	doc := encoded(t, m)
	if doc["path"] != "internal/poll" {
		t.Fatalf("path %v", doc["path"])
	}
	wait := findFunc(doc, "runtime_pollWait", "opaque")
	if wait == nil || wait["no_body"] != true {
		t.Fatalf("runtime_pollWait %v", wait)
	}
	found := false
	for _, l := range list(doc["linknames"]) {
		if obj(l)["name"] == "runtimeNano" && obj(l)["target"] == "runtime.nanotime" {
			found = true
		}
	}
	if !found {
		t.Fatalf("linknames %v", doc["linknames"])
	}
	rt, err := load(filepath.Join(build.Default.GOROOT, "src", "runtime"))
	if err != nil {
		t.Fatal(err)
	}
	found = false
	for _, l := range rt.Linknames {
		if l.Name == "poll_runtime_pollWait" && l.Target == "internal/poll.runtime_pollWait" {
			found = true
		}
	}
	if !found {
		t.Fatal("runtime does not record poll_runtime_pollWait's linkname")
	}
}
