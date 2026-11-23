#!/bin/sh
# Package shared BEAM code and the locked Gleam parser next to the executables.
# Paths are relative to the checked-out repository; the installed programs relocate.
set -eu
dest=$1
version=$2
mkdir -p "$dest/beam/licenses"
(cd adapters/gleam/parser && gleam build --no-print-progress)
for lib in codespec_gleam_parser glance gleam_stdlib glexer splitter; do
  cp adapters/gleam/parser/build/dev/erlang/"$lib"/ebin/*.beam "$dest/beam/"
done
for lib in glance gleam_stdlib glexer; do
  cp adapters/gleam/parser/build/packages/"$lib"/LICEN[CS]E "$dest/beam/licenses/$lib"
done
cp LICENSE "$dest/beam/licenses/splitter-Apache-2.0"
CODESPEC_ADAPTER_VERSION="$version" elixirc --warnings-as-errors --ignore-module-conflict \
  -pa "$dest/beam" -o "$dest/beam" adapters/elixir/lib/*.ex adapters/gleam/lib/*.ex
for lang in elixir gleam; do
  for role in read write run; do
    install -m 755 adapters/elixir/bin/codespec-beam "$dest/codespec-$role-$lang"
  done
done
