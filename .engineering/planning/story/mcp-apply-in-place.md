---
format: aep.planning-md/3
id: story:mcp-apply-in-place
kind: story
status: draft
title: 'MCP apply: write a previewed change into the original files, then read it back'
relations:
- decomposes: epic:agent-mcp-server
- depends_on: story:mcp-rewrite-preview
- depends_on: story:reader-function-spans
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: crates/mcp
revision: 4
---
# Story: MCP apply: write a previewed change into the original files, then read it back

## Outcome

The tool `apply` takes the id of a preview (`story:mcp-rewrite-preview`: a rewrite or a replaced
function body) and writes it into the source files: for each changed function, the bytes of its
span (`story:reader-function-spans`) are replaced by the writer's text for that function. The
one other place it may write is the file's import range from the same span report: when the
written module imports something the file does not, that import is added there. Every byte
outside the changed functions' spans and the import range is kept. The writer's text for one
function is cut from the written module by the same span report, read from the written text.
Then the server reads each changed file again and requires the `cir/1` of each changed function
to equal the preview's "after"; a file that fails this is restored byte-identical and reported.
`apply` is listed only when the server was started with `--write`.

## Acceptance

- After `apply` of the rename rewrite on a fixture in each of Go, Rust and C: `git diff --stat`
  names only the files of changed functions, a function not matched is byte-identical, and the
  fixture builds and its harness run passes; one test per language.
- `apply` of a `replace_preview` id on the Go fixture writes that one function, and the re-read
  function equals the body the agent gave.
- The read-back check has a red case: a fixture rewrite whose result the writer writes with a
  name the file already uses reads back differently; the test asserts that the file is restored
  byte-identical and that the answer names the function.
- `apply` with an id whose file changed on disk since the preview is refused and writes nothing;
  an unknown id is refused.
- A Go fixture rewrite whose template calls `strings.ToUpper` in a file that does not import
  `strings`: after `apply` the file imports it and `go build` passes.
- A module whose reader gives no spans (Java) is refused by `apply` naming the missing span
  report, and nothing is written.
- Writes stay under `--root`: a test where a file under the root is a symbolic link to a file
  outside it asserts a refusal and that the outside file's hash is unchanged.
- Without `--write`, `tools/list` does not name `apply` and a `tools/call` of it is the unknown
  tool error; with `--write` it is listed.

## Scope

- Own: `crates/mcp/src/tools/apply.rs`, `crates/mcp/src/spans.rs` (loading the span report into
  the read set), the tool table in `crates/mcp/src/lib.rs`, `crates/cli` (the `--write` flag),
  `CHANGELOG.md`. Runs after `story:mcp-rewrite-preview` and `story:reader-function-spans`.
  Inferred. `cir/1` itself is not changed.
