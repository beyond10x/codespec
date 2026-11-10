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
| `stmt-raise` | `stmt.raise` | read only in a project, in a function an entry names (Projects and Quarkus); a kit case is a package read with no entry, so its module is written by hand |
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

Refused at their line, since the written Go would print otherwise than Java
(`testdata/refuse/kit-limits` and `testdata/adversary1`, run by `ReaderTest` and `AdversaryTest`):

- a shift of a literal by a count that is not constant (`1 << n`): Go gives the literal the type its
  context wants, 64 bits in a comparison or a `println`, where Java shifts an int;
- an operator whose operands are all floating-point constants (`0.1 + 0.2`), and `-` of one
  (`-0.0`): Go evaluates a constant expression exactly and rounds once, where Java rounds each
  operation, and a Go constant has no negative zero;
- a compound assignment (`+=` and the like) to an array element, a field or a module-level variable
  whose right operand calls a method or constructor of the package or a function value: Java reads
  the target before the call, the written Go after it;
- a static field named through its class (`Case.count`) in a method with a local or parameter of
  its name: the module-level variable and the local are one identifier in the written function;
- an assignment to a local of an interface with several implementations after its declaration: the
  Go writer declares the local with the type of the value it starts with, so another implementation
  is not assignable to it.

An integer constant expression that wraps (`2147483647 + 1`), or whose shift count Java masks, is
read as the literal of Java's value: written as an operator of literals, Go would compute it exactly.
In a project read, shifts, labeled statements and floating-point literals are refused as before
the kit work.

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
project an extern of that package, named as that package's module declares it. A call of another
package's method or constructor is an extern with the parameters and results that package's module
declares for it (its structs, enums and interfaces the extern types of the package). It is refused
when that module declares no function for it, when it returns an error (a checked exception) or an
Optional's value and `bool`, which the call would have to handle, when it is generic or of variable
arity, and when it goes through an interface. Extern overloads and generic signatures of a jar are
refused or erased. A static method of `PanacheEntityBase` called on an entity (`Fruit.deleteAll()`)
is a generic extern with one type parameter `E`, the entity its type argument: Panache gives each
entity its own copy of the method, run on that entity's table, and the jar's own body only throws.
The entity is the class javac calls the method on: the one the call names, the calling class when
it inherits the method, the one a static import names. A call javac makes on the base itself (one
naming it, or an unqualified one in a class nested in an entity) is refused.
Quarkus 3 maps to the additive `entries`, `beans` and `injects` of the module:

| Source | Module |
|---|---|
| `@Path` with `@GET`, `@POST`, `@PUT`, `@DELETE` or `@PATCH` | `http` entry with the method and the joined path |
| `@Scheduled`, `@Incoming` | `scheduled`, `incoming` entry |
| `@ApplicationScoped`, `@Singleton`, `@RequestScoped`, `@Dependent` | bean with its scope |
| `@Inject` | injection, resolved to the one bean when its package lists it (the bean is read as a struct), else an extern |
| `@RegisterRestClient` interface | externs, summarised `net` |
| Panache, `EntityManager`, JDBC calls | externs, summarised `db` |

What the reader cannot map stays `foreign` with one refusal. `@Entity`, `@Cacheable`, `@Id`,
`@GeneratedValue` and `@Column` on a class or on one of its instance fields are recorded in
`persistence`, one mark each: `at` the type (`Fruit`) or field (`Fruit.name`), `mark` the
annotation's simple name and `args` its arguments as written (`length = 40`), each on one line:
comments dropped, single spaces outside its literals, each literal byte for byte. They tell what Hibernate does with the objects: a field write to
a managed entity reaches the database at commit, `persist()` sets the generated key, a unique column
refuses a duplicate. Elsewhere (a method, a static field, an interface, a class read as no struct)
they stay refusals, and so does one whose argument holds a tab or line break inside a literal. `@Transactional` is recorded in `transactional`, the keys of the functions
that run in a transaction, a class-level one for each of its methods. `@Produces` and `@Consumes` are
recorded as `produces` and `consumes` on the `http` entry, the method's own or else its class's.

