---
format: aep.planning-md/3
id: story:edit-replace-template
kind: story
status: draft
title: 'Template edits: replace, replace_stmt, insert_before, insert_after'
summary: Source-language templates with $X holes, proved local by re-read
relations:
- decomposes: epic:code-edit-language
- depends_on: story:edit-cli-verified-apply
- supersedes: story:cir-rewrite
revision: 1
---
# Story: Template edits: replace, replace_stmt, insert_before, insert_after

## Outcome

`replace(E, template)`, `replace_stmt(S, template)`, `insert_before(S, template)` and
`insert_after(S, template)` (`docs/code-edit-language.md` § 3.2 examples 6 and 7). A template is
source text of the module's language; `$X` is the source text of the node bound to the rule
variable `X`. Readers give `node` spans of expressions. Verification proves the edit local: the
re-read set equals the original outside the replaced nodes (positions shifted), and each replaced
node reads as the template read as a fragment with its holes filled. Supersedes
`story:cir-rewrite`: the replacement lands in the original file's span instead of a module
re-emitted by the Go writer.

## Done When

- Examples 6 and 7 on the Go example, and a Rust `?` wrap, give checked-in diffs, verify and build;
  a template that does not read as an expression of the replaced node's type is refused.

## Scope

- `crates/edit`, `crates/query` (template holes), `adapters/go/cmd/codespec-read-go`,
  `adapters/rust-read` (expression spans, fragment reading), `examples/edit`. Inferred.
