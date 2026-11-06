# codespec-read-java

Reads one Java package directory with javac's attributed trees (`JavacTask.analyze()`) and prints
it as one `cir/1` module. The program is `codespec.read.Main` under `src/`, on the JDK only; what
it maps and what it refuses is the class comment of `src/codespec/read/Reader.java`. Its JVM type
table (primitives, boxes, `String`, lists and maps) is `codespec.jvm.JvmTypes` in
[`../jvm/src`](../jvm/src/codespec/jvm/JvmTypes.java), shared with the other JVM readers and compiled
with this one.

```console
task adapters        # compiles src/ into target/java/classes and writes target/adapters/codespec-read-java
task adapters-check  # compiles src/ and test/, runs codespec.read.ReaderTest, reads testdata/ledger twice
```

`testdata/ledger` is the package every construct of the reader's subset is read from with no
refusal; `testdata/refuse`, `testdata/adversary` and `testdata/adversary2` hold one package per
construct the reader keeps as `foreign` or refuses. `testdata/nullable` holds fields that are null
until assigned; `task adapters-check` reads it, writes it as Go and as Rust, and runs both and the
Java original (`driver/Main.java`) on its `calls.json`, byte-equal.

## The claim and the conformance kit

`codespec-read-java --describe` claims the features its `examples/ledger` output uses and the
subject of every kit case under `../../conformance/cases` that holds `java/Case.java`: the case in
Java, package `<case>` with `-` read as `_`, `long` for Go's `int`. `ReaderTest` reads each through
`codespec read --lang java` with no refusal and no `foreign`, passes it, writes it as Go and runs it
with `codespec-run-go` on the case's `calls.json`, and requires the case's `expected.txt`; `task
adapters-check` then requires each module to use its case's subject and every claimed feature to be
used by the ledger or a kit module.

The kit cases without a Java source, and why:

| case | subject | why |
|---|---|---|
| `assign-and-not`, `op-and-not` | `assign.and_not`, `op.and_not` | Java has no `&^`; `a & ~b` is `op.and` and `unop.complement` |
| `assign-shl`, `assign-shr`, `op-shl`, `op-shr` | `assign.shl`, `assign.shr`, `op.shl`, `op.shr` | refused by design: Java masks the shift count, cir/1 does not say so |
| `decl-enum` | `decl.enum` (claimed through the ledger) | an enum is read over `string`, its constants' names; the case prints an enum over `int` |
| `decl-newtype` | `decl.newtype` | Java has no defined type over a primitive or `String`; a class is a struct |
| `expr-intrinsic` | `expr.intrinsic` (claimed through the ledger) | the case takes `len` of a string; `String.length()` counts UTF-16 units, `len` bytes, and is not read |
| `expr-nil` | `expr.nil` (claimed through the ledger) | the case returns a nil slice; `null` where a `List` is expected is refused, since a null `List` is no empty one |
| `expr-zero` | `expr.zero` (claimed through the ledger) | the case's error message carries the value (`errorf`); a Java error is a sentinel with a literal message, and a Java `""` is read as a literal |
| `expr-to-error`, `intrinsic-errorf` | `expr.to_error`, `intrinsic.errorf` | refused by design: an error is a sentinel, an exception leaf with a literal message and no fields |
| `fact-*` | `fact.*` | facts are computed by `codespec pass`, never claimed by a reader |
| `intrinsic-println`, `stmt-expr` | `intrinsic.println`, `stmt.expr` (claimed through the ledger) | the case prints; `System.out` is not in the reader's library table |
| `sem-multi-assign-order` | `sem.multi_assign.order` | Java has no assignment of several targets at once |
| `sem-switch-break` | `sem.switch.break` | refused by design: a `break` inside a `switch` is read only at the end of a case, where it leaves the switch; a labeled `break` is refused |
| `stmt-foreign` | `stmt.foreign` | a `foreign` statement holds Java source, which the Go writer cannot write |
| `stmt-lookup-else` | `stmt.lookup_else` | the reader never produces it: `Map.get` of a value that is not a reference is refused (an absent key gives null), and `containsKey` is not read |
| `stmt-return` | `stmt.return` (claimed through the ledger) | the case returns two values; a Java method returns one |
| `stmt-send`, `type-chan`, `unop-recv` | `stmt.send`, `type.chan`, `unop.recv` | Java has no channels |
| `stmt-var` | `stmt.var` | a Java local has no zero value: javac rejects reading one that was never assigned |
| `type-unit` | `type.unit` (claimed through the ledger) | the case prints; `System.out` is not in the reader's library table |
| `type-any` | `type.any` | `Object` is outside the JVM type table |
| `unop-deref` | `unop.deref` | Java has no pointer to a local; an object reference is dereferenced implicitly |

The ledger-only rows are `ReaderTest.LEDGER_ONLY`, which the test holds against the kit: a claimed
case subject needs a Java source or a row there.

## Assumptions

The document describes the Java program only under these assumptions; the reader does not check
them.

- Every reference parameter of a method is not null. A String or boxed parameter printed as it is,
  by string concatenation or `String.format`, in a method that never compares it with `null`, is
  refused, since Java would print `null`; every other use of a parameter takes it to hold an object.
- A reference field without an initializer that some constructor leaves unassigned holds null
  until assigned. It is a nil pointer, not a refusal: a pointer to its type (`ptr(string)`,
  `ptr(int32)`; a class of the package is a pointer already), whose zero value is `nil`, compared
  with `nil` where Java compares it with `null`. A use that would print `null` (concatenation,
  `String.format`), unbox or compare it (an operator, `+=`, `++`), hand it to a library method or
  write the collection it holds is refused at the use, since the written program stops at the nil
  pointer where Java goes on. Every other read of such a field takes it to hold an object, as a
  parameter does.
- The program runs under `Locale.ROOT`. `String.format("%d", n)` is read as `sprintf`, which prints
  ASCII digits; under a default locale whose digits are not ASCII, Java prints others.
