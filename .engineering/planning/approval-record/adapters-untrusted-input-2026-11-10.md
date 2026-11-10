---
format: aep.planning-md/3
id: approval-record:adapters-untrusted-input-2026-11-10
kind: approval-record
status: draft
title: Accept two hardening stories; they gate the return to public
relations:
- decides: epic:adapters-untrusted-input
revision: 1
---
# Approval: two hardening stories before the repository is public again

## Words

- Timo, 2026-11-10: "i approve all future 10 waves"; and, on creating a private repository for
  the patterns and making this one public again after the release gate: "A".

## Decision

- A peer session's review reproduced five defects in the writers and readers (2026-11-10). The
  coordinator read `adapters/rust/src/write.rs` and confirms that the generated `Cargo.toml` takes
  `m.name` unescaped; the other four are taken from the peer's report, not re-run.
- Inferred by the coordinator, not said by Timo: the two stories join the wave in progress, and
  the repository does not go public before both are implemented and released.
- No plan-critic panel read these two stories. An `aep:security-reviewer` pass runs on each unit
  beside the adversary.
- Move to active: `epic:adapters-untrusted-input` and its two stories.
