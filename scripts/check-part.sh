#!/bin/sh
# The gate's parts. scripts/check-parts.txt lists them, one per line; `task check` runs them all in
# order and CI runs each as one job of its matrix (.github/workflows/check.yml), so both run the
# same list through this script.
#
# A part is one of:
#   lint                     cargo fmt --check, clippy -D warnings, and the inventory below
#   test <word>...           cargo test; a word starting with -- is a target selector (--lib,
#                            --bins, --doc, --test=<name>), any other word a package
#   task <name>...           each task in turn
#
# usage: scripts/check-part.sh run <part>   one part (the line, as one argument)
#        scripts/check-part.sh all          every part, stopping at the first that fails
#        scripts/check-part.sh matrix       the parts as a JSON array, CI's matrix
#        scripts/check-part.sh inventory    fail unless every test target cargo knows (each
#                                           package's lib, bins, integration tests and doc tests)
#                                           is run by a test part, and every package and
#                                           --test=<name> a part names exists
#
# A part with a --test=<name> selector runs each test of its binaries alone, killed after 180 s.
# Any other test part is killed after 600 s. JDK and MVN_FLAGS, when set, are passed to each task.
# CHECK_PART_LIMITS=1 (set by CI) caps a test part's processes and memory once it is built and
# prints the largest processes every 30 s.
set -eu
cd "$(dirname "$0")/.."
PARTS=scripts/check-parts.txt
W=target/check-parts

group() { if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::group::$*"; else echo "== $*"; fi; }
endgroup() { if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::endgroup::"; fi; }

inventory() {
  mkdir -p "$W"
  cargo metadata --no-deps --format-version 1 --locked |
    jq -r '.packages[] | .name as $p | .targets[]
      | ((select((.kind[0] == "lib" or .kind[0] == "bin" or .kind[0] == "test") and .test) | "\($p) \(.kind[0]) \(.name)"),
         (select(.kind[0] == "lib" and .doctest) | "\($p) doc \(.name)"))' > "$W/targets.txt"
  awk '
    FNR == NR { req[++n] = $0; pkg[$1] = 1; if ($2 == "test") tgt[$1 " test " $3] = 1; next }
    $1 == "test" {
      np = 0; ns = 0
      for (i = 2; i <= NF; i++) if ($i ~ /^--/) sel[++ns] = $i; else p[++np] = $i
      for (j = 1; j <= np; j++) {
        if (!(p[j] in pkg)) { print "check-parts: line " FNR " names " p[j] ", not a workspace package"; bad = 1 }
        if (ns == 0) cov[p[j] " *"] = 1
        for (k = 1; k <= ns; k++) {
          s = sel[k]
          if (s == "--lib") cov[p[j] " lib *"] = 1
          else if (s == "--bins") cov[p[j] " bin *"] = 1
          else if (s == "--doc") cov[p[j] " doc *"] = 1
          else if (s ~ /^--test=/) {
            t = p[j] " test " substr(s, 8)
            if (!(t in tgt)) { print "check-parts: line " FNR " names " s ", not a test target of " p[j]; bad = 1 }
            cov[t] = 1
          } else { print "check-parts: line " FNR ": unknown selector " s; bad = 1 }
        }
      }
    }
    END {
      for (i = 1; i <= n; i++) {
        split(req[i], f, " ")
        if (!((f[1] " *") in cov) && !((f[1] " " f[2] " *") in cov) && !(req[i] in cov)) {
          print "check-parts: cargo test target " req[i] " is in no test part of scripts/check-parts.txt"; bad = 1
        }
      }
      exit bad
    }' "$W/targets.txt" "$PARTS"
  echo "check-parts: every cargo test target of the workspace is in a test part ($(wc -l < "$W/targets.txt") targets)"
}

limits() {
  [ "${CHECK_PART_LIMITS:-}" = 1 ] || return 0
  ( while sleep 30; do
      echo "== ps $(date +%T)"
      ps -eo rss,nlwp,etimes,args --sort=-rss --no-headers | head -5 | cut -c1-160
      echo "procs $(ps -e --no-headers | wc -l)"
    done ) &
  MONITOR=$!
  trap 'kill "$MONITOR" 2>/dev/null || true' EXIT
  # bash names the process limit -u, dash -p
  ulimit -u 2048 2>/dev/null || ulimit -p 2048
  ulimit -v 8388608
}

# test_part <word>...
test_part() {
  args=
  for w in "$@"; do
    case "$w" in --*) args="$args $w" ;; *) args="$args -p $w" ;; esac
  done
  # shellcheck disable=SC2086 # the words are package names and selectors, never spaced
  set -- $args
  # cargo refuses --no-run with --doc: a doc-test part builds as it runs
  case " $* " in *" --doc "*) ;; *) cargo test --locked "$@" --no-run ;; esac
  limits
  case " $* " in
    *" --test="*)
      # One test at a time, each killed after three minutes; its output goes to a file so a child
      # it leaves behind cannot hold this step open, and a test that does not finish is named
      # with the processes still running.
      mkdir -p "$W"
      cargo test --locked "$@" --no-run --message-format=json |
        jq -r 'select(.profile.test == true) | .executable // empty' > "$W/bins"
      failed=0
      while read -r bin; do
        for t in $("$bin" --list --format terse | sed -n 's/: test$//p'); do
          s=$(date +%s); rc=0
          timeout -s KILL 180 setsid "$bin" --exact "$t" --test-threads 1 \
            > "$W/test.out" 2>&1 < /dev/null || rc=$?
          group "$t rc=$rc $(( $(date +%s) - s ))s"
          tail -c 100000 "$W/test.out"
          endgroup
          if [ "$rc" -ne 0 ]; then
            failed=1
            echo "::error::$t exited $rc after $(( $(date +%s) - s ))s"
            ps -eo pid,pgid,etimes,rss,args --forest | cut -c1-200
          fi
        done
      done < "$W/bins"
      return "$failed" ;;
    *)
      timeout -k 10 600 cargo test --locked "$@" ;;
  esac
}

run() {
  # shellcheck disable=SC2086 # a part is words without spaces inside them
  set -- $1
  kind=$1; shift
  case "$kind" in
    lint)
      cargo fmt --all --check
      cargo clippy --workspace --all-targets --locked -- -D warnings
      inventory ;;
    test) test_part "$@" ;;
    task)
      for t in "$@"; do
        group "task $t"
        task "$t" ${JDK:+"JDK=$JDK"} ${MVN_FLAGS:+"MVN_FLAGS=$MVN_FLAGS"}
        endgroup
      done ;;
    *) echo "check-part: unknown part kind $kind" >&2; return 2 ;;
  esac
}

case "${1:-}" in
  run) run "$2" ;;
  all)
    while read -r part; do
      group "part $part"
      run "$part" < /dev/null
      endgroup
    done < "$PARTS" ;;
  matrix) jq -R -s -c 'split("\n") | map(select(length > 0))' "$PARTS" ;;
  inventory) inventory ;;
  *) echo "usage: scripts/check-part.sh run <part> | all | matrix | inventory" >&2; exit 2 ;;
esac
