# codespec as a static analysis tool — design (2026-11-08)

Status: proposed, 2026-11-08. Epic `epic:externs-and-effects` implements § 3 and § 4; epic
`epic:whole-repo-to-syscalls` implements § 5 to § 5c, § 5d records what is built; § 6 is later work.

## 1. The question

`cir/1` is one typed representation of what code does inside a function, read from Go, Rust, Java
and Groovy. A pass written once over it is an analysis for every language a reader exists for. The
passes that exist today (`crates/passes`) already are analyses: `fallible`, `read_only`,
`comma_ok`, `unordered_iteration`. Two things stop codespec from being a static analysis tool for
real programs:

| gap | where | effect |
|---|---|---|
| a module cannot name a function outside itself | `Expr::Call` is "a call of a module function" (`crates/model/src/lib.rs`) | every call into the standard library or a dependency is a `foreign` statement, so the analysis sees nothing at the point where a program does IO |
| nothing says what a library function does | there is no summary format | even a named call would carry no meaning an analysis could use |

The Go reader shows the first: only `fmt.Sprintf`, `fmt.Errorf` and `fmt.Println` become
`intrinsic`; any other selector call on a package returns "expression outside cir/1" and the
statement is kept `foreign` with a refusal (`adapters/go/cmd/codespec-read-go/front.go`, `call`).
The Groovy reader does the same for GORM and Spring calls (`adapters/groovy/README.md`).

## 2. The approach: name the boundary, summarise it, never guess past it

Static analysers for many languages solve dependencies in one of two ways: analyse the whole
program including every library (precise, expensive, needs every source), or replace library code
with **summaries** — a declared behaviour per library function (CodeQL's models-as-data, Infer's
compositional summaries). codespec takes the second, in three layers:

| layer | what | where its content comes from |
|---|---|---|
| extern declaration | a module lists every outside function it calls: import path, name, parameter and result types; a call names it | the reader. Each reader already resolves these types with its language's own tooling (`go/types`, `javac`), so the declaration costs nothing to produce and is typed like every other expression |
| effect summary | per extern: the effects it may have, from a closed vocabulary | the standard library: written once per language, checked in. A dependency with source: computed by reading it with the same reader (§ 5) |
| unknown | a call with no summary, a `foreign` statement, dynamic dispatch the module cannot resolve | the analysis reports it as `unknown` with its position, never as pure |

The third row is the existing rule of this repository applied to analysis: a construct a reader
cannot express becomes `foreign` plus a refusal and is never dropped or approximated. An analysis
that treated an unsummarised call as pure would be the approximation that rule forbids.

### Syscalls

codespec does not model syscalls. The standard library is the layer that wraps them, so a summary
of `os.ReadFile` as `fs.read` is a summary of the `openat`/`read`/`close` beneath it, at the level a
program's author can act on. A program that makes a syscall directly (`syscall.Syscall`,
`unsafe`, JNI, FFI) calls an extern whose summary is `unknown`, or has a `foreign` statement.

## 3. `cir/1` additions (additive)

Both are new optional parts; every earlier document reads unchanged, as with `for`, `break` and
`continue` (`docs/design.md` § 3). `Expr::Call` keeps its meaning.

| addition | fields | meaning |
|---|---|---|
| `Module.externs` | ordered list of `{path, name, params: [Type], results: [Type]}` | every function outside the module that the module calls, sorted by `path` then `name`, each once |
| `Expr::Extern` (`k: extern`) | `path`, `name`, `args`, `t` | a call of the extern `path`.`name`; it must be declared in `externs` with matching arity |

Feature id: `expr.extern`. Readers and writers that do not handle it list it in their manifest's
`not`. A writer that does not claim it refuses the function, as for any unclaimed feature.

What stays outside: a variadic extern, an extern whose parameter or result type has no `cir/1`
type, a method on a type from another package, a function value. Each stays `foreign` as today.

## 4. Effect summaries and the report

### `codespec-effects/1`: a summary document

```json
{
  "format": "codespec-effects/1",
  "language": "go",
  "externs": [
    {"path": "os", "name": "Getenv", "effects": ["env"]},
    {"path": "strings", "name": "ToUpper", "effects": []}
  ]
}
```

- `effects` is a sorted set from the closed vocabulary below; `[]` declares the function pure.
- Read strictly: unknown fields, unknown effects, duplicate `(path, name)` and any other format are
  refused.
- `summaries/go-std.effects.json` is the checked-in Go standard library summary (below).

| effect | meaning |
|---|---|
| `env` | reads or writes the process environment or arguments |
| `fs.read` | reads the file system |
| `fs.write` | writes the file system |
| `io` | reads or writes a file descriptor whose origin the analysis does not follow: a file, a socket or a standard stream |
| `net` | opens or uses a network connection |
| `proc` | starts, signals or ends a process, including `os.Exit` |
| `rand` | reads a random source |
| `stdin` | reads standard input |
| `stdout` | writes standard output or standard error |
| `time` | reads a clock or sleeps |

### The Go standard library summary

`summaries/go-std.effects.json` holds 864 entries in 26 packages, for the standard library
packages a typical service calls. The signatures are those of **Go 1.23**, the `go` line of
`adapters/go/go.mod` and the Go CI installs: every entry is a function or method of the API lists
`api/go1.txt` to `api/go1.23.txt` of the Go distribution. These tests hold the file:

