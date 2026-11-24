#!/bin/sh
# A reader fixture standing in for codespec-read-java: prints the bare cir/1 module beside it
# (module.cir.json), with or without --spans, as a reader that gives no spans does.
here=$(dirname "$0")
exec cat "$here/module.cir.json"
