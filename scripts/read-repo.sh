#!/bin/sh
# Read every non-test Go package of a repository with codespec; print one line per package:
# exit, functions read of total, refusals; documents land in $OUT.
# With --deps, also read every package the repository imports (`go list -deps`, the standard
# library included), each document named by its import path with / as _, its `path` set to the
# import path; packages are read in parallel and the lines printed sorted.
# usage: scripts/read-repo.sh [--deps] <repo dir> <out dir> [codespec checkout, default: this one; needs task build]
# CODESPEC and CODESPEC_ADAPTERS, when set, name the codespec binary and the adapter directory.
DEPS=
if [ "$1" = --deps ]; then DEPS=1; shift; fi
REPO=$1 OUT=$2 WT=${3:-$(cd "$(dirname "$0")/.." && pwd)}
CS=${CODESPEC:-$WT/target/debug/codespec} AD=${CODESPEC_ADAPTERS:-$WT/target/adapters}
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO" || exit 1
go mod download >/dev/null 2>&1

# read1 <import path or ""> <dir> <document name>
read1() {
  if CODESPEC_READ_GO_IMPORT_PATH="$1" "$CS" read --lang go "$2" --out "$OUT/$3.cir.json" --adapters "$AD" > "$OUT/$3.log" 2>&1; then
    m=$("$CS" measure "$OUT/$3.cir.json" | tail -1)
    r=$(grep -c '"what"' "$OUT/$3.cir.json")
    x=$(grep -c '"k": "extern"' "$OUT/$3.cir.json")
    echo "${1:-$4} | ok | $m | refusals $r | extern calls $x"
  else
    echo "${1:-$4} | FAIL | $(grep -v '^codespec:' "$OUT/$3.log" | head -1 | cut -c1-160)"
  fi
}

if [ -n "$DEPS" ]; then
  export CS AD OUT
  go list -deps -f '{{.ImportPath}} {{.Dir}}' ./... 2>/dev/null |
    xargs -P "$(nproc)" -n 2 sh -c '
      name=$(echo "$0" | tr / _)
      if CODESPEC_READ_GO_IMPORT_PATH="$0" "$CS" read --lang go "$1" --out "$OUT/$name.cir.json" --adapters "$AD" > "$OUT/$name.log" 2>&1; then
        m=$("$CS" measure "$OUT/$name.cir.json" | tail -1)
        r=$(grep -c "\"what\"" "$OUT/$name.cir.json")
        echo "$0 | ok | $m | refusals $r"
      else
        echo "$0 | FAIL | $(grep -v "^codespec:" "$OUT/$name.log" | head -1 | cut -c1-160)"
      fi' | LC_ALL=C sort > "$OUT/read.txt"
  cat "$OUT/read.txt"
  ! grep -q '| FAIL |' "$OUT/read.txt"
  exit $?
fi

for d in $(go list -f '{{.Dir}}' ./... 2>/dev/null); do
  rel=${d#"$REPO"}; rel=${rel#/}; name=$(echo "${rel:-root}" | tr / _)
  read1 "" "$d" "$name" "$rel"
done
