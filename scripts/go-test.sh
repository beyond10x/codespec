#!/bin/sh
# The tests of the Go adapters (adapters/go) within the test budget of the "Tests stay fast" rules:
# the whole go test run, compiling included, is killed after 120 s (go test's own -timeout, 115 s
# per package, ends a package first where it can, with the stack of the test under way); a test
# that ran over 60 s fails the step; the ten slowest tests are printed. go test has no bound per
# test, so a test over 60 s fails once it ends, not when it reaches 60 s. Prints the compiler's
# output, the output of every failed test and of every test that did not end (named), and each
# package's result line; exits non-zero when go test did, was killed, or a test ran over 60 s.
#
# usage: scripts/go-test.sh        the tests (CODESPEC, when set, reaches the tests that run the
#                                  codespec binary)
#        scripts/go-test.sh test   this script's own cases, with a stand-in for go (also run by
#                                  the lint part of scripts/check-part.sh)
set -eu
cd "$(dirname "$0")/.."
STEP_SECS=120

# gotest <dir>: the step, its files under $W
gotest() {
  mkdir -p "$W"
  rc=0
  (cd "$1" && timeout -k 5 "$STEP_SECS" go test -count=1 -timeout 115s -json ./...) > "$W/test.json" || rc=$?
  # A test that started and has no result was under way when go test or the step was killed.
  open='[.[] | select(.Test != null and .Action == "run") | [.Package, .Test]]
    - [.[] | select(.Test != null and (.Action == "pass" or .Action == "fail" or .Action == "skip"))
        | [.Package, .Test]]'
  jq -rjs "$open as \$open | "'
    [.[] | select(.Action == "fail" and .Test != null) | [.Package, .Test]] as $failed
    | .[]
    | if .Action == "build-output" then .Output
      elif .Action == "output"
        and (.Test == null or ([.Package, .Test] | IN($failed[], $open[])))
        and (.Output | test("^(=== RUN|=== PAUSE|=== CONT|PASS$)") | not)
      then .Output
      else empty end' "$W/test.json"
  under_way=$(jq -rs "$open"' | .[] | "\(.[0] | sub(".*/"; ""))/\(.[1])"' "$W/test.json" | tr "\n" " ")
  if [ "$rc" -eq 124 ]; then echo "go-test: the step ran over ${STEP_SECS} s and was killed" >&2; fi
  if [ -n "$under_way" ]; then
    echo "go-test: did not end: $under_way" >&2
    if [ "$rc" -eq 0 ]; then rc=1; fi
  fi
  jq -r 'select((.Action == "pass" or .Action == "fail") and .Test != null and (.Test | contains("/") | not))
    | "\(.Elapsed) \(.Package | sub(".*/"; ""))/\(.Test)"' "$W/test.json" | sort -rn > "$W/times"
  echo "== the ten slowest Go tests"
  head -10 "$W/times" | awk '{ printf "%8.1fs %s\n", $1, $2 }'
  over=$(awk '$1 > 60 { printf "%s ", $2 }' "$W/times")
  if [ -n "$over" ]; then
    echo "go-test: a test ran over 60 s: $over" >&2
    if [ "$rc" -eq 0 ]; then rc=1; fi
  fi
  return "$rc"
}

