#!/bin/sh
# The tools CI downloads, pinned by version and SHA-256, in one place. check.yml downloads them once
# per run in its `check / tools` job, caches them keyed by this file's hash, and every matrix job
# restores that cache; a job fetches only what the cache did not give it. release-build.yml fetches
# go-task the same way.
#
# go-task comes from github.com through `gh release download` with the job's token (GH_TOKEN), whose
# rate limit is far higher than an anonymous download's; Groovy comes from Maven Central.
#
# usage: scripts/ci-tools.sh fetch <dir> <tool>...     download each missing file into <dir>, then verify
#        scripts/ci-tools.sh verify <dir> <tool>...    check each file's SHA-256
#        scripts/ci-tools.sh install <dir> <tool>...   verify, unpack, and put each on GITHUB_PATH
# A tool is `task` or `groovy`.
set -eu

TASK_VERSION=3.52.0
TASK_SHA256=02c679ffae53dca791804847d78b31731615894e292948397c971c87ac9e95bd
GROOVY_VERSION=2.4.10
GROOVY_SHA256=1c4dff3b6edf9a8ced3bca658ee1857cee90cfed1ee3474a2790045033c317a9

file() {
  case "$1" in
    task) echo task_linux_amd64.tar.gz ;;
    groovy) echo "groovy-binary-$GROOVY_VERSION.zip" ;;
    *) echo "ci-tools: unknown tool $1" >&2; return 2 ;;
  esac
}

sum() {
  case "$1" in
    task) echo "$TASK_SHA256" ;;
    groovy) echo "$GROOVY_SHA256" ;;
  esac
}

fetch_one() {
  f=$(file "$2")
  if [ -f "$1/$f" ]; then echo "ci-tools: $f present"; return 0; fi
  echo "ci-tools: downloading $f"
  case "$2" in
    task)
      gh release download --repo go-task/task "v$TASK_VERSION" --pattern "$f" --dir "$1" ;;
    groovy)
      curl --fail --silent --show-error --location --retry 5 --retry-delay 15 --retry-all-errors \
        --output "$1/$f.part" \
        "https://repo1.maven.org/maven2/org/codehaus/groovy/groovy-binary/$GROOVY_VERSION/$f"
      mv "$1/$f.part" "$1/$f" ;;
  esac
}

verify_one() {
  echo "$(sum "$2")  $1/$(file "$2")" | sha256sum --check --strict
}

install_one() {
  verify_one "$1" "$2"
  case "$2" in
    task)
      mkdir -p "$1/task-bin"
      tar -xzf "$1/$(file task)" -C "$1/task-bin" task
      echo "$1/task-bin" >> "$GITHUB_PATH" ;;
    groovy)
      rm -rf "$1/groovy-$GROOVY_VERSION"
      unzip -q "$1/$(file groovy)" -d "$1"
      echo "GROOVY_HOME=$1/groovy-$GROOVY_VERSION" >> "$GITHUB_ENV"
      echo "$1/groovy-$GROOVY_VERSION/bin" >> "$GITHUB_PATH" ;;
  esac
}

cmd=${1:-}
[ $# -ge 3 ] || { echo "usage: scripts/ci-tools.sh fetch|verify|install <dir> <tool>..." >&2; exit 2; }
dir=$2
shift 2
mkdir -p "$dir"
for tool in "$@"; do
  file "$tool" > /dev/null
  case "$cmd" in
    fetch) fetch_one "$dir" "$tool"; verify_one "$dir" "$tool" ;;
    verify) verify_one "$dir" "$tool" ;;
    install) install_one "$dir" "$tool" ;;
    *) echo "ci-tools: unknown command $cmd" >&2; exit 2 ;;
  esac
done
