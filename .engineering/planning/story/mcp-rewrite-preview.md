---
format: aep.planning-md/3
id: story:mcp-rewrite-preview
kind: story
status: draft
title: 'MCP rewrite preview: a rule in, a diff out, nothing written'
relations:
- decomposes: epic:agent-mcp-server
- depends_on: story:cir-rewrite
- depends_on: story:mcp-semantic-tools
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/mcp
revision: 3
---
# Story: MCP previews: a rewrite or a replaced function body in, a diff out, nothing written

## Outcome

Two tools answer in the same form and write nothing to disk.
`rewrite_preview` takes a rewrite as `story:cir-rewrite` defines it (a cq query that selects
nodes, and a `cir/1` template). `replace_preview` takes the name of one function and a new body
as `cir/1`. Each answers, per changed module: the `cir/1` of each changed function before and
after, and a unified diff between the source the writer gives for the module before and after.
Functions that matched but hold a refusal, modules whose language has no writer, and a result
that strict reading or the writer refuses are listed as not changed, each with the reason. The
answer carries an id that names this preview.

## Acceptance

- On a fixture, the rewrite renaming a call of `a.F` to `a.G` previews every call site and no
  other function; the answer's "after" documents pass strict reading.
- The diff of that preview equals a checked-in expected diff, and applying it with `patch` to the
  writer's "before" text gives the writer's "after" text, by a test.
- `replace_preview` with a body that strict reading refuses (a call of an undeclared function) is
  answered as not changed with strict reading's message; with a valid body it answers the diff for
  that one function, and no other function appears in the answer.
- A match inside a function with a `foreign` statement is listed under "not changed" with that
  refusal, by a test.
- A preview over a Java set answers "no writer for java" for every match and changes nothing.
- A test compares the tree's file hashes before and after both previews: equal.
- Each answer carries an id; two previews in one session have different ids, and a preview's id
  is stable when the same preview is asked for again over unchanged files.

## Scope

- Own: `crates/mcp/src/tools/` (two new files) and the tool table in `crates/mcp/src/lib.rs`,
  `CHANGELOG.md`. Runs after `story:mcp-semantic-tools` (the shared tool table is the only reason
  for that order). Depends on `story:cir-rewrite` for the rewrite engine. Inferred.
