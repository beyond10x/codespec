---
format: aep.planning-md/3
id: story:mcp-apply-in-place
kind: story
status: draft
title: 'MCP apply: write a previewed change into the original files, then read it back'
relations:
- decomposes: epic:agent-mcp-server
- depends_on: story:mcp-rewrite-preview
- depends_on: story:edit-cli-verified-apply
- depends_on: story:edit-imports
- depends_on: story:edit-spans-c-reader
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: crates/mcp
revision: 6
---
# Story: MCP apply: write a previewed change into the original files

## Outcome

The tool `apply` takes the id of a preview (`story:mcp-rewrite-preview`) and writes it through
`codespec_edit::run`, the apply of `codespec edit --apply` (`story:edit-cli-verified-apply`): the change was
verified on a cache copy when it was previewed, every file must still have the SHA-256 it was read
with, and each file is written through a temporary file and a rename. Only the edited spans and,
for a template naming another module, the import range (`story:edit-imports`) change. `apply` is
listed only when the server was started with `--write`.

## Acceptance

- After `apply` of a rename preview on a fixture in each of Go, Rust and C (C through
  `story:edit-spans-c-reader`): `git diff --stat` names only the files of the diff, a function not
  matched is byte-identical, and the fixture builds; one test per language.
- `apply` of a `replace_preview` id on the Go fixture writes that one function's body.
- A preview whose verification failed cannot be applied: `apply` refuses it and writes nothing.
- `apply` with an id whose file changed on disk since the preview is refused and writes nothing; an
  unknown id is refused.
- A Go template calling `strings.ToUpper` in a file that does not import `strings`: after `apply`
  the file imports it and `go build` passes.
- A Java preview (no spans) has nothing to apply: `apply` refuses it naming the reader.
- Writes stay under `--root`: a test where a file under the root is a symbolic link to a file
  outside it asserts a refusal and that the outside file's hash is unchanged.
- Without `--write`, `tools/list` does not name `apply` and a `tools/call` of it is the unknown
  tool error; with `--write` it is listed.

## Scope

- Own: `crates/mcp/src/tools/apply.rs`, the tool table in `crates/mcp/src/lib.rs`, `crates/cli`
  (the `--write` flag), `CHANGELOG.md`. Runs after `story:mcp-rewrite-preview`,
  `story:edit-cli-verified-apply`, `story:edit-imports` and `story:edit-spans-c-reader`. Inferred.

Restated on `codespec edit` on 2026-11-24 (`epic:code-edit-language`); before, it replaced
function spans of `story:reader-function-spans` (superseded) with the writer's text and restored a
file that read back differently.
