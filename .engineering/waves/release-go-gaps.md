# Wave plan: release and Go reader gaps (wave 6)

Derived 2026-11-26 (AEP skill 0.26.0, `aep` 0.80.0) from the three stories the owner accepted
("accept G5 G6 musl into this wave", relayed by the host-operator session;
`approval-record:release-go-gaps-wave-2026-11-26`). The same order starts the wave once #74 and #78
merged (603faba6, da023271). Epic: `epic:release-diff-go-field-gaps`. Consumer: the session moving
its code facts onto the 0.14.0 release (gaps G5, G6).

`aep plan artifact waves --kind story --status active` places the three in three waves, one each:
`wave 1 story:read-go-newer-toolchain`, `wave 2 story:release-ships-go-summaries`,
`wave 3 story:release-musl-static`; unassessed none; cycles none. Collisions (verbatim pairs):

- read-go-newer-toolchain / release-musl-static: `.github/workflows/release-build.yml` (cited), `AGENTS.md` (inferred)
- read-go-newer-toolchain / release-ships-go-summaries: `.github/workflows/release-build.yml` (cited), `AGENTS.md`, `CHANGELOG.md`, `Taskfile.yml` (inferred)
- release-musl-static / release-ships-go-summaries: `.github/workflows/release-build.yml` (cited), `AGENTS.md`, `crates/cli/src/main.rs` (inferred), `website/docs/getting-started.md` (cited)

Every pair collides on `release-build.yml`, so the units run one after another on one integration
branch; at most one implementor runs at a time (the session's token hold). Order: G5 first (the
consumer's first gap; musl depends on it), then G6, then musl. Scope of every unit: typed entries,
cited and inferred as above.

Integration branch `wave/release-go-gaps` from `main` at da023271, managed tree `wave6`. Each unit:
branch `impl/<story>` from the integration head, its own managed tree, build directory `target/`
inside it. Agents: `aep:implementor` (opus), then `aep:adversary` (sonnet). Builds and tests
through `slot`; a test over 60 s is killed, the test step ends within 120 s, a pool job ends within
10 minutes.

Store on the way out: each story needs `test_result` evidence before `active -> implemented`.

Commits this wave makes: this page and the approval, one commit per unit, its merge into the
integration branch, the closing store commit, one pull request to `main` that the host-operator
session merges. No tag, no release.

| order | story | unit tree | unit commit | merge | adversary | state |
|---|---|---|---|---|---|---|
| 1 | story:release-ships-go-summaries | | | | | planned |
| 2 | story:read-go-newer-toolchain | | | | | planned |
| 3 | story:release-musl-static | | | | | planned |
