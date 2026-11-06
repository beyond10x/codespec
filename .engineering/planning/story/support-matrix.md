---
format: aep.planning-md/3
id: story:support-matrix
kind: story
status: active
title: The language support matrix on the documentation site
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:runners-and-manifests
- depends_on: story:codespec-conform
scope:
- confidence: cited
  path: crates/docs/src/main.rs
- confidence: cited
  path: crates/docs/src/support.rs
- confidence: cited
  path: website/data/support-matrix.json
- confidence: cited
  path: website/docs/reference/support.mdx
- confidence: cited
  path: website/docusaurus.config.ts
- confidence: cited
  path: website/src/components/SupportMatrix
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:26Z", actor: "human:timo", revision: 4}
---
# Story: The language support matrix on the documentation site

## Outcome

A public page, `/docs/reference/support`, shows per `cir/1` feature and per language whether it is
supported for reading and for writing: one row per feature, grouped by family (statements,
expressions, operators, intrinsics, types, facts, semantics), one column pair (read, write) per
language. Added to the epic by the operator on 2026-11-06: "some support matrix like which language
constructs … actually support in reading versus writing … displayed in a very appealing way".

## Acceptance

- `codespec-docs` generates `website/data/support-matrix.json` from `schemas/cir-1.features.json`,
  each adapter's `--describe` manifest and the reports in `website/data/conformance/`;
  `task docs-check` fails when it drifts.
- Each cell is one of: `verified` (claimed, conformance pass), `claimed` (in the manifest, no
  report), `failing` (claimed, conformance fail), `refused` (not claimed), `none` (no adapter for
  that role). A unit test of the generator builds the matrix from fixture manifests and reports and
  gets each of the five states at least once, including a language with a reader manifest and no
  writer, whose write column is `none` for every row.
- The page shows a legend with the five states and a count per column; on this repository's data
  Go and Rust have both roles.
- The page is an MDX page with a React component (`website/src/components/SupportMatrix`) reading
  the JSON: a sticky header, a family filter, colour plus a symbol per state (readable without
  colour), each feature id linking to its case under `conformance/cases`. `npm run build` of the
  site passes.

## Scope

- `crates/docs/src/support.rs` (new), the `outputs` list in `crates/docs/src/main.rs`.
- `website/data/support-matrix.json` (new), `website/docs/reference/support.mdx` (new),
  `website/src/components/SupportMatrix/` (new), `website/docusaurus.config.ts` (a navbar entry).