A project read maps these where a package read refuses them, each with what Java does that the
module keeps, and refuses at its line the forms it cannot keep (`testdata/project-forms`, one
`ReaderTest` case each; `testdata/quarkus` holds one source per mapped form):

| Java | Module | Refused |
|---|---|---|
| a second constructor | `New<Class>2`, the n-th `New<Class><n>` in source order; each starts from the field initializers, then runs its body, and `new C(…)` calls the one javac chose | a constructor calling `this(…)`: Java runs the field initializers once, in the constructor it calls; a constructor whose function name another function of the package has |
| a static nested class | a struct of its simple name with its constructors and methods: it has no object of the class it is nested in | an inner class, whose objects hold one; a nested interface, enum or record; a class nested in a nested class; a nested class named as another type of the package |
| a `static final` field that is not a compile-time constant, set by a literal or by a call of a jar's static method whose arguments are literals, constants or `C.class.getName()` | a module-level `var` with that initializer, `C.class.getName()` the literal of the class's binary name | any other initializer: Java runs it when it first uses the class, the module when it is loaded, and a call of the package's own code or of a value computed at run time can see another state; a second module-level variable of one name |
| a field of a class of another package of the project | — | every read and write: the object is an `extern` here, whose fields are not part of the module, and a write Java makes to the caller's object would change a copy |
| `id` of a class extending `PanacheEntity` | the struct's first field `id`, a pointer nil until `persist()` sets it: the jar declares `public Long id` without an initializer and no constructor assigns it; a class extending `PanacheEntityBase` has only its own fields | a class declaring its own `id`, which hides it; a field of any other jar superclass, whose constructor the reader cannot see |
| a public field of a class Quarkus manages (`@Entity`, `@MappedSuperclass`, `@Embeddable`, or a Panache entity), read or written anywhere, the class's own methods included | a field, as above | when the class the access goes through, or a managed superclass up to the field's, declares the field's getter or setter by Quarkus's name and descriptor: `get` (`is` for a `boolean`) without parameters and of the field's type, `set` with one parameter of the field's type and no result, then the field's name with its first letter upper-cased, unchanged when its second letter is upper case (`getxCoord`). Quarkus turns the access into a call of it everywhere but in a constructor of that class or of a managed subclass (not in a lambda there) and in the getter or setter itself; a `@Transient` field is not rewritten, a non-public one only into Hibernate's own read or write of the field |
| a `@Provider` class implementing `ExceptionMapper` of a jar's exception | an `exception-mapper` entry naming its `toResponse`, `on` the extern type it takes as `<path>.<name>` (`java.lang.Exception.Exception`, a jar class's path being its qualified name); `@Provider` is no refusal | a mapper without `@Provider`, without its own `toResponse`, or of a class of the project; in its body, as anywhere, `instanceof` and a cast (`cir/1` has no test or conversion of a runtime type) |
| `throw new X(…)` of a jar's unchecked exception (`WebApplicationException`) in a method an entry names | `raise` of the constructed value, its type listed in the function's `raises`: Java leaves the method and every caller up to the framework, which hands it to its mapper; the signature the framework calls is unchanged | the throw in any other method, or in one overriding a method of the project, and every call of a method an entry names whose body throws: a caller would have to list what it raises, and so would its callers |
| a parameter of an http entry named by a template variable `{name}` of its joined path, without an annotation or with `@PathParam` of its own name, printed as it is | the parameter, printed: the framework passes the non-empty segment it matched, converted, never null | the same parameter when a source of the project calls the method, and a template variable with a regular expression (`{id: \d*}`), which may match an empty segment |

Any other `throw` of an exception outside the package's checked-exception hierarchy stays refused
and `foreign`: a package read has no entries, and a checked exception of a jar would have to be
declared by every caller.

