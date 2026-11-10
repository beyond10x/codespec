---
format: aep.planning-md/3
id: review-result:adversary-reader-java-quarkus-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:reader-java-quarkus-refusals'
relations:
- reviews: story:reader-java-quarkus-refusals
revision: 1
---
```
unit: story:reader-java-quarkus-refusals, unit commit 3f37d68 (base 9dd7fd7); the findings cover 3f37d68 plus my untracked test files
verdict: NEEDS-CHANGE
cases: executed 79→95, red 8
origin: introduced 11 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, /home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/java-quarkus/adv1/
needs-coordinator: yes. The quickstart has 12 refusals and the acceptance allows at most 10; the unit changed cir/1 although the story's Scope forbids it; wiring Adversary2Test needs a Taskfile line
```

**1. Diff**

`git --no-pager diff --stat` is empty: I only added files. `git status --short`:
```
?? adapters/java/test/codespec/read/Adversary2Test.java
?? adapters/java/testdata/adversary2/project-packages/
?? adapters/java/testdata/adversary2/project-panache/
```
Every path is a test or a fixture. No implementation file was touched. I wrote no Rust test, because no model behaviour I tried should fail (see part 5).

To wire the new class into `adapters-check`, add this line to `Taskfile.yml` after the `AdversaryTest` line. The existing javac line already compiles the class.
```
- 'CODESPEC_JAVA_CLASSPATH="$(cat target/java-classpath.txt)" {{.JDK}}/bin/java -cp target/java/test-classes codespec.read.Adversary2Test {{.ROOT_DIR}} {{.CODESPEC}} target/java/test-work/adversary2 {{.ADAPTERS}}'
```

**2. Cases added**

All 16 are in `adapters/java/test/codespec/read/Adversary2Test.java`, with fixtures under `adapters/java/testdata/adversary2/project-panache` (package `shop`) and `adversary2/project-packages` (packages `app` and `app.lib`). Eight are red now. The other eight are green now and exist to catch mutants (part 6).

Before the run below I fixed three bugs in my own test, none of them a finding: `boxed` became `Use.boxed`, the `Lone` fixture's constructor became generic so that it is actually refused, and a failure message no longer evaluates eagerly. This is the run of the class alone against 3f37d68; long refusal and persistence dumps are cut to `[...]`:
```
FAIL adversary2_a_field_of_a_panache_entity_whose_getter_quarkus_names_getx_coord_is_refused: java.lang.AssertionError: no refusal holding 'Panache' at Desk.java:5; refusals [...]
ok   adversary2_a_write_of_a_panache_entity_field_whose_setter_the_entity_declares_is_refused
FAIL adversary2_a_persistence_argument_written_over_two_lines_reads_into_a_module_strict_reading_admits: java.lang.AssertionError: codespec validate exit 1: codespec: <adv1>/adv2-work/panache.shop.json: not a cir/1 module: persistence mark Column on Note.body: argument "name =\n        \"body_text\"" is empty or holds a control character
ok   adversary2_c_class_get_name_of_a_nested_class_is_its_binary_name_with_a_dollar
ok   adversary2_a_static_final_field_set_by_a_java_lang_call_is_refused
ok   adversary2_a_static_final_field_set_by_an_instance_method_of_a_jar_object_is_refused
FAIL adversary2_a_field_initializer_refused_is_one_refusal_not_one_per_constructor: java.lang.AssertionError: 3 refusals at Pair.java:5
ok   adversary2_a_class_whose_one_constructor_is_refused_gets_no_made_up_constructor
ok   adversary2_a_persistence_annotation_on_a_static_field_stays_a_refusal
ok   adversary2_a_persistence_annotation_on_a_method_stays_a_refusal
FAIL adversary2_an_entity_annotation_on_a_class_read_as_no_struct_is_refused_or_recorded: java.lang.AssertionError: @Entity on Oops neither refused nor recorded; persistence [...]
FAIL adversary2_an_injection_point_names_as_its_bean_only_a_bean_the_module_lists: java.lang.AssertionError: injection points naming a bean the module does not list: [Keeper.helper -> shop.Svc.Helper]; beans [shop.Keeper]
FAIL adversary2_an_extern_type_of_another_package_names_a_type_that_package_declares: java.lang.AssertionError: app names types of app.lib it does not declare: [Outer.Nested]; app.lib declares [Box, Chain, Nested, Outer, Thing]
FAIL adversary2_an_extern_call_of_another_package_names_a_function_that_package_declares: java.lang.AssertionError: app calls functions of app.lib it does not declare: [NewChain]; app.lib declares [NewBox, NewBox2, NewChain2, NewNested, NewOuter, NewThing]
FAIL adversary2_a_field_written_through_an_object_of_another_package_is_refused_or_reached_through_a_pointer: java.lang.AssertionError: the write at Use.java:22 goes through t of type {"k":"extern","path":"app.lib","name":"Thing"}
ok   adversary2_a_call_of_another_packages_second_constructor_names_its_second_function
8 passed, 8 failed
```

