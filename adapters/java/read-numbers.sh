#!/bin/sh
# Print the numbers of a project read: the cir/1 documents (*.cir.json) in the directory given,
# canonical as codespec-read-java --all writes them. Modules, functions, refusals, then the
# refusals grouped by reason, most frequent first.
set -eu
dir=${1:?usage: read-numbers.sh <directory of *.cir.json>}
modules=0
funcs=0
refusals=0
for f in "$dir"/*.cir.json; do
    modules=$((modules + 1))
    n=$(awk '/^  "funcs": \[/ {on=1; next} on && /^  \]/ {on=0} on && /^    \{/ {c++} END {print c+0}' "$f")
    r=$(awk '/^  "refusals": \[/ {on=1; next} on && /^  \]/ {on=0} on && /^    \{/ {c++} END {print c+0}' "$f")
    funcs=$((funcs + n))
    refusals=$((refusals + r))
done
echo "modules $modules"
echo "functions $funcs"
echo "refusals $refusals"
echo "refusals by reason:"
for f in "$dir"/*.cir.json; do
    awk '/^  "refusals": \[/ {on=1; next} on && /^  \]/ {on=0} on && /^      "reason": / {sub(/^      "reason": "/, ""); sub(/",?$/, ""); print}' "$f"
done | sort | uniq -c | sort -k1,1nr -k2 | sed 's/^ *\([0-9]*\) /  \1 /'
