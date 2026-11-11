# codespec-read-groovy

Reads the `.groovy` files of one package directory and prints them as one `cir/1` module, typed by
the Groovy 2.4.10 static type checker. The program is `codespec.read.groovy.Main` under `src/`, run
on the JDK with `groovy-all-2.4.10.jar` on the class path. What it maps and what it refuses is the
class comment of `src/codespec/read/groovy/Reader.java`. Its JVM type table (primitives, boxes,
`String`, lists and maps) is `codespec.jvm.JvmTypes` in [`../jvm/src`](../jvm/src/codespec/jvm/JvmTypes.java),
shared with the Java reader.

```console
codespec read --lang groovy <dir> --out <file> --adapters target/adapters
```

The reader compiles the files through Groovy's `CANONICALIZATION` phase, so every name resolves, then
runs `StaticTypeCheckingVisitor` over every class, whether it carries `@TypeChecked`,
`@CompileStatic` or nothing. It does not run `INSTRUCTION_SELECTION`: the `@CompileStatic` transform
there rewrites the tree the reader reads. Each expression takes the type the checker infers; a `def`
local takes the type inferred for its declaration, and a `def` parameter is `any`. Where the checker
reports an error, the statement holding it becomes `foreign`; the read goes on.

Integer division is `x.intdiv(y)`: Groovy's `/` on two integers divides as `BigDecimal` and is
refused. On two `int`s or two `long`s (an `int` literal taking a `long`'s type), `intdiv` is `/` of
`cir/1` in that type: Groovy's `NumberMath` divides as Java's `/` does, truncating toward zero, the
minimum divided by -1 wrapping to itself, and division by zero throwing `ArithmeticException` where
Go panics. The checker types `intdiv` as `Number`; a type-checking extension (`IntdivTyping`) types
it as Groovy runs it, `int` or `long`, except in `@CompileStatic` or `@TypeChecked` code, where
groovyc refuses the `Number` and the statement stays `foreign` (`dynamic`). On other operand types
it is refused (`groovy-semantics`), and as a statement of its own, like `a + b` alone, it is
`unsupported`.
Groovy has no compound `intdiv`, so `x = x.intdiv(y)`, with `x` a local or a field read through no
call, is `x /= y`; any other target is an assignment of the division.

`testdata/typed` is a package read with no refusal: an unannotated class, a `@CompileStatic` class
and an enum. Each directory under `testdata/refuse` holds one construct the reader keeps as
`foreign` or refuses; its line is marked `// refused <category>`. `testdata/lending` is a small
Grails application; `testdata/lending/EXPECTED.md` is what the reader reads of it, and
`testdata/lending.read.cir.json` is that read. `testdata/holds` is a second one, most of whose
calls go through code the reader refuses: a method that returns `def`, a second overload, a closure
and a service that extends another. `testdata/holds.read.cir.json` is its read, and "Call lists"
below is what it shows.

## Call lists

Code the reader does not read still calls what it calls, and the document says so, so that
`codespec reach`, `cq` and `calls` answer over an application whose bodies are mostly outside
`cir/1`:

- Every `foreign` statement carries `calls`: the calls the statement makes.
- A method or constructor refused at its declaration is an `opaque` function of the module, with
  the `reason` of its refusal and the `calls` of its body. So is every method and constructor of a
  class refused as a whole, with the class's reason, and of a class none of whose members is read
  because one of its fields is refused, with the reason of the first such field.

In `testdata/holds`, `DeskService.checkIn` calls `shelve`, which returns `def` and is refused, and
only `shelve` calls `NoticeService.pickupReady`:

```console
$ codespec reach adapters/groovy/testdata/holds.read.cir.json --entry holds.DeskService.checkIn
```

reaches `DeskService.shelve` and `NoticeService.pickupReady`, where without call lists the path
ended at the `foreign` statement in `checkIn`.

What a call list holds, a call before its receiver and its arguments, otherwise in source order:

| The source | The call ref |
| --- | --- |
| a call the type checker resolved to a method of the module | `method`, `recv` its class; `func` for a static method that is read and for a constructor, `New<Class>` |
| one it resolved to a class outside the module, a resolve-only class of the application among them | as the Java reader names an extern: `method` with `path` the package and `recv` the class; `func` with `path` the qualified class name for a static method and for a constructor, `<init>` |
| one it resolved to a GDK method, `names.each { … }` | `func`, the static method that implements it: `path` `org.codehaus.groovy.runtime.DefaultGroovyMethods`, `name` `each` |
| a call it did not resolve | `value`, `sig` `groovy dynamic call of <name>`: it names no function, and the fact base holds it as a call whose target is unknown |
| `new T(…)` | the constructor call, then `new` of `T`, once for each class in a list |
| a closure | `lambda` of `sig` `groovy.lang.Closure`, holding the calls of its body |
| a method pointer, `x.&name` | `lambda` holding one `value` call, `groovy dynamic call of <name>`: Groovy resolves it when it runs |
| a call of a closure, `step.call()` or `step()` | `value`, `sig` `groovy.lang.Closure` |

A list holds what is written as a call. It does not hold what Groovy calls for a construct that is
not one: an operator (`a + b` is `a.plus(b)`), a property read or write (`hold.title` is
`getTitle()`), Groovy truth, a for-each loop's iteration, a coercion or a GString's `toString`.
Nor does it hold a call of a member the compiler generates for a class of the module, a property's
getter or setter called by its name.

A call of a declared method the module holds no function for, a method without a body or one of
two classes with one simple name, is a `value` call,
`groovy call of <class>.<name>, which the module holds no function for`; so is a call into a class
without a package, which has no `path` that is not the module's own.

### Overloads are functions of their own

`cir/1` keys a function by its receiver type and its name, and Groovy overloads. The reader reads
the first declaration of a name and refuses each later one,
`unsupported: a second method named <name>: overloading is outside cir/1`. The refused overload is
an `opaque` function of its own, not part of the first: `cir/1` refuses two functions of one key,
an `opaque` function has no field that says it stands for several declarations, and one function
could not hold both the read body of the first and the call list of the second.

A refused declaration keeps its name where no function of the module has it. Where one has, it
takes the lowest number from 2 that is free: the second `renew` of `DeskService` in
`testdata/holds` is `renew2`, the function `DeskService.renew2`; a third would be `renew3`, and a
refused second constructor of `Desk` is `NewDesk2`, as the Java reader names a second constructor. The numbers follow source order. A call
names the overload the type checker picked for it: `renew(hold)` is a call of `DeskService.renew`,
`renew(hold, days)` one of `DeskService.renew2`. The number is the reader's; the source's name is
the `what` of the refusal at the same position.

A refused static method is `<Class>.<name>` too, with `recv` its class, where a static method that
is read is the function `<name>`: two refused classes may each declare a static method of one name.
A nested class of the module, which is refused, is the receiver `Outer$Inner`, the name the JVM
gives it; outside the module it is `Outer.Inner`, as the Java reader names one.

### What yields no function

- A method without a body: it calls nothing.
- A class refused for a duplicate simple name: neither of the two has a name in the module.
- An artefact refused as a whole at file level, for an import that does not resolve or a failure
  while it is compiled: the file is taken out of the compilation, so there is no body to list.
- The statements of a script outside any method, and a field's initializer in a class that is
  refused: a call there is in no call list.
- A class refused as a whole that declares no constructor has no `New<Class>`.

## Grails 2.5.6 applications

A directory holding `application.properties` with `app.grails.version=2.5.6` is read as a legacy
Grails application, into one module named by its `app.name`. A directory without
`application.properties` is read as plain Groovy, as above.

- Read: the domain classes (every `.groovy` file under `grails-app/domain`) and the services (every
  `*Service.groovy` under `grails-app/services`).
- Resolve-only: every other `.groovy` file under `grails-app` (`utils`, `taglib`, `jobs`, `conf`,
  `controllers`) and under `src`. They are compiled so that references to them resolve, and yield no
  struct, function or refusal; a call of one of them is `unsupported`, its target not being read. A
  resolve-only file whose own imports do not resolve is left out of the compilation. Plugins are not
  read.
- `src/java` is resolve-only too: its `.java` files are compiled first, with javac in memory, against
  the Groovy jar and `CODESPEC_GROOVY_CLASSPATH`, so a Groovy file that names one of their classes
  resolves. Their classes are defined where those jars are visible, so one that extends or
  implements a class of a jar resolves too. A Java file javac reports an error in is left out. The
  Java files do not see the Groovy files.
- Positions name a file by its path in the application: `grails-app/services/lending/LoanService.groovy:15`.
- Two artefacts with the same simple name in different packages are both refused,
  `unsupported: duplicate simple name`: one module has one namespace.
- An artefact whose imports do not resolve is refused as a whole at its first unresolved line,
  `unresolved: <class>`; the others are read. A resolve-only file that does not resolve is left out.
  What does not resolve, besides a class that is nowhere:
  - a class a jar holds that the JVM cannot load, `unresolved: <class>: <the JVM's error>`: its
    superclass or the type of one of its members is on no jar (`NoClassDefFoundError`), or it comes
    from a stub jar of API-only class files (`ClassFormatError: Absent Code attribute`);
  - the class of a constant an annotation member names, `@Tag(Levels.HIGH)` with `Levels` on no jar,
    refused at the annotation, `unresolved: Levels.HIGH`;
  - a name in a static method or initializer that is no variable, static field or class, as the
    name of a class is once its own file is refused or left out, `unresolved: <name>`;
  - a class that is not named in the file and cannot be loaded when the compiler or the type
    checker reaches it, through a call or as the class of a local transformation: the artefact is
    refused at its class (its first annotation's line), `unresolved: <the JVM's error>`.

### A file that fails is refused; the read goes on

Whatever fails while one file is compiled, type-checked or read takes that file out, and only that
file. An artefact is refused as a whole, one refusal at a line with what failed; a resolve-only
file is left out and yields nothing; every other artefact is read, and the document is written
with exit 0. The reader does not know the failures by their messages:

- A compiler error that names a file and a line, whatever it says, a syntax error among them,
  refuses the artefact at that line: `unresolved: <name>` for a name that does not resolve, as
  above, else `unsupported: compile error: <the compiler's message>`. A file with several errors is
  refused once, at its first unresolved name, else at its first error.
- An exception or error thrown while the compiler, one of Groovy's own transformations, the type
  checker or the reader works on a file refuses the artefact at its class (its first annotation's
  line): `unresolved: <exception>: <its first cause>` when it or a cause of it is a class that
  cannot be found or loaded, as `@Slf4j` without the logging jar, else
  `unsupported: the compiler failed: <exception>` (`the type checker failed:`,
  `the reader failed:`). An error the compiler reports without a file while it works on one is
  that file's, `unsupported: compile error: <message>`.
- A read file that is not UTF-8 text is refused at line 1, `unsupported: the file is not UTF-8
  text`, and not compiled.
- A failure that names no file, as an exception thrown between two files, refuses every artefact
  still being read at line 1, `unsupported: no file can be blamed: <what failed>`. The artefacts
  refused before it keep their own refusal.

The files that are left are compiled again after each file is taken out, since a file can fail
only once another is gone: a class of a file that was left out no longer resolves. Each round
takes out at least one file, so the read ends, and no file is both read and refused.

What still ends a read with exit 1 and a message, no document: an input the reader cannot read at
all (no such directory, a jar named on `CODESPEC_GROOVY_CLASSPATH` that is no file,
`application.properties` naming another Grails version or no `app.name`, no domain class and no
service, a file that cannot be opened); a plain Groovy directory with no `.groovy` file, with files
of two packages or of none, or none of whose files compiles far enough to name its package, the
module's name, in which case the message holds the refusals; a source nested deeper than the
reader's limit; and the JVM running out of memory.

### Classpath and transformations

The compile classpath is the Groovy jar, then the jars `CODESPEC_GROOVY_CLASSPATH` names
(separated by `:`), so a real application can be read with its dependency jars.

```console
CODESPEC_GROOVY_CLASSPATH=<jar>:<jar> codespec read --lang groovy <grails app> --out <file> --adapters target/adapters
```

No AST transformation of those jars is run. Every global transformation they register is disabled,
so no GORM method is injected and every GORM call is visible as written. A local transformation, the
one an annotation names with `@GroovyASTTransformationClass`, is run only when it is a class of the
Groovy jar itself (`@Slf4j`, `@ToString`, the one behind a trait); one of any other jar is not run,
Grails's and its plugins' among them, since they need the running framework and rewrite the code
read. What a caller sees of such an annotation:

- The annotation is a mark: refused at its line, `unsupported: annotation <class>: annotations with
  behaviour are outside cir/1`, as every annotation outside the reader's list is, and the class that
  carries it is read as its source says.
- A member the transformation would have added is not there: a call of it is `foreign`,
  `dynamic: call of <name>, which the type checker does not resolve`.
- Groovy's compiler still loads the transformation's class to see that it is one. When that class
  cannot be loaded, its superclass being on no jar, the artefact is refused as a whole,
  `unresolved: <the JVM's error>`.
- `@grails.transaction.Transactional` is not refused, and each method it covers is read from its
  source body: `TransactionalTransform` is not run, so no identifier of its rewrite reaches the
  module. `@grails.persistence.Entity` on a resolve-only class leaves the class a plain one.

For a service that imports `grails.transaction.Transactional`, put these jars of the Grails 2.5.6
distribution on `CODESPEC_GROOVY_CLASSPATH`: they hold the annotation and the Spring enums its
members name. Without them, such a service is refused as a whole,
`unresolved: grails.transaction.Transactional`.

- `$GRAILS_HOME/dist/grails-core-2.5.6.jar`
- `$GRAILS_HOME/lib/org.springframework/spring-tx/jars/spring-tx-4.1.9.RELEASE.jar`

The reader runs on JDK 17 or later, which no longer ships `javax.activation`, JAXB (`javax.xml.bind`)
and the other Java EE packages a Grails 2 application could take from JDK 8. A class of a jar that
needs one of them is `unresolved: <class>: java.lang.NoClassDefFoundError: javax/activation/…`, and
the artefact naming it is refused, until the jar that holds the package (`activation`, `jaxb-api`)
is on `CODESPEC_GROOVY_CLASSPATH`.

What the Grails layer reads and refuses:

- GORM calls stay `foreign`, `gorm: <name>`, on a domain class, an instance of one or `this` in one:
  `save`, `delete`, `validate`, `refresh`, `merge`, `attach`, `discard`, `get`, `read`, `load`,
  `list`, `count`, `exists`, `find`, `findAll`, `findWhere`, `findAllWhere`, `where`,
  `withCriteria`, `createCriteria`, `executeQuery`, `executeUpdate`, `withTransaction`,
  `withNewSession`, `errors`, `hasErrors`, `first`, `ident`, `getAll`, the implicit `id` and
  `version`, and every `findBy*`, `findAllBy*`, `listOrderBy*`, `countBy*`, `findOrCreateBy*`,
  `findOrSaveBy*`, `addTo*` and `removeFrom*`. The classification wins over `dynamic` and `closure`.
- Domain metadata (`static constraints`, `mapping`, `hasMany`, `belongsTo`, `hasOne`, `transients`,
  `embedded`, `namedQueries`) is no field: one refusal at its line, `gorm: mapping metadata <name>`.
- A domain property of a reference type without an initializer holds `null` until set: it is a
  `ptr(T)` field, zero value `nil` (`JvmTypes.unassignedField`, shared with the Java reader), and is
  used as the Java reader uses such a field. Compared with `null` it is its pointer compared with
  `nil`; assigned `null`, `nil`; assigned another such property of its type, that pointer; assigned
  any other value, a pointer to a fresh local holding it, refused through a receiver that is not a
  name. Every other read dereferences the pointer. A read Groovy would hand on (a return, an
  argument, a local's initializer, an assignment), print (concatenation, a GString), unbox or
  compare (an operator, `+=`, `++`), iterate or switch on (Groovy takes a null as empty or as no
  case), or call `equals` on (Groovy's null answers false), and a collection write to it, are
  refused at the use, `unsupported`: the written program stops at the nil pointer where Groovy goes
  on. A read as any other receiver assumes an object is there.
- A field of a service or domain class that Spring autowires by name is no field: refused at its
  line, `spring: injected dependency`, whatever its initializer, since Spring replaces it. That is a
  field named `grailsApplication`, `dataSource`, `sessionFactory` or `xService`, a
  `javax.sql.DataSource`, or a field of a service type. A call through it is
  `spring: call through an injected dependency`.

`codespec-run-groovy` (`codespec.run.groovy.Main`, on `codespec.run.Runner` in
[`../jvm/src`](../jvm/src/codespec/run/Runner.java)) compiles a Groovy package to classes with the
same Groovy 2.4.10 and makes a `calls.json` against its static methods. The conformance corpus
`conformance/reader/groovy` is each kit case's `groovy/Case.groovy`, else its `java/Case.java`, as
`Case.groovy`: Groovy compiles every Java source unchanged, and step 1 of `codespec conform`
requires each to run to its case's `expected.txt` under Groovy before the reader reads it. The cases
`op-div`, `assign-div` and `sem-int-wrap` carry a `groovy/Case.groovy` that divides with `intdiv`,
since their Java `/` is a BigDecimal division in Groovy.

`codespec-read-groovy --describe` claims every feature whose cases in that corpus all pass, and
`task adapters-check` requires each claimed feature to be used by the reader's output for
`testdata/typed` or for the corpus. The features whose cases the reader refuses are not claimed:
`expr.errvar`, `expr.nil`, `expr.zero`, `sem.error.propagate` and `type.error` (a thrown exception
is not read as an error).

## Refusal categories

Each refusal and each `foreign` reason begins with one of these categories, then `: ` and the detail.

### `dynamic`

The call or property is resolved at run time: the type checker does not resolve it, the class
declares `methodMissing` or `propertyMissing`, the class declares `getProperty` or `setProperty`
(the whole class is refused), the code changes a metaclass, or a `def` local is assigned a value of
another type after its declaration.

```groovy
String label(def item) {
    item.label()          // dynamic: call of label, which the type checker does not resolve
}
```

### `closure`

A closure: functions as values are outside `cir/1`.

```groovy
names.each { n -> total += n.size() }   // closure: a closure passed to each
```

### `groovy-semantics`

The construct means something in Groovy that the same text does not mean in `cir/1`: Groovy truth,
`?.`, `?:`, integer division giving a `BigDecimal`, default parameter values, `String.equals` of a
GString (false in Groovy), a property write through another object that the class's `set<Name>`
receives, a write to a final property outside its own constructor.

```groovy
if (names) {              // groovy-semantics: Groovy truth
    return names[0]
}
int half = count / 2      // groovy-semantics: integer / integer is a BigDecimal division
long share = cents.intdiv(parts)  // groovy-semantics, cents an int and parts a long: divides in the wider type
```

### `type`

A type outside the reader's table, or a reference field with no initializer, which holds `null`
until it is assigned. In a Grails domain class such a property is read as a pointer instead.

```groovy
def rate = 1.5            // type: type java.math.BigDecimal is outside the reader's types
String title              // type: property title holds null until assigned
```

### `unresolved`

A file whose imports or classes do not resolve is refused as a whole, at its first line that does
not; the other files are read. A class a jar holds that the JVM cannot load does not resolve
either, and the reason says why; "Grails 2.5.6 applications" lists every form.

```groovy
import com.example.missing.Gone   // unresolved: com.example.missing.Gone
import com.example.stub.Kind      // unresolved: com.example.stub.Kind: java.lang.ClassFormatError: Absent Code attribute …
```

### `gorm`

A GORM construct in a Grails application: a domain class's metadata (mapping, constraints,
associations), dynamic finders, criteria and persistence methods. Outside a Grails application the
same text is refused under the categories above.

```groovy
static constraints = { title blank: false }   // gorm: mapping metadata constraints
Loan.findAllByMember(member)                  // gorm: findAllByMember on Loan: …
```

### `spring`

A dependency Spring injects into a Grails service by its name, and a call through it.

```groovy
def catalogService                            // spring: injected dependency
FeeService feeService                         // spring: injected dependency
feeService.lateFeeCents(days)                 // spring: call through an injected dependency, …
```

### `unsupported`

Everything else outside the reader's subset: inheritance, interfaces, generic classes, nested
classes, initializer blocks, scripts, exceptions, calls the reader has no mapping for, and, inside a
for-each, a call of a function that may change a collection (Groovy throws
`ConcurrentModificationException`).

```groovy
class Overdue extends Loan { }    // unsupported: class extends ...Loan: inheritance is outside cir/1
```

## Limits

Groovy's parser (ANTLR 2 in Groovy 2.4) does not bound nesting depth: it recurses several levels
per parenthesis. The reader runs on a thread with a 64 MiB stack (`codespec.jvm.Bounded`, shared
with the Java reader); a source nested deeper than that holds is refused with exit 1 and
`codespec-read-groovy: the source nests deeper than the reader's stack of 64 MiB holds`, not a
crash. The reader sets no size limit of its own. `codespec.jvm.BoundsTest` reads a source a
million parentheses deep.

## Assumptions

The document describes the Groovy program only under these assumptions; the reader does not check
them.

- No metaclass is changed at run time, and no category or extension module beyond the GDK is in
  use. Groovy dispatches every call of an unannotated class through its metaclass; the reader takes
  each call to go where the type checker resolves it.
- The overload Groovy picks at run time is the one the type checker picks. Groovy chooses by the
  run-time types of the arguments, the checker by their static types; they differ only where an
  argument's run-time type is a subtype that has its own overload.
- A call list names the method the type checker resolved a call to, in the receiver's static type.
  Where the receiver is an instance of a subclass that overrides the method, the override runs and
  the list does not name it. A call the checker did not resolve names no target at all, so no
  answer over the call graph rests on a target the reader guessed; what such a call reaches is
  unknown, not nothing.
- Every reference parameter of a method is not null.
- The program runs under `Locale.ROOT`: `sprintf` prints ASCII digits.
- Transactions: Grails 2 services are transactional by default, and the reader does not model a
  transaction. A method without a GORM call has no transactional effect to lose; every GORM call is
  kept `foreign`.