# The cases: the step run with a stand-in go that prints recorded events and exits as told.
selftest() {
  T=$PWD/target/go-test-test
  rm -rf "$T"
  mkdir -p "$T/bin"
  printf '%s\n' '#!/bin/sh' 'cat "$STANDIN_JSON"' 'if [ -n "${STANDIN_SLEEP:-}" ]; then exec sleep "$STANDIN_SLEEP"; fi' 'exit "$STANDIN_RC"' > "$T/bin/go"
  chmod +x "$T/bin/go"
  ev() { printf '{"Action":"%s","Package":"example.com/m/p","Test":"%s","Elapsed":%s}\n' "$1" "$2" "$3"; }
  out() { printf '{"Action":"output","Package":"example.com/m/p","Test":"%s","Output":"%s\\n"}\n' "$1" "$2"; }
  fail=0
  standin() {
    rc=0
    (W=$T/w STEP_SECS=${2:-120} STANDIN_SLEEP=${3:-} PATH=$T/bin:$PATH STANDIN_JSON=$T/events STANDIN_RC=$1 && export PATH STANDIN_JSON STANDIN_RC STANDIN_SLEEP && gotest "$T") > "$T/out" 2>&1 || rc=$?
  }
  ok() {
    if ! "$@"; then
      echo "go-test test: $case: not so: $*" >&2
      sed 's/^/  | /' "$T/out" >&2
      fail=1
    fi
  }
  has() { grep -qF -- "$1" "$T/out"; }
  lacks() { ! grep -qF -- "$1" "$T/out"; }

  case="tests within 60 s pass, the slowest first, a passing test's output left out"
  { out TestQuick "quiet line"; ev pass TestQuick 0.5; ev pass TestSlower 12.3; ev pass TestSlower/sub 99; } > "$T/events"
  standin 0
  ok [ "$rc" -eq 0 ]
  ok has "12.3s p/TestSlower"
  ok lacks "p/TestSlower/sub"
  ok lacks "quiet line"
  ok [ "$(sed -n '/^== the ten slowest/{n;p;}' "$T/out" | tr -s ' ')" = " 12.3s p/TestSlower" ]

  case="a test that ran over 60 s fails the step, named"
  { ev pass TestQuick 0.5; ev pass TestLong 61.5; } > "$T/events"
  standin 0
  ok [ "$rc" -ne 0 ]
  ok has "go-test: a test ran over 60 s: p/TestLong"

  case="a failed test's output is printed and go test's exit kept"
  { out TestBad "want 1, got 2"; ev fail TestBad 0.1; } > "$T/events"
  standin 1
  ok [ "$rc" -eq 1 ]
  ok has "want 1, got 2"

  # The events below are what go test -json of go1.27 writes (recorded on a module of two
  # packages, the names replaced): a compiler error arrives as "build-output" events keyed by
  # ImportPath, not as "output" events of the package.
  case="a package that does not build: the compiler's message is printed"
  printf '%s\n' \
    '{"ImportPath":"example.com/m/p [example.com/m/p.test]","Action":"build-output","Output":"# example.com/m/p [example.com/m/p.test]\n"}' \
    '{"ImportPath":"example.com/m/p [example.com/m/p.test]","Action":"build-output","Output":"p/p.go:3:23: undefined: missing\n"}' \
    '{"ImportPath":"example.com/m/p [example.com/m/p.test]","Action":"build-fail"}' \
    '{"Action":"start","Package":"example.com/m/p"}' \
    '{"Action":"output","Package":"example.com/m/p","Output":"FAIL\texample.com/m/p [build failed]\n","OutputType":"frame"}' \
    '{"Action":"fail","Package":"example.com/m/p","Elapsed":0,"FailedBuild":"example.com/m/p [example.com/m/p.test]"}' > "$T/events"
  standin 1
  ok [ "$rc" -eq 1 ]
  ok has "undefined: missing"

  # go test -timeout ends the package with a panic attributed to the running test, and writes no
  # "fail" event for that test, only for the package.
  case="a test killed by the 120 s timeout is named with go test's panic"
  printf '%s\n' \
    '{"Action":"start","Package":"example.com/m/p"}' \
    '{"Action":"run","Package":"example.com/m/p","Test":"TestHang"}' \
    '{"Action":"output","Package":"example.com/m/p","Test":"TestHang","Output":"=== RUN   TestHang\n","OutputType":"frame"}' \
    '{"Action":"output","Package":"example.com/m/p","Test":"TestHang","Output":"panic: test timed out after 2m0s\n"}' \
    '{"Action":"output","Package":"example.com/m/p","Test":"TestHang","Output":"\trunning tests:\n"}' \
    '{"Action":"output","Package":"example.com/m/p","Test":"TestHang","Output":"\t\tTestHang (2m0s)\n"}' \
    '{"Action":"output","Package":"example.com/m/p","Output":"FAIL\texample.com/m/p\t120.012s\n","OutputType":"frame"}' \
    '{"Action":"fail","Package":"example.com/m/p","Elapsed":120.012}' > "$T/events"
  standin 1
  ok [ "$rc" -eq 1 ]
  ok has "panic: test timed out after 2m0s"
  ok has "TestHang"

  case="the whole go test run, compiling included, is killed at the step's bound, naming the test under way"
  printf '%s\n' \
    '{"Action":"start","Package":"example.com/m/p"}' \
    '{"Action":"run","Package":"example.com/m/p","Test":"TestSlow"}' > "$T/events"
  s=$(date +%s)
  standin 0 2 30
  ok [ "$rc" -ne 0 ]
  ok [ $(( $(date +%s) - s )) -lt 10 ]
  ok has "go-test: the step ran over 2 s and was killed"
  ok has "go-test: did not end: p/TestSlow"

  if [ "$fail" -ne 0 ]; then return 1; fi
  echo "go-test test: 6 cases of the test budget hold"
}

case "${1:-}" in
  "") W=target/go-test gotest adapters/go ;;
  test) selftest ;;
  *) echo "usage: scripts/go-test.sh [test]" >&2; exit 2 ;;
esac