- `adapters/go/cmd/codespec-read-go/std_summary_test.go` resolves every entry with `go/types`
  against the standard library of the Go that runs it, each package imported once, and refuses an
  entry that an API list after Go 1.23 adds; an entry for a function the pinned Go lacks fails
  there, locally on a newer Go too.
- `adapters/go/cmd/codespec-read-go/std_summary_adversary_test.go` refuses an entry that names a
  promoted method (no call is named so), and checks that a named function passed to an entry
  taking a function value reads as a `func_ref` argument or a `ref` beside the call; the Go
  tests share one import pass.
- `crates/effects/tests/effects.rs::the_table_in_static_analysis_md_is_the_go_standard_library_summary`
  requires the table below to be the file: one row per package and effect set, rows by package
  and then effect set, names sorted.

These rules decide the effect of a function that does not name a resource itself, so that a cell
is not a taste. Where a function may or may not touch a resource, the entry says it may: sound
over precise.

- A function that reads or writes through a stream its caller supplies (an `io.Reader`,
  `io.Writer`, `io.RuneReader`, `http.ResponseWriter`) has `io`, the vocabulary's own meaning: a
  descriptor whose origin the analysis does not follow. It is not `net` and not `fs.write`. A
  constructor that only wraps the stream (`bufio.NewReader`, `json.NewEncoder`,
  `http.MaxBytesReader`) reads nothing and is pure, and so is a method that only reads or resets
  the buffer (`bufio.Reader.Size`, `Buffered`, `Reset`; `bufio.Writer.Available`,
  `AvailableBuffer`). In `bytes`, `strings`, `regexp` and the other packages whose every function
  is listed, a function or method with such a parameter has `io` (`bytes.Buffer.WriteTo`,
  `strings.Replacer.WriteString`, `regexp.Regexp.MatchReader`).
- A logger writes `stdout`, which the vocabulary defines as standard output or standard error;
  where the destination is a handler chosen at run time the entry says what the default handler
  does. Every record is stamped with the clock (`log/log.go` `output`, `log/slog/logger.go`
  `log`), so a logging call also has `time`, and the stamp is formatted in the local zone (next
  rule). A constructor of a logger (`log.New`, `slog.New`) writes nothing and is pure.
- A function that may present a time in the local zone loads that zone once per process: `$TZ`
  and `/etc/localtime` (`time/zoneinfo.go`, `initLocal`). `time.Date`, `Parse`,
  `ParseInLocation`, `Time.Format` and every logging call have `env` and `fs.read`.
- A function given a file name reads or writes the file system: `filepath.Abs`, `EvalSymlinks`,
  `Glob`, `Walk` and `WalkDir` have `fs.read`, `http.ServeFile` has `fs.read` and `io`.
