package main

import "strings"

// The reader's claim: the features it was seen to read, outside a `foreign` statement, from the
// Go source of their conformance cases, plus `foreign`, which the stmt-foreign case reads to (a
// `defer` kept verbatim). No fact: facts are computed by `codespec pass`.
//
// claimNot is every feature Go source does not reach, each with its reason (crates/conformance
// src/lib.rs § Cases written by hand; another reader produces it, and its case is written by hand):
//
//   - decl.union: Go has no tagged union; the reader writes nothing for it.
//   - expr.zero: the reader writes a literal (`0`, `""`) or a struct literal that sets no field.
//   - intrinsic.sorted_keys: Go has no expression for a map's keys in order.
//   - type.unit: a call of a function without results is typed as an empty tuple.
//   - stmt.lookup_else: the reader writes a comma-ok `let` and an `if`; only the lookup-guard pass
//     makes them a `lookup_else`, and a reader's claim covers only what the reader prints
//     (docs/conformance.md § 5).
var (
	claimFeatures = []string{
		"assign.*", "decl.*", "expr.*", "intrinsic.*", "op.*", "sem.*", "stmt.*", "type.*", "unop.*",
	}
	claimNot = []string{
		"decl.union", "expr.zero", "intrinsic.sorted_keys", "stmt.lookup_else", "type.unit",
	}
)

// describe is the reader's codespec-adapter/1 manifest, written by hand in the canonical
// encoding of crates/model (two-space indentation, a final newline).
func describe() string {
	return manifest("reader", "go", version, claimFeatures, claimNot)
}

func manifest(role, lang, version string, features, not []string) string {
	var b strings.Builder
	field := func(name, value string) {
		b.WriteString("  \"" + name + "\": \"" + value + "\",\n")
	}
	list := func(name string, ids []string, last bool) {
		b.WriteString("  \"" + name + "\": [\n")
		for i, id := range ids {
			b.WriteString("    \"" + id + "\"")
			if i < len(ids)-1 {
				b.WriteString(",")
			}
			b.WriteString("\n")
		}
		b.WriteString("  ]")
		if !last {
			b.WriteString(",")
		}
		b.WriteString("\n")
	}
	b.WriteString("{\n")
	field("format", "codespec-adapter/1")
	field("role", role)
	field("lang", lang)
	field("version", version)
	field("cir", "cir/1")
	list("features", features, len(not) == 0)
	if len(not) > 0 {
		list("not", not, true)
	}
	b.WriteString("}\n")
	return b.String()
}
