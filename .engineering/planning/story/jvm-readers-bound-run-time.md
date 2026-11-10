---
format: aep.planning-md/3
id: story:jvm-readers-bound-run-time
kind: story
status: draft
title: The JVM readers bound their run time on long unbracketed chains
relations:
- decomposes: epic:adapters-untrusted-input
revision: 1
---
# Story: the JVM readers bound their run time on long unbracketed chains

## Outcome

`codespec-read-java` and `codespec-read-groovy` end with a result or a one-line refusal within a
stated time on a source whose expressions nest without brackets, and state a size limit for the
sources they read, as the Rust and C readers do.

## Why

Found by the adversary pass on `story:readers-bound-untrusted-input`
(`review-result:readers-bound-untrusted-input-adversary-1`): with 200 000 terms of `a + …`, of
ternaries or of `else if`, `codespec-read-java` neither ends nor refuses within 170 s. The base
was not run, so whether the time is javac's or the reader's is not known. The Go, Java and Groovy
readers have no size limit of their own.

## Acceptance

- A test gives each JVM reader a 200 000-term binary chain, ternary chain and `else if` chain; each
  run ends with exit 0 or exit 1 and one line within the limit the reader's README states.
- Each JVM reader refuses a source set over a stated number of bytes with one line, and its README
  states the figure.
- The readers' pass counts in `codespec conform --role reader` do not change.

## Scope

- Own: `adapters/jvm/src/codespec/jvm/Bounded.java`, `adapters/java/src/codespec/read/Main.java`,
  `adapters/groovy/src/codespec/read/groovy/Main.java`, `adapters/jvm/test/codespec/jvm/BoundsTest.java`,
  the two READMEs. Inferred.