- A request through `http.DefaultClient` or a `Client` without its own transport reads the proxy
  environment (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`; `net/http/transport.go`,
  `ProxyFromEnvironment`): `Get`, `Head`, `Post`, `PostForm` and the `Client` methods have `env`
  and `net`.
- An entry may take a function value (`sync.Once.Do`, `sort.Slice`, `filepath.WalkDir`,
  `http.HandleFunc`, `time.AfterFunc`): its effects are those of the function itself.
  `crates/effects` counts a function of the set given to a call outside the set as called by the
  caller at that call, so the caller has the effects and the unknowns of what it passes. A
  method of a caller's value that such a function calls through an interface (`sort.Sort(x)`
  calling `x.Less`, `fmt.Sprint(x)` calling `x.String`, `json.Marshal(v)` calling
  `v.MarshalJSON`) is not counted: that is a call through an interface the set cannot see.
- A method is named by the type that declares it, as the reader names the call: a call of a
  promoted method (`rw.Flush()` on a `bufio.ReadWriter`) is the method of the embedded type
  (`bufio.Writer.Flush`), so the table has no entry for a promoted method.

A package outside the table (`crypto/rand`, `database/sql`, `os/exec`, a third-party module)
stays `extern without summary: <path>.<name>`. A call no entry can name stays unknown too: a
call of a function value, such as `defer cancel()` after `ctx, cancel :=
context.WithTimeout(…)`, is a `call_value` and has `function value with no candidate in the set:
context.CancelFunc` while no document of the set uses a function of that type as a value.

| package | entries | effect |
|---|---|---|
| `bufio` | `NewReadWriter`, `NewReader`, `NewReaderSize`, `NewScanner`, `NewWriter`, `NewWriterSize`, `Reader.Buffered`, `Reader.Reset`, `Reader.Size`, `ScanBytes`, `ScanLines`, `ScanRunes`, `ScanWords`, `Scanner.Buffer`, `Scanner.Bytes`, `Scanner.Err`, `Scanner.Split`, `Scanner.Text`, `Writer.Available`, `Writer.AvailableBuffer`, `Writer.Buffered`, `Writer.Reset`, `Writer.Size` | pure |
| `bufio` | `Reader.Discard`, `Reader.Peek`, `Reader.Read`, `Reader.ReadByte`, `Reader.ReadBytes`, `Reader.ReadLine`, `Reader.ReadRune`, `Reader.ReadSlice`, `Reader.ReadString`, `Reader.UnreadByte`, `Reader.UnreadRune`, `Reader.WriteTo`, `Scanner.Scan`, `Writer.Flush`, `Writer.ReadFrom`, `Writer.Write`, `Writer.WriteByte`, `Writer.WriteRune`, `Writer.WriteString` | `io` |
| `bytes` | `Buffer.Available`, `Buffer.AvailableBuffer`, `Buffer.Bytes`, `Buffer.Cap`, `Buffer.Grow`, `Buffer.Len`, `Buffer.Next`, `Buffer.Read`, `Buffer.ReadByte`, `Buffer.ReadBytes`, `Buffer.ReadRune`, `Buffer.ReadString`, `Buffer.Reset`, `Buffer.String`, `Buffer.Truncate`, `Buffer.UnreadByte`, `Buffer.UnreadRune`, `Buffer.Write`, `Buffer.WriteByte`, `Buffer.WriteRune`, `Buffer.WriteString`, `Clone`, `Compare`, `Contains`, `ContainsAny`, `ContainsFunc`, `ContainsRune`, `Count`, `Cut`, `CutPrefix`, `CutSuffix`, `Equal`, `EqualFold`, `Fields`, `FieldsFunc`, `HasPrefix`, `HasSuffix`, `Index`, `IndexAny`, `IndexByte`, `IndexFunc`, `IndexRune`, `Join`, `LastIndex`, `LastIndexAny`, `LastIndexByte`, `LastIndexFunc`, `Map`, `NewBuffer`, `NewBufferString`, `NewReader`, `Reader.Len`, `Reader.Read`, `Reader.ReadAt`, `Reader.ReadByte`, `Reader.ReadRune`, `Reader.Reset`, `Reader.Seek`, `Reader.Size`, `Reader.UnreadByte`, `Reader.UnreadRune`, `Repeat`, `Replace`, `ReplaceAll`, `Runes`, `Split`, `SplitAfter`, `SplitAfterN`, `SplitN`, `Title`, `ToLower`, `ToLowerSpecial`, `ToTitle`, `ToTitleSpecial`, `ToUpper`, `ToUpperSpecial`, `ToValidUTF8`, `Trim`, `TrimFunc`, `TrimLeft`, `TrimLeftFunc`, `TrimPrefix`, `TrimRight`, `TrimRightFunc`, `TrimSpace`, `TrimSuffix` | pure |
| `bytes` | `Buffer.ReadFrom`, `Buffer.WriteTo`, `Reader.WriteTo` | `io` |
| `context` | `AfterFunc`, `Background`, `Cause`, `TODO`, `WithCancel`, `WithCancelCause`, `WithValue`, `WithoutCancel` | pure |
| `context` | `WithDeadline`, `WithDeadlineCause`, `WithTimeout`, `WithTimeoutCause` | `time` |
| `encoding/json` | `Compact`, `Decoder.DisallowUnknownFields`, `Decoder.UseNumber`, `Encoder.SetEscapeHTML`, `Encoder.SetIndent`, `HTMLEscape`, `Indent`, `Marshal`, `MarshalIndent`, `NewDecoder`, `NewEncoder`, `Unmarshal`, `Valid` | pure |
| `encoding/json` | `Decoder.Decode`, `Decoder.More`, `Decoder.Token`, `Encoder.Encode` | `io` |
| `errors` | `As`, `Is`, `Join`, `New`, `Unwrap` | pure |
| `fmt` | `Append`, `Appendf`, `Appendln`, `Errorf`, `FormatString`, `Sprint`, `Sprintf`, `Sprintln`, `Sscan`, `Sscanf`, `Sscanln` | pure |
| `fmt` | `Fprint`, `Fprintf`, `Fprintln`, `Fscan`, `Fscanf`, `Fscanln` | `io` |
| `fmt` | `Scan`, `Scanf`, `Scanln` | `stdin` |
| `fmt` | `Print`, `Printf`, `Println` | `stdout` |
| `io` | `LimitReader`, `MultiReader`, `MultiWriter`, `NewOffsetWriter`, `NewSectionReader`, `NopCloser`, `Pipe`, `TeeReader` | pure |
| `io` | `Copy`, `CopyBuffer`, `CopyN`, `ReadAll`, `ReadAtLeast`, `ReadFull`, `WriteString` | `io` |
| `log` | `New` | pure |
| `log` | `Fatal`, `Fatalf`, `Fatalln`, `Logger.Fatal`, `Logger.Fatalf`, `Logger.Fatalln` | `env`, `fs.read`, `proc`, `stdout`, `time` |
| `log` | `Logger.Panic`, `Logger.Panicf`, `Logger.Panicln`, `Logger.Print`, `Logger.Printf`, `Logger.Println`, `Panic`, `Panicf`, `Panicln`, `Print`, `Printf`, `Println` | `env`, `fs.read`, `stdout`, `time` |
| `log/slog` | `Any`, `Bool`, `Default`, `Duration`, `Float64`, `Group`, `Int`, `Int64`, `Logger.With`, `Logger.WithGroup`, `New`, `NewJSONHandler`, `NewTextHandler`, `SetDefault`, `String`, `Time` | pure |
| `log/slog` | `Debug`, `DebugContext`, `Error`, `ErrorContext`, `Info`, `InfoContext`, `Log`, `LogAttrs`, `Logger.Debug`, `Logger.DebugContext`, `Logger.Error`, `Logger.ErrorContext`, `Logger.Info`, `Logger.InfoContext`, `Logger.Log`, `Logger.LogAttrs`, `Logger.Warn`, `Logger.WarnContext`, `Warn`, `WarnContext` | `env`, `fs.read`, `stdout`, `time` |
| `maps` | `All`, `Clone`, `Collect`, `Copy`, `DeleteFunc`, `Equal`, `EqualFunc`, `Insert`, `Keys`, `Values` | pure |
| `math` | `Abs`, `Acos`, `Acosh`, `Asin`, `Asinh`, `Atan`, `Atan2`, `Atanh`, `Cbrt`, `Ceil`, `Copysign`, `Cos`, `Cosh`, `Dim`, `Erf`, `Erfc`, `Erfcinv`, `Erfinv`, `Exp`, `Exp2`, `Expm1`, `FMA`, `Float32bits`, `Float32frombits`, `Float64bits`, `Float64frombits`, `Floor`, `Frexp`, `Gamma`, `Hypot`, `Ilogb`, `Inf`, `IsInf`, `IsNaN`, `J0`, `J1`, `Jn`, `Ldexp`, `Lgamma`, `Log`, `Log10`, `Log1p`, `Log2`, `Logb`, `Max`, `Min`, `Mod`, `Modf`, `NaN`, `Nextafter`, `Nextafter32`, `Pow`, `Pow10`, `Remainder`, `Round`, `RoundToEven`, `Signbit`, `Sin`, `Sincos`, `Sinh`, `Sqrt`, `Tan`, `Tanh`, `Trunc`, `Y0`, `Y1`, `Yn` | pure |
| `math/rand` | `Int`, `Intn` | `rand` |
| `net` | `LookupHost` | `net` |
| `net/http` | `Handle`, `HandleFunc`, `Header.Add`, `Header.Del`, `Header.Get`, `Header.Set`, `MaxBytesReader`, `NewRequest`, `NewRequestWithContext`, `NewServeMux`, `ServeMux.Handle`, `ServeMux.HandleFunc`, `StatusText` | pure |
| `net/http` | `Client.Do`, `Client.Get`, `Client.Head`, `Client.Post`, `Client.PostForm`, `Get`, `Head`, `Post`, `PostForm` | `env`, `net` |
| `net/http` | `ServeFile` | `fs.read`, `io` |
| `net/http` | `Error`, `NotFound`, `Redirect` | `io` |
| `net/http` | `ListenAndServe`, `ListenAndServeTLS`, `Serve`, `ServeTLS`, `Server.Close`, `Server.ListenAndServe`, `Server.ListenAndServeTLS`, `Server.Serve`, `Server.ServeTLS`, `Server.Shutdown` | `net` |
| `net/url` | `Error.Error`, `Error.Temporary`, `Error.Timeout`, `Error.Unwrap`, `EscapeError.Error`, `InvalidHostError.Error`, `JoinPath`, `Parse`, `ParseQuery`, `ParseRequestURI`, `PathEscape`, `PathUnescape`, `QueryEscape`, `QueryUnescape`, `URL.EscapedFragment`, `URL.EscapedPath`, `URL.Hostname`, `URL.IsAbs`, `URL.JoinPath`, `URL.MarshalBinary`, `URL.Parse`, `URL.Port`, `URL.Query`, `URL.Redacted`, `URL.RequestURI`, `URL.ResolveReference`, `URL.String`, `URL.UnmarshalBinary`, `User`, `UserPassword`, `Userinfo.Password`, `Userinfo.String`, `Userinfo.Username`, `Values.Add`, `Values.Del`, `Values.Encode`, `Values.Get`, `Values.Has`, `Values.Set` | pure |
| `os` | `Environ`, `ExpandEnv`, `Getenv`, `Hostname`, `LookupEnv`, `Setenv`, `Unsetenv`, `UserHomeDir` | `env` |
| `os` | `Executable`, `Getwd`, `Lstat`, `Open`, `ReadDir`, `ReadFile`, `Stat` | `fs.read` |
| `os` | `OpenFile` | `fs.read`, `fs.write` |
| `os` | `Chmod`, `Create`, `Mkdir`, `MkdirAll`, `Remove`, `RemoveAll`, `Rename`, `Truncate`, `WriteFile` | `fs.write` |
| `os` | `Exit`, `Getpid` | `proc` |
| `path/filepath` | `Base`, `Clean`, `Dir`, `Ext`, `FromSlash`, `HasPrefix`, `IsAbs`, `IsLocal`, `Join`, `Localize`, `Match`, `Rel`, `Split`, `SplitList`, `ToSlash`, `VolumeName` | pure |
| `path/filepath` | `Abs`, `EvalSymlinks`, `Glob`, `Walk`, `WalkDir` | `fs.read` |
| `regexp` | `Compile`, `CompilePOSIX`, `Match`, `MatchString`, `MustCompile`, `MustCompilePOSIX`, `QuoteMeta`, `Regexp.Copy`, `Regexp.Expand`, `Regexp.ExpandString`, `Regexp.Find`, `Regexp.FindAll`, `Regexp.FindAllIndex`, `Regexp.FindAllString`, `Regexp.FindAllStringIndex`, `Regexp.FindAllStringSubmatch`, `Regexp.FindAllStringSubmatchIndex`, `Regexp.FindAllSubmatch`, `Regexp.FindAllSubmatchIndex`, `Regexp.FindIndex`, `Regexp.FindString`, `Regexp.FindStringIndex`, `Regexp.FindStringSubmatch`, `Regexp.FindStringSubmatchIndex`, `Regexp.FindSubmatch`, `Regexp.FindSubmatchIndex`, `Regexp.LiteralPrefix`, `Regexp.Longest`, `Regexp.MarshalText`, `Regexp.Match`, `Regexp.MatchString`, `Regexp.NumSubexp`, `Regexp.ReplaceAll`, `Regexp.ReplaceAllFunc`, `Regexp.ReplaceAllLiteral`, `Regexp.ReplaceAllLiteralString`, `Regexp.ReplaceAllString`, `Regexp.ReplaceAllStringFunc`, `Regexp.Split`, `Regexp.String`, `Regexp.SubexpIndex`, `Regexp.SubexpNames`, `Regexp.UnmarshalText` | pure |
| `regexp` | `MatchReader`, `Regexp.FindReaderIndex`, `Regexp.FindReaderSubmatchIndex`, `Regexp.MatchReader` | `io` |
| `slices` | `All`, `AppendSeq`, `Backward`, `BinarySearch`, `BinarySearchFunc`, `Chunk`, `Clip`, `Clone`, `Collect`, `Compact`, `CompactFunc`, `Compare`, `CompareFunc`, `Concat`, `Contains`, `ContainsFunc`, `Delete`, `DeleteFunc`, `Equal`, `EqualFunc`, `Grow`, `Index`, `IndexFunc`, `Insert`, `IsSorted`, `IsSortedFunc`, `Max`, `MaxFunc`, `Min`, `MinFunc`, `Repeat`, `Replace`, `Reverse`, `Sort`, `SortFunc`, `SortStableFunc`, `Sorted`, `SortedFunc`, `SortedStableFunc`, `Values` | pure |
| `sort` | `Find`, `Float64Slice.Len`, `Float64Slice.Less`, `Float64Slice.Search`, `Float64Slice.Sort`, `Float64Slice.Swap`, `Float64s`, `Float64sAreSorted`, `IntSlice.Len`, `IntSlice.Less`, `IntSlice.Search`, `IntSlice.Sort`, `IntSlice.Swap`, `Ints`, `IntsAreSorted`, `IsSorted`, `Reverse`, `Search`, `SearchFloat64s`, `SearchInts`, `SearchStrings`, `Slice`, `SliceIsSorted`, `SliceStable`, `Sort`, `Stable`, `StringSlice.Len`, `StringSlice.Less`, `StringSlice.Search`, `StringSlice.Sort`, `StringSlice.Swap`, `Strings`, `StringsAreSorted` | pure |
| `strconv` | `AppendBool`, `AppendFloat`, `AppendInt`, `AppendQuote`, `AppendQuoteRune`, `AppendQuoteRuneToASCII`, `AppendQuoteRuneToGraphic`, `AppendQuoteToASCII`, `AppendQuoteToGraphic`, `AppendUint`, `Atoi`, `CanBackquote`, `FormatBool`, `FormatComplex`, `FormatFloat`, `FormatInt`, `FormatUint`, `IsGraphic`, `IsPrint`, `Itoa`, `NumError.Error`, `NumError.Unwrap`, `ParseBool`, `ParseComplex`, `ParseFloat`, `ParseInt`, `ParseUint`, `Quote`, `QuoteRune`, `QuoteRuneToASCII`, `QuoteRuneToGraphic`, `QuoteToASCII`, `QuoteToGraphic`, `QuotedPrefix`, `Unquote`, `UnquoteChar` | pure |
| `strings` | `Builder.Cap`, `Builder.Grow`, `Builder.Len`, `Builder.Reset`, `Builder.String`, `Builder.Write`, `Builder.WriteByte`, `Builder.WriteRune`, `Builder.WriteString`, `Clone`, `Compare`, `Contains`, `ContainsAny`, `ContainsFunc`, `ContainsRune`, `Count`, `Cut`, `CutPrefix`, `CutSuffix`, `EqualFold`, `Fields`, `FieldsFunc`, `HasPrefix`, `HasSuffix`, `Index`, `IndexAny`, `IndexByte`, `IndexFunc`, `IndexRune`, `Join`, `LastIndex`, `LastIndexAny`, `LastIndexByte`, `LastIndexFunc`, `Map`, `NewReader`, `NewReplacer`, `Reader.Len`, `Reader.Read`, `Reader.ReadAt`, `Reader.ReadByte`, `Reader.ReadRune`, `Reader.Reset`, `Reader.Seek`, `Reader.Size`, `Reader.UnreadByte`, `Reader.UnreadRune`, `Repeat`, `Replace`, `ReplaceAll`, `Replacer.Replace`, `Split`, `SplitAfter`, `SplitAfterN`, `SplitN`, `Title`, `ToLower`, `ToLowerSpecial`, `ToTitle`, `ToTitleSpecial`, `ToUpper`, `ToUpperSpecial`, `ToValidUTF8`, `Trim`, `TrimFunc`, `TrimLeft`, `TrimLeftFunc`, `TrimPrefix`, `TrimRight`, `TrimRightFunc`, `TrimSpace`, `TrimSuffix` | pure |
| `strings` | `Reader.WriteTo`, `Replacer.WriteString` | `io` |
| `sync` | `Cond.Broadcast`, `Cond.Signal`, `Cond.Wait`, `Map.Clear`, `Map.CompareAndDelete`, `Map.CompareAndSwap`, `Map.Delete`, `Map.Load`, `Map.LoadAndDelete`, `Map.LoadOrStore`, `Map.Range`, `Map.Store`, `Map.Swap`, `Mutex.Lock`, `Mutex.TryLock`, `Mutex.Unlock`, `NewCond`, `Once.Do`, `OnceFunc`, `OnceValue`, `OnceValues`, `Pool.Get`, `Pool.Put`, `RWMutex.Lock`, `RWMutex.RLock`, `RWMutex.RLocker`, `RWMutex.RUnlock`, `RWMutex.TryLock`, `RWMutex.TryRLock`, `RWMutex.Unlock`, `WaitGroup.Add`, `WaitGroup.Done`, `WaitGroup.Wait` | pure |
| `sync/atomic` | `AddInt32`, `AddInt64`, `AddUint32`, `AddUint64`, `AddUintptr`, `AndInt32`, `AndInt64`, `AndUint32`, `AndUint64`, `AndUintptr`, `Bool.CompareAndSwap`, `Bool.Load`, `Bool.Store`, `Bool.Swap`, `CompareAndSwapInt32`, `CompareAndSwapInt64`, `CompareAndSwapPointer`, `CompareAndSwapUint32`, `CompareAndSwapUint64`, `CompareAndSwapUintptr`, `Int32.Add`, `Int32.And`, `Int32.CompareAndSwap`, `Int32.Load`, `Int32.Or`, `Int32.Store`, `Int32.Swap`, `Int64.Add`, `Int64.And`, `Int64.CompareAndSwap`, `Int64.Load`, `Int64.Or`, `Int64.Store`, `Int64.Swap`, `LoadInt32`, `LoadInt64`, `LoadPointer`, `LoadUint32`, `LoadUint64`, `LoadUintptr`, `OrInt32`, `OrInt64`, `OrUint32`, `OrUint64`, `OrUintptr`, `Pointer.CompareAndSwap`, `Pointer.Load`, `Pointer.Store`, `Pointer.Swap`, `StoreInt32`, `StoreInt64`, `StorePointer`, `StoreUint32`, `StoreUint64`, `StoreUintptr`, `SwapInt32`, `SwapInt64`, `SwapPointer`, `SwapUint32`, `SwapUint64`, `SwapUintptr`, `Uint32.Add`, `Uint32.And`, `Uint32.CompareAndSwap`, `Uint32.Load`, `Uint32.Or`, `Uint32.Store`, `Uint32.Swap`, `Uint64.Add`, `Uint64.And`, `Uint64.CompareAndSwap`, `Uint64.Load`, `Uint64.Or`, `Uint64.Store`, `Uint64.Swap`, `Uintptr.Add`, `Uintptr.And`, `Uintptr.CompareAndSwap`, `Uintptr.Load`, `Uintptr.Or`, `Uintptr.Store`, `Uintptr.Swap`, `Value.CompareAndSwap`, `Value.Load`, `Value.Store`, `Value.Swap` | pure |
| `time` | `Duration.Abs`, `Duration.Hours`, `Duration.Microseconds`, `Duration.Milliseconds`, `Duration.Minutes`, `Duration.Nanoseconds`, `Duration.Round`, `Duration.Seconds`, `Duration.String`, `Duration.Truncate`, `ParseDuration`, `Ticker.Stop`, `Time.Add`, `Time.After`, `Time.Before`, `Time.Equal`, `Time.IsZero`, `Time.Sub`, `Time.Truncate`, `Time.UTC`, `Time.Unix`, `Timer.Stop`, `Unix`, `UnixMilli` | pure |
| `time` | `Date`, `Parse`, `ParseInLocation`, `Time.Format` | `env`, `fs.read` |
| `time` | `After`, `AfterFunc`, `NewTicker`, `NewTimer`, `Now`, `Since`, `Sleep`, `Tick`, `Ticker.Reset`, `Timer.Reset`, `Until` | `time` |
| `unicode/utf8` | `AppendRune`, `DecodeLastRune`, `DecodeLastRuneInString`, `DecodeRune`, `DecodeRuneInString`, `EncodeRune`, `FullRune`, `FullRuneInString`, `RuneCount`, `RuneCountInString`, `RuneLen`, `RuneStart`, `Valid`, `ValidRune`, `ValidString` | pure |


### `codespec effects`: the report

`codespec effects <module.cir.json> --summaries <file>…` prints `codespec-effects-report/1`: one
entry per function (`Func::key`, in module order) with

- `effects`: the union of the effects of every extern it calls, every intrinsic with an effect
  (`println` → `stdout`) and every module function or method it calls, to a fixed point over the
  module's call graph;
- `unknown`: every reason the set may be incomplete, each with a position: an extern with no
  summary, a `foreign` statement, a method call whose receiver type the module does not declare,
  a call naming no function of the module, or a module function that itself has one.

The report is deterministic (canonical JSON, ordered collections only). A function with an empty
`effects` and an empty `unknown` is pure in the sense the summaries define.

## 5. Later: dependencies and the standard library from source, down to the syscall barrier

A standard library is mostly code in its own language: Go's `os.ReadFile` is Go down to
`syscall.Syscall6`, Rust's `std::fs::read` is Rust down to `libc`. The checked-in
`summaries/go-std.effects.json` is therefore a stopgap. The target is to read the standard library
and every dependency with the same reader, compute their summaries bottom-up, and write by hand
only what sits at the barrier, where no source in the language exists.

- **Summaries computed from source.** A dependency or standard library package is read like the
  module itself; `codespec effects --emit-summary` writes the `codespec-effects/1` of its exported
  functions, keyed by import path, to be passed as `--summaries` to its callers. Cached by the
  dependency's version from the lockfile (`go.sum`, `Cargo.lock`) or the toolchain version.
- **The barrier, Go on Linux (read in go1.27.1's source).** `syscall.Syscall6` calls
  `RawSyscall6`, which calls `internal/runtime/syscall/linux.Syscall6`, declared without a body
  and implemented in `asm_linux_amd64.s`. Every call site passes the syscall number as a
  constant: `openat` calls `Syscall6(SYS_OPENAT, …)` (`zsyscall_linux_amd64.go`), and
  `zsysnum_linux_amd64.go` names 303 numbers. A function without a body is the barrier; its
  summary is the one hand-written part: syscall name → effect.
- **What the barrier cannot say.** `read` and `write` take a file descriptor, and the syscall is
  the same for a file, a socket and standard input. The effect class comes from where the
  descriptor was made (`openat` → `fs`, `socket`/`connect` → `net`), so an effect at the barrier
  is `io` unless the analysis follows the descriptor back to the call that made it.
- **Calls inside `foreign` statements.** Most standard library code uses constructs `cir/1` does
  not express (`unsafe`, `uintptr`, closures, generics, `defer`). A reader that keeps such a
  statement `foreign` should still list the calls inside it, so the call graph reaches the barrier
  even where the body is not expressible.
- **Loading.** A Go package directory holds test packages and files for other platforms; the
  reader must select files by build constraints (`GOOS`, `GOARCH`) as the toolchain does.
- **Several modules at once.** One module per document stays; a link step reads a set of documents
  and resolves an extern to a function in another document of the set.
- **Interfaces and dynamic dispatch.** `cir/1` has no interface types today; when it gains them, a
  call through one has the union of the summaries of every implementation in the analysed set,
  and `unknown` when the set is open.

## 5a. A whole repository: what `cir/1` must hold first

Measured 2026-11-08 on a third-party Go library (6 packages, 756 `func`
declarations, 56 structs, 11 interfaces), read package by package with the Go reader after it was
changed to refuse a declaration it cannot express instead of failing the whole package: 36
functions read with no `foreign` statement, 971 refusals. The root causes, by declaration:

| construct | declarations | what `cir/1` needs |
|---|---|---|
| a type from another package: the standard library (`sync.Mutex`, `net.Conn`, `net.IP`, `slog.Logger`) or another package of the repository (a URI type declared in a sibling package) | 10 struct-field roots (stdlib), 17 (another package of the repository); 658 refusals name a package-qualified type, the cascade included | an opaque `extern` type `{path, name}` beside `extern` functions, extern methods (a receiver type), and a link step across the repository's packages (§ 5) |
| interface types | 11 declarations | an `interface` declaration and a method call that dispatches on it |
| named func, map and slice types; func-typed fields and parameters | 27 declarations, 4 fields | a `func` type, a function literal, a call of a function value; a newtype over any type |
| embedded fields | 16 structs | a field marked embedded; the reader resolves promotion to an explicit path |
| variadic functions | 20 | a variadic last parameter and a spread argument |
| package-level variables | 44 | module-level `var` declarations |
| generics | 12 | type parameters; last, being the largest change |
| arrays, `uintptr`, floats | 7 | an array type, a float type; `uintptr` and `unsafe` stay at the barrier |

A refusal cascades: a struct refused for one field takes every method on it, every function with
it in a signature and every statement using it. The first row therefore unblocks the most.

## 5b. The goal: a repository mapped down to syscalls

`epic:whole-repo-to-syscalls` implements § 5 and § 5a for one repository, the third-party Go library
of § 5a: its 6 packages and the 228 packages they import (217 of the standard library, 11 of
other modules; `go list -deps`, Go 1.27.1, linux/amd64) are read, linked into one set, and
`codespec effects` reports for every function of the library the syscalls it may reach and the effects
they map to. The pieces, in the order they depend on each other:

| piece | what it adds |
|---|---|
| reading a whole package | build-constraint file selection; refusal per declaration (done in the reader, § 5a) |
| the types of § 5a | extern types and methods, interfaces, function values, embedded fields, module `var`s, variadic, arrays and floats, generics |
| calls inside what stays outside | a `foreign` statement and a refused function still list the calls they make |
| a module's import path and a link step | `Module.path`; `codespec effects` over a set of documents resolves an extern to the function of another document |
| dispatch | a call through an interface or a function value reaches every method or function in the set that could stand there (class-hierarchy analysis over the set) |
| the barrier | a function with no Go body, `//go:linkname`, and a table from syscall number to effect; the report gains `syscalls` |

## 5c. Questions the linked set answers

Each question below is a pass over the linked call graph of § 5b. The first four are stories of
`epic:whole-repo-to-syscalls`; the rest need that graph and are later work.

| question | pass | status |
|---|---|---|
| which syscalls can this function, handler or endpoint reach | effects with the barrier, from an entry function | epic |
| where does a side effect happen | the effect report with the position of each extern or syscall call | epic |
| which functions are never called (tree shaking) | reachability from the entry functions: `main`, exported functions, registered handlers | epic |
| what does one call do, with its branches | a call tree from one function: each `if`, `switch` and loop as a branch node, each call expanded once, syscalls at the leaves, drawn on the documentation site | built: `codespec trace`, `codespec-trace/1` (`crates/effects/src/set/trace.rs`) |
| which branch is not handled | a `switch` over a type or constant set with no case for a member and no `default`; an error result that is dropped (`crates/passes` `fallible` per call) | later |
| where is it buggy or insecure | taint from a source (`net` read, `stdin`, `env`) to a sink (`proc`, `fs.write`, a query) along the linked graph | later |

## 5d. As built: calls, the linked set, dispatch and the barrier

Built 2026-11-08 without the `cir/1` types of § 5a (interfaces, function values, extern types):
everything the analysis needs from code `cir/1` cannot express is read from `go/types` directly.

| piece | what it is |
|---|---|
| `CallRef` | one call named outside `cir/1` expressions: `func` (with its integer constant arguments), `method`, `iface` (the interface's whole method set), `value` (a call of a function value, by signature), `ref` (a function or method used as a value), `lambda` (a function literal and its calls). A signature is spelled without parameter names, named types by full import path: `func([]byte) (int, error)` |
| `foreign.calls`, `Module.opaque` | a `foreign` statement lists its calls; a refused or body-less function stays as an opaque function with its calls. A `foreign` statement without `calls` (other readers) is `unknown` |
| `Module.path`, `methods`, `linknames`, `inits` | the import path; the method set of each named type with the method each entry runs (promotion resolved by `go/types`); every `//go:linkname`; every call and function value of the variable initializers. The last three only for a module with an import path |
| the link | `codespec effects a.cir.json b.cir.json …`: an extern or `CallRef` naming the `path` of a document of the set is an edge to its function; a document wins over a summary |
| dispatch | class-hierarchy analysis over the set: an `iface` call reaches the method of every type whose method set holds all the interface's methods with identical signatures; a `value` call reaches every `ref` and `lambda` of the set with its signature. A literal's calls also belong to the function holding it, so a literal passed to a function outside the set is still counted |
| rapid type analysis | `--dispatch rta`, the default with `--entry` or `--exported`: from the entries (with every `init` and every variable initializer), an `iface` candidate counts once reached code makes its type, a `value` candidate once the code using it as a value is reached, to a fixed point. A type is made by a `new` call ref (the Go reader lists composite literals, `new`, `make` of a slice, conversions, constants and `var` zero values, and the named types they hold by value in fields), a `cir/1` `struct`, `conv`, `zero`, `lit`, `const`, `to_error` or `var`, a struct of `cir/1` holding it in a field, or a method of it being reached. Values made where the set cannot see (reflection, `unsafe`, a function outside the set) are missed. Unknown reasons stay those of class-hierarchy analysis |
| reach | `codespec reach`: per module, the functions the entries reach by a call, a use as a value or a dispatch candidate, and those they do not (`codespec-reach-report/1`) |
| the barrier | `summaries/go-linux-amd64.barrier.json` (`codespec-barrier/1`): the 13 functions taking the syscall number as argument 0 (`syscall.Syscall6`, `internal/runtime/syscall/linux.Syscall6`, …), 44 body-less runtime functions with fixed syscalls, 29 assembly-only packages, and the 385 linux/amd64 syscalls with their effects. A body-less function with a linkname resolves in either direction (`internal/poll.runtime_pollWait` → `runtime.poll_runtime_pollWait`) |
| the fixed point | one call graph over every function, literal and dispatch site of the set; strongly connected components (iterative Tarjan), callees first, bitsets of effects and syscalls: linear in nodes plus edges |
| `cir/1` function values and extern methods | merged after § 5d was built: a `call_value` is a `value` call, a `func_ref` a `ref`, a `lambda` a `lambda` node whose statements also count for the function holding it; the `cir/1` function type is spelled as `go/types` spells the signature, named types by import path. An `extern_method` is the method `<type>.<method>` of the document with its path, or of a summary |

Measured on the third-party Go library of § 5a with its 228 imports (234 packages, 40 MB of
`cir/1`, read in 27 s in parallel): `codespec effects` over all 234 documents takes 3.6 s reporting
two of the library's packages and 15.7 s reporting all 14,958 functions. Its client's request
entry point reaches 74 syscalls, its server's listening entry point 75. Class-hierarchy analysis
over the standard library over-approximates widely: a call through `io.Writer` or of a
`func() error` value reaches every candidate in 234 packages, so most functions that do IO share
one large component. What remains `unknown` for the client entry point (155 functions it may
reach): 66 interface calls with no implementation in the set (generic interfaces such as
`ecdh.Point[P]`, interfaces only callers implement such as a tracing hook), 59 function values
with no candidate (generic signatures, values from outside the set), 27 body-less runtime and
reflect functions that call back into Go (`systemstack`, `mcall`, `reflectcall`, `makeFuncStub`),
3 methods on undeclared types.

The measured example and its report are kept outside this repository. With rapid type analysis
from every exported function of the library's 6 packages and its entry points, all 234 packages
read, 273 of its 756 functions have an empty `unknown`, and the client and server entry points
reach 56 and 57 syscalls. Every function of the library that reaches a syscall reaches at least
56: allocation alone (`runtime.mallocgc`) reaches the scheduler, the garbage collector and through
them the network poller and file IO by direct calls, so a parser of text shows every effect.

## 6. Later: checking summaries against a run

A summary is a claim. The harness already runs every implementation of an example on seeded
traces; running the original under `strace -f` (Linux) or a seccomp filter in log mode and mapping
the observed syscalls to the effect vocabulary gives the observed effects of a run. An observed
effect that the report does not predict is a wrong or missing summary, found by a run rather than
by review — the evidence rule of `docs/design.md` applied to analysis.
