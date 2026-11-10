---
format: aep.planning-md/3
id: story:mcp-semantic-tools
kind: story
status: draft
title: 'MCP tools for meaning: cq queries, callers and callees, effects'
relations:
- decomposes: epic:agent-mcp-server
- depends_on: story:mcp-server-read-tools
- depends_on: story:query-core
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/mcp
revision: 3
---
# Story: MCP tools for meaning: cq queries, callers and callees, effects

## Outcome

Three tools over the read set: `query` (a cq query, answered as rows), `calls` (callers and
callees of one function through the linked set, to a stated depth), and `effects` (what a function
reaches: the report `codespec effects` gives, for one function or the set).

## Acceptance

- `query` answers a cq query from `docs/query-language.md` over `examples/inventory` with the
  rows `codespec query` prints for the same query and documents; a cq error is an MCP tool error
  carrying the position in the query.
- `calls` on a fixture of two linked modules lists a caller in the other module; a call through a
  function value or an interface is listed as unresolved with its position, never omitted.
- `effects` for one function equals the entry for that function in `codespec effects` over the
  same set, and `effects` with no function equals the whole report of `codespec effects` over the
  set; one test each.
- A query or a call graph walk has a stated budget of steps and rows; past it the answer says it
  is cut, by a test with a generated set.

## Scope

- Own: `crates/mcp/src/tools/` (three new files) and the tool table in `crates/mcp/src/lib.rs`,
  `CHANGELOG.md`. Runs after `story:mcp-server-read-tools`, which creates the crate. Reads
  `crates/query` and `crates/effects` through their library interfaces; changes neither. Inferred.
