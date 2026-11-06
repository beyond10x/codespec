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

`testdata/typed` is a package read with no refusal: an unannotated class, a `@CompileStatic` class
and an enum. Each directory under `testdata/refuse` holds one construct the reader keeps as
`foreign` or refuses; its line is marked `// refused <category>`. `testdata/lending` is a small
Grails application, read in part by this reader; `testdata/lending/EXPECTED.md` is what the Grails
layer is to read of it.

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
```

### `type`

A type outside the reader's table, or a reference field with no initializer, which holds `null`
until it is assigned.

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

A GORM construct: a domain class's mapping and constraints, dynamic finders, criteria and
persistence methods. This reader refuses them under the categories above; the Grails layer names
them `gorm`.

```groovy
static constraints = { title blank: false }
Loan.findAllByMember(member)
```

### `spring`

A dependency Spring injects into a Grails artefact by its name. This reader refuses it under the
categories above; the Grails layer names it `spring`.

```groovy
def catalogService
FeeService feeService
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
