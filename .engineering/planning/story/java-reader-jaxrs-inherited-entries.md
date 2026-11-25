---
format: aep.planning-md/3
id: story:java-reader-jaxrs-inherited-entries
kind: story
status: implemented
title: A resource method gets its http entry from JAX-RS annotations on the interface it implements
summary: 'Misread: an implementing class of an annotated interface (generated sources or class path) gets no http entry and no refusal.'
relations:
- decomposes: epic:java-quarkus-reader
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-inherited-entries
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T01:45:25Z", actor: "human:operator", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-25T01:45:25Z", actor: "human:operator", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-25T01:45:26Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: a resource method gets its http entry from JAX-RS annotations on the interface it implements

## Outcome

A project read gives a method of a class an `http` entry when the class implements an interface
whose method it overrides carries `@GET`, `@POST`, `@PUT`, `@DELETE` or `@PATCH` (and `@Path` on
the interface or the method), whether the interface is a source of the project or a class on
`CODESPEC_JAVA_CLASSPATH` (an interface generated into `target/generated-sources` and compiled into
`target/classes`). JAX-RS applies an interface method's annotations to the implementing method that
declares none of its own. An interface whose annotations the reader cannot apply (the class method
has JAX-RS annotations of its own that differ, two interfaces give one method two verbs) is a
refusal at the class's line, never a module without the entry.

## Why

Misread, ranked first: wrong meaning without a refusal. Reported by the consumer of a Java service
(codespec 0.12.0): its resource classes implement interfaces an OpenAPI generator writes into
`target/generated-sources`; the reader reads `src/main/java` only, the modules have no `http`
entry and no refusal, and `codespec reach` has nothing to start from. Reproduced on main
`5f82fff97` with the reader built from the tree (javac `--release 17`, run on JDK 21).

## Reproducer

`gen/org/example/api/ItemsApi.java`, compiled with `javac` into a class directory put on
`CODESPEC_JAVA_CLASSPATH` beside the `jakarta.ws.rs-api` jar:

```java
@Path("/items")
public interface ItemsApi {
    @GET @Path("/{id}") String get(@PathParam("id") String id);
    @POST String create(String body);
}
```

`src/main/java/org/example/api/ItemsResource.java`:

```java
@ApplicationScoped
public class ItemsResource implements ItemsApi {
    @Override public String get(String id) { return "item " + id; }
    @Override public String create(String body) { return "created " + body; }
}
```

## Observed

`codespec-read-java --all <project> <out>`: exit 0. `entries` of `org.example.api` holds only the
methods of a class annotated directly (`DirectResource.get`, `DirectResource.load`); none for
`ItemsResource.get` or `ItemsResource.create`, and no refusal saying so. The two refusals the
module has for them follow from the missing entry:

```text
ItemsResource.java:9  parameter id printed where Java prints a null one as null, which a cir/1 string cannot hold; compare it with null first
ItemsResource.java:14 parameter body printed where Java prints a null one as null, which a cir/1 string cannot hold; compare it with null first
```

The same with `ItemsApi.java` copied into `src/main/java`: no entry, no refusal.
`codespec check unreached --entry org.example.api.DirectResource.load` then reports
`ItemsResource.get` and `ItemsResource.create` as unreached: dead code that JAX-RS calls.

## Done When

- A failing `ReaderTest` case first: a project fixture under `adapters/java/testdata/` with an
  annotated interface in `src/main/java` and one compiled onto the class path, each implemented by
  a class without annotations; the read has an `http` entry `GET /items/{id}` naming
  `ItemsResource.get` and `POST /items` naming `ItemsResource.create`, and the path parameter
  `id` reads without the null refusal. The class-path interface also declares a `@PUT`, a
  `@DELETE` and a `@PATCH` method, each implemented without annotations, and the read has the
  three `http` entries with those verbs.
- A class method with its own differing verb or path is a refusal at its line (a second case).
- A class implementing two annotated interfaces that give one method two verbs (`@GET` on one,
  `@POST` on the other) is a refusal at the class's line, and the method has no `http` entry (a
  third case).
- `adapters/java/README.md` "Projects and Quarkus" names the inherited form.
- `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java` (entries), `adapters/java/test/codespec/read/ReaderTest.java`,
  a fixture under `adapters/java/testdata/`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
