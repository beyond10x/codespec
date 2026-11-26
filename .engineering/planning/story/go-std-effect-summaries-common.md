---
format: aep.planning-md/3
id: story:go-std-effect-summaries-common
kind: story
status: active
title: Effect summaries for the Go standard library a service calls
summary: log/slog, context, time, net/http, encoding/json, errors, fmt, sync, io and the rest of os, strings and strconv give effects, not extern without summary, in codespec diff
relations:
- decomposes: epic:release-fact-diff
- informed_by: story:jvm-library-effect-summaries
- informed_by: epic:externs-and-effects
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go
- confidence: inferred
  path: crates/effects/tests/data
- confidence: inferred
  path: crates/effects/tests/effects.rs
- confidence: inferred
  path: docs/static-analysis.md
- confidence: inferred
  path: summaries/go-std.effects.json
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T04:43:16Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-26T04:43:16Z", actor: "human:operator", revision: 4}
---
# Story: Effect summaries for the Go standard library a service calls

## Outcome

`codespec effects`, `reach` and `diff` over Go documents know what the calls into the standard
library packages a typical service uses do. `summaries/go-std.effects.json` (`codespec-effects/1`,
language `go`) holds one entry per function or method for these packages and no others in the
first version. The file today holds 37 entries in `math/rand`, `net` (`LookupHost`), `os`
(11), `strconv` (7), `strings` (15) and `time` (`Sleep`); it has nothing for the rest:

| package | entries | effect |
|---|---|---|
| `log/slog` | `String`, `Int`, `Int64`, `Bool`, `Float64`, `Duration`, `Time`, `Any`, `Group` (attributes), `New`, `Default`, `SetDefault`, `NewTextHandler`, `NewJSONHandler`; `Logger.With`, `Logger.WithGroup` | pure |
| `log/slog` | `Logger.Debug`, `Info`, `Warn`, `Error` and their `Context` forms, `Logger.Log`, `Logger.LogAttrs`, and the package functions of the same names | `stdout` |
| `log` | `Print*`, `Fatal*`, `Panic*`, `New`, `Logger.Print*`, `Logger.Fatal*` | `stdout`; `Fatal*` also `proc` |
| `context` | `Background`, `TODO`, `WithValue`, `WithCancel`, `WithoutCancel`, `Cause`, `WithCancelCause` | pure |
| `context` | `WithTimeout`, `WithDeadline`, `AfterFunc` | `time` |
| `time` | `Now`, `Since`, `Until`, `After`, `Tick`, `NewTimer`, `NewTicker`, `AfterFunc`, `Timer.Reset`, `Ticker.Reset` | `time` |
| `time` | `Date`, `Unix`, `UnixMilli`, `Parse`, `ParseDuration`, `Duration.*`, `Time.Format`, `Add`, `Sub`, `Before`, `After`, `Equal`, `Unix`, `UTC`, `IsZero`, `Truncate` | pure |
| `net/http` | `Get`, `Head`, `Post`, `PostForm`, `Client.Do`, `Client.Get`, `Client.Head`, `Client.Post`, `ListenAndServe`, `ListenAndServeTLS`, `Server.ListenAndServe`, `Server.Shutdown`, `Server.Close`, `Serve` | `net` |
| `net/http` | `NewRequest`, `NewRequestWithContext`, `NewServeMux`, `HandleFunc`, `Handle`, `ServeMux.HandleFunc`, `ServeMux.Handle`, `StatusText`, `Header.Get`, `Header.Set`, `Header.Add`, `Header.Del` | pure |
| `net/http` | `Error`, `NotFound`, `Redirect`, `ServeFile`, `MaxBytesReader` | `io` |
| `encoding/json` | `Marshal`, `MarshalIndent`, `Unmarshal`, `Valid`, `NewEncoder`, `NewDecoder` | pure |
| `encoding/json` | `Encoder.Encode`, `Decoder.Decode` | `io` |
| `errors` | `New`, `Is`, `As`, `Unwrap`, `Join` | pure |
| `fmt` | `Sprint`, `Sprintf`, `Sprintln`, `Errorf`, `Sscan`, `Sscanf` | pure |
| `fmt` | `Print`, `Printf`, `Println` | `stdout` |
| `fmt` | `Fprint`, `Fprintf`, `Fprintln`, `Fscan`, `Fscanf` | `io` |
| `strings`, `strconv` | what the file lacks: every other function and method of the two packages (`strings.Builder` methods, `Fields`, `Map`, `ParseFloat`, `FormatFloat`, `AppendInt`, `Unquote`), none of them reads or writes anything outside its arguments | pure |
| `sync`, `sync/atomic` | `Mutex`, `RWMutex`, `WaitGroup`, `Once`, `Map`, `Pool` methods; `atomic.Int32`, `Int64`, `Bool`, `Value` methods and the `Add*`, `Load*`, `Store*`, `CompareAndSwap*` functions | pure |
| `io`, `bufio`, `bytes` | `io.ReadAll`, `io.Copy`, `io.CopyN`, `io.ReadFull`, `io.WriteString`, `bufio.Reader` and `Writer` methods, `bufio.Scanner.Scan` | `io` |
| `io`, `bufio`, `bytes` | `io.NopCloser`, `io.MultiReader`, `io.TeeReader`, `bufio.NewReader`, `NewWriter`, `NewScanner`, every `bytes` function and `bytes.Buffer` method | pure |
| `os` | `Open`, `Stat`, `Lstat`, `ReadDir`, `Executable`, `UserHomeDir`, `Environ`, `ExpandEnv` | `fs.read` for the first five, `env` for the last three |
| `os` | `Create`, `Mkdir`, `MkdirAll`, `Rename`, `RemoveAll`, `Truncate`, `Chmod` | `fs.write` |
| `os` | `OpenFile` (the flags decide, so both) | `fs.read`, `fs.write` |
| `path/filepath`, `net/url`, `sort`, `slices`, `maps`, `math`, `regexp`, `unicode/utf8` | the public functions | pure |

