---
format: aep.planning-md/3
id: story:reader-groovy-aborts-on-grails-2
kind: story
status: draft
title: The Groovy reader refuses instead of aborting on a Grails 2 application
revision: 3
---
# Story: the Groovy reader refuses instead of aborting on a Grails 2 application

## Outcome

`codespec-read-groovy` 0.8.0 exits 1 without a `cir/1` document on five conditions met while
reading a Grails 2.5.6 application (one tree with `grails-app/` and `src/{groovy,java}` of 13
in-place plugins, 278 domain and service files; JDK 21, groovy-all 2.4.10, the Grails 2.5.6
`lib` and `dist` jars and the application's dependencies on `CODESPEC_GROOVY_CLASSPATH`). After
this story each of them is either read or a refusal at a line, and the read writes its document.

## Reported conditions

Reported by another session on 2026-11-11; the application is not named. Only item 1 was checked
against the source in this repository; items 2 to 5 are as reported.

1. **No workaround.** `NoClassDefFoundError` for a class on the classpath
   (`org.springframework.web.filter.OncePerRequestFilter`), thrown from
   `JavaSources$Defined.findClass` (`adapters/groovy/src/codespec/read/groovy/JavaSources.java:120`).
   `Reader.java:375` calls `JavaSources.compile(app.java, classpath, new
   Hidden(Reader.class.getClassLoader()))`: the loader that defines the compiled `src/java` classes
   has `Hidden` as its parent and does not see the jars of `CODESPEC_GROOVY_CLASSPATH`. A
   `src/java` class that extends a classpath class fails at `defineClass`. Direction: a class
   loader over `classpath` between `Hidden` and `Defined`.
2. `NoClassDefFoundError: org/apache/commons/logging/Log` while resolving `grails.util.Holders`
   with only the six jars the adapter's README lists.
3. `ClassFormatError: Absent Code attribute` for `javax/persistence/EnumType` from a stub jar
   that ships in Grails' own `lib/` (`javaee-web-api-6.0.jar`).
4. `unable to find class ... for annotation attribute constant` when the jar holding an annotation
   constant's class is absent or a newer version lacks the class.
5. Reading one plugin directory alone: a file left out for an unresolved import leaves a
   reference in a file that is read (`Apparent variable '<name>' was found in a static scope`),
   and the compile error ends the read.

## Why

`adapters/groovy/README.md` says a file whose imports or classes do not resolve is "refused as a
whole" (lines 63, 190) and a resolve-only or Java file that does not resolve is "left out" (lines
50 to 56). Items 2 to 5 end the read instead.

## Acceptance

- One test per item, each with a made-up source tree and jars the test builds, that fails on 0.8.0
  with exit 1 and no document.
- Item 1: a `src/java` class extending a class from a classpath jar is compiled, defined and
  visible to the Groovy files.
- Items 2 to 5: the read exits 0 or with the refusal exit, writes its document, and names the
  refused file and line; no `Error` or compiler failure escapes `Reader`.
- `task check` exits 0.

## Evidence

Two passes, `c5c548e9` and `7d116fa4`, on `story/groovy-reader-aborts`.

In this repository, on `7d116fa4`: `ReaderTest` 80 passed, 0 failed, 0 skipped, on made-up trees
and jars the test builds; `task check` exit 0 (136 cargo suites, 1011 passed, 0 failed).

On the Grails 2.5.6 application the conditions were reported from, run by the reporting session on
2026-11-11 with an adapter built from `7d116fa4`; as reported, not reproduced here, the logs are
not in this repository:

- `c5c548e9`: the class-loader condition (item 1) was gone; the other four ended in new aborts
  (a logging class that could not be loaded, a compile error after a trait's file was left out, an
  exception thrown by a local AST transformation of a jar, a Java EE package the JDK no longer
  ships).
- `7d116fa4`: 6 of 6 reads exit 0 and write a document: the merged tree with the full classpath,
  with only `grails-core` and `spring-tx`, and as a copy with annotations stripped by hand; one
  plugin directory alone with the full classpath, with the six jars the README named, and with
  Grails' `lib/` including the stub jar.
- Refusals in those six reads: `unresolved` 33 to 248, `unsupported: annotation` 4 to 189,
  `unsupported: compile error`, `the compiler failed` and `no file can be blamed` 0 in all six.
- The merged tree as written and its hand-stripped copy give the same 203 functions; they differ
  by 9 `unsupported: annotation` refusals.
