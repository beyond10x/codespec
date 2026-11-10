---
format: aep.planning-md/3
id: review-result:adversary-reader-java-quarkus-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the Quarkus project read
relations:
- reviews: story:reader-java-quarkus-refusals
revision: 2
---
unit: story:reader-java-quarkus-refusals, adversary pass 2. Covers `c35dad2` plus my untracked test files in `~/.local/state/worktree/trees/b10x/codespec/rd-java-quarkus`. Origins were checked against `e6c404b`, the unit's parent `9dd7fd7` and `3f37d68`.
verdict: NEEDS-CHANGE
cases: executed 98→114, red 9
origin: introduced 6 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, `~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/java-quarkus/adv2/`
needs-coordinator: yes. F7: the quickstart now has 13 refusals against the acceptance limit of 10. To wire `Adversary3Test` into `adapters-check`, add a `Taskfile.yml` line after the `Adversary2Test` line with the same arguments, the work dir changed to `adversary3`.

**1. Diff**

`git --no-pager diff --stat` is empty. Every path I added is a test or a fixture.

One charter breach. My first fixture script ran `rm -rf adapters/java/testdata/adversary3`, which deleted three tracked files of ReaderTest:
- `null-field-escape/Card.java`
- `null-field-order/Shelf.java`
- `two-constructors/Card.java`

The tree was clean before. I rewrote each file from `git show HEAD:<path>` without touching the index, and `git diff` is empty again. For a few minutes, any other session reading the tree saw those files missing.

**2. Cases added**

`adapters/java/test/codespec/read/Adversary3Test.java`, with fixtures under `adapters/java/testdata/adversary3/project-{access,statics,throws,marks}`.
- 9 cases are red now.
- 7 are green and exist to catch mutants (part 6).

This is the class's first run that was red for the right reasons, run alone and cut to 240 characters. The run before it had two bugs in my own test: `Json.write(null)`, and a whitespace-stripping compare. I fixed both.
```
FAIL adversary3_a_public_field_of_a_plain_jpa_entity_with_a_declared_getter_read_from_outside_is_refused: java.lang.AssertionError: no refusal at Use.java:5 (return c.yield;); refusals [{"pos":"Use.java:9",...
FAIL adversary3_a_public_field_of_a_panache_entity_read_in_its_own_method_where_it_declares_the_getter_is_refused: java.lang.AssertionError: no refusal at Fruit.java:16 (return weight + by;); ...
FAIL adversary3_a_panache_field_whose_entity_declares_only_a_same_named_method_with_a_parameter_is_read: java.lang.AssertionError: p.size refused: {"pos":"Use.java:9",...
FAIL adversary3_an_unqualified_panache_static_call_in_a_class_nested_in_the_entity_is_refused: java.lang.AssertionError: no refusal at Fruit.java:15 (return deleteAll();); refusals []
FAIL adversary3_a_statically_imported_panache_method_is_called_on_the_entity_it_was_imported_from: java.lang.AssertionError: imported reads [...,"name":"deleteAll","targs":[{"k":"named","name":"Use"}],...]
FAIL adversary3_a_call_of_another_packages_method_that_throws_its_checked_exception_keeps_the_error: java.lang.AssertionError: b.lib declares load with results [{"k":"int",...},{"k":"error"}], b calls it as an extern with results [{"k":"int",...}] and no refusal
FAIL adversary3_a_call_of_another_packages_optional_method_is_declared_as_that_module_declares_it: java.lang.AssertionError: b.lib declares find with results [{"k":"int",...},{"k":"bool"}], b calls it as an extern with results [{"k":"extern","path":"java.util.Optional",...}] and no refusal
FAIL adversary3_a_line_comment_inside_a_persistence_argument_does_not_swallow_its_value: java.lang.AssertionError: Row.label argument name = // the legacy column "row_label": its value is inside a line comment
FAIL adversary3_a_quote_in_a_comment_inside_a_persistence_argument_keeps_the_literal_as_written: java.lang.AssertionError: Row.spaced arguments [name = /* " */ "two spaces"]: the literal lost a space
ok   adversary3_each_project_reads_twice_byte_equal
1 passed, 9 failed
```

**3. Suite run** (`adv2/suite.sh`, after the cases existed)

