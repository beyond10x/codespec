---
format: aep.planning-md/3
id: story:java-reader-synchronized
kind: story
status: draft
title: synchronized methods and blocks, with the monitor recorded
summary: 'Misread: a synchronized method reads with no record of its lock; blocks are refused. Record the monitor and the statements it holds; cq holds_monitor reads them.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-finally-and-resources
- depends_on: story:cq-entry-relation
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: crates/query/src/facts.rs
- confidence: inferred
  path: crates/query/tests/expected/schema.txt
- confidence: inferred
  path: crates/query/tests/query.rs
- confidence: inferred
  path: docs/query-language.md
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 6
---
# Story: synchronized methods and blocks, with the monitor recorded

## Outcome

The Java reader reads a `synchronized` method and a `synchronized (m) { … }` block, and records
the lock as a fact: a new `cir/1` module list `monitors`, one record per synchronized method or
block, `{func, monitor, pos, held}`. `monitor` names the object locked: `this` for an instance
method, `<Type>.class` for a static one, and for a block `this`, a class literal, a field (`T.f`)
or a local variable or parameter by name. `held` lists the node ids of the statements read while
the monitor is held (each as cq spells a node, `<module path>.<function key>#<index path>`): every statement of a synchronized method's body, and the statements of a
block (nested ones included), none outside it. The method or block body reads as it does without
the lock. A block's statements are read in place in the enclosing body; a block that declares a
name the enclosing body uses after it or already holds stays refused, with the reason "a
synchronized block whose names leave it".

cq gets a relation `holds_monitor{func, monitor, node}`, derived from `monitors`: one row for each
node id in a record's `held`, so a lock rule can ask what runs under which lock (a blocking call,
a second monitor taken inside the first). cq computes no lock region of its own.

Stays refused, each with its reason: a monitor that is any other expression ("a monitor other
than this, a class literal, a field or a variable"); `Object.wait`, `notify` and `notifyAll` ("wait
and notify are outside cir/1"). A synchronized method or block changes nothing in what the
evaluator gives: it runs one thread.

## Why

Misread first. A `synchronized` method is read today with no refusal and no record of the lock,
so a rule about locks sees an unlocked method. A `synchronized` block is refused, which makes
the whole function unread. codesec's Java lock rules need both; codesec names the gap
`needed-from-codespec-java-synchronized`.

## Reproducer

```java
public class Counter {
    private final Object guard = new Object();
    private int n;
    public synchronized int next() { n = n + 1; return n; }
    public int add(int k) {
        synchronized (guard) { n = n + k; return n; }
    }
    public void await() throws InterruptedException {
        synchronized (guard) { guard.wait(); }
    }
}
```

## Observed

Reader at main `5f82fff97` (unchanged at the planning branch's head), `codespec-read-java --all`
over a project holding the reproducer: `next` is read with its two statements, no refusal and no
record of the lock (the misread); the two blocks are refused:

```text
Counter.java:10 synchronized: threads and locks are outside cir/1
Counter.java:14 synchronized: threads and locks are outside cir/1
```

## Done When

- Failing `ReaderTest` cases first, on the reproducer added to
  `adapters/java/testdata/project-forms`: `next` and `add` read with no refusal; `monitors` holds
  `{func: "Counter.next", monitor: "this"}` and `{func: "Counter.add", monitor: "Counter.guard"}`;
  `held` of `next` names its two statements and `held` of `add` the block's statements and none
  of `add`'s outside it; the evaluator gives `next` 1 then 2 and `add(5)` 5 on a fresh
  `Counter`, as Java does.
- A `static synchronized` method records `Counter.class`; a block on a local variable records the
  variable's name; a block on a parameter (`void with(Object lock) { synchronized (lock) { … } }`)
  records the parameter's name; a block on `this` records `this` (one case each).
- Near misses: a method without `synchronized` has no `monitors` record; `await` stays a refusal
  with the reason for `wait`; a block calling `guard.notify()` and one calling `guard.notifyAll()`
  each stay a refusal with the same reason (one case each); a block on a method call's result
  stays a refusal with the monitor reason; a block declaring a name used after it stays a refusal
  with its reason.
- `crates/model` validates `monitors` (the function exists, `monitor` is non-empty, every node id
  of `held` is a statement of that function), with tests in `crates/model/tests/model.rs`: a
  valid record, and a record whose `held` names a node of another function is refused;
  `schemas/cir-1.schema.json` regenerated.
- `crates/query`: a test over the reproducer's document gets one `holds_monitor` row per node of
  `held`: both statements of `next`, the block's statements of `add` and none of `add`'s outside
  it, and none for a method without a lock; a hand-edited document whose `held` lists one node
  gives exactly that row (the relation reads `held`, it does not compute a region);
  `crates/query/tests/expected/schema.txt` and `docs/query-language.md` § 4.2 name the relation.
- README row; `task adapters-check` and `task check` green; each new test inside the 60 s bound.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `crates/model/src/lib.rs`
  (`monitors`), `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `crates/query/src/facts.rs`, `crates/query/tests/query.rs`,
  `crates/query/tests/expected/schema.txt`, `docs/query-language.md`. Inferred; recorded as typed
  scope.
- In the Reader.java chain after `story:java-reader-finally-and-resources` (a `depends_on` edge):
  shares `adapters/java/src/codespec/read/Reader.java`, `ReaderTest.java`, `testdata/project-forms`.
- After `story:cq-entry-relation` (a `depends_on` edge): both add a relation to
  `crates/query/src/facts.rs`, `crates/query/tests/query.rs`, the expected schema and
  `docs/query-language.md`.
- `crates/model/src/lib.rs`, `crates/model/tests/model.rs` and the schema are changed by the Rust
  wave `wave/rust-misreads-1` too: this story starts after that wave merges, rebased on it.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
