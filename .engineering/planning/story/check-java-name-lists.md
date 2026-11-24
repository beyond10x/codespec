---
format: aep.planning-md/3
id: story:check-java-name-lists
kind: story
status: draft
title: Built-in checks carry Java name lists and say when a set has no name for its language
summary: 'Defect: 17 of 18 checks give 0 rows on Java documents silently; the logic works with Java names via --names.'
relations:
- decomposes: epic:codebase-transformations
revision: 2
---
# Story: built-in checks carry Java name lists and say when a set has no name for its language

## Outcome

`codespec check` has Java entries in the name lists of its checks, a first set: `java.util.concurrent`
locks (`ReentrantLock`, `ReentrantReadWriteLock` read and write locks, `StampedLock`) for
`lock-without-unlock`; `java.util.Random` and `ThreadLocalRandom` against `SecureRandom` for
`insecure-random`; JDBC (`Connection`, `Statement`, `ResultSet`) and `EntityManager` as resources
for `conn-leak`; and a check that `MessageDigest.isEqual` and not `equals` compares a value whose
name marks it a secret (`token`, `secret`, `password`, `mac`, `digest`). A check run over documents
of a language its lists name nothing of prints `<check>: not applicable to <language>` on standard error and reports the check as not applicable, never as 0 rows; the line names the check and the
language.

## Why

Defect (silent false negative). Reported by the consumer of a Java service: 17 of 18 built-in
checks give 0 rows on its Java documents; their name lists are Go standard-library names. The
check logic itself works on Java documents: with `--names` the rows appear. A Java
`ReentrantLock` is reentrant, so a relock in its own span is no self-deadlock: a Java list must
not carry it into a self-deadlock check.

## Reproducer

```java
public class Counter {
    private final ReentrantLock lock = new ReentrantLock();
    public int leaky(boolean early) {
        lock.lock();
        if (early) { return 0; }
        lock.unlock();
        return 1;
    }
    public static int token() { Random r = new Random(); return r.nextInt(); }
}
```

## Observed

On main `5f82fff97`, every check of `codespec check --list` over the Java documents of the
reproducer: exit 0, 0 rows (except `unreached`, exit 2: no `main.main`). With `--names` naming
`java.util.Random` for `random` and `java.util.concurrent.locks.ReentrantLock.ReentrantLock.lock`
/ `.unlock`:

```text
org.example.locks/Counter.java:33 org.example.locks.token insecure-random: calls java.util.Random.<init> at org.example.locks/Counter.java:33; reached from org.example.locks.token (its name has "token") at org.example.locks/Counter.java:32
org.example.locks/Counter.java:24 org.example.locks.Counter.leaky lock-without-unlock: takes java.util.concurrent.locks.ReentrantLock.ReentrantLock.lock at org.example.locks/Counter.java:24; returns at org.example.locks/Counter.java:26 with no java.util.concurrent.locks.ReentrantLock.ReentrantLock.unlock on the path
```

A Java extern name repeats the class (`<package>.<Class>.<Class>.<method>`); the lists must use
that form.

## Done When

- Failing tests in `crates/check/tests/checks.rs` first: per Java list entry, a true positive and a
  near miss read with `codespec-read-java` (fixtures checked in as `cir/1` documents with their
  Java source), as the Go fixtures are.
- A test runs a check whose lists name nothing of the set's language and finds the standard-error
  line, exit 0.
- `codespec check --list` prints the Java entries; the crate doc of `b10x-codespec-check` names them.
- `task check` green.

## Scope

- Own: `crates/check/src/lib.rs` (lists), `crates/check/src/checks.rs`, `crates/check/tests/checks.rs`,
  `crates/check/tests/fixtures/`, `CHANGELOG.md`. Inferred.