The table is the list the first version commits to; the story's first step is to settle each
cell against the standard library of the Go the gate runs (an entry for a function the pinned Go
does not have is refused by the signature test below) and the final table lives in
`docs/static-analysis.md`, the summary format having no header field for it.

Two rules decide the effect of a function that does not name a resource itself, so that the
cells above are not a taste:

- A function that reads or writes through a stream its caller supplies (`io.Reader`,
  `io.Writer`, `http.ResponseWriter`) has `io`, the vocabulary's own meaning: a descriptor whose
  origin the analysis does not follow. It is not `net` and not `fs.write`.
- A logger writes `stdout`, which the vocabulary defines as standard output or standard error;
  where the destination is a handler chosen at run time the summary says what the default
  handler does.

## Why

A release review of a Go service ran `codespec diff` on two tags with `summaries/go-std.effects.json`
loaded. Among its `unknown-added` deltas, `extern without summary` named `log/slog` calls 4
times, `context` 3 times and `time.Now` 2 times, none of them a function the file names. The file
covers 37 functions of six packages; a service calls into a dozen more. Each missing entry is an
`unknown` in the diff and, through `calls <func> which is incomplete`, an `unknown` in every
caller (`story:diff-unknown-at-the-incomplete-function` reports it once; this story removes the
cause). A reviewer cannot tell a new log line from a new network call from a new clock read.

## Reproducer

Written for this story, `example.com/probe`, one revision:

```go
package probe

import (
	"context"
	"log/slog"
	"time"
)

func Run(parent context.Context) time.Time {
	ctx, cancel := context.WithTimeout(parent, time.Second)
	defer cancel()
	_ = slog.String("k", "v")
	_ = ctx
	return time.Now()
}
```

