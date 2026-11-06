---
format: aep.planning-md/3
id: epic:adapter-conformance
kind: epic
status: active
title: 'Adapter conformance: verify an outside reader or writer per claimed feature'
summary: codespec conform runs a versioned per-feature kit against an adapter's declared features and writes a report bound to both by digest.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 3}
---
# Epic: Adapter conformance

## Outcome

An adapter built outside this repository runs `codespec conform` and gets a
`codespec-conformance-report/1`: per `cir/1` feature it claims in its `--describe` manifest, pass,
fail or refused, bound to the adapter version and the kit by digest. The in-repo readers and writers
produce conforming reports in `task check`.

Design: `docs/conformance.md` (proposed 2026-11-06).

## Why Now

Requested by the operator on 2026-11-06: "if someone from outside this repo provides a reader or a
writer, we would need a way to basically formally verify that those support all the features they
claim to support". Today only strict validation at the boundary and the harness on 4 in-repo
examples exist (`docs/conformance.md` § 1); neither checks a claim per feature.

## Scope

`docs/conformance.md` § 10, in order: the feature vocabulary generated from the model; per-feature
cases with Go reference programs and expected outputs; `codespec-run-<lang>` runners and
`--describe` manifests for the in-repo adapters; `codespec conform` for writers, then readers; the
kit in release archives; the adapter guide rewritten around it; and the support matrix of § 8 on the documentation site (added by the operator on 2026-11-06).

## Out of Scope

- ESS as the report format (`docs/conformance.md` § 9, D2).
- Hosting or registering outside adapters.

## Open decisions

D1–D3 in `docs/conformance.md` § 9, each with a proposed default.

## Done When

`task check` runs `codespec conform` on the Go and Rust readers and writers, each report says
`conforms: true` for its manifest, and a writer whose `sem.int.wrap` case is broken by hand gets
`conforms: false` naming that feature.
