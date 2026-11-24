---
format: aep.planning-md/3
id: story:check-java-name-lists
kind: story
status: draft
title: Built-in checks carry Java name lists
summary: 'Defect: 17 of 18 checks give 0 rows on Java documents silently; the logic works with Java names via --names.'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:check-not-applicable
scope:
- confidence: inferred
  path: crates/check/src/checks.rs
- confidence: inferred
  path: crates/check/src/lib.rs
- confidence: inferred
  path: crates/check/tests/checks.rs
- confidence: inferred
  path: crates/check/tests/fixtures
- confidence: inferred
  path: crates/cli/tests/check.rs
revision: 9
---
# Story: built-in checks carry Java name lists and say when a set has no name for its language

## Outcome

`codespec check` has Java entries in the name lists of its checks, a first set: `java.util.concurrent`
locks (`ReentrantLock`, `ReentrantReadWriteLock` read and write locks, `StampedLock`) for
`lock-without-unlock`; `java.util.Random` and `ThreadLocalRandom` against `SecureRandom` for
`insecure-random`; JDBC (`Connection`, `Statement`, `ResultSet`) and `EntityManager` as resources
for `conn-leak`; and a new check `secret-compare`: `MessageDigest.isEqual` and not `equals`
compares a value whose name marks it a secret (`token`, `secret`, `password`, `mac`, `digest`).
A check that still names nothing of Java reports "not applicable" through
`story:check-not-applicable`, which this story depends on.

## Why

Defect (silent false negative). Reported by the consumer of a Java service: 17 of 18 built-in
checks give 0 rows on its Java documents; their name lists are Go standard-library names. The
check logic itself works on Java documents: with `--names` the rows appear. A Java
`ReentrantLock` is reentrant, so a relock in its own span is no self-deadlock: a Java list must
not carry it into a check that reports a second lock in one span as a defect.

The source of the lists is part D of the consumer's ask, quoted: "Java name lists:
java.util.concurrent locks, JDBC and EntityManager, transactions, SecureRandom against Random,
constant-time comparison of secrets". Transactions are not in this first set: a transaction begun
with no commit or rollback on a path is a check of its own, recorded as Later.

Later: a check for a transaction (`EntityTransaction.begin`, `Connection.setAutoCommit(false)`)
with a path to a return that passes neither commit nor rollback; drafted when this story is
implemented, not part of it.

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

- Failing tests in `crates/check/tests/checks.rs` first: per Java list entry (the locks, the
  random sources, JDBC and `EntityManager`, `secret-compare`), a true positive and a near miss read
  with `codespec-read-java` (fixtures checked in as `cir/1` documents with their Java source), as
  the Go fixtures are.
- `secret-compare`: `token.equals(given)` is a row; `MessageDigest.isEqual(token, given)` and
  `name.equals(given)` are none (tests).
- `lock-without-unlock`, `insecure-random` and `conn-leak` over the Java reproducer are applicable
  and give their rows with no `--names` (a `crates/cli/tests/check.rs` case). The same case's
  fixture has a `ReentrantReadWriteLock` read lock, its write lock and a `StampedLock` each taken
  with a return before the unlock, and each is a `lock-without-unlock` row.
- A method that takes one `ReentrantLock` twice and releases it twice gives no row from any
  built-in check (a near-miss case run over every check of `codespec check --list`).
- `codespec check --list` prints `secret-compare` and the Java entries (a test); the crate doc of
  `b10x-codespec-check` names them.
- `task check` green.

## Scope

- Own: `crates/check/src/lib.rs` (lists, `secret-compare`), `crates/check/src/checks.rs`,
  `crates/check/tests/checks.rs`, `crates/check/tests/fixtures/`, `crates/cli/tests/check.rs`.
  Inferred.
- After `story:check-not-applicable` (same files).
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
