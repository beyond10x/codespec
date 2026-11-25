---
format: aep.planning-md/3
id: story:java-reader-null-results
kind: story
status: draft
title: A null result of a collection or string method reads as a nil pointer
summary: 'Refused 5 times as null where a map is expected: a result and its null test follow the nullable-field rule.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-static-jdk-calls
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
revision: 2
---
# Story: a null result of a collection or string method

## Outcome

A project read reads `return null` in a method whose result is a `List`, a `Map`, a `String` or a
box, and a comparison of such a result with `null`, by the rule `story:nullable-reference-fields`
sets for a field: the method's result is `ptr(T)` with `nil` for `null`, a comparison with `null` is
a comparison with `nil`, and a use of the result that would print, unbox, compare with an operator,
hand on or write it is refused at the use, as the README's Assumptions say for a nullable field. A
caller that tests the result against `null` and returns or throws before using it (the shape
`lookup_else` already reads for `Map.get`) reads in full.

Stays refused, with its reason named: `null` as an argument of a jar method (Java hands the null to
code the module cannot see), and the use of a nullable result as the README's Assumptions list them.

## Why

The consumer of a Java service counts 5 refusals "null where a `{ "k": … }` is expected" (codespec
0.12.0): a lookup method returns `null` for "not found" and its callers test it. A `cir/1` map or
slice has a nil value that reads as empty, where Java's `null` fails at the first call; the field rule
already keeps that difference for a field, and a result of the same type has no rule, so the method
and every caller that tests its result are refused.

## Reproducer

`org/example/api/Registry.java`:

```java
package org.example.api;

import java.util.Map;

public class Registry {
    public Map<String, Integer> find(String key) { return null; }
    public int count(String key) {
        Map<String, Integer> found = find(key);
        if (found == null) { return 0; }
        return found.size();
    }
}
```

## Observed

Reader built from the tree of branch `plan/java-consumer-stories` at `2f65bb14` (javac `--release 17`,
JDK 21), `codespec.read.Main --all` on a project with a `pom.xml` holding this source:

```text
Registry.java:6 null where a { "k": "map", "key": { "k": "string" }, "elem": { "k": "int", "width": "32", "signed": true } } is expected
Registry.java:9 null where a { "k": "map", "key": { "k": "string" }, "elem": { "k": "int", "width": "32", "signed": true } } is expected
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `find` of the
  reproducer reads with result `ptr(map)` and body `return nil`, and `count` reads, its `found ==
  null` a comparison with `nil` and `found.size()` after the early return a use as a receiver.
- The near miss, a second case: a caller that prints the result (`"" + find(k)`) or passes it to a
  method of the project that does is refused at that use, with the nullable-field reason.
- A third case: `null` as an argument of a jar method is refused at its line.
- The evaluator and the Go writer run the case (`task adapters-check`), the `calls.json` calling
  `count` for an absent key and giving 0.
- README table row and the Assumptions paragraph; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-static-jdk-calls`, the next of the Reader.java chain.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
