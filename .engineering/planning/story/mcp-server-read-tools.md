---
format: aep.planning-md/3
id: story:mcp-server-read-tools
kind: story
status: draft
title: 'codespec mcp: a stdio server with the read tools'
relations:
- decomposes: epic:agent-mcp-server
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/cli
- confidence: inferred
  path: crates/mcp
- confidence: inferred
  path: website/docs
revision: 3
---
# Story: codespec mcp: a stdio server with the read tools

## Outcome

`codespec mcp --root <dir>` speaks MCP (JSON-RPC 2.0 over stdio: `initialize`, `tools/list`,
`tools/call`) and serves four tools over one read set held in the process:
`read_set` (read a directory under the root with the reader for its language, link what links),
`outline` (modules, types and functions with signatures and `file:line`),
`show_function` (one function as `cir/1` JSON, with its refusals), and
`refusals` (every refusal of the set, by position, with its reason).

## Acceptance

- A test starts the binary, sends `initialize`, `tools/list` and one `tools/call` per tool over
  stdio against `examples/series`, and compares each answer with a checked-in expected file;
  `tools/list` gives each tool a JSON Schema for its input.
- A path outside `--root` (absolute, `..`, or through a symbolic link) is refused with an MCP tool
  error naming the path, by a test for each of the three forms.
- The server writes no file: a test hashes every file under the root and under the test's
  temporary home before and after a session that calls every tool, and the hashes are equal. It
  opens no network connection: a test over `cargo metadata` asserts that `crates/mcp` and what it
  adds to the dependency graph hold no crate from a checked-in list of network crates, and the
  crate's source names no `std::net`.
- Every answer is bounded: a stated limit on bytes per answer and a `cursor` to continue; a test
  reads a set whose outline exceeds the limit and pages through it, and the pages joined equal the
  unpaged outline.
- `show_function` of a function holding a `foreign` statement lists that refusal in the same answer.
- Malformed JSON, an unknown method and an unknown tool each get the JSON-RPC error the protocol
  names, and the server keeps running; a test sends all three and then a good call.

## Scope

- Own: a new crate `crates/mcp` (each tool in its own file under `crates/mcp/src/tools/`, listed
  in one table in `crates/mcp/src/lib.rs`), `crates/cli` (the `mcp` subcommand), `Cargo.toml`
  (the members line), `website/docs` (one guide page), `CHANGELOG.md`, `AGENTS.md` (one row).
  Inferred.
- The later stories of `epic:agent-mcp-server` add files to `crates/mcp` and a row to that table,
  and run after this one, one at a time.
- Depends on the readers holding against untrusted input (`epic:adapters-untrusted-input`), since
  an agent points the server at code nobody reviewed.
