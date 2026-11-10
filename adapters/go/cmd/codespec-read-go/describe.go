package main

import "strings"

// The reader's claim: the features it was seen to read, outside a `foreign` statement, from the
// Go source of their conformance cases, plus `foreign`, which the stmt-foreign case reads to (a
// `defer` kept verbatim). No fact: facts are computed by `codespec pass`.
//
// claimNot is every feature whose kit case this reader does not read to it, each with its reason
// (README.md § Features left unclaimed):
//
//   - decl.union: Go has no tagged union.
//   - expr.zero: this reader never writes `zero`; it reads a zero value as the Go source writes
//     it: a literal (`0`, `""`, `false`), a struct literal that sets no field, `var x T`.
//   - intrinsic.sorted_keys: this reader reads `slices.Sorted(maps.Keys(m))` as sorted_keys over
//     integer keys only (Go orders strings by byte, sorted_keys by UTF-16 code unit); the kit
//     case's Go source collects the keys in a loop and sorts them with `sort.Strings` and
//     `sort.Ints`, which it does not read as sorted_keys.
//   - type.unit: Go gives a call of a function without results no type; this reader types it as
//     an empty tuple.
//   - stmt.lookup_else: this reader writes a comma-ok `let` and an `if`; only the lookup-guard
//     pass makes them a `lookup_else`, and a reader's claim covers only what the reader prints
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
