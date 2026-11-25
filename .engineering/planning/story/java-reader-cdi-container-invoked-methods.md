---
format: aep.planning-md/3
id: story:java-reader-cdi-container-invoked-methods
kind: story
status: draft
title: Methods the CDI container calls, and calls through injected fields, are roots and edges of reach
summary: 'Production code of a Quarkus service reads as unreached (5 entries reach 8 of 1294 functions): producers, interceptors, messaging consumers and injected-interface calls need a root or an edge each.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-scheduled-startup-entries
- depends_on: story:java-reader-cross-package-interface-calls
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-cdi-roots
- confidence: inferred
  path: crates/cli/tests/reach.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 3
---
# Story: methods the CDI container calls, and calls through injected fields, are roots and edges of reach

## Outcome

On a Quarkus service, `codespec reach` from the recorded entries reaches the production code the
container calls. Each container-invoked construct is one of two things, and the story says which
per construct and gives each a reproducer:

- a root (an entry of `Module.entries`, kind in `ENTRY_KINDS`): a `@Produces` method of a bean
  (`producer`, `on` the produced type), an `@AroundInvoke` (or `@AroundConstruct`, `@PostConstruct`,
  `@PreDestroy` of a bean not annotated `@Startup`) method of a class annotated `@Interceptor`
  (`interceptor`, `on` the binding), and a method annotated `@Incoming` (`messaging`, `on` the
  channel);
- an edge (no entry): a call through an `@Inject` field, constructor parameter or `Instance<T>` of
  an interface or an abstract class is a call that reaches every bean of the read packages that has
  the type, in the way `reach` resolves an interface call today; a bean with a qualifier is chosen
  by it, an ambiguous or an unsatisfied injection point is a refusal at its line naming the type
  and the beans.

`@Observes` is `story:java-reader-cdi-observes-entry`; `@Scheduled` and `@Startup` are
`story:java-reader-scheduled-startup-entries`; an injection point with one bean of another package
is `story:java-reader-cross-package-interface-calls`. This story does not redo them: it covers what
they leave.

## Why

A release review of a Quarkus service ran `codespec reach` over the read modules: 5 entries reached
8 of 1294 functions. The rest, the production code, read as not reached, so the reach delta of a
revision named nothing and `codespec check unreached` listed nearly every function. Entries are the
resource methods only; the container starts everything else.

## Reproducer

Package `org.example.app`, written for this story:

```java
public interface Notifier { String send(String to); }

@ApplicationScoped public class MailNotifier implements Notifier {
    public String send(String to) { return "mail " + to; }
}
@ApplicationScoped public class SmsNotifier implements Notifier {
    public String send(String to) { return "sms " + to; }
}

@ApplicationScoped public class Config {
    @Produces @ApplicationScoped Clock clock() { return Clock.systemUTC(); }
}

@Interceptor @Audited @Priority(1)
public class AuditInterceptor {
    @AroundInvoke Object around(InvocationContext ctx) throws Exception { return ctx.proceed(); }
}

@ApplicationScoped public class Orders {
    @Incoming("orders") public void consume(String order) { handled = handled + 1; }
    int handled;
}

@Path("/n") public class NotifyResource {
    @Inject Notifier notifier;
    @GET public String get() { return notifier.send("x"); }
}
```

## Observed

Not run. The measurement above (5 entries, 8 of 1294 functions) is from a private service and is
not reproduced here; the first test reproduces its shape on this fixture.

## Done When

- A failing `ReaderTest` case first on a new fixture `adapters/java/testdata/project-cdi-roots`
  with the sources above: the read records an entry for `Config.clock`, `AuditInterceptor.around`
  and `Orders.consume`, each of its own kind, and none of the three annotations is refused.
- A failing `crates/cli/tests/reach.rs` case: `codespec reach` over the fixture's documents with
  the recorded entries reaches `NotifyResource.get`, both `send` methods (the interface call has
  two beans) and the three roots above; the unreached list holds no function of the fixture.
- Near misses, each its own case: an interface with a qualifier (`@Inject @Named("sms") Notifier`)
  reaches only `SmsNotifier.send`; an injection point of a type no bean has is a refusal at its
  line naming the type; a method of a class that is not a bean with `@Produces` is not an entry.
- `crates/model` test: a document with each new kind validates; an unknown kind is still refused.
- `adapters/java/README.md`, the schema and `docs/` name each construct as a root, an edge or
  refused.
- `task adapters-check` and the model tests green; each test under 60 s.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-cdi-roots`, `adapters/java/README.md`, `crates/model/src/lib.rs`
  (`ENTRY_KINDS`), `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `crates/cli/tests/reach.rs`. Inferred.
- After `story:java-reader-scheduled-startup-entries` and `story:java-reader-cross-package-interface-calls`
  (same `ENTRY_KINDS`, schema and Reader.java chain).
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
