#!/bin/sh
# The gate's parts. scripts/check-parts.txt lists them, one per line; `task check` runs them all in
# order and CI runs each as one job of its matrix (.github/workflows/check.yml), so both run the
# same list through this script.
#
# A part is one of:
#   lint                     cargo fmt --check, clippy -D warnings, the inventory below, and the
#                            cases of this script and of scripts/go-test.sh
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
#        scripts/check-part.sh test         this script's own cases, with stand-ins for cargo and a
#                                           test binary (also run by the lint part)
#
# A test part keeps the test budget of the "Tests stay fast" rules: no test over 60 s, the part's
# tests within 120 s once built. A part with a --test=<name> selector runs each test of its
# binaries alone, killed after 60 s, and starts none once its tests ran 120 s. Any other test part
# runs cargo test on one thread, as CI does (RUST_TEST_THREADS=1), timed test by test from what
# libtest writes: it ends when a test has run 60 s, naming it, and is killed after 120 s. Each test
# part prints its ten slowest tests. JDK and MVN_FLAGS, when set, are passed to each task.
# CHECK_PART_LIMITS=1 (set by CI) caps a test part's processes and memory once it is built and
# prints the largest processes every 30 s.
set -eu
cd "$(dirname "$0")/.."
PARTS=scripts/check-parts.txt
W=target/check-parts
TEST_SECS=60
PART_SECS=120

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
  mkdir -p "$W"
  : > "$W/times"
  case " $* " in
    *" --test="*)
      # One test at a time, each killed after TEST_SECS and none started once the part's tests ran
      # PART_SECS; its output goes to a file so a child it leaves behind cannot hold this step
      # open, and a test that does not finish is named with the processes still running.
      cargo test --locked "$@" --no-run --message-format=json |
        jq -r 'select(.profile.test == true) | .executable // empty' > "$W/bins"
      failed=0
      end=$(( $(ms) + PART_SECS * 1000 ))
      while read -r bin; do
        for t in $("$bin" --list --format terse | sed -n 's/: test$//p'); do
          left=$(( (end - $(ms)) / 1000 ))
          if [ "$left" -le 0 ]; then
            failed=1
            echo "::error::the part's tests ran over ${PART_SECS}s: $t and every test after it did not run"
            break 2
          fi
          lim=$TEST_SECS
          if [ "$left" -lt "$lim" ]; then lim=$left; fi
          s=$(ms); rc=0
          timeout -s KILL "$lim" setsid "$bin" --exact "$t" --test-threads 1 \
            > "$W/test.out" 2>&1 < /dev/null || rc=$?
          d=$(( $(ms) - s ))
          echo "$d $t" >> "$W/times"
          group "$t rc=$rc $(secs "$d")s"
          tail -c 100000 "$W/test.out"
          endgroup
          if [ "$rc" -ne 0 ]; then
            failed=1
            if [ "$rc" -eq 137 ] && [ "$d" -ge $(( lim * 1000 )) ]; then
              echo "::error::$t killed after ${lim}s"
            else
              echo "::error::$t exited $rc after $(secs "$d")s"
            fi
            ps -eo pid,pgid,etimes,rss,args --forest | cut -c1-200
          fi
        done
      done < "$W/bins"
      slowest tests
      return "$failed" ;;
    *)
      # One thread, as CI runs every part (check.yml sets RUST_TEST_THREADS=1): libtest then
      # writes `test <name> ... ` as a test starts and its result as it ends. A watcher reads the
      # output every 0.2 s for the test under way, times each test, and ends the part when one
      # has run TEST_SECS, naming it; PART_SECS ends the part in any case. The output goes to a
      # file so a child a test leaves behind cannot hold this step open.
      rm -f "$W/slow" "$W/rc" "$W/tpid"
      : > "$W/part.out"
      (
        RUST_TEST_THREADS=1 timeout -k 10 "$PART_SECS" cargo test --locked "$@" \
          > "$W/part.out" 2>&1 < /dev/null &
        echo $! > "$W/tpid"
        r=0
        wait $! || r=$?
        echo "$r" > "$W/rc"
      ) &
      sub=$!
      tail -n +1 -f --pid="$sub" "$W/part.out" &
      tailpid=$!
      cur= since=0
      while [ ! -s "$W/rc" ]; do
        now=$(ms)
        run=$(tail -c 65536 "$W/part.out" | awk "$UNDER_WAY")
        if [ "$run" != "$cur" ]; then
          if [ -n "$cur" ]; then echo "$(( now - since )) $cur" >> "$W/times"; fi
          cur=$run since=$now
        elif [ -n "$cur" ] && [ $(( now - since )) -ge $(( TEST_SECS * 1000 )) ]; then
          echo "$cur" > "$W/slow"
          kill -TERM "$(cat "$W/tpid")" 2>/dev/null || true
          break
        fi
        sleep 0.2
      done
      if [ -n "$cur" ]; then echo "$(( $(ms) - since )) $cur" >> "$W/times"; fi
      wait "$sub" || true
      wait "$tailpid" || true
      rc=$(cat "$W/rc")
      slowest tests
      if [ -s "$W/slow" ]; then
        if [ "$rc" -eq 0 ]; then rc=1; fi
        echo "::error::$(cat "$W/slow") ran over ${TEST_SECS}s: the part was ended"
      elif [ "$rc" -eq 124 ]; then
        echo "::error::the part's tests ran over ${PART_SECS}s: the part was ended"
      fi
      return "$rc" ;;
  esac
}

