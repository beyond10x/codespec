#!/bin/sh
# The checked-in edit examples, examples/edit/<lang>: a source tree (source/) and programs. Each
# program <name>.cq has <name>.args, its flags, and either <name>.diff, the diff a dry run prints
# (exit 0), or <name>.refusal, what it prints on standard error (exit 1, nothing on standard
# output). Go holds four operation programs and six refusals, Rust four and five.
#
# usage: scripts/examples-edit.sh             every program as a dry run on examples/edit, every
#                                             file of each source/ unchanged after; then each on
#                                             a copy under target/examples-edit/apply: an
#                                             operation with --apply --build, then once more,
#                                             which must change nothing; a refusal with --apply,
#                                             every file of the copy unchanged
#        scripts/examples-edit.sh --self-test the Go rename program run once, its output
#                                             compared with its .diff (must match: the
#                                             positive control) and with a copy of the .diff
#                                             with one byte changed (must not match)
#
# CODESPEC and ADAPTERS name the codespec binary and the directory of codespec-read-go and
# codespec-read-rust (default target/debug/codespec and target/adapters). CODESPEC_CACHE_DIR is
# set to target/examples-edit/cache, so the copy --build compiles is under target/.
set -eu
cd "$(dirname "$0")/.."
ROOT=$(pwd)
CODESPEC=${CODESPEC:-$ROOT/target/debug/codespec}
ADAPTERS=${ADAPTERS:-$ROOT/target/adapters}
W=$ROOT/target/examples-edit
CODESPEC_CACHE_DIR=$W/cache
export CODESPEC_CACHE_DIR

# The input directories of a language's programs, relative to the directory holding source/.
inputs() {
  case $1 in
    go) echo "source/stock source/front" ;;
    rust) echo "source" ;;
  esac
}

# run <lang> <dir holding source/> <program dir> <name> <out> [flag...]: run the program in the
# directory; standard output, standard error and the exit code go to <out>.out, .err and .code.
run() {
  lang=$1 cwd=$2 prog=$3 name=$4 out=$5
  shift 5
  # shellcheck disable=SC2046 # the flags and inputs are words
  if (cd "$cwd" && "$CODESPEC" edit --lang "$lang" --adapters "$ADAPTERS" \
    $(cat "$prog/$name.args") "$@" $(inputs "$lang") -f "$prog/$name.cq") \
    > "$out.out" 2> "$out.err"; then
    echo 0 > "$out.code"
  else
    echo $? > "$out.code"
  fi
}

# same <what> <expected file> <actual file>: fail, showing the difference, unless byte-equal.
same() {
  if cmp -s "$2" "$3"; then
    return 0
  fi
  echo "examples-edit: $1 differs from $2:"
  diff -u "$2" "$3" | head -40 || true
  return 1
}

# check <program dir> <name> <out>: the run at <out> printed what the program expects.
check() {
  prog=$1 name=$2 out=$3
  : > "$W/empty"
  if [ -f "$prog/$name.diff" ]; then
    code=0
    same "standard output of $name" "$prog/$name.diff" "$out.out" || return 1
    same "standard error of $name" "$W/empty" "$out.err" || return 1
  else
    code=1
    same "standard output of $name" "$W/empty" "$out.out" || return 1
    same "standard error of $name" "$prog/$name.refusal" "$out.err" || return 1
  fi
  if [ "$(cat "$out.code")" != "$code" ]; then
    echo "examples-edit: $name exited $(cat "$out.code"), not $code"
    cat "$out.err"
    return 1
  fi
}

# sums <dir>: the SHA-256 of every file under it, by path.
sums() {
  (cd "$1" && find . -type f | LC_ALL=C sort | xargs sha256sum)
}

