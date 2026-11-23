#!/bin/sh
# Read every non-test Go package of a repository with codespec; print one line per package:
# exit, functions read of total, refusals; documents land in $OUT.
# With --deps, also read every package the repository imports (`go list -deps`, the standard
# library included), each document's `path` set to the import path; packages are read in parallel
# and the lines printed sorted.
# A document is named by the package's import path (with --deps) or its directory relative to the
# repository (without; `root` for the repository's own directory), encoded so that two packages
# never share a name: % as %25, _ as %5F, then / as _ (`a/b` is `a_b`, `a_b` is `a%5Fb`).
# Every package is tried; the exit status is 1 when `go list` failed, two packages map to one
# name, or a package failed to read, 0 otherwise.
# usage: scripts/read-repo.sh [--deps] <repo dir> <out dir> [codespec checkout, default: this one; needs task build]
# CODESPEC and CODESPEC_ADAPTERS, when set, name the codespec binary and the adapter directory.
DEPS=
if [ "$1" = --deps ]; then DEPS=1; shift; fi
REPO=$1 OUT=$2 WT=${3:-$(cd "$(dirname "$0")/.." && pwd)}
CS=${CODESPEC:-$WT/target/debug/codespec} AD=${CODESPEC_ADAPTERS:-$WT/target/adapters}
mkdir -p "$OUT" || exit 1
OUT=$(cd "$OUT" && pwd)
cd "$REPO" || exit 1
REPO=$(pwd)
go mod download >/dev/null 2>&1
status=0

# Enumerate into go-list.txt, one `<label> <dir>` line per package.
if [ -n "$DEPS" ]; then
  go list -deps -f '{{.ImportPath}} {{.Dir}}' ./... > "$OUT/go-list.txt" 2> "$OUT/go-list.log"
else
  go list -f '{{.Dir}}' ./... > "$OUT/go-dirs.txt" 2> "$OUT/go-list.log"
fi
if [ $? -ne 0 ]; then
  status=1
  echo "go list | FAIL | $(head -1 "$OUT/go-list.log" | cut -c1-160)"
fi
if [ -z "$DEPS" ]; then
  while read -r d; do
    rel=${d#"$REPO"}; rel=${rel#/}
    echo "${rel:-.} $d"
  done < "$OUT/go-dirs.txt" > "$OUT/go-list.txt"
  rm -f "$OUT/go-dirs.txt"
fi

# packages.txt: `<label> <dir> <document name>`; a package go list gives no directory for (an
# import no module provides) is a failure.
awk -v nodir="$OUT/no-dir.txt" '
  NF < 2 { print $1 " | FAIL | go list gives no directory" > nodir; next }
  { n = $1; if (n == ".") n = "root"; gsub(/%/, "%25", n); gsub(/_/, "%5F", n); gsub(/\//, "_", n); print $1, $2, n }' \
  "$OUT/go-list.txt" > "$OUT/packages.txt"
if [ -s "$OUT/no-dir.txt" ]; then
  status=1
  cat "$OUT/no-dir.txt"
fi
rm -f "$OUT/no-dir.txt"
if [ ! -s "$OUT/packages.txt" ]; then
  status=1
  echo "go list | FAIL | no package to read"
fi
for n in $(awk '{ print $3 }' "$OUT/packages.txt" | LC_ALL=C sort | uniq -d); do
  status=1
  echo "$n | FAIL | two packages map to the document name $n"
done

# read1 <import path or ""> <dir> <document name> <label>
read1() {
  if CODESPEC_READ_GO_IMPORT_PATH="$1" "$CS" read --lang go "$2" --out "$OUT/$3.cir.json" --adapters "$AD" > "$OUT/$3.log" 2>&1; then
    m=$("$CS" measure "$OUT/$3.cir.json" | tail -1)
    r=$(grep -c '"what"' "$OUT/$3.cir.json")
    x=$(grep -c '"k": "extern"' "$OUT/$3.cir.json")
    echo "${1:-$4} | ok | $m | refusals $r | extern calls $x"
  else
    echo "${1:-$4} | FAIL | $(grep -v '^codespec:' "$OUT/$3.log" | head -1 | cut -c1-160)"
    return 1
  fi
}

if [ -n "$DEPS" ]; then
  export CS AD OUT
  : > "$OUT/read.txt"
  if [ -s "$OUT/packages.txt" ]; then
    xargs -P "$(nproc)" -n 3 sh -c '
      if CODESPEC_READ_GO_IMPORT_PATH="$0" "$CS" read --lang go "$1" --out "$OUT/$2.cir.json" --adapters "$AD" > "$OUT/$2.log" 2>&1; then
        m=$("$CS" measure "$OUT/$2.cir.json" | tail -1)
        r=$(grep -c "\"what\"" "$OUT/$2.cir.json")
        echo "$0 | ok | $m | refusals $r"
      else
        echo "$0 | FAIL | $(grep -v "^codespec:" "$OUT/$2.log" | head -1 | cut -c1-160)"
      fi' < "$OUT/packages.txt" | LC_ALL=C sort > "$OUT/read.txt"
  fi
  cat "$OUT/read.txt"
  if grep -q '| FAIL |' "$OUT/read.txt"; then status=1; fi
  exit $status
fi

while read -r label d name; do
  read1 "" "$d" "$name" "$label" || status=1
done < "$OUT/packages.txt"
exit $status
