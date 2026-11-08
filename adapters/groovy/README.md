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
`testdata/lending.read.cir.json` is that read.

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
  resolves. A Java file javac reports an error in is left out. The Java files do not see the Groovy
  files.
- A resolve-only file that declares a GORM entity with `@grails.persistence.Entity` is left out of
  the compilation: its local transformation needs the whole Grails runtime and would otherwise fail
  the read of every file, and the class is never read. A read file that names it is refused
  `unresolved`.
- Positions name a file by its path in the application: `grails-app/services/lending/LoanService.groovy:15`.
- Two artefacts with the same simple name in different packages are both refused,
  `unsupported: duplicate simple name`: one module has one namespace.
- An artefact whose imports do not resolve is refused as a whole at its first unresolved line,
  `unresolved: <class>`; the others are read.

The compile classpath is the Groovy jar, then the jars `CODESPEC_GROOVY_CLASSPATH` names
(separated by `:`), so a real application can be read with its dependency jars. Every global AST
transformation those jars register is disabled, so no GORM method is injected and every GORM call
is visible as written.

```console
CODESPEC_GROOVY_CLASSPATH=<jar>:<jar> codespec read --lang groovy <grails app> --out <file> --adapters target/adapters
```

For a service that imports `grails.transaction.Transactional`, put these jars of the Grails 2.5.6
distribution on `CODESPEC_GROOVY_CLASSPATH`. The first two hold the annotation, its transformation
and the Spring enums its members name; the transformation loads classes from the other four as it
runs.

- `$GRAILS_HOME/dist/grails-core-2.5.6.jar`
- `$GRAILS_HOME/lib/org.springframework/spring-tx/jars/spring-tx-4.1.9.RELEASE.jar`
- `$GRAILS_HOME/lib/javax.annotation/jsr250-api/jars/jsr250-api-1.0.jar`
- `$GRAILS_HOME/lib/org.springframework/spring-beans/jars/spring-beans-4.1.9.RELEASE.jar`
- `$GRAILS_HOME/lib/org.springframework/spring-core/jars/spring-core-4.1.9.RELEASE.jar`
- `$GRAILS_HOME/dist/grails-bootstrap-2.5.6.jar`

Without them, such a service is refused as a whole, `unresolved: grails.transaction.Transactional`.
With them, `@Transactional`'s local transformation, `TransactionalTransform`, runs at
`CANONICALIZATION` (disabling global transformations does not turn it off) and rewrites each method
it covers into a call through a transaction template. The reader never reads a rewritten method:
each is refused, `unsupported: grails transform`, and no identifier the rewrite introduces reaches
the module. A method counts as rewritten when the transform moved its body: an overload it did not
cover, with other parameter types, is read or refused on its own.

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

A file whose imports or classes do not resolve is refused as a whole, at its first line; the other
files are read.

```groovy
import com.example.missing.Gone   // unresolved: com.example.missing.Gone
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

## Assumptions

The document describes the Groovy program only under these assumptions; the reader does not check
them.

- No metaclass is changed at run time, and no category or extension module beyond the GDK is in
  use. Groovy dispatches every call of an unannotated class through its metaclass; the reader takes
  each call to go where the type checker resolves it.
- The overload Groovy picks at run time is the one the type checker picks. Groovy chooses by the
  run-time types of the arguments, the checker by their static types; they differ only where an
  argument's run-time type is a subtype that has its own overload.
- Every reference parameter of a method is not null.
- The program runs under `Locale.ROOT`: `sprintf` prints ASCII digits.
- Transactions: Grails 2 services are transactional by default, and the reader does not model a
  transaction. A method without a GORM call has no transactional effect to lose; every GORM call is
  kept `foreign`.