`GOENV` points at a file that sets `GOFLAGS=-buildvcs=false`. Without it, `go build` fails on the empty `~/.git`, an environment problem.
```
javac exit 0
ReaderTest exit 0      74 passed, 0 failed
AdversaryTest exit 0   8 passed, 0 failed
Adversary2Test exit 0  16 passed, 0 failed
Adversary3Test exit 1  7 passed, 9 failed   (the 9 above; the 7 mutant cases ok)
```

**4. Findings**

**F1, blocker, introduced, `Reader.java:3894`.** On a plain `@Entity`, `Use.harvest` reads `c.yield` as a plain field.
- Java: Quarkus 3.27 (`HibernateOrmProcessor.candidatesForFieldAccess` with `PanacheHibernateCommonResourceProcessor`) rewrites public field accesses into getters on every `@Entity`, `@MappedSuperclass` and `@Embeddable` class, not only Panache ones.
- I ran Quarkus's own enhancer classes over the compiled fixture (`adv2/q/Enhance.java`): plain Java gives `harvest 3`, Quarkus gives `harvest 30` (`getYield()`).
- The reader only checks Panache entities and writes no refusal.
- At base, `@Entity` was refused; the unit removed that refusal.
- What reaches it: a Hibernate ORM entity with public fields and a custom getter.
- Test: `..._plain_jpa_entity_...`.

**F2, blocker, introduced, `Reader.java:3898`.** In `Fruit.heavier`, `weight + by` is read as a field.
- Quarkus also rewrites accesses inside the entity, except in the getter itself and in its own constructors. Enhanced it gives `heavier 11`; plain Java gives 6.
- What reaches it: the Panache guide's own pattern, a custom `getName()` plus any other entity method that reads `name`.
- The README row at `:177` states this rule only for accesses "outside the entity".
- Test: `..._read_in_its_own_method_...`.

**F3, warning, introduced, `Reader.java:3907`.** Accessors are matched by name only.
- `Plum.getSize(long)` causes `p.size` to be refused.
- Quarkus checks name plus descriptor, so it generates `getSize()`. Enhanced it gives `plumSize 7`, the same as a plain read.
- Green at base, red since `3f37d68`.

**F4, blocker, introduced (`c35dad2`), `Reader.java:4246`.** For an unqualified Panache static call, the entity is taken from the calling class.
- `deleteAll()` inside `Fruit.Helper` gets `targs [Helper]`. javac emits `invokestatic PanacheEntityBase.deleteAll`, whose jar body only throws.
- A static import `import static shop.Fruit.deleteAll;` used in `Use` gets `targs [Use]`. javac emits `invokestatic shop/Fruit.deleteAll`.
- Both are read with no refusal, and each names a wrong table.
- At base, the nested class was refused, and the static import dropped the entity (pass 1, F12).
- Tests: `..._nested_in_the_entity_is_refused`, `..._statically_imported_...`.

**F5, blocker, pre-existing, `Reader.java:4176`.** An extern for another package's function is built from the Java signature, not from that module's declaration. `declared()` only checks that the function exists.
- `b.lib.load` is declared `(int64, error)`; module `b` calls it as returning `int64`.
- Running the Java: `total(2) = 5`, while `total(-1)` throws `b.lib.Negative`.
- Both modules read with 0 refusals and pass `validate`.
- `find`, which returns an `Optional`, is the same: declared `(int64, bool)`, called as a jar `Optional`.
- It reads identically at `e6c404b`.
- Tests: `..._keeps_the_error`, `..._optional_method_...`.

**F6, warning, introduced, `Reader.java:2190`.** `oneLine` does not skip comments.
- `@Column(name = // the legacy column⏎"row_label")` is recorded as `name = // the legacy column "row_label"`, so the value now sits inside a comment.
- `name = /* " */ "two  spaces"` loses a space inside the literal.
- What reaches it: a comment inside an annotation argument, which is rare.

**F7, warning, introduced, `.engineering/planning/story/reader-java-quarkus-refusals.md:34`.** The quickstart now gives 13 refusals: 9 throws, 2 ExceptionMapper, 2 cross-package field. The acceptance allows at most 10, at most 2 of them not a jar throw. This is pass 1's F9 again, now at 13 instead of 12.

