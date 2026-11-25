---
format: aep.planning-md/3
id: story:go-http-entries
kind: story
status: implemented
title: The Go reader records net/http and echo routes as http entries
summary: codespec-read-go records net/http and echo routes in Module.entries with method and path; Entry gains an optional path for a handler of another package
relations:
- decomposes: epic:release-fact-diff
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go
- confidence: inferred
  path: crates/cli/tests/fixtures/go-routes
- confidence: inferred
  path: crates/cli/tests/measure.rs
- confidence: inferred
  path: crates/edit/src/delete.rs
- confidence: inferred
  path: crates/edit/src/rename.rs
- confidence: inferred
  path: crates/edit/tests/change_sets
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: schemas/cir-1.schema.json
- confidence: inferred
  path: website/docs/reference/cir-1.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T01:41:31Z", actor: "human:operator", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-25T01:41:31Z", actor: "human:operator", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-25T02:21:35Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader records net/http and echo routes as http entries

## Outcome

Reading a Go package, `codespec-read-go` records in `Module.entries` one `http` entry per route
the package registers, with `method`, `on` (the path) and `func` (the handler):

- `net/http`: `http.HandleFunc(p, h)`, `http.Handle(p, h)` and the same methods of a
  `*http.ServeMux` (`mux.HandleFunc`, `mux.Handle`). A Go 1.22 pattern `"GET /items/{id}"` gives
  method `GET` and path `/items/{id}`; a pattern without a method gives an empty `method` (any
  method).
- echo (`github.com/labstack/echo/v4`): `e.GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`,
  `OPTIONS` on an `*echo.Echo`, and the same on a group: `g := e.Group("/api")` then
  `g.GET("/items", h)` gives `/api/items`; a group of a group joins both prefixes. `e.Any` gives
  an empty `method`. Middleware arguments are not entries.
- The path is a string constant (a literal or a named constant, joined with constant prefixes);
  the handler is a function or a method value of a named type (`h.List`), or for `Handle` a value
  of a named type whose `ServeHTTP` is the entry, or `http.HandlerFunc(f)`, whose `f` is.

