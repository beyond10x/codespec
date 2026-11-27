#!/bin/sh
# Whether a release archive holds every file of the repository's summaries/ (the Go standard
# library's effect summaries and the linux/amd64 syscall barrier) under
# <dir>share/codespec/summaries/, where `codespec diff` and `codespec effects` look for them beside
# the binary. The release workflow runs it on each platform archive, `task conformance-kit` on the
# conformance archive.
#
# usage: scripts/release-summaries-check.sh <archive.tar.gz> [<dir>]
#          <dir>: the directory the archive holds everything under, with its trailing slash
#          (codespec-conformance-<version>/); empty for a flat archive
#        exit 0 when every file is listed by `tar -tzf`, 1 naming each missing one on stderr
set -eu
[ "$#" -ge 1 ] && [ "$#" -le 2 ] || {
  echo "usage: $0 <archive.tar.gz> [<dir>]" >&2
  exit 2
}
archive=$1
dir=${2-}
summaries="$(dirname "$0")/../summaries"
listing=$(tar -tzf "$archive") || {
  echo "$archive: tar -tzf failed" >&2
  exit 1
}
found=0
missing=0
for f in "$summaries"/*.json; do
  [ -e "$f" ] || continue
  found=$((found + 1))
  want="${dir}share/codespec/summaries/$(basename "$f")"
  if ! printf '%s\n' "$listing" | grep -Fxq -e "$want" -e "./$want"; then
    echo "$archive: missing $want" >&2
    missing=$((missing + 1))
  fi
done
[ "$found" -gt 0 ] || {
  echo "$summaries: no *.json" >&2
  exit 1
}
[ "$missing" -eq 0 ]
