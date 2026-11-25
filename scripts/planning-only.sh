#!/bin/sh
# Whether a pull request changes only planning or prose files, so check.yml may pass its required
# check without building or testing (~/beyond10x/AGENTS.md, "Planning-only pull requests skip the
# tests"). This is the one path list:
#
#   planning or prose   .engineering/**, and *.md anywhere else
#   code                everything else, and always website/** (pages.yml checks the site),
#                       docs/** (tests read docs/query-language.md, docs/conformance.md) and
#                       **/testdata/** (adapters/groovy/testdata/lending/EXPECTED.md is a fixture)
#                       and the root README.md (codespec-docs writes its badge block)
#
# usage: scripts/planning-only.sh < <changed paths, one per line>
#          exit 0 when every path is planning or prose, 1 when one is code (named on stderr) or
#          the list is empty
#        scripts/planning-only.sh test
#          the path list's own cases, both sides
set -eu

kind() {
  case "$1" in
    .engineering/*) echo planning ;;
    website/* | docs/* | */testdata/* | testdata/* | README.md) echo code ;;
    *.md) echo planning ;;
    *) echo code ;;
  esac
}

classify() {
  n=0
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    n=$((n + 1))
    if [ "$(kind "$path")" = code ]; then
      echo "planning-only: $path is code" >&2
      return 1
    fi
  done
  if [ "$n" -eq 0 ]; then
    echo "planning-only: no changed path" >&2
    return 1
  fi
  echo "planning-only: all $n changed paths are planning or prose" >&2
}

# expect <want: skip|run> <path>...
expect() {
  want=$1
  shift
  got=run
  if printf '%s\n' "$@" | classify 2> /dev/null; then got=skip; fi
  if [ "$got" = "$want" ]; then
    echo "ok   $want: $*"
  else
    echo "FAIL want $want, got $got: $*"
    failed=1
  fi
}

selftest() {
  failed=0
  expect skip .engineering/planning/story/x.md
  expect skip .engineering/planning/story/x.md .engineering/reviews/r.md AGENTS.md \
    CHANGELOG.md adapters/java/README.md ux/findings/UXF-001.md .claude/skills/language-wave/SKILL.md
  expect skip .engineering/aep.toml
  expect run .engineering/planning/story/x.md crates/cli/src/main.rs
  expect run .engineering/planning/story/x.md .github/workflows/check.yml
  expect run README.md scripts/check-parts.txt
  expect run README.md
  expect run website/docs/getting-started.md
  expect run .engineering/planning/story/x.md website/docs/index.md
  expect run docs/query-language.md
  expect run adapters/groovy/testdata/lending/EXPECTED.md
  expect run Cargo.lock
  expect run
  return "$failed"
}

case "${1:-}" in
  test) selftest ;;
  "") classify ;;
  *) echo "usage: scripts/planning-only.sh [test] < <paths>" >&2; exit 2 ;;
esac
