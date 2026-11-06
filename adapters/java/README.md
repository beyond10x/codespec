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
construct the reader keeps as `foreign` or refuses.

## Assumptions

The document describes the Java program only under these assumptions; the reader does not check
them.

- Every reference parameter of a method is not null. A String or boxed parameter printed as it is,
  by string concatenation or `String.format`, in a method that never compares it with `null`, is
  refused, since Java would print `null`; every other use of a parameter takes it to hold an object.
- The program runs under `Locale.ROOT`. `String.format("%d", n)` is read as `sprintf`, which prints
  ASCII digits; under a default locale whose digits are not ASCII, Java prints others.
