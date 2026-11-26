//! `crates/corpus`, one module per story of the reader self-improvement loop. Every test runs the
//! `codespec-corpus` binary over recorded data: a stub `gh` (POSIX sh) in `tests/fixtures/` prints
//! canned JSON, so no test touches the network.

mod common;
mod fetch;
mod select;
