# codespec-read-java

Reads one Java package directory, or a project of them, with javac's attributed trees (`JavacTask.analyze()`) and prints
it as one `cir/1` module, or a project as one module per package. The program is `codespec.read.Main` under `src/`, on the JDK only; what
it maps and what it refuses is the class comment of `src/codespec/read/Reader.java`. Its JVM type
table (primitives, boxes, `String`, lists and maps) is `codespec.jvm.JvmTypes` in
[`../jvm/src`](../jvm/src/codespec/jvm/JvmTypes.java), shared with the other JVM readers and compiled
with this one.

```console
task adapters        # compiles src/ into target/java/classes and writes target/adapters/codespec-read-java
task adapters-check  # compiles src/ and test/, runs codespec.read.ReaderTest, reads testdata/ledger twice
```

`testdata/ledger` is the package every construct of the reader's subset is read from with no
refusal; `testdata/refuse`, `testdata/adversary` and `testdata/adversary2` hold one package per
construct the reader keeps as `foreign` or refuses. `testdata/nullable` holds fields that are null
until assigned; `task adapters-check` reads it, writes it as Go and as Rust, and runs both and the
Java original (`driver/Main.java`) on its `calls.json`, byte-equal.

## The claim and the conformance kit

`codespec-read-java --describe` claims the features its `examples/ledger` output uses and the
subject of every kit case under `../../conformance/cases` that holds `java/Case.java`: the case in
Java, package `<case>` with `-` read as `_`, `long` for Go's `int`. `ReaderTest` reads each through
`codespec read --lang java` with no refusal and no `foreign`, passes it, writes it as Go and runs it
with `codespec-run-go` on the case's `calls.json`, and requires the case's `expected.txt`; `task
adapters-check` then requires each module to use its case's subject and every claimed feature to be
used by the ledger or a kit module.

A feature may be the subject of several kit cases. Where the Go-shaped case cannot be written in
Java, a variant beside it is: `decl-enum-string` (an enum over `string`), `expr-intrinsic-slice`
(`len` of a list), `expr-nil-error` and `expr-zero-error` (a value beside a nil error, the zero
value beside a thrown sentinel), `stmt-expr-method` and `type-unit-method` (a call of a `void`
method as a statement), `stmt-return-one` (one returned value), `sem-int-wrap-no-div` (wrapping
without the division the Groovy reader refuses), `decl-var-int` (a module-level integer, where
`decl-var` prints a string parameter), `type-float-compare` (floats compared, where `type-float`
prints them with `%v`), `type-tuple-error` (a value or an error, where `type-tuple` prints a
float), `intrinsic-println-int` (a line of a string and an integer), `decl-variadic-local` and
`expr-spread-local` (a variable arity method and a spread array without the `path.Join` extern).

`decl-union` holds a Java source: a sealed interface whose permitted classes are records without
members. The reader recognises it as a `union` but refuses it at its line (`Reader.READ_UNIONS` is
off): the reader check writes every read as Go, and the Go writer refuses a union, so a read union
would fail step 4 where a refused one is `refused`. `ReaderTest` requires that one refusal. The
features left unclaimed, and why:

| case | subject | why |
|---|---|---|
| `assign-and-not`, `op-and-not` | `assign.and_not`, `op.and_not` | Java has no `&^`; `a & ~b` is `op.and` and `unop.complement` |
| `decl-embedded` | `decl.embedded` | Java has no embedded field; a class extending another is refused as inheritance |
| `decl-newtype` | `decl.newtype` | Java has no defined type over a primitive or `String`; a class is a struct |
| `decl-union` | `decl.union` | refused at the sealed interface while the Go writer, which checks a read, refuses a union |
| `expr-extern`, `expr-extern-method`, `type-extern` | `expr.extern`, `expr.extern_method`, `type.extern` | a package read maps the JDK calls of its table and refuses the rest; externs come only from a project's jars, which a kit case has none of |
| `expr-to-error` | `expr.to_error` | not mapped: the reader reads one checked-exception hierarchy whose leaves are sentinels or formats (`errorf`); an exception carrying fields would be a struct with an `Error` method, which that model does not hold |
| `fact-*` | `fact.*` | facts are computed by `codespec pass`, never claimed by a reader |
| `sem-multi-assign-order` | `sem.multi_assign.order` | Java has no assignment of several targets at once |
| `stmt-foreign` | `stmt.foreign` | a `foreign` statement holds Java source, which the Go writer cannot write |
| `stmt-lookup-else` | `stmt.lookup_else` | the reader never produces it: `Map.get` of a value that is not a reference is refused (an absent key gives null), and `containsKey` is not read |
| `stmt-send`, `type-chan`, `unop-recv` | `stmt.send`, `type.chan`, `unop.recv` | Java has no channels |
| `stmt-var` | `stmt.var` | a Java local has no zero value: javac rejects reading one that was never assigned |
| `type-array-float` | `type.array` | a Java array's length belongs to the object, not to its type: an array is read as a slice |
| `unop-deref` | `unop.deref` | Java has no pointer to a local; an object reference is dereferenced implicitly |