Read with `codespec-read-go`, then `codespec effects probe.cir.json --summaries
summaries/go-std.effects.json`.

## Observed

Not run. By the entries of the file (none of `log/slog`, `context`, `time.Now`) and the reason
table of `crates/effects` (`extern without summary: <path>.<name>`), `Run` has an `unknown` for
each of `context.WithTimeout`, `slog.String` and `time.Now`. Whether the reader gives the three
calls as externs with a result type `context.Context` (an extern type) is the first thing the
story's first step reads off a real run; the consumer's report says it does.

## Done When

- A failing test first, in the integration-test binary of `crates/effects` (`tests/effects.rs`),
  over a checked-in document of the reproducer (read once, not per test): with
  `--summaries summaries/go-std.effects.json`, `Run` has the effect `time` and no `extern without summary` (its one `unknown`, for `defer cancel()`, moves to story:call-value-extern-func-type);
  `slog.String` adds nothing, `context.WithTimeout` and `time.Now` add `time`.
- Near misses: `slog.Logger.Info` adds `stdout`; `log.Fatal` adds `stdout` and `proc`;
  `context.Background` and `errors.New` add nothing and no unknown; `http.Get` adds `net`;
  `fmt.Fprintf(w, ...)` adds `io`; a call into a package outside the table (`crypto/rand`)
  stays `extern without summary: crypto/rand.Read`; a function a document of the set defines wins
  over an entry (the report's `notes` say so, as today).
- The file stays valid `codespec-effects/1` (strict reading), canonical and sorted; the existing
  test that reads it holds its 37 entries and gains one case per package of the table.
- A signature test: every entry resolves against the standard library of the Go the gate runs
  (a Go test in `adapters/go/cmd/codespec-read-go` through `go/types`, run once for the whole
  file, not per entry); a function the pinned Go lacks fails it, so a Go upgrade is noticed. It
  runs in under 60 s.
- `docs/static-analysis.md` holds the table (package, entries, effect, the Go version the
  signatures were taken from) and a test compares it with the file; `task check` green for the
  parts touched; each new test under 60 s and the test step within the 120 s budget.

## Scope

- Own: `summaries/go-std.effects.json`, `crates/effects/tests/effects.rs`,
  `crates/effects/tests/data` (the reproducer's document), `adapters/go/cmd/codespec-read-go`
  (the signature test only, a new `_test.go`), `docs/static-analysis.md`. Inferred; recorded as
  typed scope. `CHANGELOG.md` gets one line, outside the typed scope.
- Shares `summaries/` and `docs/static-analysis.md` with `story:go-library-effect-summaries`;
  that story adds a separate file and the format change, this one only entries, so whichever
  starts second rebases on the other. It is the Go counterpart of the standard-library part of
  `story:jvm-library-effect-summaries` and follows its method (a table in the doc, a signature
  test).

## Out of Scope

- A reader change: if a call of the table is not an extern in the document (a `foreign`
  statement), the story records it in `## Observed` and files a reader story; it does not
  summarise around it.
- Third-party modules (`story:go-library-effect-summaries`), `database/sql` and `os/exec`
  (a story of their own, for the effects `db` and `proc` by argument), and cgo.

## Amended in the adversary round, 2026-11-26

- "An empty `unknown`" for `Run` cannot be met by summary entries: `defer cancel()` is a `call_value` of a local of the extern type `context.CancelFunc`, which `crates/effects` reports as `function value with no candidate in the set: context.CancelFunc` and which no entry can name. That clause moves to story:call-value-extern-func-type.
- A named function passed to a summarised extern (`once.Do(initX)`, `sort.Slice(x, less)`) counts as a call candidate of the caller, so its effects and unknowns are not lost.
- `log` and `log/slog` log calls add `time`; `time.Time.Format`, `time.Date` and `time.Parse` add `env` and `fs.read` (the Local zone); the default-client `net/http` calls add `env` (the proxy environment).
