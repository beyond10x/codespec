---
format: aep.planning-md/3
id: story:typescript-node-service-reader
kind: story
status: draft
title: 'A TypeScript reader for a Node service: functions, calls and express-style routes'
summary: adapters/typescript-read reads .ts files into cir/1 with functions, calls, imports as externs and express routes as http entries; refusal-first
relations:
- decomposes: epic:typescript-reader
- informed_by: story:go-http-entries
- informed_by: story:readers-bound-untrusted-input
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: adapters/typescript-read
- confidence: inferred
  path: conformance/reader/typescript
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: website/data/support-matrix.json
- confidence: inferred
  path: website/docs/reference
revision: 2
---
# Story: A TypeScript reader for a Node service: functions, calls and express-style routes

## Outcome

`codespec-read-typescript`, a Rust crate `adapters/typescript-read` (library
`codespec_read_typescript`), reads the `.ts` files of a directory into `cir/1`, one module per
file, and `codespec read --lang typescript <dir>` prints them. The parser is the one decision D1
of `epic:typescript-reader` settles in this story's first step; every use of it sits in one module
of the crate, so a change of parser changes that module only.

The slice, each line a rule a test holds. A construct outside it is refused with a reason and
`<file>:<line>`; the smallest enclosing function becomes an opaque function carrying the reason,
and the document is never lost:

- Modules. A module's `path` is the package name of the nearest `package.json` plus the file's
  path relative to it, without the extension (`@example/api/src/items`); a directory without a
  `package.json` is refused at its root. `.d.ts` files and `node_modules` are not read. A file
  that does not parse makes the command exit non-zero naming the file and line, and no document
  for it.