To set the origins I ran the same class against the base reader (`git show 9dd7fd7:` sources compiled into scratch). Five of my red cases are green there: two-line argument, field initializer, `@Entity` on Oops, injection point, extern call, and the field write. The getxCoord case and the `Outer.Nested` type case are red at base too; the reason each is still counted as introduced is under F1 and F3.

The Java behaviour I rely on, compiled and run with JDK 21. The getter and setter names come from Quarkus 3.27.0's own `JavaBeanUtil`:
```
twice(new Thing()) = 2
Thing.Part.class.getName() = lib.Thing$Part
Quarkus getter of xCoord = getxCoord
Quarkus setter of xCoord = setxCoord
Quarkus getter of level = getLevel
```

**3. Suite run, after the cases existed**

The scratch script compiles the suites the way `adapters-check` does. `GOENV` turns off Go's VCS stamping: `/home/timo/.git` is an empty directory, so `go build` failed with exit status 128 in scratch build directories (an environment problem, not a finding).
```
## javac          javac exit 0
## ReaderTest     71 passed, 0 failed    ReaderTest exit 0
## AdversaryTest  8 passed, 0 failed     AdversaryTest exit 0
## Adversary2Test 8 passed, 8 failed     Adversary2Test exit 1
```
The count goes from 79 executed before (71 + 8) to 95 after, with 8 red.

**4. Findings** (all against 3f37d68)

- **F1, blocker, `Reader.java:3788`.** The reader looks for a Panache accessor named with the first letter upper-cased (`getXCoord`). Quarkus keeps the name unchanged when its second letter is upper case, so the accessor it calls is `getxCoord`/`setxCoord`, and IntelliJ generates exactly that name. As a result `s.xCoord` is read as a plain field while Java returns `getxCoord()` (42), with no refusal.
  - The README table row says this case is refused.
  - Base read it silently as well, but it refused the `@Entity` on the class. This unit removes that refusal, so it is counted as introduced.
  - Fix: apply Quarkus's capitalize rule, or check both spellings.
- **F2, warning, `Reader.java:2143`.** Persistence arguments are copied as source text, so `@Column(name =\n "x")`, or a tab inside an argument, gives a module that strict reading refuses.
  - A `codespec read` of such a project fails with exit 1; the probe output is in scratch as `p1`.
  - Fix: collapse whitespace in each argument, or refuse the mark at its line.
- **F3, warning, `Reader.java:5201`.** Another package's nested class is named `Outer.Nested` in this module, while that package now declares it as `Nested`. A field read through it is now admitted.
  - The test is also red at base, but there `app.lib` declared no `Nested` at all and `n.v` was refused. The unit exposed it, so it is counted as introduced.
