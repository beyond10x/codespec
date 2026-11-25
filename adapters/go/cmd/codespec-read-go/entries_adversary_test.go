package main

import (
	"path/filepath"
	"strings"
	"testing"
)

func advEntries(t *testing.T, name string) (*Module, []string, []string) {
	t.Helper()
	m := loadModule(t, filepath.Join("testdata", "entries", "adv", name))
	return m, entryLines(t, m), routeRefusals(m)
}

// echo runs a route registered on a struct embedding an *echo.Group under the group's prefix,
// /api/items. The reader follows the prefix only when the receiver's static type is *echo.Group;
// here it is the struct, so groupPrefix answers "no prefix" and the entry claims /items, a path
// nothing serves. Either the joined path or "not discovered" is right; /items is a false fact.
func TestAdvARouteOnAStructEmbeddingAGroupKeepsTheGroupsPrefix(t *testing.T) {
	_, got, refused := advEntries(t, "embed")
	joined := strings.Join(got, "\n") == "http GET /api/items list -"
	notDiscovered := len(got) == 0 && len(refused) == 1
	if !joined && !notDiscovered {
		t.Fatalf("entries %q, route refusals %q: want the entry GET /api/items or one route not discovered", got, refused)
	}
}

// Every echo v4 registration is an entry or listed as not discovered (the story: "A registration
// the reader cannot turn into an entry is listed as not discovered"). Add, Match, CONNECT and
// TRACE register routes, and the reader passes over all four without a trace.
func TestAdvEveryEchoRegistrationIsAnEntryOrNotDiscovered(t *testing.T) {
	_, got, refused := advEntries(t, "extra")
	// Match with two constant methods is one entry per method: 5 entries.
	if n := len(got) + len(refused); n != 5 {
		t.Fatalf("4 registrations (Add, Match, CONNECT, TRACE) give %d entries or refusals, want 5: entries %q, refusals %q", n, got, refused)
	}
}

// The story: for Handle, "a value of a named type whose ServeHTTP is the entry, or
// http.HandlerFunc(f), whose f is". appHandler(view) is a value of a named type whose own
// ServeHTTP net/http runs (it calls view and handles its error); the reader treats every
// conversion to a named function type as http.HandlerFunc and names view, so reach from the
// entry misses appHandler.ServeHTTP.
func TestAdvHandleOfAFunctionTypeWithItsOwnServeHTTPNamesThatServeHTTP(t *testing.T) {
	_, got, refused := advEntries(t, "ownserve")
	served := strings.Join(got, "\n") == "http - /view appHandler.ServeHTTP -"
	notDiscovered := len(got) == 0 && len(refused) == 1
	if !served && !notDiscovered {
		t.Fatalf("entries %q, route refusals %q: want appHandler.ServeHTTP on /view or one route not discovered", got, refused)
	}
}
