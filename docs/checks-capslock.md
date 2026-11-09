# The built-in checks on google/capslock

This is the record of `codespec check` (crate `crates/check`) run on its test repository: every
check over google/capslock read with every package it imports, and every finding in capslock's
own packages triaged by reading the capslock source. What each check matches is the crate doc of
`crates/check`.

## The run

| | |
|---|---|
| repository | [google/capslock](https://github.com/google/capslock), Apache-2.0 |
| commit | `2ee85fd0bfeaafffbd58d2c38066e443f69d98f6` (2026-10-22) |
| read | `CGO_ENABLED=0 scripts/read-repo.sh --deps <capslock> <docs>`, codespec 0.6.1 built from this tree, Go 1.27.1 |
| documents | 237 of the 240 packages `go list -deps ./...` lists |
| read time | 15.8 s wall clock, packages read in parallel |
| check time | 4.6 to 5.7 s per check over the 237 documents (debug build) |

Three packages are not read. `github.com/google/capslock/testing` holds only a `_test.go` file, and
the reader reads non-test files ("want exactly one package, found 0"). `testpkgs/usecgo` uses cgo
and has no buildable file with `CGO_ENABLED=0`. `testpkgs/transitive` imports `usecgo`.

Each check ran as

```console
codespec check <check> <the 237 documents> --report github.com/google/capslock
```

The check reads the whole set. `--report` keeps the findings in capslock's own packages, the source
this record triages. The counts over the whole set, standard library and dependencies included, are
in [Outside capslock](#outside-capslock); those findings are not triaged.

The same documents give the same bytes: the last two runs are byte-equal for every check.

## Triage rule

A finding is **true** when what it states holds in the capslock source and it is a defect a
maintainer would change. It is **false** otherwise: either the statement does not hold, or it holds
and the code is meant to be that way. Each false finding says which, and why the check matched.

## Results

Two rounds were run on the same documents. Round 1 had the first eight checks. Round 2 added ten
checks and narrowed `dropped-error`, `lock-without-unlock` and `unreached`; its check time is 4.5 to
7.5 s per check.

| check | round 1 | round 2 | true | false | false share | in `--list` |
|---|---:|---:|---:|---:|---:|---|
| `unbounded-growth` | 0 | 0 | 0 | 0 | — | narrowed on dependency findings, see below |
| `insecure-random` | 0 | 0 | 0 | 0 | — | |
| `int-from-text` | 0 | 0 | 0 | 0 | — | |
| `unsigned-below-zero` | — | 0 | 0 | 0 | — | |
| `case-fold-asymmetry` | 0 | 0 | 0 | 0 | — | |
| `clone-aliasing` | 0 | 0 | 0 | 0 | — | |
| `writer-bypass` | — | 0 | 0 | 0 | — | |
| `dropped-error` | 8 | 8 | 0 | 8 | 100 % | `experimental` |
| `dropped-reader` | — | 0 | 0 | 0 | — | |
| `error-eq` | — | 0 | 0 | 0 | — | |
| `lock-without-unlock` | 0 | 0 | 0 | 0 | — | narrowed on dependency findings, see below |
| `timer-not-stopped` | — | 0 | 0 | 0 | — | |
| `conn-leak` | — | 0 | 0 | 0 | — | |
| `accept-blocking` | — | 0 | 0 | 0 | — | |
| `after-in-loop` | — | 0 | 0 | 0 | — | |
| `frame-not-consumed` | — | 0 | 0 | 0 | — | |
| `quadratic-insert` | — | 0 | 0 | 0 | — | |
| `unreached` | 34 | 0 | 0 | 0 | — | narrowed (114 → 44 → 34 → 0), `experimental` |

No finding is true, so no reproducer test exists for either round.

Most checks report nothing in capslock's packages, and their false share is undefined. The source
agrees: a search of capslock's non-test Go files finds no lock taken, no receive from the network,
no integer parsed from text, no case folded and no slice cloned. The one match is a comment,
`analyzer/load.go:67`. `math/rand` is called only in two test inputs. `testpkgs/usereflect` is one,
and no function named for a token reaches its call; `testpkgs/transitive` is not read. For these
checks, the only true positive on record is the one in each fixture under
`crates/check/tests/fixtures/<check>/hit`.

`insecure-random --strict` (every `math/rand` call) gives 1 finding, false:

| position | function | verdict |
|---|---|---|
| `testpkgs/usereflect/usereflect.go:159` | `ChangeSliceCapacityWithSliceHeader` | false: `rand.Intn(2)` chooses which of two slices to allocate in a test input that probes slice headers; nothing secret is made from it |

Positions below are relative to `github.com/google/capslock/`.

## `dropped-error`: 8 findings, 8 false

| position | function | matched | verdict |
|---|---|---|---|
| `analyzer/compare.go:166` | `printCallPath` | `text/tabwriter.Writer.Flush()` as a statement | false: the writer is `os.Stdout` (`compare.go:154`), and every write of this output, the `fmt.Fprint` into the same writer (`:162`, `:164`) and the `fmt.Printf` of its callers, goes unchecked by the same choice. The check exempts `fmt` printing, but it cannot tell a tabwriter over standard output from one over a file. |
| `cmd/capslock-git-diff/main.go:309` | `sortAndPrintCapabilities` | `text/tabwriter.Writer.Flush()` | false: same as above, the writer is `os.Stdout` (`main.go:281`). |
| `cmd/capslock-git-diff/main.go:455` | `printCallPath` | `text/tabwriter.Writer.Flush()` | false: same as above (`main.go:443`). |
| `cmd/capslock-git-diff/main.go:469` | `listCommits` | `run(&b, "git", "log", …)` as a statement | false: `run` reports its own failure: on error it logs the command and writes the command's stderr (`main.go:74-79`). The commit list is informational, printed after both revisions were already resolved by `AnalyzeAtRevision` (`main.go:206`, `:211`). |
| `testpkgs/callos/callos.go:33` | `Baz` | `u, _ := user.Current()` | false: `testpkgs` are inputs of capslock's own tests (`testing/analyzepackages_test.go:90` runs capslock on `../testpkgs/...`). The function exists to call `os/user.Current`, and its doc says so. |
| `testpkgs/callos/callos.go:39` | `Setenv` | `os.Setenv("A", "B")` | false: test input, "a test function which calls os.Setenv". |
| `testpkgs/importname/sort/notsort.go:14` | `Sort` | `os.ReadFile("/tmp/zzzzzz")` | false: test input, "Sort calls os.ReadFile", in a package named `sort` to test import names. |
| `testpkgs/usegenerics/usegenerics.go:23` | `a.Baz` | `is, _ := net.Interfaces()` | false: test input that gives a generic function's type argument the NETWORK capability. |

Round 2 gives the same 8. Its narrowing skips writes to a writer that cannot fail (`never-fail`:
`strings.Builder`, `bytes.Buffer`, the `hash` interfaces), also through a parameter every caller
fills with one; none of the 8 writes to such a writer.

More than half are false, and no narrowing that reads only `cir/1` separates these from a dropped
error that matters. Telling standard output from a file needs the writer's value, and telling a test
input from a library needs intent. The check is `experimental`.

## `unreached`

### Run 1: 114 findings, 114 false

The entries were both commands' `main.main` and every `init` of the set.

- 80 findings were in 14 `testpkgs` packages: `buildtags` 1, `callnet` 1, `callos` 4,
  `callruntime` 2, `callutf8` 1, `importname` 4, `importname/sort` 1, `indirectcalls` 10,
  `rewrites` 1, `useasm` 2, `usegenerics` 6, `uselinkname` 3, `usereflect` 34, `useunsafe` 10.
  All false: no command imports these packages. They are inputs of capslock's tests
  (`testing/analyzepackages_test.go:90`), read because `go list -deps ./...` lists every package
  of the repository.
- 34 findings were in `proto`: the 34 of run 3 below.

**Narrowed:** a module none of whose functions is reached is not part of the program and is not
reported. The rerun gave 44 findings. The 10 left were `testpkgs/indirectcalls`, which has an `init`
(`indirectcalls.go:55`). Every `init` of the set is an entry, so that `init` is reached, but no
command imports the package.

**Narrowed again:** a module none of whose functions but its `init` is reached is not reported. The
rerun gave the 34 below.

### Run 3: 34 findings, 34 false

All 34 are in `proto/capability.pb.go`, which protoc-gen-go generated from `capability.proto`
("Code generated by protoc-gen-go. DO NOT EDIT.", line 7). Reading the source confirms that no
command calls them: where capslock calls a method of the same name, its receiver is another type.
`analyzer/load.go:140` calls `PackageInfo.GetPath`. `analyzer/compare.go:76` and
`cmd/capslock-git-diff/main.go:246` call `CapabilityInfo.GetCapabilityName`.
`analyzer/util.go:630` calls `Capability.Enum`. Every `.Type()` call is on a `go/types` or
`reflect` value. The statement holds for every finding. All 34 are false because the code is meant
to be this way: they are the generated message API (getters, the `ProtoMessage` marker of
`protoiface.MessageV1`, the `protoreflect.Enum` methods) that the package offers its users. It is
regenerated from the `.proto` file, not edited.

| position | function | kind |
|---|---|---|
| `proto/capability.pb.go:102` | `Capability.Type` | `protoreflect.Enum` method |
| `proto/capability.pb.go:106` | `Capability.Number` | `protoreflect.Enum` method |
| `proto/capability.pb.go:148` | `CapabilityType.Enum` | enum helper |
| `proto/capability.pb.go:162` | `CapabilityType.Type` | `protoreflect.Enum` method |
| `proto/capability.pb.go:166` | `CapabilityType.Number` | `protoreflect.Enum` method |
| `proto/capability.pb.go:220` | `CapabilityInfo.ProtoMessage` | marker |
| `proto/capability.pb.go:239` | `CapabilityInfo.GetPackageName` | getter |
| `proto/capability.pb.go:260` | `CapabilityInfo.GetDepPath` | getter |
| `proto/capability.pb.go:267` | `CapabilityInfo.GetPath` | getter |
| `proto/capability.pb.go:281` | `CapabilityInfo.GetCapabilityType` | getter |
| `proto/capability.pb.go:311` | `Function.ProtoMessage` | marker |
| `proto/capability.pb.go:337` | `Function.GetSite` | getter |
| `proto/capability.pb.go:373` | `ModuleInfo.ProtoMessage` | marker |
| `proto/capability.pb.go:392` | `ModuleInfo.GetPath` | getter |
| `proto/capability.pb.go:399` | `ModuleInfo.GetVersion` | getter |
| `proto/capability.pb.go:432` | `PackageInfo.ProtoMessage` | marker |
| `proto/capability.pb.go:458` | `PackageInfo.GetIgnoredFiles` | getter |
| `proto/capability.pb.go:489` | `CapabilityInfoList.ProtoMessage` | marker |
| `proto/capability.pb.go:515` | `CapabilityInfoList.GetModuleInfo` | getter |
| `proto/capability.pb.go:522` | `CapabilityInfoList.GetPackageInfo` | getter |
| `proto/capability.pb.go:552` | `CapabilityCountList.ProtoMessage` | marker |
| `proto/capability.pb.go:571` | `CapabilityCountList.GetCapabilityCounts` | getter |
| `proto/capability.pb.go:578` | `CapabilityCountList.GetModuleInfo` | getter |
| `proto/capability.pb.go:612` | `CapabilityStats.ProtoMessage` | marker |
| `proto/capability.pb.go:638` | `CapabilityStats.GetCapabilityName` | getter |
| `proto/capability.pb.go:645` | `CapabilityStats.GetDescription` | getter |
| `proto/capability.pb.go:652` | `CapabilityStats.GetDirectCount` | getter |
| `proto/capability.pb.go:659` | `CapabilityStats.GetTransitiveCount` | getter |
| `proto/capability.pb.go:666` | `CapabilityStats.GetExampleCallpath` | getter |
| `proto/capability.pb.go:673` | `CapabilityStats.GetCount` | getter |
| `proto/capability.pb.go:702` | `CapabilityStatList.ProtoMessage` | marker |
| `proto/capability.pb.go:721` | `CapabilityStatList.GetCapabilityStats` | getter |
| `proto/capability.pb.go:728` | `CapabilityStatList.GetModuleInfo` | getter |
| `proto/capability.pb.go:758` | `Function_Site.ProtoMessage` | marker |

`cir/1` does not say that a file is generated: the Go reader drops the `// Code generated … DO NOT
EDIT.` comment. A library's API is named as entries with `--exported` (here
`--exported github.com/google/capslock/proto`).

**Narrowed in round 2:** a function in a file whose name is in the `generated` list (`*.pb.go`,
`*.pb.gw.go`, `*_gen.go`, `*.gen.go`, `*_generated.go`, `zz_generated*`) is not reported. The file
name stands in for the missing mark; a generated file named otherwise is still reported. Rerun: 0.
The check stays `experimental`.

## Outside capslock

Over the whole set (no `--report`), round 1 gives: `unbounded-growth` 0, `insecure-random` 0,
`int-from-text` 0, `case-fold-asymmetry` 0, `clone-aliasing` 0, `dropped-error` 105,
`lock-without-unlock` 1, `unreached` 5409 (the standard library and dependency functions capslock
does not use). These are not triaged.

Round 2 gives: `unsigned-below-zero` 2 (`math/big.RoundingMode.String`,
`runtime.waitReason.String`), `dropped-error` 105, `quadratic-insert` 1 (`go/types.NewMethodSet`),
`lock-without-unlock` 0, `unreached` 5303, every other check 0. These are not triaged.

Two checks were narrowed on what they first reported here. Their capslock count was 0 before and
after.

- `unbounded-growth` first gave 21 findings over the whole set, every one false. 3 matched `Recv`
  on `go/types` accessors (`chautil.LazyCallees`, `ssa.Program.MethodValue`,
  `ssa.canonizer.Type`) followed by a memo `typeutil.Map.Set`. 2 matched `io.Reader.Read` followed
  by `sync.Pool.Put` (`drbg.Read`, `io.discard.ReadFrom`). 15 matched a protobuf text or JSON
  decoder's token `Read` followed by a `Set` on the message being decoded. 1 matched the memory
  sanitizer hook `msan.Read` in `runtime_mapassign`. **Narrowed:** `receive` names receives from
  the network (`net.PacketConn.ReadFrom`, `net.Listener.Accept` and the like); a function of the
  set named `Read…`, `Accept…` or `Recv…` that calls one is a receive too; `sync.Pool` is in
  `not-containers`. Rerun: 0.
- `lock-without-unlock` first gave 8 findings, every one false. 6 are functions whose purpose is to
  return with the lock held: `pprof.lockProfiles`, `sync.RWMutex.Lock`, `sync.rlocker.Lock` and
  `syscall.acquireForkLock` (3). 1 locks and unlocks under the same condition `b == 10`
  (`math/big.divisors`, `natconv.go:511`, `:544`). 1 is hand-over-hand locking whose caller holds
  the first lock (`go/constant.stringVal.appendReverse`, `value.go:175`). **Narrowed:** a function
  with `lock`, `rlock` or `acquire` among its words is not checked; a lock taken in one branch of an
  `if` is not held in the other branch of a later `if` on the same condition; locks are matched by
  kind, not by receiver. Rerun: 1 (`appendReverse`, still false). **Round 2:** a lock a function
releases before it takes it again is owed to its caller, not held; a function whose body the reader
left opaque is not checked; an unlock reached through a callee or a `defer` counts. Rerun: 0.