The handler may be declared in another package than the one registering the route, which is how
a Go service is usually laid out (`main` registers, `handlers` serves). The entry cannot go into
the handler package's document: `codespec-read-go` reads one package directory per run
(`main.go`: `codespec-read-go <package dir>`; `load` parses that one directory) and
`scripts/read-repo.sh` runs it once per package, in parallel, so the run that reads `handlers`
never sees `main`'s registrations. So `cir/1` `Entry` gains an optional `path`: the import path
of the module declaring `func`, omitted when it is this module. Strict reading checks `func`
against `funcs` only when `path` is empty, refuses a `path` equal to the module's own, and
refuses `path` on an entry of any kind but `http` (an `exception-mapper` entry's check reads its
function's parameter, which only a local function has). The
doc of `Entry.method` says empty means any method and lists any upper-case method token. A
document without `path` reads as before; the schema and `website/docs/reference/cir-1.md` are
regenerated. This is an added optional field, as `beans`, `injects` and `persistence` were
(`docs/design.md` lines 76-85, the rules for `cir/1` in `AGENTS.md`): no field changes its
meaning when `path` is absent.

Every consumer on main that reads `Entry.func` as a key of its own module is changed here:

- `crates/model/src/lib.rs` (strict reading, the set of entry functions that may list `raises`,
  near line 2662): only entries without `path` admit a function of this module.
- `crates/edit/src/rename.rs` (near line 969): renaming a function rekeys an entry of its own
  module only when the entry has no `path`, and an entry of any other module of the set whose
  `path` is the renamed function's module path.
- `crates/edit/src/delete.rs` (near line 236): deleting a function is refused for an entry of its
  own module without `path`, and for an entry of any module of the set whose `path` is the
  function's module path; an entry whose `path` names another module does not block it.
- `crates/edit/tests/change_sets/delete_retarget.rs` (line 688) builds an `Entry` literal: it
  gets the new field.

Consumers in draft stories that must resolve `path` when they read entries:
`story:reach-entry-file` (`kind: http` lines), `story:entry-bindings` (bindings per entry) and
`story:cq-entry-relation` (`entry_point{func, module}`).

A registration the reader cannot turn into an entry is listed as not discovered: a `refusals`
item at the registration's position, `what` `http route`, and a reason naming why. It never
becomes a refused function in `codespec measure`.

## Why

The release diff (`epic:release-fact-diff`) reaches from entries and reports entries added and
removed. For a Go service there are none: the Go reader records no `Module.entries`, so reach
needs every handler named by hand, and a new route shows up in no fact. The consumer's check: an
echo fixture yields one `http` entry per route, with method and path.

## Reproducer

A module `example.com/shop` with two packages, read with the echo module resolved offline
(a stub of `github.com/labstack/echo/v4` with the same API shape, through a `replace` in the
fixture's `go.mod`; no network):

```go
// handlers/items.go
package handlers

import "github.com/labstack/echo/v4"

type Items struct{}

func (Items) List(c echo.Context) error { return nil }
func (Items) Show(c echo.Context) error { return nil }
func Health(c echo.Context) error       { return nil }
```

```go
// main.go
package main

import (
	"github.com/labstack/echo/v4"
	"example.com/shop/handlers"
)

func main() {
	e := echo.New()
	e.GET("/health", handlers.Health)
	api := e.Group("/api")
	h := handlers.Items{}
	api.GET("/items", h.List)
	api.GET("/items/:id", h.Show)
	e.Start(":8080")
}
```

A second fixture with `net/http`: `mux.HandleFunc("GET /items/{id}", show)`,
`http.HandleFunc("/health", health)` and `http.Handle("/static/", files)` with `files` of a named
type of the package.

## Observed

Not run on main `5f82fff9`. The Go reader's module type (`adapters/go/cmd/codespec-read-go/ir.go`)
has no entries field, so `main` of the reproducer is read with three echo calls and no entry.

## Done When

- A failing test first in the Go reader's tests (`entries_test.go`): the echo reproducer.
- The consumer's check: the echo fixture yields one `http` entry per route, with method and
  path: `GET /health` on `Health`, `GET /api/items` on `Items.List`, `GET /api/items/:id` on
  `Items.Show`, each with `path` `example.com/shop/handlers`, recorded in `main`'s document.
- The `net/http` fixture gives `GET /items/{id}`, `/health` with an empty method, and
  `/static/` on `<type>.ServeHTTP`.
- Near misses, each listed as not discovered with its reason and no entry: a route registered in
  a loop over a table; a path that is not a constant (`cfg.Prefix + "/x"`); a handler that is a
  function literal (no function key to name); a handler held in a variable of a function type or
  of an interface type; a group whose prefix the reader cannot follow (a group passed in as a
  parameter, or assigned twice). A route whose path is a named constant is discovered.
- `codespec measure` over the near-miss fixture counts `main` as read, not refused (a test in
  `crates/cli/tests/measure.rs`, whose binary already builds `codespec-read-go` once).
- Strict reading: an entry with `path` naming no function of its own module is accepted, one
  with `path` equal to the module's path is refused naming it, a `scheduled`, `incoming` or
  `exception-mapper` entry with `path` is refused naming its kind, and an entry without `path`
  still needs its function in `funcs` (tests in `crates/model/tests/model.rs`).
- Near misses for the consumers, each a test:
  - strict reading: a function of the registering module whose key equals a `path` entry's
    `func` and that lists `raises` is refused, as it would be with no entry
    (`crates/model/tests/model.rs`);
  - rename: renaming a local function whose key equals a `path` entry's `func` leaves that entry
    unchanged; renaming the handler in its own module rekeys the `path` entry of the registering
    module (`crates/edit/tests/change_sets/`);
  - delete: deleting a local function whose key equals a `path` entry's `func` is not refused by
    that entry; deleting the handler in its own module is refused naming the registering
    module's entry (`crates/edit/tests/change_sets/delete_retarget.rs`).
- `docs/design.md`'s table of additive changes gains a row for `Entry.path`.
- `task schema` and `task docs-generate` run; checked-in documents that hold no entry are
  unchanged; `task check` green; `go test` of the reader within the 120 s step budget, each new
  test under 60 s.

## Scope

- Own: `adapters/go/cmd/codespec-read-go/` (a new `entries.go`, `ir.go`, `encode.go`,
  `entries_test.go`, `testdata/entries/`), `crates/model/src/lib.rs` (`Entry.path`, the doc of
  `Entry.method`, `check_framework`), `crates/model/tests/model.rs`,
  `crates/cli/tests/measure.rs`, `crates/cli/tests/fixtures/go-routes/`,
  `crates/edit/src/rename.rs`, `crates/edit/src/delete.rs`, `crates/edit/tests/change_sets/`,
  `schemas/cir-1.schema.json`, `website/docs/reference/cir-1.md`, `docs/design.md`. Inferred;
  recorded as typed scope. `CHANGELOG.md` gets one line, outside the typed scope; conflicts there are resolved at
  merge.
- Shares no file with `story:stable-site-identity`; both can run at once.
- `story:entry-bindings` (epic `slice-to-service`, draft) adds `params` and `responses` to the
  same `Entry`, the schema, `cir-1.md` and the `Entry` literal of `delete_retarget.rs`; it
  excludes recognising `net/http` in the Go reader, which this story does. It runs after this
  one: `story:entry-bindings` depends_on `story:go-http-entries`.
- `story:reach-entry-file` and `story:cq-entry-relation` (drafts) read `Module.entries`; each must
  resolve `path` (listed in the Outcome).
