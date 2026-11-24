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
- informed_by: epic:code-edit-language
revision: 10
---
# Epic: codespec as an MCP server: agents read code by structure and meaning, and change it by transformation

## Outcome

`codespec mcp` is a Model Context Protocol server over stdio. An agent connected to it reads a
codebase as `cir/1` instead of as text (modules, types, functions with signatures, calls, effects,
and every refusal with its position), asks questions in cq, and changes code by a cq program with
`edit` items or by one replaced function body given as source text, instead of by text edit. Both
run through `codespec_edit::run`, the library behind `codespec edit` (`epic:code-edit-language`,
`docs/code-edit-language.md`). Every change is previewed first: the preview is verified on a cache
copy (the re-read must give the `cir/1` the operation predicts) and shown as a unified diff of the
original files, and only a previewed change can be written. It is written only to the edited spans
and, when a template names another module, to the import range of its file; every other byte stays.

The server writes nothing unless it was started with `--write`: without the flag the writing
tools are not listed. An agent that only reads cannot change a file by mistake.

Restated on 2026-11-24 against `codespec edit`; before, changes were a `cir/1` template
(`story:cir-rewrite`, superseded) written back as the writer's text.

## Why Now

Filed from the operator's idea (2026-11-10): "mcp to read (structured, semantically) and modify
code (transformations)". The parts exist or are planned as CLI verbs: readers for Go, Rust, Java,
Groovy and C (`codespec read`), linked sets and effects (`story:link-modules`), cq
(`story:query-core`, `docs/query-language.md`), the evaluator (`epic:cir-evaluator`), and edits
(`codespec edit`, `epic:code-edit-language`). What is missing is one long-lived process that holds
a read set, serves these as tools with bounded answers, and writes a change back into the original
files.

## What an agent gets that text tools do not give

- A refusal list: the server states which lines it did not read. An answer about a function with a
  refusal says so; an edit never touches it.
- Answers by meaning: callers and callees through the linked set, effects (which functions reach
  the file system, the network, a process), not text matches.
- Changes that are checked: the refusals of `codespec edit` (a reference in unread code, an
  exported target without `--closed-world`), the re-read comparison on a cache copy, and for a
  change claimed to keep behaviour a differential run in the evaluator.

## Out of Scope

- A transport other than stdio, authentication, a hosted service.
- Free-form text edits: the server changes code only through a cq `edit` program or a replaced
  function body; anything else stays with the agent's text tools.
- Re-formatting: the edited spans hold the operation's text, every other byte stays (Go's `gofmt`
  of a file that was `gofmt`-clean aside, as `codespec edit` does it).
- Writing a language whose reader gives no spans (Java and Groovy until
  `story:edit-spans-java-reader`; C until `story:edit-spans-c-reader`): read tools work, write
  tools refuse, naming the reader.

## Stories, in the order they are worked

They share the crate `crates/mcp`, so they are worked one after another, each depending on the one
before: `story:mcp-server-read-tools`, `story:mcp-semantic-tools`, `story:mcp-rewrite-preview`,
`story:mcp-apply-in-place`, `story:mcp-verify-change`. The write tools call `codespec_edit::run`
and wait for the stories of `epic:code-edit-language` they name (`story:edit-replace-template`,
`story:edit-cli-verified-apply`, `story:edit-imports`, `story:edit-spans-c-reader`).

## Done When

- Each story is implemented, and the recorded session that `story:mcp-verify-change` owns passes:
  a scripted MCP client against `examples/series` does read, outline, one cq query, one rewrite
  previewed, applied, re-read equal, and verified in the evaluator.

## Open questions (not decided here)

- Whether the server keeps the read set in memory only or also on disk between sessions. The
  first story holds it in memory.
- Spans are a side document beside the `cir/1` document, `codespec-spans/1`
  (`docs/code-edit-language.md` § 6.2, decision D1 of 2026-11-24); `cir/1` is unchanged.