Claimed features whose Go-shaped case has no Java source, held by another case: `decl.enum` by
`decl-enum-string` (the case prints an enum over `int`), `expr.intrinsic` by `expr-intrinsic-slice`
(`String.length()` counts UTF-16 units, `len` bytes), `expr.nil` by `expr-nil-error` (a null
`List` is no empty one), `expr.zero` by `expr-zero-error`, `stmt.expr` by `stmt-expr-method`,
`stmt.return` by `stmt-return-one` (a Java method returns one value), `type.unit` by
`type-unit-method`, `decl.var`, `type.float`, `type.tuple`, `intrinsic.println`, `decl.variadic` and
`expr.spread` by the variants above.

The constructs the kit work maps, each only in a package read (a project's modules are read as
before):

| Java | cir/1 |
|---|---|
| `<<`, `>>`, `<<=`, `>>=` on `long` or `int` | the shift with the count masked as Java masks it (`& 63`, `& 31`; a literal count masked as it is read); `>>>` is refused |
| a labeled `break` or `continue` of the innermost loop | `break`, `continue`; one of an outer loop is refused |
| `double`, `float`, their literals and `+ - * /` | `float` 64 and 32; `%` on a float is refused |
| a mutable static field of a primitive or `String` with a literal initializer, or a primitive one without | a module-level `var` |
| `Object` | `any` |
| an interface with two or more implementations | a cir/1 interface, calls through it `iface_call`, a class converted to it `to_iface` |
| a static generic method whose type parameters are unbounded | `tparams`, `param` types, calls with the type arguments javac infers |
| `LongUnaryOperator`, `LongBinaryOperator`, `IntUnaryOperator`, `IntBinaryOperator`, `LongPredicate`, `IntPredicate`, `LongSupplier`, `IntSupplier`, `BooleanSupplier` | `func` types; a lambda `lambda`, a method reference of the same signature `func_ref`, a call of the abstract method `call_value` |
| a variable arity method; an array initializer, `a[i]`, `.length`, for-each over an array | `variadic`, a spread when an array is passed; `append` to an empty slice, `index`, `len`, `for_range` |
| an exception leaf whose one constructor passes `String.format("<format>", <its parameters>)` to `super` | `errorf` at its throw |
| `System.out.println` of an integer, a boolean or a `String` | `println` |
| a sealed interface whose permitted classes are records without members | `union`, refused while `READ_UNIONS` is off |

`ReaderTest` holds the claim against the kit: every claimed feature is the subject of at least one
kit case with a Java source.

`codespec-run-java` (`codespec.run.java.Main`, on `codespec.run.Runner` in
[`../jvm/src`](../jvm/src/codespec/run/Runner.java)) compiles a Java package with the JDK's
compiler and makes a `calls.json` against its static methods; `codespec conform --role reader
--lang java` runs every `java/Case.java` of the kit with it (step 1) over the corpus
`conformance/reader/java`, built from those sources. A claimed feature that is the subject of no
case with a Java source would have no corpus case, and the report would fail it as `no_case`.

## Projects and Quarkus

A directory with a `pom.xml`, `build.gradle` or `build.gradle.kts` is a project. The reader reads
every package under `src/main/java` in one javac analysis and makes one module per package.
`src/test/java` is not read. `codespec read --lang java <project>` writes one module: with several
packages, `CODESPEC_JAVA_PACKAGE` names it. `codespec-read-java --packages <project>` lists the
packages, `--all <project> <out>` writes `<out>/<package>.cir.json` for each, and `--effects
<project>` prints the `codespec-effects/1` summary for `codespec effects --summaries`.

The jars come from `CODESPEC_JAVA_CLASSPATH`, jar files separated by `:`. The reader never runs Maven
or Gradle. `task java-classpath -- <project>` writes the value to `target/java-classpath.txt` with
`mvn -o dependency:build-classpath`. Without it, a read that cannot find a type says so and names
the variable.

A call into a jar is an extern with its declared signature, a class of another package of the
project an extern of that package. Extern overloads and generic signatures are refused or erased.
Quarkus 3 maps to the additive `entries`, `beans` and `injects` of the module:

