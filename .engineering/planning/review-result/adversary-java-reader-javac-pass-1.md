---
format: aep.planning-md/3
id: review-result:adversary-java-reader-javac-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:java-reader-javac'
relations:
- reviews: story:java-reader-javac
revision: 1
---
unit: story:java-reader-javac, uncommitted working tree on `impl/java-reader-javac` (base 90bb66f; `adapters/java/` untracked)
verdict: NEEDS-CHANGE
cases: executed 16→24, red 8
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory tree, `<session scratch>/wave-sbw/java-reader-javac/adversary/`
needs-coordinator: whether cir/1 `sorted_keys` means UTF-16 code-unit order or byte order (finding 8)

**1. Diff stat.** `adapters/java/` is untracked. The adversary added 8 `adversary_*` cases and 2 helpers to `adapters/java/test/codespec/read/ReaderTest.java`, and fixtures under `adapters/java/testdata/adversary/` (null-field, list-alias, tree-order, format-verbs, static-receiver). Nothing under `src/` was touched.

**2. Cases added.** All 8 were red when run alone. Each one asserts a refusal at the line that was misread.

- `adversary_a_reference_field_left_at_its_null_default_is_not_read_as_a_zero_value`: the fields at Label.java:5-7 get no refusal.
- `adversary_an_add_to_a_list_parameter_is_refused`: Fill.java:12 gets no refusal.
- `adversary_an_add_through_a_local_alias_of_a_list_is_refused`: Fill.java:24-25 get no refusal.
- `adversary_list_equality_by_reference_is_refused`: Fill.java:33 gets no refusal.
- `adversary_a_tree_map_over_enum_keys_is_refused`: Order.java:14 gets no refusal.
- `adversary_a_tree_map_built_by_the_caller_is_refused`: Order.java:23 gets no refusal.
- `adversary_string_format_verbs_java_and_go_print_differently_are_refused`: these format calls are read without a refusal: `%s` on an int, `%x` on a negative number, `%n`, `%1$s`, `%s` on a List, and a format that is not a literal.
- `adversary_a_static_call_through_an_expression_keeps_the_expression`: `bump().zero()` is read as `zero()`.

Runtime evidence:
- Java printed `text=null count=null shade=null`. The Go generated from the read printed `text= count=0 shade=`.
- Java gives `Order.byEnum = [RED GREEN BLUE]`; sorting the names gives `[BLUE GREEN RED]`.
- Go `Sprintf("%s", int32(5))` gives `%!s(int32=5)`, while Java gives `5`.

**3. Suite.** The patched Taskfile ran `adapters-check`: 16 passed, 8 failed, EXIT=201.

**4. Findings.** All 8 are introduced; `adapters/java` does not exist at 90bb66f.

1. `Reader.java:863`, blocker. A reference field left at its null default is read as a zero value.
2. `Reader.java:1334`, blocker. `list.add` on a parameter or a local alias becomes a rebinding `append`, so the shared mutation is lost.
3. `Reader.java:2038`, warning. `==` on Lists is read as a slice `==`, but Java compares identity.
4. `Reader.java:1415`, blocker. A TreeMap with enum keys iterates in declaration order, but it is read as `sorted_keys` by name.
5. `Reader.java:1416`, warning. A TreeMap the reader did not see constructed may have any comparator, but it is read as natural order.
6. `Reader.java:1881`, blocker. The `String.format` text is copied into a Go-semantics `sprintf` without being checked.
7. `Reader.java:1865`, note. A static call through an instance expression drops the receiver and its side effects.
8. `Reader.java:1416`, warning. `sorted_keys` over `TreeMap<String>` stands for UTF-16 code-unit order, while Go and Rust sort by bytes. The model's text says only "ascending".

**5. Attacked and could not break**
- Determinism: each read twice, compared with `cmp`; `--canonical` passes.
- Integer widths and widening.
- String concatenation order.
- `==` on String and boxed values.
- Non-short-circuit boolean operators.
- `switch` fall-through and labeled `break`/`continue`.
- `for` with comma separators, `try`/`finally`, `do-while`, the ternary operator, `instanceof` and casts.
- Throwing calls in conditions.
- Consuming an Optional.
- `Map.get` on values.
- Static fields.

```findings
- file: adapters/java/src/codespec/read/Reader.java
  line: 863
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a String, boxed-integer or enum field without an initializer is read with a zero value where Java holds null, so "text=" + text prints "text=" instead of "text=null" with no refusal
- file: adapters/java/src/codespec/read/Reader.java
  line: 1334
  category: property
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: list.add on a parameter or local alias becomes a rebinding append, losing Java's shared-list mutation for the caller or the aliased field with no refusal
- file: adapters/java/src/codespec/read/Reader.java
  line: 2038
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: == on two Lists is read as a slice binop instead of being refused, though Java compares object identity
- file: adapters/java/src/codespec/read/Reader.java
  line: 1415
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a TreeMap keyed by an enum iterates in declaration order in Java but is read as sorted_keys over the string enum values, which sorts by name
- file: adapters/java/src/codespec/read/Reader.java
  line: 1416
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a TreeMap the reader did not see constructed (a parameter) may carry any comparator but is read as natural-order sorted_keys
- file: adapters/java/src/codespec/read/Reader.java
  line: 1881
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: String.format text is copied unchecked into Go-semantics sprintf, so %s on an int, %x on a negative, %n, %1$s, %s on a List and a non-literal format all misprint with no refusal
- file: adapters/java/src/codespec/read/Reader.java
  line: 1865
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a static method called through an instance expression drops the receiver expression and its side effects
- file: adapters/java/src/codespec/read/Reader.java
  line: 1416
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: sorted_keys over TreeMap<String> stands for Java's UTF-16 code-unit order, while Go and Rust sort by bytes, and the model text says only ascending order
```
