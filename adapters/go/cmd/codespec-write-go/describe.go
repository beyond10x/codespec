package main

import "strings"

// The writer's claim: every feature whose conformance case it writes and codespec-run-go runs to
// the case's expected output (the acceptance test of adapters/rust-run); less the features whose
// case it refuses: a union, which Go has no form for.
var (
	claimFeatures = []string{
		"assign.*", "decl.*", "expr.*", "fact.*", "intrinsic.*", "op.*", "sem.*", "stmt.*", "type.*",
		"unop.*",
	}
	claimNot = []string{"decl.union"}
)

// describe is the writer's codespec-adapter/1 manifest, written by hand in the canonical
// encoding of crates/model (two-space indentation, a final newline).
func describe() string {
	return manifest("writer", "go", version, claimFeatures, claimNot)
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
