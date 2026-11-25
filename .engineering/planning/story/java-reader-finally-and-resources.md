---
format: aep.planning-md/3
id: story:java-reader-finally-and-resources
kind: story
status: draft
title: finally, try-with-resources and a broad catch
summary: Read try/finally, try-with-resources (reverse close, suppressed exceptions) and catch of Exception or RuntimeException; Throwable refused by name.
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-try-catch
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
revision: 3
---
# Story: finally, try-with-resources and a broad catch

## Outcome

The Java reader reads three try forms that `story:java-reader-try-catch` leaves refused, under the
same condition it sets: every call in the block that can throw is one the module knows raises
what it raises.

- `try { B } finally { F }`: `F` runs on every way out of `B`, in Java's order: before a `return`
  (after the returned value is computed), before a `break` or `continue` that leaves the block,
  after `B` falls off its end, and before an exception of `B` goes on to the caller. A `return`,
  `break`, `continue` or `throw` inside `F` itself stays refused, with the reason "a jump out of
  finally discards what the try block was doing".
- try-with-resources: each resource is opened in source order and closed in reverse order on
  every way out of the body, a `null` resource is not closed, and a resource whose initializer
  raises is not closed while the ones before it are. Suppressed exceptions as Java defines them:
  when the body raises `E` and a `close` raises `E2`, `E` goes on and `E2` is not raised; when the
  body completes and a `close` raises, that exception goes on after the remaining resources are
  closed, and a later `close`'s exception is suppressed in turn. A call of `getSuppressed()` stays
  refused, with the reason "suppressed exceptions are not held in cir/1".
- A broad catch, `catch (Exception e)` and `catch (RuntimeException e)`: read as the catch of
  every exception the block's calls are known to raise that is a subtype of the caught type.
  `catch (Throwable e)` and `catch (Error e)` stay refused, with the reason "a catch of Throwable
  or Error catches errors of the virtual machine, which cir/1 does not model": a stated narrowing.

A block with a call whose exceptions the module cannot list stays refused, with the reason
`story:java-reader-try-catch` gives it. No new `cir/1` construct: the reader writes the error
checks and the duplicated `finally` and `close` statements.

## Why

codesec's Java rules (resource release, swallowed exceptions) need these forms read; codesec names
the gap `needed-from-codespec-java-try-forms`. `story:java-reader-try-catch` covers none of them,
and a Java service closes its JDBC connections, streams and locks with exactly these forms.

## Reproducer

```java
public class TryForms {
    static final class Res implements AutoCloseable {
        final StringBuilder log; final String name;
        Res(StringBuilder log, String name) { this.log = log; this.name = name; log.append("open " + name + ";"); }
        @Override public void close() { log.append("close " + name + ";"); }
    }
    public static String twoResources() {
        StringBuilder log = new StringBuilder();
        try (Res a = new Res(log, "a"); Res b = new Res(log, "b")) { log.append("body;"); }
        return log.toString();
    }
    public static int withFinally(String s) {
        int n = 0;
        try { n = Integer.parseInt(s); } finally { n = n + 1; }
        return n;
    }
    public static int broad(String s) {
        try { return Integer.parseInt(s); }
        catch (Exception e) { return -1; }
    }
}
```

## Observed

Reader at main `5f82fff97` (unchanged at the planning branch's head), `codespec-read-java --all`
over a project holding the reproducer:

```text
TryForms.java:13 try: catching exceptions is outside cir/1
TryForms.java:21 try: catching exceptions is outside cir/1
TryForms.java:26 try: catching exceptions is outside cir/1
```

## Done When

- Failing `ReaderTest` cases first, on the reproducer added to
  `adapters/java/testdata/project-forms`: `twoResources`, `withFinally` and `broad` read with no
  refusal, and the evaluator gives what Java prints: `open a;open b;body;close b;close a;` for
  `twoResources`; for `withFinally` and `broad`, the values for `"12"`, `"x"` and `""`.
- try-with-resources cases: the body raises and a `close` raises (the body's exception goes on, the
  later resources are still closed); the body completes and the first of two `close` calls
  raises (that exception goes on, the second resource is closed); the body completes and two of
  three `close` calls raise (the first raised goes on, the later one is suppressed in turn: not
  raised, and the remaining resource is still closed); a `null` resource is not closed; the second
  resource's initializer raises (only the first is closed). Each compares the evaluator's log
  and result with what Java prints.
- `finally` cases: a `return` in the block runs `F` after the value is computed (`F` changes the
  variable, the returned value is the old one); a `break` out of a loop runs `F`; a `continue` in
  a loop runs `F` before the next iteration (the log shows `F` once per iteration). Each compares
  with what Java prints.
- `finally` refusals, one case each, each with the reason "a jump out of finally discards what the
  try block was doing": a `return` in `F`, a `break` in `F`, a `continue` in `F`, a `throw` in `F`.
- Broad catch: `catch (RuntimeException e)` around a known raising call reads (the second case of
  `story:java-reader-try-catch` turns from a refusal into a read case);
  `catch (Throwable e)`, `catch (Error e)` and a broad catch around a call whose exceptions the
  module cannot list stay refusals with their named reasons; a `getSuppressed()` call stays a
  refusal.
- README row per form; `task adapters-check` green; each new test inside the 60 s bound.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred; recorded as typed
  scope.
- In the Reader.java chain after `story:java-reader-try-catch` (a `depends_on` edge): shares
  `adapters/java/src/codespec/read/Reader.java`, `ReaderTest.java`, `testdata/project-forms`.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
