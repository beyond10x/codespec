package main

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

// loadModule reads the package in dir of a fixture module whose imports resolve offline: no
// workspace, no proxy, the local toolchain. It runs in dir, as scripts/read-repo.sh runs the reader
// in the repository: go/build asks `go list` in the working directory for a package outside the
// standard library.
func loadModule(t *testing.T, dir string) *Module {
	t.Helper()
	t.Setenv("GOWORK", "off")
	t.Setenv("GOPROXY", "off")
	t.Setenv("GOTOOLCHAIN", "local")
	t.Setenv("GOFLAGS", "-mod=mod")
	abs, err := filepath.Abs(dir)
	if err != nil {
		t.Fatal(err)
	}
	was, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	if err := os.Chdir(abs); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.Chdir(was) })
	m, err := load(abs)
	if err != nil {
		t.Fatal(err)
	}
	return m
}

// entryLines spells each encoded entry as "kind method on func path", empty fields as "-".
func entryLines(t *testing.T, m *Module) []string {
	t.Helper()
	var out []string
	for _, e := range list(encoded(t, m)["entries"]) {
		o := obj(e)
		var parts []string
		for _, k := range []string{"kind", "method", "on", "func", "path"} {
			v, _ := o[k].(string)
			if v == "" {
				v = "-"
			}
			parts = append(parts, v)
		}
		for k := range o {
			switch k {
			case "kind", "method", "on", "func", "path":
			default:
				t.Errorf("entry %v has field %s", o, k)
			}
		}
		out = append(out, strings.Join(parts, " "))
	}
	return out
}

func routeRefusals(m *Module) []string {
	var out []string
	for _, r := range m.Refusals {
		if r.What == "http route" {
			out = append(out, r.Pos+" "+r.Reason)
		}
	}
	sort.Strings(out)
	return out
}

func sameLines(t *testing.T, what string, got, want []string) {
	t.Helper()
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("%s:\n%s\nwant:\n%s", what, strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

// The echo reproducer: main registers three routes whose handlers the package handlers declares;
// main's document holds one http entry per route, each naming the handler's package.
func TestEchoRoutesAreHttpEntriesNamingTheHandlersPackage(t *testing.T) {
	m := loadModule(t, filepath.Join("testdata", "entries", "shop"))
	sameLines(t, "entries", entryLines(t, m), []string{
		"http GET /health Health example.com/shop/handlers",
		"http GET /api/items Items.List example.com/shop/handlers",
		"http GET /api/items/:id Items.Show example.com/shop/handlers",
	})
	sameLines(t, "route refusals", routeRefusals(m), nil)
}

// net/http: a Go 1.22 pattern gives its method, a pattern without one an empty method; Handle with
// a value of a named type is that type's ServeHTTP, with http.HandlerFunc(f) it is f. Every
// handler is of this package: no path.
func TestNetHTTPRoutesAreHttpEntries(t *testing.T) {
	m := loadModule(t, filepath.Join("testdata", "entries", "web"))
	sameLines(t, "entries", entryLines(t, m), []string{
		"http GET /items/{id} show -",
		"http - /health health -",
		"http - /static/ static.ServeHTTP -",
		"http POST /items create -",
	})
	sameLines(t, "route refusals", routeRefusals(m), nil)
}

// A group of a group joins both prefixes; Any has an empty method; middleware is no entry; a
// group made straight off a call is followed.
func TestEchoGroupsOfGroupsJoinPrefixes(t *testing.T) {
	dir := filepath.Join("testdata", "entries", "shop", "nested")
	m := loadModule(t, dir)
	sameLines(t, "entries", entryLines(t, m), []string{
		"http - /all all -",
		"http POST /api/v2/items create -",
		"http DELETE /api/v2/items/:id drop -",
		"http GET /admin/stats stats -",
	})
	sameLines(t, "route refusals", routeRefusals(m), nil)
}

// Each registration the reader cannot turn into an entry is listed as not discovered, with its
// reason, and gives no entry; a route whose path is a named constant is discovered. The fixture is
// the one `codespec measure` reads in crates/cli/tests/measure.rs.
func TestRoutesTheReaderCannotFollowAreListedAsNotDiscovered(t *testing.T) {
	m := loadModule(t, filepath.Join("..", "..", "..", "..", "crates", "cli", "tests", "fixtures", "go-routes"))
	sameLines(t, "entries", entryLines(t, m), []string{
		"http GET /v1/named ping -",
	})
	got := routeRefusals(m)
	want := []string{
		"main.go:25 the group is a parameter of the function: its prefix is not known",
		"main.go:33 registered in a loop: the reader does not run the loop",
		"main.go:35 the path is not a constant",
		"main.go:36 the handler is a function literal: no function key names it",
		"main.go:38 the handler is held in a variable of a function type",
		"main.go:40 the handler is held in a variable of an interface type",
		"main.go:43 the group is assigned more than once: its prefix is not known",
	}
	sort.Strings(want)
	sameLines(t, "route refusals", got, want)
	for _, f := range m.Funcs {
		for _, k := range kinds(f.Body) {
			if k == "foreign" {
				t.Errorf("%s holds a foreign statement; refusals %v", f.Name, m.Refusals)
			}
		}
	}
	if len(m.Opaque) != 0 {
		t.Errorf("opaque functions %v", m.Opaque)
	}
}

// Every echo registration: CONNECT and TRACE like GET, Add with a constant method, Match with
// constant methods one entry per method. A method that is not a constant, and a route on a
// receiver that is not an *echo.Echo or *echo.Group (a struct embedding one), are not discovered.
func TestEchoAddMatchConnectTraceAndReceiversTheReaderCannotFollow(t *testing.T) {
	m := loadModule(t, filepath.Join("testdata", "entries", "shop", "more"))
	sameLines(t, "entries", entryLines(t, m), []string{
		"http PUT /added one -",
		"http GET /matched two -",
		"http HEAD /matched two -",
		"http CONNECT /tunnel three -",
		"http TRACE /g/trace four -",
	})
	want := []string{
		"main.go:30 the method is not a constant",
		"main.go:32 the methods are not a list of constants",
		"main.go:33 the methods are not a list of constants",
		"main.go:35 the route's receiver is not an *echo.Echo or *echo.Group: its prefix is not known",
		"main.go:37 the route's receiver is not an *echo.Echo or *echo.Group: its prefix is not known",
	}
	sort.Strings(want)
	sameLines(t, "route refusals", routeRefusals(m), want)
}

// Handle with a value of a named function type that has its own ServeHTTP starts that ServeHTTP;
// only http.HandlerFunc(f) starts f. A mounted ServeMux is no entry: its own routes are.
func TestHandleOfAFunctionTypeWithItsOwnServeHTTPAndAMountedServeMux(t *testing.T) {
	m := loadModule(t, filepath.Join("testdata", "entries", "web", "mount"))
	sameLines(t, "entries", entryLines(t, m), []string{
		"http - /view appHandler.ServeHTTP -",
		"http - /held appHandler.ServeHTTP -",
		"http - /plain plain -",
	})
	sameLines(t, "route refusals", routeRefusals(m), []string{
		"main.go:18 the handler is a mounted ServeMux: the routes registered on it are the entries",
		"main.go:19 the handler is a mounted ServeMux: the routes registered on it are the entries",
	})
}
