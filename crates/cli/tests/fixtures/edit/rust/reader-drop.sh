#!/bin/sh
# A reader fixture: calls the real Rust reader beside it (real-reader), and when the directory it
# reads lies under $CODESPEC_CACHE_DIR deletes the last function of funcs from the output, with
# its spans and unspanned entries.
here=$(dirname "$0")
case "$2" in
"$CODESPEC_CACHE_DIR"/*) ;;
*) exec "$here/real-reader" "$@" ;;
esac
out=$("$here/real-reader" "$@") || exit 1
printf '%s\n' "$out" | jq '
	((.cir.funcs | length) - 1 | tostring) as $n
	| def mine: (.at | split(".")) as $p | $p[0] == "funcs" and $p[1] == $n;
	.cir.funcs |= .[:-1]
	| .spans |= map(select(mine | not))
	| .unspanned |= map(select(mine | not))'
