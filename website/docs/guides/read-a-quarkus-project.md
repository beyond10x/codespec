---
title: Read a Quarkus project
sidebar_position: 4
description: Read a Java Maven project, a Quarkus 3 application among them, into cir/1 with its class path, and run codespec effects over it.
lede: One class path variable and one summary take a Quarkus project to a report that says which resource method touches the database and which calls another service.
source: adapters/java/src/codespec/read/Reader.java, adapters/java/src/codespec/read/Main.java, adapters/java/testdata/quarkus, Taskfile.yml; every command run with the repository's build on 2026-11-09, JDK 21, Maven 3.9 offline, Quarkus 3.27.0
---

# Read a Quarkus project

`codespec read --lang java` reads a package directory and a project directory. A project directory
holds a `pom.xml`, a `build.gradle` or a `build.gradle.kts`. The reader reads every package under
`src/main/java` in one analysis and makes one `cir/1` module per package. It does not run Maven or
Gradle and does not read `src/test/java`.

## 1. The class path

The project's jars come from `CODESPEC_JAVA_CLASSPATH`, jar files separated by `:`. Maven writes
that value:

```console
$ mvn -o -q dependency:build-classpath -Dmdep.outputFile=classpath.txt
```

Without the jars the read stops at the first type it cannot find and names the variable.

## 2. Read one package

`codespec read` writes one module. A project of several packages needs the one to read in
`CODESPEC_JAVA_PACKAGE`. Without it the read fails and lists the packages.

```console
$ export CODESPEC_JAVA_CLASSPATH="$(cat classpath.txt)"
$ CODESPEC_JAVA_PACKAGE=org.acme codespec read --lang java fruits --out fruits.read.cir.json
```

`codespec-read-java --packages fruits` lists the packages. `codespec-read-java --all fruits out`
writes every module to `out/<package>.cir.json`.

## 3. What a call into a jar becomes

A call into a jar is an extern with its declared signature. A static method is an extern of the
class. An instance method is an extern method of its type. Two overloads of one name are refused,
and so is a signature with a generic type, which is erased to the bound.

## 4. The Quarkus 3 mapping

| In the source | In the module |
|---|---|
| `@Path` class, method with `@GET`, `@POST`, `@PUT`, `@DELETE` or `@PATCH` | an `entries` item of kind `http` with the method, the joined path and its media types |
| `@Scheduled` method | an `entries` item of kind `scheduled` |
| `@Incoming` method | an `entries` item of kind `incoming` |
| `@ApplicationScoped`, `@Singleton`, `@RequestScoped`, `@Dependent` class | a `beans` item with its scope |
| `@Inject` field | an `injects` item, with the bean when exactly one class of the project is that type |
| `@RegisterRestClient` interface | externs of effect `net` at each call, the interface is not read |
| Panache, `EntityManager`, `Query` and JDBC calls | externs of effect `db` |

An `@Inject` type with no bean, or with several, is an extern. Anything else the reader cannot map
stays `foreign` with one refusal. `@Entity` and the other persistence annotations are refusals: the class
or method is still read. `@Transactional` is recorded in `transactional`, the keys of the functions that
run in a transaction. `@Produces` and `@Consumes` are recorded as `produces` and `consumes` on the
`http` entry.

A `throw` of an exception outside the package's checked-exception hierarchy, such as a
`WebApplicationException` from a jar, is refused and kept as `foreign`. `cir/1` has no statement
that leaves a function by an exception no caller declares, and an error result would change the
signature that the framework calls.

## 5. Effects

`codespec effects` takes effects from summaries. The reader prints the one for the project:

```console
$ codespec-read-java --effects fruits > fruits.effects.json
$ codespec effects fruits.read.cir.json --summaries fruits.effects.json
```

In the repository's fixture `FruitResource.create` persists and reports `db`. `FruitResource.origin`
calls a REST client and reports `net`. `Cleanup.sweep` deletes and reports `db`.

## 6. Try it on the fixture

`task java-quarkus-prepare` builds `adapters/java/testdata/quarkus` and writes its class path.
`task java-quarkus-check` reads it twice, byte-equal, against
`adapters/java/testdata/quarkus.read.cir.json`, and runs `codespec effects` over it.