**F8, note, mutant, `Reader.java:4155`.** Of 20 mutants of changed lines, `ReaderTest` and `Adversary2Test` kill 9. Eleven survive both:
- M13 (`call-unchecked`) disables the declared check for method calls, the core of claim 3.
- M12: an interface method of another package counts as declared.
- M20: every type counts as declared, so `Outer.In` becomes the top-level `In` silently.
- M04: owner-only field check.
- M08: an unqualified Panache call takes the base class as its entity.
- M15: control characters allowed in persistence arguments.
- Each of these six is killed by one of my green cases.
- Still surviving: M05, M11, M17 (near-equivalent), M18 (more conservative) and M19 (`'"'` in an annotation).

**5. Claims that held** (one line each)
- Accessor naming: `Boolean` vs `boolean`, a one-letter name, `_x`, a digit second letter and non-ASCII all match `JavaBeanUtil` byte for byte.
- Cross-package fields: chained access, static field, `static final`, enum constant, `+=`, `++`, array element and nested class are all refused.
- `declared()` across a call cycle is symmetric in both package orders and terminates.
- Overloads, interface defaults, varargs and generics of another package are refused.
- A nested type that clashes in another package keeps `Outer.In`.
- Panache: `Fruit.count() + Veg.count()` gives one extern with targs `Fruit` and `Veg`.
- Text blocks are refused, and `'\''` is kept.
- Refusal dedupe hides nothing distinct: only an identical position, text and reason collapse.
- Determinism: the fixture and every probe read twice byte-equal, and the fixture equals `quarkus.read.cir.json`.
- The `persistence` field matches `Basket.java` and `Fruit.java`.
- Persistence on a class that is no struct is refused, and injection names only read beans.

**6. Written outside the worktree**

`~/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/java-quarkus/adv2/` holds:
- probe projects `p1`–`p7` with their builds, `q/` (Quarkus enhancer harness and disassembly), `src-*` and `classes-*` for the base commits, `mutants/`, `twice/`, `link/`, the `suite*`/`test-classes` directories, logs and scripts.

The Go build cache (`~/.cache/go-build`) was used as normal.

**7. `git status --short`**
```
?? adapters/java/test/codespec/read/Adversary3Test.java
?? adapters/java/testdata/adversary3/project-access/
?? adapters/java/testdata/adversary3/project-marks/
?? adapters/java/testdata/adversary3/project-statics/
?? adapters/java/testdata/adversary3/project-throws/
```

```findings
- file: adapters/java/src/codespec/read/Reader.java
  line: 3894
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A public field of a plain JPA entity with a declared getter is read as a field, while Quarkus 3.27 rewrites the access of every managed class into the getter (harvest 3 vs 30), and no refusal is written.
- file: adapters/java/src/codespec/read/Reader.java
  line: 3898
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: An entity method reading its own public field is exempt from the accessor check, but Quarkus rewrites it into the declared getter everywhere except the getter and the hierarchy's constructors (heavier 6 vs 11).
- file: adapters/java/src/codespec/read/Reader.java
  line: 3907
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Accessors are matched by name only, so a same-named method with a parameter refuses a field read that Quarkus serves from its generated getter with the plain value.
- file: adapters/java/src/codespec/read/Reader.java
  line: 4246
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: An unqualified Panache static call takes the calling class as its entity, so a call in a nested class reads as Helper's table (Java throws) and a statically imported Fruit.deleteAll reads as Use's table.
- file: adapters/java/src/codespec/read/Reader.java
  line: 4176
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: An extern of another package's function is built from the Java signature rather than that module's declaration, dropping a checked exception's error result (Java throws, module returns) and an Optional's (T, bool), with no refusal in either module.
- file: adapters/java/src/codespec/read/Reader.java
  line: 2190
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: oneLine folds a persistence argument without skipping comments, so a line comment swallows the value and a quote in a block comment collapses spaces inside the literal.
- file: .engineering/planning/story/reader-java-quarkus-refusals.md
  line: 34
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The pinned quickstart now reads with 13 refusals, 4 of them not a jar throw, above the acceptance bound of at most 10 with at most 2 non-throws.
- file: adapters/java/src/codespec/read/Reader.java
  line: 4155
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Eleven of twenty mutants of the fix round's lines survive ReaderTest and Adversary2Test, among them disabling the declared check for method calls, interface methods and nested type names.
```