- **F4, warning, `Reader.java:4271`.** `new Chain()`, whose constructor `app.lib` refuses because it calls `this(7)`, reads as an extern call of `NewChain`. No module declares that function, and there is no refusal.
  - Fix: refuse a call of another package's constructor that package does not read.
- **F5, blocker, `Reader.java:3752`.** A field write through an object of another package reads as a `field` of an `extern`-typed value, with no refusal.
  - cir/1 defines `extern` as opaque: its "fields and representation are not part of the module". The Go reader reads the same Go program with the same type and refuses `t.Count`.
  - Java's `twice` gives 2; under value semantics the module gives 0.
  - This is reached by the usual Panache layout: probe `p4` has the entity in `model` and `i.name = n` in `api`, read with no refusal.
- **F6, note, `Reader.java:2416`.** A refused field initializer is copied into each constructor, so one construct becomes 3 refusals. Counts of refusals then count constructors, not constructs.
- **F7, note, INFEASIBLE, `Reader.java:916`.** Persistence annotations are skipped on every class but recorded only on structs. `@Entity` on a class extending `Exception` is therefore lost with no trace. Hibernate would refuse to start with such an entity, so no real project reaches this.
- **F8, note, `Reader.java:581`.** The bean list now takes nested classes of any top-level type, interfaces included. An injection point then names `shop.Svc.Helper` as its bean, a class that no module reads.
- **F9, warning, `.engineering/planning/story/reader-java-quarkus-refusals.md:34`.** The acceptance asks for at most 10 refusals on the quickstart; it has 12. I recounted from the sources: 5 throws plus 1 `ErrorMapper` per module, so 12 is correct. The README says 10 is out of reach without a cir/1 change.
- **F10, warning, `crates/model/src/lib.rs:113`.** The story's Scope says "`cir/1` does not change … reported and left unclaimed" (line 67). The unit added `persistence` to the model, the schema and the docs. It also committed `website/data/conformance/*.json`, which the story says the coordinator regenerates and no unit commits (line 61).
- **F11, note, mutant, `Reader.java:1134`.** If `extendsPanacheEntity` accepts any Panache base, an entity extending `PanacheEntityBase` gets an extra `id` field silently. Neither the existing suite nor my class catches it.
- **F12, note, pre-existing, `ReaderTest.java:1047`.** A Panache static call drops the entity it is called on: `Fruit.deleteAll()` reads as `PanacheEntityBase.deleteAll`. It is the same at base.

**5. What I attacked and could not break**

- **Package reads are unchanged.** The base and head readers read 139 package directories (testdata, kit cases, examples) byte for byte the same.
- **Documents with `persistence` survive every consumer.** That covers `pass` (which keeps the field), `effects`, `reach`, `trace`, `measure`, every `check`, and the go, rust and c writers, with no panic. The written output is byte-equal with and without the marks.
  - The model admits duplicate marks, a right-to-left override character (U+202E) and a 100,000-character argument; none of these is claimed to be refused.
  - It refuses an unknown type or field, a mark that is not an identifier, and dotted `at` forms.
  - `query` and `eval` do not exist in this tree, so I could not run them.
- **Mapped forms that hold:**
  - Overloaded constructors of the same arity resolve to the one javac chose.
  - `this(…)` is refused.
  - The binary name of a nested class has `$`.
  - A `static final` set by a `java.*` call or by an instance call of a jar object is refused.
  - `a.id == b.id` and `==` on boxed values are refused.
  - An initializer block refuses the struct.
  - `FruitResource.create` is still `db`.
  - The quickstart recount is 10 throws + 2 mappers = 12, and the two modules map the rest correctly.

**6. Mutants**

I mutated copies of `Reader.java` in scratch, 14 mutants in all.

