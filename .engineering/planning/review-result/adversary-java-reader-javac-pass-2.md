---
format: aep.planning-md/3
id: review-result:adversary-java-reader-javac-pass-2
kind: review-result
status: active
title: 'Adversary, pass 2: story:java-reader-javac'
relations:
- reviews: story:java-reader-javac
revision: 1
---
unit: story:java-reader-javac, uncommitted working tree on `impl/java-reader-javac` (base 90bb66f; `adapters/java/` untracked), after correction round 1
verdict: NEEDS-CHANGE
cases: executed 24→28, red 4
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory tree, `<session scratch>/wave-sbw/java-reader-javac/adversary2/`
needs-coordinator: refuse null String/boxed parameters or record an assumption (finding 4); the locale dependence of `%d` (finding 5)

**1. Diff stat.** `adapters/java/` is untracked.
- `ReaderTest.java` gained 4 `adversary2_*` cases and 2 helpers.
- New fixtures are under `testdata/adversary2/`: shared-field, escaping-field, grow-in-loop and null-param.
- `src/` was not touched.

**2. Cases added.** All 4 were red when run alone.
- `adversary2_a_field_given_a_list_another_name_holds_is_refused`
  - `this.items = items` from a constructor parameter, and `this.held = l` from a local, are later added to.
  - Java gives 1, the generated Go gives 0.
- `adversary2_a_field_list_handed_out_by_a_getter_is_refused`
  - A list returned by `return this.items` is then added to through the field.
  - Java gives 1, Go gives 0.
- `adversary2_a_collection_changed_inside_a_for_each_over_itself_is_refused`
  - Covers adds or puts on a field list, a fresh local list, and a TreeMap `keySet()`.
  - Java throws `ConcurrentModificationException`.
- `adversary2_a_string_or_boxed_parameter_printed_where_java_prints_null_is_refused`
  - `hello(null)` gives `hi null` and `count(null)` gives `null items` in Java.

**3. Suite.** `adapters-check`: 24 passed, 4 failed, EXIT=201.

**4. Findings.** All are introduced.
1. `Reader.java:1459`, blocker. `owned()` trusts every field of `this`, including a field assigned a parameter or a local.
2. `Reader.java:1460`, blocker. A field list that a getter hands out still counts as owned.
3. `Reader.java:1646`, warning. An add or put on a for-each's own source is read as a range that runs to completion.
4. `Reader.java:2436`, warning, INFEASIBLE inside the package. A null String or Integer parameter passed by an outside caller prints `null` in Java and an empty string or 0 in the read.
5. `Reader.java:2214`, warning, CONFIRMED. `String.format("%d")` prints the default locale's digits: `ar_EG` gives `x=-١٢٣٤٥٦٧;`. `sprintf` does not, and the ledger relies on `%d`.
6. `Reader.java:2164`, note, CONFIRMED. `List.size` is read as `len` typed int32. Every other reader and writer treats `len` as the platform int, so the written Go and Rust fail to compile.

**5. Attacked and could not break**
- `remove`, `clear`, `addAll`, an iterator, `Collections.sort`.
- A fresh local list returned and then aliased by the caller.
- Fields assigned across constructors, including through `this(...)`.
- Null literals.
- `%d` with a non-null boxed argument.
- TreeMap with Long or Integer keys.
- A field initializer that reads `this`.

```findings
- file: adapters/java/src/codespec/read/Reader.java
  line: 1459
  category: property
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: owned() trusts every field of this, so a field assigned a constructor parameter or a local list is appended to as if unshared while Java's other name sees the add
- file: adapters/java/src/codespec/read/Reader.java
  line: 1460
  category: property
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a field list returned by a getter stays owned, so the caller's alias misses later adds through the field
- file: adapters/java/src/codespec/read/Reader.java
  line: 1646
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an add or put on a for-each's own source inside its body is read as a range that completes, where Java throws ConcurrentModificationException
- file: adapters/java/src/codespec/read/Reader.java
  line: 2436
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a String or Integer parameter passed as null by an outside caller prints null in Java and an empty string or 0 in the read, with no refusal or recorded precondition
- file: adapters/java/src/codespec/read/Reader.java
  line: 2214
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: String.format %d prints locale digits under the JVM default locale (ar_EG, fa_IR observed) while sprintf does not, and the ledger itself relies on it
- file: adapters/java/src/codespec/read/Reader.java
  line: 2164
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: List.size is read as len typed int32 while every other reader and both writers treat len as the platform int, so the written Go and Rust do not compile
```
