# What reading the lending fixture gives

A neighbourhood tool-lending library written as a legacy Grails 2.5.6 application, for the Groovy
reader. Only the domain classes (every file under `grails-app/domain`) and the services (the
`*Service.groovy` files under `grails-app/services`) are read. `ToolController.groovy`,
`src/groovy` and `grails-app/utils` are resolve-only: compiled so references resolve, never read,
and they carry no marker.

`crates/cli/tests/fixture.rs` checks this file against the markers in the source.

## Markers

Every marker is a trailing comment on one line:

- `// expect: read` on a method declaration: the function is read, and no line in its body is
  marked refused. On any other line (a property, a statement) it names a construct that is read.
- `// expect: refused <category>` on any line: one refusal at that line, in that category. On a
  method declaration it also marks the function refused.
- `// expect: refused` without a category, on a method declaration only: the function is refused,
  and its refusals are the lines of its body marked `// expect: refused <category>`.

Every method declaration carries one of these: `def`, typed, or untyped with a modifier only. The
categories are the closed list of the Groovy reader: `dynamic`, `closure`, `groovy-semantics`,
`type`, `unresolved`, `gorm`, `spring`, `unsupported`. A refusals line below names each category
once.

A refused function is one whose body holds a `foreign` statement, or which is refused at its
declaration. A refusal outside every function (a property, domain metadata, an injected field, an
import) is counted under its file and category, not as a function.

An import marked `// expect: refused unresolved` refuses its artefact as a whole: the artefact
yields that one refusal and no function, read or refused. `TransactionalAuditService.groovy` is
such an artefact when it is read with the Groovy jar alone. Its other markers describe the read
with the Grails 2.5.6 jars on the classpath and are not counted here.

## The shared JVM rules

The Groovy reader reads a construct as the Java reader does where the construct is shared
(`story:groovy-reader-core`), so a function marked `read` holds only what those rules read. Four
constructs common in legacy code are kept and marked refused with `unsupported`, the category for
anything outside the other seven, each next to a readable twin:

- A conditional expression `c ? a : b`, in `Loan.daysLate`. Twin: `Loan.daysOverdue`, an
  `if`/`else`.
- `long * int`, an implicit widening, in `Tool.feeFor` and `FeeService.lateFeeCents`. Twins:
  `Tool.feeForSpan` and `FeeService.lateFeeForDays`, `long` operands.
- A `for` over a `TreeMap` parameter, whose comparator the reader cannot see, and `Map.get` of an
  `Integer` value, which gives `null` for an absent key, in `CatalogService.sumByShelf`. Twin:
  `CatalogService.shelfList`, a `for` over the keys of the field `shelfNotes`, built with
  `new TreeMap<>()` and holding `String` values.
- A call of `DueDates`, a class in `grails-app/utils`, which is resolve-only and so not read, in
  `LoanService.dueDayFor`. No twin.

The `DueDates` call resolves, so it is not `unresolved`; it is refused because its target is not
read. `story:grails-artefacts` promises only that the method holding it "is not refused as
`unresolved`", which this keeps.

Two more choices the source leaves open, made here:

- A GString with a `boolean` value (`FeeService.overdueLine`) is `groovy-semantics`: a GString is
  read only when every value is a `String` or an integer.
- Where one line holds two constructs, the marker names the one that wins: `gorm` over `closure`
  for `Tool.withCriteria { }`, `dynamic` over the String value assigned through `metaClass`,
  `spring` over the property read after a call through an injected service.

## Totals

| file | read | refused | dynamic | closure | groovy-semantics | type | unresolved | gorm | spring | unsupported |
|---|---|---|---|---|---|---|---|---|---|---|
| `grails-app/domain/lending/Loan.groovy` | 2 | 1 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 1 |
| `grails-app/domain/lending/LoanStatus.groovy` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `grails-app/domain/lending/Member.groovy` | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 |
| `grails-app/domain/lending/Tool.groovy` | 3 | 2 | 1 | 0 | 0 | 0 | 0 | 3 | 0 | 1 |
| `grails-app/services/lending/CatalogService.groovy` | 2 | 11 | 3 | 2 | 2 | 0 | 0 | 3 | 0 | 1 |
| `grails-app/services/lending/FeeService.groovy` | 5 | 4 | 0 | 0 | 3 | 0 | 0 | 0 | 0 | 1 |
| `grails-app/services/lending/LoanService.groovy` | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 4 | 4 | 1 |
| `grails-app/services/lending/TransactionalAuditService.groovy` | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| total | 14 | 24 | 4 | 2 | 5 | 0 | 1 | 14 | 4 | 5 |

## Per artefact

### `grails-app/domain/lending/Loan.groovy`

- read: `daysOverdue`, `isOverdue`
- refused: `daysLate`
- refusals: gorm 2, unsupported 1

### `grails-app/domain/lending/LoanStatus.groovy`

- read: none
- refused: none
- refusals: none

### `grails-app/domain/lending/Member.groovy`

- read: `canBorrow`, `greeting`
- refused: none
- refusals: gorm 2

### `grails-app/domain/lending/Tool.groovy`

- read: `feeForSpan`, `markLent`, `shelfTag`
- refused: `feeFor`, `methodMissing`
- refusals: dynamic 1, gorm 3, unsupported 1

### `grails-app/services/lending/CatalogService.groovy`

- read: `doubled`, `shelfList`
- refused: `countLent`, `describe`, `installShelfDefault`, `nameOr`, `names`, `onShelf`, `shelfCount`, `shelfOf`, `sumByShelf`, `toolById`, `toolNamed`
- refusals: dynamic 3, closure 2, groovy-semantics 2, gorm 3, unsupported 1

### `grails-app/services/lending/FeeService.groovy`

- read: `countOverdue`, `feeForStatus`, `lateFeeForDays`, `receipt`, `sumLateFees`
- refused: `lateFeeCents`, `loansText`, `overdueLine`, `splitFeeText`
- refusals: groovy-semantics 3, unsupported 1

### `grails-app/services/lending/LoanService.groovy`

- read: none
- refused: `dueDayFor`, `forget`, `lateFee`, `lend`, `loansOf`, `toolName`
- refusals: gorm 4, spring 4, unsupported 1

### `grails-app/services/lending/TransactionalAuditService.groovy`

- read: none
- refused: none
- refusals: unresolved 1