`testdata/quarkus` is a Quarkus 3.27.0 project: one resource with two HTTP methods, a service bean,
a Panache entity, a REST client interface, a `@Scheduled` method, and a source for each form
above: `Crate` (two constructors), `Prices` (a static nested class), `Audit` (a `static final`
logger), `Stock` (the entity's `id` and `name`) and `Basket` (an entity with each persistence
annotation). `task java-quarkus-prepare`
builds it with `mvn -o -q package`, `task java-quarkus-check` reads it twice, byte-equal, against
`testdata/quarkus.read.cir.json`, and runs `codespec effects` over it: `FruitResource.create` is
`db`, `FruitResource.origin` is `net`, `Cleanup.sweep` is `db`.

### A quickstart

`hibernate-orm-panache-quickstart` of [quarkus-quickstarts](https://github.com/quarkusio/quarkus-quickstarts),
commit `81a58443e92d0080c962f4ef01aa9e6118deb7a5`, read against the class path of
`testdata/quickstart` (Quarkus 3.27.0, since the quickstart's own 3.40.1.1 is not in an offline
repository):

- modules: 2 (`org.acme.hibernate.orm.panache.entity`, `org.acme.hibernate.orm.panache.repository`)
- functions: 21 (the second constructors of `FruitEntity` and `Fruit`; each `ErrorMapper` with its
  constructor and `toResponse`)
- refusals: 8 (13 before `raise`, the `exception-mapper` entry and path parameters; 25 before the
  forms of a project read above were mapped and the persistence annotations recorded; 35 before
  `@Transactional`, `@Produces` and `@Consumes` were recorded)
- raises: 8 resource functions raise `WebApplicationException`; an `exception-mapper` entry on
  `java.lang.Exception.Exception` names each `ErrorMapper.toResponse`
- persistence marks: 8 (`Entity`, `Cacheable` and `Column` in each package, `Id` and
  `GeneratedValue` on the repository's `Fruit.id`)

| Count | Reason |
|---|---|
| 2 | `fruit.name` in `FruitEntityResource.update`, a field of `Fruit` of the `repository` package: the object is an extern in the `entity` module. The refused `if` holds the throw of line 67 |
| 2 | `exception instanceof WebApplicationException` in each `ErrorMapper.toResponse`, the `if` refused whole with the cast it holds: `cir/1` has no test of a runtime type |
| 2 | `exception.getClass().getName()` in each `ErrorMapper.toResponse`: reflection |
| 2 | `exception.getMessage() != null` in each `ErrorMapper.toResponse`: a jar method's `String` is a `cir/1` string, which has no null to compare with |

The 9 throws are `raise` now: 8 in resource methods an `http` entry names, and the one in
`FruitEntityResource.update` inside the `if` refused for `fruit.name`. 6 of them print the path
parameter `id`, which reads since only the framework passes it. The `ErrorMapper` classes read, and
their `@Provider` is an `exception-mapper` entry. 2 refusals, the field ones, are out of reach
while a module holds one package; the 6 in the mappers need a type test and a conversion of an
extern value, the name of its runtime class, a nullable extern `String`, and, once
`getClass().getName()` reads, an extern overload (`ObjectNode.put` of a `String` and of an `int`
in one module), none of which `cir/1` has.

Reproduce with a clone at that commit:

```console
task java-quickstart QUICKSTARTS=<clone>
```
## Assumptions

The document describes the Java program only under these assumptions; the reader does not check
them.

- Every reference parameter of a method is not null. A String or boxed parameter printed as it is,
  by string concatenation or `String.format`, in a method that never compares it with `null`, is
  refused, since Java would print `null`, unless it is a path parameter only the framework passes
  (Projects and Quarkus); every other use of a parameter takes it to hold an object.
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
- In a project read, a `static final` field set by a call of a jar's static method with constant
  arguments (`Logger.getLogger(C.class.getName())`) holds what that call gives whenever it runs:
  Java calls it when the class is first used, the module's `var` when the module is loaded.
