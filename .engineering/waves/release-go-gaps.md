# Wave plan: release and Go reader gaps (wave 6)

Derived 2026-11-26 (AEP skill 0.26.0, `aep` 0.80.0) from the three stories the owner accepted
("accept G5 G6 musl into this wave", the owner through the operator role, 2026-11-26;
`approval-record:release-go-gaps-wave-2026-11-26`). The same order starts the wave once #74 and #78
merged (603faba6, da023271). Epic: `epic:release-diff-go-field-gaps`. Consumer: the downstream consumer moving
its code facts onto the 0.14.0 release (gaps G5, G6).

`aep plan artifact waves --kind story --status active` places the three in three waves, one each:
`wave 1 story:read-go-newer-toolchain`, `wave 2 story:release-ships-go-summaries`,
`wave 3 story:release-musl-static`; unassessed none; cycles none. Collisions (verbatim pairs):

- read-go-newer-toolchain / release-musl-static: `.github/workflows/release-build.yml` (cited), `AGENTS.md` (inferred)
- read-go-newer-toolchain / release-ships-go-summaries: `.github/workflows/release-build.yml` (cited), `AGENTS.md`, `CHANGELOG.md`, `Taskfile.yml` (inferred)
- release-musl-static / release-ships-go-summaries: `.github/workflows/release-build.yml` (cited), `AGENTS.md`, `crates/cli/src/main.rs` (inferred), `website/docs/getting-started.md` (cited)

Every pair collides on `release-build.yml`, so the units run one after another on one integration
branch; at most one implementor runs at a time (the coordinator's limit). Order: G5 first (the
consumer's first gap; musl depends on it), then G6, then musl. Scope of every unit: typed entries,
cited and inferred as above.

Integration branch `wave/release-go-gaps` from `main` at da023271, managed tree `wave6`. Each unit:
branch `impl/<story>` from the integration head, its own managed tree, build directory `target/`
inside it. Agents: `aep:implementor` (opus), then `aep:adversary` (sonnet). Builds and tests
through `slot`; a test over 60 s is killed, the test step ends within 120 s, a pool job ends within
10 minutes.

Store on the way out: each story needs `test_result` evidence before `active -> implemented`.

Commits this wave makes: this page and the approval, one commit per unit, its merge into the
integration branch, the closing store commit, one pull request to `main` that the operator role
merges. No tag, no release.

| order | story | unit tree | unit commit | merge | adversary | state |
|---|---|---|---|---|---|---|
| 1 | story:release-ships-go-summaries | w6-g5 | 977d2370, e11f9db5 | 50dc6fa8 | 5 notes; fixed: canonical exe path (macOS symlink), kit desc; 3 cases kept; 1 pre-existing (barrier language) to the backlog | implemented |
| 2 | story:read-go-newer-toolchain | w6-g6 | 6e34190f, c067cf07 | 5e9ad1a6 | 2 test gaps (mutants), 2 cases kept; go env under the caller GOTOOLCHAIN kept as ensureGOROOT does | implemented |
| 3 | story:release-musl-static | w6-musl | 0cb3f92f, 19b11ceb | 1a67c688 | 2 defects in the static check (unreadable headers, no target/), fixed; workflow guard case added | implemented |

Closed 2026-11-26: forge gate on f80ac24c (check run 45867215703, 47 of 47 checks of #79 success) is the test_result of each story; all three moved active -> implemented. Backlog draft from the wave: story:barrier-language-matches-module (pre-existing, adversary of G5). Every wave of this plan is delivered.
