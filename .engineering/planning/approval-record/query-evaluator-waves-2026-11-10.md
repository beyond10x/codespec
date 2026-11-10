---
format: aep.planning-md/3
id: approval-record:query-evaluator-waves-2026-11-10
kind: approval-record
status: draft
title: Accept the query and evaluator stories; ten waves approved in advance
relations:
- decides: epic:cir-evaluator
- decides: epic:codebase-transformations
revision: 1
---
# Approval: the query-and-evaluator wave, and the next ten waves

## Words

- Timo, 2026-11-10: "use /planning to select the next 7 work items in each: ignite, codespec. for each of them dispatch up to 2 opus agents at once. one wave each repo, then cut new release for each of them. then continue + reopeat".
- Timo, 2026-11-10, later: "i approve all future 10 waves".

## Decision

- On 2026-11-10 the store held no accepted story. The coordinator drafted seven from the plans of `docs/query-language.md` § 9 (items 1 to 3) and `docs/evaluator.md` § 11 (stories 1 to 4). Both documents say "Status: proposed"; they were uncommitted files in two idle managed worktrees and are copied into this branch unchanged.
- Inferred by the coordinator, not said by Timo: that "select the next 7" accepts these seven, and that the two proposed designs are the direction to build. `story:cir-patterns` is superseded by `story:query-core` on that reading.
- Move to active: epic:cir-evaluator, epic:codebase-transformations and the seven stories, after the critic panel's second round (design and scope approve; acceptance and parallel-safety findings of round 2 revised, not re-reviewed).
- The stage-1 stop is waived for the next ten waves of this repository. At most two implementors run at once. A release (`AGENTS.md` § Release) follows each wave.
- Commits authorised: one per unit, the coordinator's merge and regeneration commits, one store commit per wave, pushes of the wave branch and `main` as the bot, the release commit, tag and GitHub release.
