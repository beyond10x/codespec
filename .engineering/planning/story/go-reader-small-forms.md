---
format: aep.planning-md/3
id: story:go-reader-small-forms
kind: story
status: active
title: The Go reader lowers small literal and builtin forms
summary: Positional and slice literals, make of a map, new, empty extern literals, nil arguments, extern constants.
relations:
- decomposes: epic:go-reader-refusals
- supersedes: story:go-reader-make-map
- informed_by: story:go-reader-folds-constants
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/README.md
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/small_forms_test.go
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:53Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:53Z", actor: "human:timo", revision: 6}
---
# Story: The Go reader lowers small literal and builtin forms

## Outcome

Each of these reads without a refusal, as an existing `cir/1` form of the same meaning:

- a positional struct literal `T{1, 2}`: the struct literal naming each field in declaration
  order;
- a slice literal with elements `[]T{a, b}`: `append(make []T, a, b)`;
- `make(map[K]V)`, `make(map[K]V, n)`, `make([]T, 0)`, `make([]T, 0, n)` with `n` a constant or
  `len` of a variable: `make {t}`;
- `new(T)`: `&T{}` for a struct `T`, `&zero {t: T}` otherwise;
- an empty literal of another package's struct (`sync.WaitGroup{}`, `&http.Client{}`):
  `zero {t: extern}`;
- an untyped `nil` passed as an argument, a struct field or a slice element: typed from the
  parameter, field or element;
- a constant of another package of a basic type (`http.StatusOK`): the literal of its value.

Takes over `story:go-reader-make-map` (archived as folded into this story) and the
qualified-constant part of `story:go-reader-folds-constants`.

## Done When

- `adapters/go/cmd/codespec-read-go/small_forms_test.go`: each form above reads with no refusal
  and equal (but for positions) to its hand-written form; `make([]T, n)`, `make(chan T)`, a
  constant of an extern named type (`time.Second`) and a keyed slice literal stay refused with
  their shape.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`expr` composite literal and selector, `call`
  builtins and arguments), `README.md`, new `small_forms_test.go`.
