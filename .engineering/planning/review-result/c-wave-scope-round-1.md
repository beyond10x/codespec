---
format: aep.planning-md/3
id: review-result:c-wave-scope-round-1
kind: review-result
status: active
title: Scope critic, C wave, round 1
relations:
- reviews: epic:c-reader-writer
- reviews: story:c-reader-clang
- reviews: story:c-writer
- reviews: story:builtin-defect-checks
- reviews: story:use-cases-page
revision: 1
---
needs-revision

- epic:c-reader-writer — "the support matrix lists C" is promised in Done When and neither story claims it, though the matrix is generated from each adapter's `--describe` manifest and the checked-in reports (`story:support-matrix`), so C needs a manifest and generator wiring that no body names. `story:c-writer`, which merges second, would most naturally take it. — .engineering/planning/epic/c-reader-writer.md:59
- epic:c-reader-writer — "Both adapters are Rust crates in the workspace, built into `target/adapters` by `task build`" is promised and no story claims the crates or the build wiring, although `codespec read --lang c` and `codespec conform --lang c` cannot run without them. `story:c-reader-clang` and `story:c-writer` would each take their own crate. — .engineering/planning/epic/c-reader-writer.md:31
- story:builtin-defect-checks — `dropped-error` and `lock-without-unlock` are also the outcomes of `story:defect-patterns` ("dropped error, lock without unlock on every path"), and the body excludes patterns without saying these two kinds are not claimed twice. Both stories will be marked done on the same defects over the same repository. — .engineering/planning/story/builtin-defect-checks.md:33
- story:use-cases-page — the use-3 title promises defects "across languages", but the section's content is Go-only (the checks on google/capslock, Go fixtures), and nothing says the cross-language part is narrowed or planned. That narrows the operator's ask ("defects on one IR across languages") silently. — .engineering/planning/story/use-cases-page.md:26
- story:use-cases-page — the Neighbours section adds `go vet`/staticcheck, Semgrep and a Go → Rust transpiler, but the operator asked only for "capslock as a neighbour", so those three are not traceable to the request. — .engineering/planning/story/use-cases-page.md:31

**What you read:** 6 artifacts (the epic, its two stories, `story:builtin-defect-checks`, `epic:codebase-transformations`, `story:use-cases-page`) plus `story:defect-patterns` and `story:support-matrix`. Commands: `aep plan artifact show` on each, `aep plan artifact graph`, and `grep -n` over `.engineering/planning/`. I extracted 15 promises from `epic:c-reader-writer` and traced 13 to a story. The two untraced are the matrix and the crates built by `task build`.

**What you could not establish:**
- `story:builtin-defect-checks` traces to `epic:codebase-transformations` only loosely. The epic frames defects as a use of the pattern mechanism, and the story is Rust checks outside that mechanism. I judged "find defects" in the epic's Why Now sufficient and made no finding.
- The operator's capslock instruction is claimed by `story:builtin-defect-checks`, which pins google/capslock as the test repository. The use-cases-page acceptance "No page names the third-party library of the measured runs" conflicts with its own use 3 naming capslock as the test repository. That is out of my lane (acceptance and design), and it did not set my verdict.
- Whether `codespec check` is meant to be a new command is a design question, out of my lane.

```findings
- file: .engineering/planning/epic/c-reader-writer.md
  line: 59
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the support matrix lists C\" is promised in Done When and neither story claims it, though the matrix is generated from each adapter's --describe manifest and the checked-in reports; story:c-writer would most naturally take it"
- file: .engineering/planning/epic/c-reader-writer.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"Both adapters are Rust crates in the workspace, built into target/adapters by task build\" is promised and no story claims the crates or the build wiring; story:c-reader-clang and story:c-writer would each take their own crate"
- file: .engineering/planning/story/builtin-defect-checks.md
  line: 33
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "dropped-error and lock-without-unlock are also the outcomes of story:defect-patterns (.engineering/planning/story/defect-patterns.md:17) and the body does not say these two kinds are not claimed twice"
- file: .engineering/planning/story/use-cases-page.md
  line: 26
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the use-3 title promises defects \"across languages\" but the section content is Go-only, and nothing says the cross-language part is narrowed or planned"
- file: .engineering/planning/story/use-cases-page.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Neighbours section adds go vet/staticcheck, Semgrep and a Go to Rust transpiler, but the operator asked only for \"capslock as a neighbour\""
```