| Source | Module |
|---|---|
| `@Path` with `@GET`, `@POST`, `@PUT`, `@DELETE` or `@PATCH` | `http` entry with the method and the joined path |
| `@Scheduled`, `@Incoming` | `scheduled`, `incoming` entry |
| `@ApplicationScoped`, `@Singleton`, `@RequestScoped`, `@Dependent` | bean with its scope |
| `@Inject` | injection, resolved to the one bean, else an extern |
| `@RegisterRestClient` interface | externs, summarised `net` |
| Panache, `EntityManager`, JDBC calls | externs, summarised `db` |

What the reader cannot map stays `foreign` with one refusal. `@Entity` and the other persistence
annotations are such refusals. `@Transactional` is recorded in `transactional`, the keys of the functions
that run in a transaction, a class-level one for each of its methods. `@Produces` and `@Consumes` are
recorded as `produces` and `consumes` on the `http` entry, the method's own or else its class's.

A `throw` of an exception outside the package's checked-exception hierarchy (the quickstart's
`WebApplicationException`) stays refused and `foreign`: `cir/1` has no statement that leaves a
function by an exception no caller declares, and an error result would change the signature the
framework calls.

`testdata/quarkus` is a Quarkus 3.27.0 project: one resource with two HTTP methods, a service bean,
a Panache entity, a REST client interface and a `@Scheduled` method. `task java-quarkus-prepare`
builds it with `mvn -o -q package`, `task java-quarkus-check` reads it twice, byte-equal, against
`testdata/quarkus.read.cir.json`, and runs `codespec effects` over it: `FruitResource.create` is
`db`, `FruitResource.origin` is `net`, `Cleanup.sweep` is `db`.

### A quickstart

`hibernate-orm-panache-quickstart` of [quarkus-quickstarts](https://github.com/quarkusio/quarkus-quickstarts),
commit `81a58443e92d0080c962f4ef01aa9e6118deb7a5`, read against the class path of
`testdata/quickstart` (Quarkus 3.27.0, since the quickstart's own 3.40.1.1 is not in an offline
repository):

- modules: 2 (`org.acme.hibernate.orm.panache.entity`, `org.acme.hibernate.orm.panache.repository`)
- functions: 15
- refusals: 25 (35 before `@Transactional`, `@Produces` and `@Consumes` were recorded: 6, 2 and 2 fewer)

| Count | Reason |
|---|---|
| 8 | throw of an exception outside the package's checked-exception hierarchy |
| 2 each | annotations `Cacheable`, `Column`, `Entity` |
| 2 each | a second constructor; nested class; static field other than a constant; field of `Fruit`, which is not read |
| 1 each | annotations `GeneratedValue`, `Id`; field of `PanacheEntity`, which is not read |

Reproduce with a clone at that commit:

```console
task java-quickstart QUICKSTARTS=<clone>
```
## Assumptions

The document describes the Java program only under these assumptions; the reader does not check
them.

- Every reference parameter of a method is not null. A String or boxed parameter printed as it is,
  by string concatenation or `String.format`, in a method that never compares it with `null`, is
  refused, since Java would print `null`; every other use of a parameter takes it to hold an object.
- A reference field without an initializer that some constructor leaves unassigned holds null
  until assigned. It is a nil pointer, not a refusal: a pointer to its type (`ptr(string)`,
  `ptr(int32)`; a class of the package is a pointer already), whose zero value is `nil`, compared
  with `nil` where Java compares it with `null`. A use that would print `null` (concatenation,
  `String.format`), unbox or compare it (an operator, `+=`, `++`), hand it on (return it, pass it
  to a method or constructor, bind it to a local, assign it anywhere but to another such field) or
  write the collection it holds is refused at the use, since the written program stops at the nil
  pointer where Java goes on. What is left is a use as a receiver or a loop's collection
  (`text.equals(s)`, `size()`, a for-each), where Java throws on null and the written program
  stops: the document assumes the field holds an object there. An assignment to such a field
  through a receiver that is not a name (`s.pick().f = v`) is refused, since its value is bound
  before the receiver is evaluated.
- The program runs under `Locale.ROOT`. `String.format("%d", n)` is read as `sprintf`, which prints
  ASCII digits; under a default locale whose digits are not ASCII, Java prints others.
- The package never replaces `System.out` (`System.setOut`), and the line separator is `\n`: `System.out.println` is read as `println`, which ends a line with `\n`.
- A mutable static field is initialized before any function of the module runs, as a module-level
  `var` is; Java initializes it when the class is first used, which for a literal initializer is
  the same value.