- Types. `string`, `number` (a `float` of 64 bits, decision D2 of `epic:ecmascript-to-native`
  where that lands, else the same mapping stated in this crate's doc), `boolean`, `void`,
  `T[]`, `interface` and `type` aliases of an object type with no optional or index member
  (a `struct`), `T | undefined` and `T | null` as a nullable where `cir/1` has one. Refused:
  `any`, `unknown`, other unions, intersections, generics, enums, `keyof` and mapped types,
  tuples, `typeof`.
- Functions. `function f(a: T): U`, `const f = (a: T): U => …` and `export` of either, and the
  methods of a class that has no `extends` and no decorators. A parameter or a result with no
  annotation is refused, naming the parameter. `async`, generators, `try`, `await` and `throw`
  are refused (they come in a later story of the epic).
- Calls. A call of a function of the same file or of a named import of a relative module is a
  call of that function (a module-qualified `call_ref`, by the file the import resolves to). A
  call of a named or default import of a package (`import express from 'express'`, `import {
  readFile } from 'node:fs'`) is an extern call: `path` the module specifier as written, `name`
  the export (`default` for a default import). A method call on a value whose type is an
  imported extern type or a struct of the file is an `extern_method` or a method call. A call on
  a value with no declared type is refused. Every extern is `extern without summary` in
  `codespec effects` until a summary names it.
- Routes. In a file that does `import express from 'express'` and binds `const app = express()`
  (or `const r = express.Router()` or `Router()` of a named import), a statement `app.get('/x',
  h)`, and the same for `post`, `put`, `delete` and `patch`, with a string literal path and `h`
  the name of a function of the file or an inline arrow or function expression, records
  `Module.entries` an `Entry` of kind `http`: `method` the verb upper-cased, `on` the path, `func`
  the key of the handler (an inline handler is a function named `<method> <path>`, so its
  identity does not depend on its line). `app.use('/api', r)` of a router bound in the same file
  joins the prefix. Refused, each listed with its reason and `file:line`, and no entry made:
  a path that is not a literal, a route with middleware (more than one function argument), a
  router mounted from another file, `app.route(...)`, `app.all`.
- Everything a function calls that the slice cannot read is kept as it is for the analysis: an
  opaque function lists no facts and `codespec effects` reports it `unknown`.

## Why

6 of the 10 first-party components of a Go service's release are TypeScript, and the release
review had no fact for any of them. The first slice is the one a review can use soonest: which
functions exist, what they call, and which routes the service exposes, because an added route
is the first thing a release review asks. The slice stops where it has to type a value it cannot
(`any`, a method call on an untyped value): refusing it is the answer, and the count of
refusals per construct is the measurement that orders the next stories of the epic.

## Reproducer

Written for this story, one file `src/items.ts`:

```ts
import express from 'express';

const app = express();

function list(): string[] {
  return ['a', 'b'];
}

app.get('/items', (req: express.Request, res: express.Response): void => {
  res.json(list());
});

app.post('/items', create);

function create(req: express.Request, res: express.Response): void {
  res.status(201).send('ok');
}
```

`codespec read --lang typescript <dir>` with a `package.json` named `@example/api`.

## Observed

Not run: the repository has no TypeScript reader, `codespec read --lang typescript` is refused as
an unknown language. A first run on a real service is the first thing the story does and its
count of refusals per construct is recorded in this section.

## Done When

- A failing test first, in the integration-test binary of the crate (`tests/main.rs`), over the
  reproducer: the module `@example/api/src/items` holds functions `list`, `create` and the inline
  handler `GET /items`, the calls of `list` and of the externs `express.default`, and two `http`
  entries, `GET /items` and `POST /items`, their `func` the two handlers. The output is the same
  on two runs, byte for byte.
- Near misses, one fixture each: `const app = new Map(); app.get('/items')` (a call of `get` on
  something that is not `express()`) makes no entry; `app.get(path, h)` with a variable path is
  refused with its line, no entry; `app.get('/x', auth, h)` is refused naming the middleware;
  a function with an untyped parameter becomes opaque with `file:line` and the rest of the file is
  read; `any` as a result type, an `await`, a generic function and a `try` each give their own
  refusal reason and none loses the document; a file that does not parse exits non-zero naming
  `file:line`; a moved line changes no function key and no entry (`<method> <path>`);
  `node_modules` and `.d.ts` are not read.
- The conformance layout of the other readers: a corpus under `conformance/reader/typescript`
  (the cases and their expected documents), run by `task conformance` through the reader's
  command; the crate's tests read each fixture once, not per test.
- `codespec effects` over the reproducer's documents reports `unknown` for each extern (`extern
  without summary: express.default`) and for no function the slice read; `codespec diff` over
  two revisions of the reproducer, the second with a route added, reports `entry-added http POST
  /items` and `unit-added`. (This needs `story:diff-command`; until it is implemented the check
  is the `entry-added` computed from the two documents' `entries`.)
- Bounded input as `story:readers-bound-untrusted-input` requires of every reader: nesting and
  file size limits with a test each; no name of a TypeScript source reaches output unvalidated.
- The crate in the workspace members and in a `test` part of `scripts/check-parts.txt`; a row in
  `AGENTS.md`'s table of what the repository owns; `codespec read --help`, the support matrix
  and the site's reference pages generated (`task docs-generate`); `task check` green; each new
  test under 60 s and the test step within the 120 s budget; the parser's first-step measurement
  (the candidates, their numbers and the choice) recorded in `docs/design.md`.

## Scope

- Own: `adapters/typescript-read/` (new crate), `Cargo.toml` (members), `Cargo.lock`,
  `crates/cli` (the `--lang typescript` dispatch), `conformance/reader/typescript`,
  `scripts/check-parts.txt`, `website/data/support-matrix.json`, `website/docs/reference`,
  `docs/design.md`, `AGENTS.md` (the table row). Inferred; recorded as typed scope.
  `CHANGELOG.md` gets one line, outside the typed scope.
- Shares `crates/cli` with every story that adds a language or a command, and `Cargo.toml` and
  `Cargo.lock` with every new crate; conflicts are resolved at merge. If
  `story:ecmascript-reader` (of `epic:ecmascript-to-native`) lands first and D1 of this epic
  goes to option A or C, this story reuses the parser module of that crate; no edge, because
  option B shares nothing.

## Out of Scope

- The later stories named in the epic's Order (effect summaries of Node modules, middleware,
  `async`, classes with `extends`, Vue components).
- A change to `cir/1`: `Entry` of kind `http` exists.