# The test under way at the end of libtest's output on one thread, or nothing: the last
# `test <name> ... ` with no result after it. A line a child of the test writes may stand
# between the two; libtest then writes the result at the end of that line.
UNDER_WAY='
  { gsub(/\033\[[0-9;]*m/, "") }
  /^test .+ \.\.\. / {
    name = $0; sub(/^test /, "", name); sub(/ \.\.\. .*$/, "", name)
    rest = $0; sub(/^test .+ \.\.\. /, "", rest)
    run = (rest !~ /^(ok|FAILED|ignored)/); next
  }
  /^(test result: |failures:|successes:|running [0-9]+ tests?$)/ { run = 0; next }
  /(ok|FAILED)$/ { run = 0 }
  END { if (run) print name }'

# ms: the clock in milliseconds. secs <ms>: those milliseconds as seconds, one decimal.
ms() { date +%s%3N; }
secs() { awk -v m="$1" 'BEGIN { printf "%.1f", m / 1000 }'; }

# slowest <what>: the ten largest of $W/times ("<ms> <name>" lines), slowest first.
slowest() {
  echo "== the ten slowest $1 of the part"
  sort -rn "$W/times" | head -10 | awk '{ t = $1; $1 = ""; printf "%8.1fs %s\n", t / 1000, $0 }'
}

# The test budget's cases: each runs a test part with stand-ins for cargo and a test binary first
# on PATH and the bounds lowered to seconds, and fails naming the case whose run differs.
selftest() {
  T=$PWD/target/check-part-test
  rm -rf "$T"
  mkdir -p "$T/bin"
  printf '%s\n' '#!/bin/sh' \
    'case " $* " in' \
    '  *" --message-format=json "*) printf "{\"profile\":{\"test\":true},\"executable\":\"%s\"}\n" "$STANDIN_BIN"; exit 0 ;;' \
    '  *" --no-run "*) exit 0 ;;' \
    'esac' \
    'case "$STANDIN" in' \
    '  hung) echo "     Running tests/hung.rs (target/debug/deps/hung-0)"; echo "running 1 test"' \
    '    printf "test hung_one ... "; exec sleep 30 ;;' \
    '  chatty) echo "running 1 test"; printf "test chatty_one ... "; echo "a line of its child"' \
    '    exec sleep 30 ;;' \
    '  threads) echo "threads=${RUST_TEST_THREADS:-unset}" ;;' \
    '  silent) exec sleep 30 ;;' \
    '  single) echo "     Running tests/single.rs (target/debug/deps/single-0)"' \
    '    printf "test slow_one ... "; sleep 6; echo ok' \
    '    echo "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.00s" ;;' \
    '  timed) echo "     Running tests/t.rs (target/debug/deps/t-0)"; echo; echo "running 3 tests"' \
    '    for t in short:0.4 long:1.6 middle:0.8; do printf "test %s ... " "${t%%:*}"; sleep "${t#*:}"; echo ok; done' \
    '    echo; echo "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.80s" ;;' \
    'esac' > "$T/bin/cargo"
  printf '%s\n' '#!/bin/sh' \
    'case "$1" in' \
    '  --list) for t in $STANDIN_TESTS; do echo "$t: test"; done; echo "a_bench: benchmark" ;;' \
    '  --exact) case "$2" in slow*) exec sleep 30 ;; esac; echo "ran $2" ;;' \
    'esac' > "$T/bin/standin-tests"
  chmod +x "$T/bin/cargo" "$T/bin/standin-tests"
  fail=0
  # standin <mode> <tests> <test secs> <part secs> <part word>...: one part, its output in
  # $T/out, its exit in $rc and its seconds in $took
  standin() {
    mode=$1 tests=$2 tsecs=$3 psecs=$4
    shift 4
    s=$(ms)
    rc=0
    (
      W=$T/w TEST_SECS=$tsecs PART_SECS=$psecs CHECK_PART_LIMITS=
      PATH=$T/bin:$PATH STANDIN=$mode STANDIN_TESTS=$tests STANDIN_BIN=$T/bin/standin-tests
      export PATH STANDIN STANDIN_TESTS STANDIN_BIN
      test_part "$@"
    ) > "$T/out" 2>&1 || rc=$?
    took=$(( ($(ms) - s) / 1000 ))
  }
  ok() {
    if ! "$@"; then
      echo "check-part test: $case: not so: $*" >&2
      sed 's/^/  | /' "$T/out" >&2
      fail=1
    fi
  }
  has() { grep -qF -- "$1" "$T/out"; }
  lacks() { ! grep -qF -- "$1" "$T/out"; }
  first_slowest() { sed -n '/^== the ten slowest/{n;p;}' "$T/out" | grep -qF -- "$1"; }

  case="a test over its bound is killed and named, the next test still runs"
  standin tests "fast_one slow_one fast_two" 2 30 fake --test=x
  ok [ "$rc" -ne 0 ]
  ok [ "$took" -lt 10 ]
  ok has "::error::slow_one killed after 2s"
  ok has "fast_two rc=0"
  ok lacks "a_bench rc="
  ok first_slowest slow_one

  case="no test starts once the part's tests ran its bound"
  standin tests "slow_a slow_b fast_c" 2 3 fake --test=x
  ok [ "$rc" -ne 0 ]
  ok [ "$took" -lt 10 ]
  ok has "ran over 3s"
  ok lacks "fast_c rc="

  case="a part of whole packages ends when a test has run its bound, naming it"
  standin hung "" 2 30 fake
  ok [ "$rc" -ne 0 ]
  ok [ "$took" -lt 10 ]
  ok has "::error::hung_one ran over 2s"

  case="a line a test's child writes does not end that test's bound"
  standin chatty "" 2 30 fake
  ok [ "$rc" -ne 0 ]
  ok [ "$took" -lt 10 ]
  ok has "::error::chatty_one ran over 2s"

  case="a part of whole packages runs its tests on one thread, as CI does"
  (unset RUST_TEST_THREADS; standin threads "" 60 30 fake; echo "$rc" > "$T/rc")
  ok has "threads=1"
  ok [ "$(cat "$T/rc")" -eq 0 ]

  case="a part of whole packages is killed at its bound"
  standin silent "" 60 2 fake
  ok [ "$rc" -ne 0 ]
  ok [ "$took" -lt 15 ]
  ok has "ran over 2s"

  case="a part of whole packages passes and names its slowest test first"
  standin timed "" 60 30 fake
  ok [ "$rc" -eq 0 ]
  ok first_slowest " long"
  ok has " middle"

  # CI (.github/workflows/check.yml) runs every part with RUST_TEST_THREADS=1. libtest with one
  # thread never prints "has been running for over 60 seconds" (library/test/src/lib.rs, the
  # `concurrency == 1` branch): the stand-in prints what it prints, a test that ends past its bound.
  case="a part of whole packages run with RUST_TEST_THREADS=1, as CI runs it, ends a test running over its bound"
  RUST_TEST_THREADS=1
  export RUST_TEST_THREADS
  standin single "" 2 30 fake
  ok [ "$rc" -ne 0 ]
  ok [ "$took" -lt 5 ]
  ok has "::error::slow_one"

  if [ "$fail" -ne 0 ]; then return 1; fi
  echo "check-part test: 8 cases of the test budget hold"
}

run() {
  # shellcheck disable=SC2086 # a part is words without spaces inside them
  set -- $1
  kind=$1; shift
  case "$kind" in
    lint)
      cargo fmt --all --check
      cargo clippy --workspace --all-targets --locked -- -D warnings
      inventory
      selftest
      sh scripts/go-test.sh test ;;
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
  test) selftest ;;
  *) echo "usage: scripts/check-part.sh run <part> | all | matrix | inventory | test" >&2; exit 2 ;;
esac
