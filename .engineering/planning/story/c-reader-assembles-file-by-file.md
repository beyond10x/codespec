---
format: aep.planning-md/3
id: story:c-reader-assembles-file-by-file
kind: story
status: draft
title: The C reader does not hold every file's tree until the module is assembled
relations:
- decomposes: epic:adapters-untrusted-input
revision: 1
---
# Story: the C reader does not hold every file's tree until the module is assembled

## Outcome

The C reader's memory is bounded by its largest `.c` file and not by the number of files, so the
budget of 256 MiB of clang's JSON for a whole directory can go.

## Why

Found by the adversary pass on `story:readers-bound-untrusted-input`
(`review-result:readers-bound-untrusted-input-adversary-1`), in the reader before that unit: every
file's parsed tree stays in memory until the module is assembled, at a measured 6.4 bytes of memory
per byte of JSON (313 MiB resident for one file of 42 MB of JSON, 259 MiB for each further one).
That unit bounded the cost with one budget for the directory, which refuses a directory of about 36
files of 7 MB of JSON each. A header counts once per `.c` file that includes it.

## Acceptance

- A directory of 100 `.c` files that each include one 600 kB header reads with exit 0 under an
  address-space limit of 3 GiB.
- The total budget `MAX_JSON_TOTAL_BYTES` is removed or raised, and the crate doc states what
  bounds the reader's memory.
- `codespec conform --lang c --role reader` keeps its pass count.

## Scope

- Own: `adapters/c-read/src/lib.rs` (`read`, the loop that collects `asts`), the modules that
  assemble across files, `adapters/c-read/tests/bounds.rs`. Inferred.
