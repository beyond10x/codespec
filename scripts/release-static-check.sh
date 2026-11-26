#!/bin/sh
# Whether the binaries of the musl release archive are static and run in a plain alpine container.
# The release workflow runs `archive` on the x86_64-unknown-linux-musl archive; the lint part of
# scripts/check-part.sh runs `test`.
#
# usage: scripts/release-static-check.sh static <file>...
#          exit 0 when every file is a static ELF executable: `readelf -lW` shows no INTERP program
#          header and `readelf -dW` no NEEDED entry (a static-pie has a dynamic section, without
#          NEEDED); 1 naming each other file and what it needs on stderr
#        scripts/release-static-check.sh archive <archive.tar.gz> <version>
#          unpack the archive, run `static` on every ELF file at its top level, then run each in a
#          container of $ALPINE_IMAGE with no network and nothing else mounted: `<binary> --version`
#          must print a line starting with its name (`usage: <name>` or `Usage of <name>` counts for
#          a program without --version), and `codespec --version` must print `codespec <version>`
#        scripts/release-static-check.sh test
#          this script's own cases: rustc builds one program for x86_64-unknown-linux-musl (the
#          target rust-toolchain.toml installs) and one for the host's gnu target; `static` passes
#          on the first and fails on the second. Needs no network and no container.
#
# CONTAINER (default docker) is the container engine; CONTAINER_FLAGS, word-split, are added to its
# `run`.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
ALPINE_IMAGE=${ALPINE_IMAGE:-alpine:3.22@sha256:49340b346798875b5592e5d420241f2f9cfba5f47e62ebb1446a1b34734e898f}
CONTAINER=${CONTAINER:-docker}

# static <file>...
static() {
  bad=0
  for f in "$@"; do
    if ! readelf -h "$f" > /dev/null 2>&1; then
      echo "$f: not an ELF file" >&2
      bad=1
      continue
    fi
    # A file whose headers readelf cannot read in full is not taken as static.
    if ! lw=$(readelf -lW "$f" 2>&1) || ! dw=$(readelf -dW "$f" 2>&1) ||
      printf '%s\n%s\n' "$lw" "$dw" | grep -q '^readelf: Error'; then
      echo "$f: readelf cannot read its headers" >&2
      bad=1
      continue
    fi
    interp=$(printf '%s\n' "$lw" | sed -n 's/.*\[Requesting program interpreter: \(.*\)\]/\1/p')
    needed=$(printf '%s\n' "$dw" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' | tr '\n' ' ')
    if [ -n "$interp" ]; then
      echo "$f: not static: interpreter $interp" >&2
      bad=1
    fi
    if [ -n "$needed" ]; then
      echo "$f: not static: needs ${needed% }" >&2
      bad=1
    fi
  done
  return "$bad"
}

# archive <archive.tar.gz> <version>
archive() {
  a=$1 version=$2
  tmp=${RUNNER_TEMP:-$ROOT/target}
  mkdir -p "$tmp"
  d=$(mktemp -d "$tmp/release-static-check.XXXXXX")
  trap 'rm -rf "$d"' EXIT
  tar -xzf "$a" -C "$d"
  bins=
  for f in "$d"/*; do
    if [ -f "$f" ] && readelf -h "$f" > /dev/null 2>&1; then bins="$bins $(basename "$f")"; fi
  done
  [ -n "$bins" ] || {
    echo "$a: no ELF file at its top level" >&2
    return 1
  }
  bad=0
  # shellcheck disable=SC2086 # the names are the archive's file names, never spaced
  (cd "$d" && static $bins) || bad=1
  for b in $bins; do
    rc=0
    # shellcheck disable=SC2086 # CONTAINER_FLAGS is a list of flags
    out=$("$CONTAINER" run --rm --network none ${CONTAINER_FLAGS:-} -v "$d:/release:ro" \
      "$ALPINE_IMAGE" "/release/$b" --version < /dev/null 2>&1) || rc=$?
    if [ "$rc" -ge 125 ] || ! printf '%s\n' "$out" | grep -Eq "^(usage: |Usage of )?$b([: ]|\$)"; then
      echo "$a: $b --version does not run in $ALPINE_IMAGE (exit $rc): $(printf '%s\n' "$out" | head -1)" >&2
      bad=1
    else
      echo "$b in alpine: $(printf '%s\n' "$out" | head -1)"
    fi
  done
  if [ -n "$version" ]; then
    # shellcheck disable=SC2086
    got=$("$CONTAINER" run --rm --network none ${CONTAINER_FLAGS:-} -v "$d:/release:ro" \
      "$ALPINE_IMAGE" /release/codespec --version < /dev/null 2>&1) || true
    if [ "$got" != "codespec $version" ]; then
      echo "$a: codespec --version in alpine printed \"$got\", not \"codespec $version\"" >&2
      bad=1
    fi
  fi
  return "$bad"
}

selftest() {
  T=$ROOT/target/release-static-check-test
  rm -rf "$T"
  mkdir -p "$T"
  printf '%s\n' 'fn main() { println!("hello"); }' > "$T/hello.rs"
  host=$(rustc -vV | sed -n 's/^host: //p')
  rustc --target x86_64-unknown-linux-musl -C opt-level=0 -o "$T/hello-musl" "$T/hello.rs" || {
    echo "release-static-check test: rustc cannot build for x86_64-unknown-linux-musl" \
      "(rustup target add x86_64-unknown-linux-musl; rust-toolchain.toml names it)" >&2
    return 1
  }
  rustc --target "$host" -C opt-level=0 -o "$T/hello-gnu" "$T/hello.rs"
  fail=0
  rc=0
  static "$T/hello-musl" 2> "$T/musl.err" || rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "release-static-check test: a musl binary is not taken as static:" >&2
    cat "$T/musl.err" >&2
    fail=1
  fi
  rc=0
  static "$T/hello-gnu" 2> "$T/gnu.err" || rc=$?
  if [ "$rc" -eq 0 ] || ! grep -q "hello-gnu: not static: interpreter" "$T/gnu.err" ||
    ! grep -q "hello-gnu: not static: needs .*libc.so" "$T/gnu.err"; then
    echo "release-static-check test: a $host binary is not refused naming its interpreter and libc (exit $rc):" >&2
    cat "$T/gnu.err" >&2
    fail=1
  fi
  if [ "$fail" -ne 0 ]; then return 1; fi
  echo "release-static-check test: static passes on an x86_64-unknown-linux-musl binary and fails on a $host one"
}

case "${1:-}" in
  static) shift; static "$@" ;;
  archive)
    [ "$#" -eq 3 ] || {
      echo "usage: $0 archive <archive.tar.gz> <version>" >&2
      exit 2
    }
    archive "$2" "$3" ;;
  test) selftest ;;
  *)
    echo "usage: $0 static <file>... | archive <archive.tar.gz> <version> | test" >&2
    exit 2 ;;
esac
