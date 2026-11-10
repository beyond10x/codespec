---
format: aep.planning-md/3
id: epic:agent-mcp-server
kind: epic
status: draft
title: 'codespec as an MCP server: read code by structure and meaning, change it by transformation'
relations:
- depends_on: epic:codebase-transformations
- depends_on: epic:cir-evaluator
- depends_on: epic:adapters-untrusted-input
revision: 4
---
# Epic: codespec as an MCP server: agents read code by structure and meaning, and change it by transformation

## Outcome

`codespec mcp` is a Model Context Protocol server over stdio. An agent connected to it reads a
codebase as `cir/1` instead of as text (modules, types, functions with signatures, calls, effects,
and every refusal with its position), asks questions in cq, and changes code by a rewrite (a cq
query that selects nodes and a `cir/1` template, `story:cir-rewrite`) or by one replaced function
body given as `cir/1`, instead of by text edit. Every change, of either kind, is previewed first:
the preview is checked by strict reading and shown as a diff of source text, and only a previewed
change can be written. It is written only to the bytes of the functions it changed and, when a changed
function needs a new import, to the import lines of its file; it is checked after: the changed file is read again and must give the `cir/1` the preview showed.

The server writes nothing unless it was started with `--write`: without the flag the writing
tools are not listed. An agent that only reads cannot change a file by mistake.

## Why Now

Filed from the operator's idea (2026-11-10): "mcp to read (structured, semantically) and modify
code (transformations)". The parts exist or are planned as CLI verbs: readers for Go, Rust, Java,
Groovy and C (`codespec read`), linked sets and effects (`story:link-modules`), cq
(`story:query-core`, `docs/query-language.md`), the evaluator (`epic:cir-evaluator`), and rewrites
(`story:cir-rewrite` in `epic:codebase-transformations`; `docs/query-language.md` § 9 item 6). What
is missing is one long-lived process that holds a read set, serves these as tools with bounded
answers, and writes a change back into the original files.

## What an agent gets that text tools do not give

- A refusal list: the server states which lines it did not read. An answer about a function with a
  refusal says so; a rewrite never touches it.
- Answers by meaning: callers and callees through the linked set, effects (which functions reach
  the file system, the network, a process), not text matches.
- Changes that are checked: strict reading of the result, the writer's own refusals, the re-read
  comparison, and for a change claimed to keep behaviour a differential run in the evaluator.

## Out of Scope

- A transport other than stdio, authentication, a hosted service.
- Free-form text edits: the server changes code only through a rewrite or a replaced function body
  given as `cir/1`; anything else stays with the agent's text tools.
- Preserving formatting or comments inside a changed function: its text is the writer's. Code
  outside the changed functions and the import lines stays byte-identical.
- Writing a language that has no writer (Java, Groovy today) or whose reader gives no function
  spans (`story:reader-function-spans` gives them for Go, Rust and C): read tools work, write
  tools refuse, naming which of the two is missing.

## Stories, in the order they are worked

They share the crate `crates/mcp`, so they are worked one after another, each depending on the one
before: `story:mcp-server-read-tools`, `story:mcp-semantic-tools`, `story:mcp-rewrite-preview`,
`story:mcp-apply-in-place`, `story:mcp-verify-change`. `story:reader-function-spans` edits the
reader adapters, `docs/design.md`, `crates/cli` and `CHANGELOG.md`; it runs after
`story:mcp-server-read-tools` (shared `crates/cli`) and can be worked beside the second and third.

## Done When

- Each story is implemented, and the recorded session that `story:mcp-verify-change` owns passes:
  a scripted MCP client against `examples/series` does read, outline, one cq query, one rewrite
  previewed, applied, re-read equal, and verified in the evaluator.

## Open questions (not decided here)

- Whether the server keeps the read set in memory only or also on disk between sessions. The
  first story holds it in memory.
- Function spans are reported beside the `cir/1` document, not inside it
  (`story:reader-function-spans`); putting spans into the model would be `cir/2`.
