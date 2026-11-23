#!/bin/sh
# Read a native branch twice, then run each of its translations on the original calls.
set -eu
codespec=$1
adapters=$2
work=target/language-adapters-check
fixture=conformance/cases/stmt-if
mkdir -p "$work"
for from in elixir gleam odin; do
  source="$fixture/$from"
  "$adapters/codespec-run-$from" --build "$work/$from-original" "$source" "$fixture/calls.json" > "$work/$from.original.txt"
  cmp "$fixture/expected.txt" "$work/$from.original.txt"
  for read in 1 2; do
    "$codespec" read --lang "$from" "$source" --adapters "$adapters" --out "$work/$from.$read.json"
  done
  cmp "$work/$from.1.json" "$work/$from.2.json"
  jq -e '(.refusals // [] | length) == 0 and (.funcs | length) == 1' "$work/$from.1.json" > /dev/null
  "$codespec" pass "$work/$from.1.json" --out "$work/$from.json"
  for to in elixir gleam odin; do
    out="$work/$from-to-$to"
    "$codespec" write --lang "$to" "$work/$from.json" --adapters "$adapters" --out "$out"
    # Different readers can produce the very same native package. Its execution
    # for these calls already ran in this invocation's first iteration.
    first="$work/elixir-to-$to"
    if [ "$from" != elixir ] && diff -qr "$first" "$out" > /dev/null; then
      cp "$first.txt" "$out.txt"
    else
      "$adapters/codespec-run-$to" --build "$out-build" "$out" "$fixture/calls.json" > "$out.txt"
    fi
    cmp "$work/$from.original.txt" "$out.txt"
    if [ "$from" = "$to" ]; then
      "$codespec" read --lang "$to" "$out" --adapters "$adapters" --out "$out.read.json"
      jq -e '(.refusals // [] | length) == 0' "$out.read.json" > /dev/null
      "$codespec" pass "$out.read.json" --out "$out.passed.json"
      "$codespec" eval "$out.passed.json" --calls "$fixture/calls.json" > "$out.eval.txt"
      cmp "$out.txt" "$out.eval.txt"
    fi
  done
done