- `ReaderTest` kills 2: M12 (`this(…)` allowed) and M14 (a taken constructor name not checked).
- `ReaderTest` misses 12; `Adversary2Test` kills 8 of them:
  - M1: qualified name instead of binary name.
  - M2: `java.*` initializer allowed.
  - M3: instance-method initializer allowed.
  - M4: setter ignored.
  - M5: constructor made up when the only one is refused.
  - M6: persistence annotation on a static field dropped.
  - M7: another package's second constructor named as the first.
- Surviving both:
  - M8: `@Entity` on an interface dropped.
  - M9: accessor check inside the entity (a conservative change).
  - M10: last constructor taken as the class's constructor (looks equivalent).
  - M11: any Panache base gets `id` (F11).
  - M13: another package's entity `id` not inherited (conservative).

**7. Written outside the worktree**

`/home/timo/.cache/scratch/2db1a4b7-2d2d-4a62-bab2-d71ac128826e/rd/java-quarkus/adv1/`: probe projects `p1`–`p4`, `gomod`, `javacheck`, `base-src`, `base-classes`, `mutants`, `mutant-results`, the work and output directories, logs and scripts.

Inside the worktree, `task java-quickstart` regenerated `target/java-quickstart`. That task also overwrote `target/java-classpath.txt`; I restored it and checked with `cmp` that it is byte-equal to before.

```findings
- file: adapters/java/src/codespec/read/Reader.java
  line: 3788
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The Panache accessor check spells the getter getXCoord where Quarkus calls getxCoord, so a field read that Java turns into the entity's own getter reads as a plain field with no refusal, against the README's refused row.
- file: adapters/java/src/codespec/read/Reader.java
  line: 2143
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Persistence arguments are copied as source text, so an annotation argument broken over two lines or holding a tab gives a module that strict reading refuses and the whole codespec read fails.
- file: adapters/java/src/codespec/read/Reader.java
  line: 5201
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A static nested class of another package is named Outer.Nested in the extern type while that package's module now declares it as Nested, and field reads through the mismatched name are admitted.
- file: adapters/java/src/codespec/read/Reader.java
  line: 4271
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A call of another package's constructor that the package refuses (this(...)) reads as an extern call of NewChain, which no module declares, instead of a refusal.
- file: adapters/java/src/codespec/read/Reader.java
  line: 3752
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A write to a field of another package's object reads as a field of an opaque extern-typed value, which cir/1 says has no fields in the module and the Go reader refuses, so a write Java makes to the caller's object becomes undefined or a copy without a refusal.
- file: adapters/java/src/codespec/read/Reader.java
  line: 2416
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: A refused field initializer is copied into each constructor, so one Java construct gives one refusal per constructor and refusal counts count constructors.
- file: adapters/java/src/codespec/read/Reader.java
  line: 916
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Persistence annotations are skipped on every class but recorded only on structs, so @Entity on a class read as no struct (an exception class) vanishes without refusal or mark.
- file: adapters/java/src/codespec/read/Reader.java
  line: 581
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The bean list takes scoped classes nested in interfaces, which no module reads, so an injection point names shop.Svc.Helper as its bean.
- file: .engineering/planning/story/reader-java-quarkus-refusals.md
  line: 34
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The pinned quickstart reads with 12 refusals (10 jar throws, 2 ExceptionMapper), above the acceptance bound of at most 10.
- file: crates/model/src/lib.rs
  line: 113
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The unit adds the cir/1 module field persistence and commits regenerated website/data/conformance reports, both of which the story's Scope reserves (cir/1 does not change; generated data is the coordinator's).
- file: adapters/java/src/codespec/read/Reader.java
  line: 1134
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Letting extendsPanacheEntity accept any Panache base silently gives PanacheEntityBase subclasses an extra id field, and no ReaderTest or Adversary2Test case fails.
- file: adapters/java/test/codespec/read/ReaderTest.java
  line: 1047
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: A Panache static call on an entity reads as PanacheEntityBase.<method> with the entity class dropped, so Fruit.deleteAll() and Other.deleteAll() read the same.
```
