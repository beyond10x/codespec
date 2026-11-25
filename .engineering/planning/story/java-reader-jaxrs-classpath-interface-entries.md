---
format: aep.planning-md/3
id: story:java-reader-jaxrs-classpath-interface-entries
kind: story
status: draft
title: A resource class gets its http entries from a jar-only JAX-RS interface, read from class-file annotations
summary: Jar-only interfaces, sub-interfaces and @Path on the type only give http entries; the inherited-entries story's fixtures cover sources and a class directory.
relations:
- decomposes: epic:java-quarkus-reader
- informed_by: story:java-reader-jaxrs-inherited-entries
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-jar-interfaces
revision: 3
---
# Story: a resource class gets its http entries from a jar-only JAX-RS interface, read from class-file annotations

## Outcome

A project read gives `http` entries to the methods of a class that implements a JAX-RS interface
that exists only in a jar (or class directory) on `CODESPEC_JAVA_CLASSPATH`, with no source of it
on the source path, reading the annotations from the interface's class file: `@Path` on the type,
a verb (`@GET`, `@POST`, `@PUT`, `@DELETE`, `@PATCH`) and an optional `@Path` on each method. The
forms covered beyond `story:java-reader-jaxrs-inherited-entries`:

- the interface is in a jar, not in a class directory;
- the class implements a sub-interface that extends the annotated jar interface: the methods and
  the type's `@Path` are inherited through it;
- `@Path` is on the type only and a method carries a verb only (path `/items`), or the method
  carries `@Path` and the type none.

A case the reader cannot apply stays a refusal at the class's line (the rules of the jaxrs story).

## Why

A release review of a Quarkus service: its resource classes implement interfaces an OpenAPI
generator wrote and compiled into a jar. The read gave no `http` entry for them, so reach had
nothing to start from. `story:java-reader-jaxrs-inherited-entries` (implemented) covers an interface
in the project's sources and one compiled onto the class path as a directory; the jar form and
the sub-interface form are not in its fixtures.

## Reproducer

`gen` sources, written for this story and compiled into `items-api.jar` (the test builds it with
the JDK's compiler and `jar`):

```java
package com.example.api;

@Path("/items")
public interface ItemsApi {
    @GET @Path("/{id}") String get(@PathParam("id") String id);
    @DELETE void remove();
}

public interface AdminItemsApi extends ItemsApi {
    @POST String create(String body);
}
```

`src/main/java/com/example/api/ItemsResource.java`:

```java
@ApplicationScoped
public class ItemsResource implements AdminItemsApi {
    @Override public String get(String id) { return "item " + id; }
    @Override public void remove() { }
    @Override public String create(String body) { return "created " + body; }
}
```

## Observed

Not run.

## Done When

- A failing `ReaderTest` case first, on a new fixture `adapters/java/testdata/project-jar-interfaces`
  with the jar above on the class path: the read has `http` entries `GET /items/{id}`,
  `DELETE /items` and `POST /items` naming the three methods, and `id` reads without the null
  refusal.
- A second case: `@Path` on the type only with a verb-only method gives `/items`; a method `@Path`
  with no type `@Path` gives that path.
- A near miss: a class implementing an interface of the jar with no JAX-RS annotation gets no
  entry and no refusal; a method with its own differing verb is a refusal at its line.
- The sources of the interfaces are not on the source path, and nothing is read from
  `target/generated-sources`.
- `adapters/java/README.md` "Projects and Quarkus" names the jar and sub-interface forms.
- `task adapters-check` green; each test under 60 s.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java` (entries), `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-jar-interfaces`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-jaxrs-inherited-entries`, whose entries code it extends.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
