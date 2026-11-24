---
format: aep.planning-md/3
id: story:mcp-rewrite-preview
kind: story
status: draft
title: 'MCP rewrite preview: a rule in, a diff out, nothing written'
relations:
- decomposes: epic:agent-mcp-server
- depends_on: story:mcp-semantic-tools
- depends_on: story:edit-replace-template
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/mcp
revision: 4
---
# Story: MCP previews: an edit program or a replaced function body in, a diff out, nothing written

## Outcome

Two tools answer in the same form and write nothing to disk. `rewrite_preview` takes a cq program
with `edit` items (`docs/code-edit-language.md` § 3). `replace_preview` takes the id of one function
and a new body as source text of its module's language, run as a `replace` of the body's span
(`story:edit-replace-template`). Both run the dry run of `codespec edit` through `crates/edit`,
verification included, and answer: per row its status, the unified diff of the original files
(span edits, formatting outside the edited spans kept), and every refused row with its reason (a
reference in unread code, a language whose reader gives no spans, a failed verification). The
answer carries an id that names this preview.

## Acceptance

- On a fixture, a program renaming `F` to `G` previews every call site and no other function; the
  diff equals a checked-in expected diff and `git apply --check` accepts it.
- `replace_preview` with a body that does not read as a body of that function's type is answered as
  refused with the reader's message; with a valid body it answers the diff of that one function.
- A rename whose target is called inside a `foreign` statement is answered as refused naming it.
- A preview over a Java set answers "the java reader gives no spans" until
  `story:edit-spans-java-reader`, and changes nothing.
- A test compares the tree's file hashes before and after both previews: equal.
- Each answer carries an id; two previews in one session have different ids, and a preview's id is
  stable when the same preview is asked for again over unchanged files.

## Scope

- Own: `crates/mcp/src/tools/` (two new files) and the tool table in `crates/mcp/src/lib.rs`,
  `CHANGELOG.md`. Runs after `story:mcp-semantic-tools` (the shared tool table is the only reason
  for that order). Depends on `story:edit-replace-template` for templates; calls `crates/edit`.
  Inferred.

Restated on `codespec edit` on 2026-11-24 (`epic:code-edit-language`); before, previews were a
`cir/1` template written back through the writers (`story:cir-rewrite`, superseded).
