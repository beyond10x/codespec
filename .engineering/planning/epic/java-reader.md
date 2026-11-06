---
format: aep.planning-md/3
id: epic:java-reader
kind: epic
status: implemented
title: A Java reader
summary: codespec-read-java on javac's attributed trees, and a Java example translated to Go and Rust under the harness.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T16:37:46Z", actor: "human:timo", revision: 4}
---
# Epic: A Java reader

## Outcome

`codespec read --lang java <source dir>` lifts a Java package into `cir/1` with a type on every
expression, and a hand-written Java example goes Java → `cir/1` → Go and Java → `cir/1` → Rust with
the harness byte-equal on 200 traces of 300 commands.

## Why Now

Requested by the operator on 2026-11-06, as the first step to reading legacy Grails 2.5.6
applications (`epic:groovy-grails-reader`): Groovy runs on the JVM, its classes are Java classes,
and a Grails service is a Java-shaped class with Groovy bodies. Java is typed, so a Java reader
settles the JVM type mapping before the dynamic part is attempted.

## Decision

Each language is read with the tooling that knows its types (`docs/design.md`). For Java that is
javac itself: `com.sun.source.util.JavacTask.analyze()` attributes every tree, and `Trees` gives
each expression's `TypeMirror`. No dependency beyond the JDK. The adapter is a Java program under
`adapters/java/`, built with `javac` into `target/adapters` (JDK 17 to run; it reads sources up to
Java 17, Java 8 included).

## Scope

- `codespec-read-java`: classes with fields and methods → structs and methods; `int`/`long` → `int`
  widths, `String`, `boolean`, `java.util.List`/`ArrayList` → slices, `Map`/`HashMap`/`TreeMap` →
  maps (with the iteration-order fact), checked exceptions of one closed hierarchy → sentinel
  errors and `(T, error)` results, `Optional` → comma-ok, enums → enums, statements and expressions
  of the `cir/1` subset, `String.format` → `sprintf`.
- Outside the subset, kept as `foreign` with a refusal: inheritance beyond one error hierarchy,
  interfaces with more than one implementation, generics declared by the code, lambdas and streams,
  reflection, annotations with behaviour, threads.
- A Java example with a Java driver in the harness.

## Out of Scope

- A Java writer.
- Groovy and Grails (`epic:groovy-grails-reader`).

## Risks

- Java `int` is 32 bits and overflows by wrapping; `long` is 64. `cir/1` has both widths; the
  example's trace crosses both edges.
- `HashMap` iteration order is unspecified, as Go's is; `TreeMap`'s is sorted. The facts pass must
  see the difference.

## Done When

`task examples-<java example>` is green in `task check`: the Java source read twice byte-equal with
0 refusals, written as Go and as Rust with 0 refusals, and the harness byte-equal on 200 × 300 for
the original Java, the Go output and the Rust output.
