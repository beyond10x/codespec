#!/bin/sh
# A reader fixture that prints fixed output: spanned.json beside it when called with --spans,
# module.cir.json otherwise.
here=$(dirname "$0")
if [ "$1" = "--spans" ]; then
	exec cat "$here/spanned.json"
fi
exec cat "$here/module.cir.json"
