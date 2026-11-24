#!/bin/sh
# A reader fixture: calls the real Go reader beside it (real-reader); when the directory it reads
# lies under $CODESPEC_CACHE_DIR (the copy being verified), it first appends a line to the file
# $TOUCH names, so that file changes between its read and --apply.
here=$(dirname "$0")
case "$2" in
"$CODESPEC_CACHE_DIR"/*) printf '\n// changed meanwhile\n' >> "$TOUCH" ;;
esac
exec "$here/real-reader" "$@"