self_test() {
  S=$W/self-test
  rm -rf "$S"
  mkdir -p "$S"
  cp examples/edit/go/rename.cq examples/edit/go/rename.args "$S/"
  sed '1s|^--- a/|--- b/|' examples/edit/go/rename.diff > "$S/rename.diff"
  if [ "$(cmp -l examples/edit/go/rename.diff "$S/rename.diff" | wc -l)" -ne 1 ]; then
    echo "examples-edit: self-test: the copy of go/rename.diff does not differ in one byte"
    exit 1
  fi
  run go "$ROOT/examples/edit/go" "$S" rename "$S/rename"
  # The positive control: the same run matches the .diff as checked in, so a mismatch below
  # comes from the comparison, not from a run that failed.
  if ! check examples/edit/go rename "$S/rename" > "$S/control.txt"; then
    echo "examples-edit: self-test: the Go rename program does not print its .diff unchanged:"
    cat "$S/control.txt"
    exit 1
  fi
  if check "$S" rename "$S/rename" > "$S/check.txt"; then
    echo "examples-edit: self-test: a .diff with one byte changed compared equal"
    exit 1
  fi
  echo "examples-edit: self-test: a .diff with one byte changed is detected"
}

if [ "${1:-}" = --self-test ]; then
  mkdir -p "$W"
  self_test
  exit 0
fi

rm -rf "$W"
mkdir -p "$W/cache"
fail=0
for lang in go rust; do
  E=$ROOT/examples/edit/$lang
  ops=0
  refusals=0
  for cq in "$E"/*.cq; do
    name=$(basename "$cq" .cq)
    if [ ! -f "$E/$name.args" ]; then
      echo "examples-edit: $lang/$name.cq has no $name.args"
      fail=1
    fi
    if [ -f "$E/$name.diff" ] && [ ! -f "$E/$name.refusal" ]; then
      ops=$((ops + 1))
    elif [ -f "$E/$name.refusal" ] && [ ! -f "$E/$name.diff" ]; then
      refusals=$((refusals + 1))
    else
      echo "examples-edit: $lang/$name.cq needs one of $name.diff and $name.refusal"
      fail=1
    fi
  done
  case $lang in go) want="4 6" ;; rust) want="4 5" ;; esac
  if [ "$ops $refusals" != "$want" ]; then
    echo "examples-edit: $lang holds $ops operation programs and $refusals refusals, not $want"
    fail=1
  fi
  [ "$fail" = 0 ] || exit 1

  # Every program as a dry run on the checked-in tree, which stays as it is.
  mkdir -p "$W/dry/$lang"
  sums "$E/source" > "$W/dry/$lang.before"
  for cq in "$E"/*.cq; do
    name=$(basename "$cq" .cq)
    run "$lang" "$E" "$E" "$name" "$W/dry/$lang/$name"
    check "$E" "$name" "$W/dry/$lang/$name" || fail=1
  done
  sums "$E/source" > "$W/dry/$lang.after"
  same "the files of $lang/source after the dry runs" "$W/dry/$lang.before" "$W/dry/$lang.after" || fail=1

  # Each program on a copy: an operation applied and built, then run again; a refusal with
  # --apply, writing nothing.
  for cq in "$E"/*.cq; do
    name=$(basename "$cq" .cq)
    C=$W/apply/$lang/$name
    mkdir -p "$C"
    cp -R "$E/source" "$C/source"
    sums "$C/source" > "$C/before"
    if [ -f "$E/$name.diff" ]; then
      run "$lang" "$C" "$E" "$name" "$C/apply" --apply --build
      check "$E" "$name" "$C/apply" || fail=1
      run "$lang" "$C" "$E" "$name" "$C/again"
      if [ "$(cat "$C/again.code")" != 0 ] || [ -s "$C/again.out" ] || [ -s "$C/again.err" ]; then
        echo "examples-edit: $lang/$name run again on its applied copy exited $(cat "$C/again.code"):"
        cat "$C/again.out" "$C/again.err"
        fail=1
      fi
    else
      run "$lang" "$C" "$E" "$name" "$C/apply" --apply
      check "$E" "$name" "$C/apply" || fail=1
      sums "$C/source" > "$C/after"
      same "the files of $lang/$name's copy after a refused --apply" "$C/before" "$C/after" || fail=1
    fi
  done
  echo "examples-edit: $lang: $ops operations and $refusals refusals"
done
if [ "$fail" != 0 ]; then
  exit 1
fi
echo "examples-edit: every program printed what it expects, refusals wrote nothing, applied changes built and changed nothing again"
