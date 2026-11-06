//! The algorithmic showcase: `examples/series` through codespec. Every code block, the side-by-side
//! pairs and the `cir/1` diagram are read from the checked-in files here, so they cannot drift from
//! what `task examples` writes.

use std::path::Path;

use codespec_model::Module;

use crate::showcase::{excerpt, fence, func_json, module, read};
use crate::{Result, TREE, diagram, front_matter};

/// Where the page lands, relative to the repository root.
pub const PAGE: &str = "website/docs/examples/series.md";
/// The diagram of the four functions the page walks through, after the pass.
pub const DIAGRAM: &str = "website/static/img/series-cir.svg";

const EXAMPLE: &str = "examples/series";
const SOURCE: &str = "examples/series/source/series.go";
const READ: &str = "examples/series/series.read.cir.json";
const PASSED: &str = "examples/series/series.cir.json";
const GO: &str = "examples/series/out/go/series.go";
const RUST: &str = "examples/series/out/rust/src/lib.rs";
const REFUSALS: &str = "examples/series/out/rust/refusals.json";
const RUST_CRATE: &str = "examples/series/out/rust";
const RUST_READ: &str = "examples/series/series.rust-read.cir.json";
const GO_FROM_RUST: &str = "examples/series/out/go-from-rust/series.go";

/// The functions the page walks through and the diagram draws.
const SHOWN: [&str; 4] = ["Steps", "EvenSum", "Reverse", "Residue"];

/// One construct where Go and Rust differ, the Go function that holds it and the Rust written from it.
struct Pair {
    heading: &'static str,
    why: &'static str,
    go: (&'static str, &'static str),
    rust: (&'static str, &'static str),
}

const PAIRS: [Pair; 4] = [
    Pair {
        heading: "`for` with `break`: Collatz steps",
        why: "A condition-only `for` is a `cir/1` `for` with only `cond`; the Rust writer makes it a \
              `while`. `3*n + 1` overflows `int64` for large `n`; Go wraps, so Rust uses \
              `wrapping_mul` and `wrapping_add`. `steps` is renamed `steps_` because a local may not \
              shadow the function.",
        go: ("func Steps", "}"),
        rust: ("pub fn steps", "}"),
    },
    Pair {
        heading: "`continue` in a three-clause `for`: `post` runs first",
        why: "Go runs `i++` before the next test when `continue` skips the rest of the body. Rust's \
              `while` has no post statement, so the writer puts `post` at the end of the body and \
              again before every `continue`.",
        go: ("func (s *Series) EvenSum", "}"),
        rust: ("    pub fn even_sum", "    }"),
    },
    Pair {
        heading: "Swap: every value read before any target is written",
        why: "`a, b = b, a` in Go evaluates both right-hand sides first. The writer binds each value \
              to a fresh `__value` local and then assigns the targets in order, for the swap and \
              for the two-variable post statement.",
        go: ("func (s *Series) Reverse", "}"),
        rust: ("    pub fn reverse", "    }"),
    },
    Pair {
        heading: "`switch` with a tag, and Go's remainder sign",
        why: "The tag is evaluated once into `__tag`; each `case` list becomes an `||` chain. Go's \
              `%` takes the sign of the dividend, as Rust's `wrapping_rem` does, so `-2` is a case \
              of its own.",
        go: ("func Residue", "}"),
        rust: ("pub fn residue", "}"),
    },
];

/// Where the Rust output, read back by `codespec-read-rust`, differs in the Go written from it.
const BACK_PAIRS: [Pair; 3] = [
    Pair {
        heading: "`len() as i64`: a length of another integer type",
        why: "Rust's `len()` is a `usize`; the reader reads `len() as i64` as one `cir/1` `len` \
              typed `int64`, because `cir/1` has no integer conversion. Go's `len` is an `int`, so \
              the Go writer converts it: `int64(len(s.values))`.",
        go: ("func (s *Series) Push", "}"),
        rust: ("    pub fn push", "    }"),
    },
    Pair {
        heading: "`i64` and a returned struct",
        why: "The Rust output says `i64` where the Go source said `int`, and `new` returns a \
              `Series` where the Go source returned a `*Series`. The read keeps what the Rust says, \
              so this Go has `int64` and `func New() Series`; its harness driver takes the address.",
        go: ("func New", "}"),
        rust: ("pub fn new", "}"),
    },
    Pair {
        heading: "A `while` with its post statement becomes a three-clause `for`",
        why: "The reader recognises the Rust writer's loop shape, a block with the init, a \
              `while` and the post at the end of the body, and reads it back as a three-clause \
              `for`. The tagless `switch` the Rust writer wrote as an `if` chain stays an `if` \
              chain.",
        go: ("func (s *Series) Trend", "}"),
        rust: ("    pub fn trend", "    }"),
    },
];

/// The passed module with only the functions in [`SHOWN`], so the diagram stays readable.
fn shown(passed: &Module) -> Module {
    let mut m = passed.clone();
    m.funcs.retain(|f| SHOWN.contains(&f.name.as_str()));
    m
}

fn page(root: &Path) -> Result<String> {
    let source = read(root, SOURCE)?;
    let read_module = module(root, READ)?;
    let passed = module(root, PASSED)?;
    let go = read(root, GO)?;
    let rust = read(root, RUST)?;
    let refusals = read(root, REFUSALS)?;
    let rust_read = module(root, RUST_READ)?;
    let go_from_rust = read(root, GO_FROM_RUST)?;
    let mut out = front_matter(
        "Showcase: an algorithm through codespec",
        "Showcase: series",
        "A Go package of loops, switch, slices and overflowing arithmetic read into cir/1 and written as Go and as Rust, and the Rust read back and written as Go, byte-equal under the harness.",
        2,
    );
    out.push_str(&format!(
        "# An algorithm through codespec\n\n\
         Generated by `codespec-docs` from the checked-in files of \
         [`{EXAMPLE}`]({TREE}/{EXAMPLE}); `task examples` writes them and `task check` fails when \
         any of them, or this page, drifts. [`greeter`](/docs/examples/greeter) shows the pipeline \
         step by step; this page shows where an algorithm's Go and Rust differ and what codespec \
         writes there.\n\n\
         ## The package\n\n\
         `series` holds a slice of `int` and {} functions: insertion sort, median, a sliding \
         window, gcd, Collatz steps, trial-division primes and two `switch` classifications. The \
         Go reader reads it into one `cir/1` module with {} refusals; the Rust writer writes it \
         with no refusal:\n\n{}\n\
         <details>\n<summary>The whole source, {} lines</summary>\n\n{}\n</details>\n\n\
         ## The `cir/1` document as a tree\n\n\
         Four of the functions after `codespec pass`, from [`{PASSED}`]({TREE}/{PASSED}): each \
         function's statements and each statement's expressions, with kind and type. `for`, \
         `break`, `continue` and `switch` are statements of their own; nested expressions are \
         collapsed into one line.\n\n\
         ![The cir/1 functions Steps, EvenSum, Reverse and Residue as a tree of statements and \
         expressions](/img/series-cir.svg)\n\n\
         <details>\n<summary><code>Steps</code> as <code>cir/1</code> JSON</summary>\n\n{}\n</details>\n\n",
        read_module.funcs.len(),
        read_module.refusals.len(),
        fence("json", "examples/series/out/rust/refusals.json", &refusals),
        source.lines().count(),
        fence("go", SOURCE, &source),
        fence("json", "series.cir.json, funcs: Steps", &func_json(&passed, "Steps")?),
    ));
    out.push_str("## Go and the Rust written from it\n\n");
    for pair in &PAIRS {
        out.push_str(&format!(
            "### {}\n\n{}\n\n{}\n{}\n",
            pair.heading,
            pair.why,
            fence(
                "go",
                "source/series.go",
                &excerpt(&source, pair.go.0, Some(pair.go.1))?
            ),
            fence(
                "rust",
                "out/rust/src/lib.rs",
                &excerpt(&rust, pair.rust.0, Some(pair.rust.1))?
            ),
        ));
    }
    out.push_str(&format!(
        "## Rust → `cir/1` → Go\n\n\
         The other direction: the Rust output above is read back by `codespec-read-rust`, passed \
         and written as Go.\n\n\
         ```shell-session\n\
         $ codespec read --lang rust {RUST_CRATE} --out series.rust-read.cir.json\n\
         $ codespec pass series.rust-read.cir.json --out series.rust-passed.cir.json\n\
         $ codespec write --lang go series.rust-passed.cir.json --out out/go-from-rust\n\
         ```\n\n\
         The read gives one `cir/1` module of {} functions with {} refusals, checked in as \
         [`{RUST_READ}`]({TREE}/{RUST_READ}); the Go is checked in as \
         [`{GO_FROM_RUST}`]({TREE}/{GO_FROM_RUST}).\n\n",
        rust_read.funcs.len(),
        rust_read.refusals.len(),
    ));
    for pair in &BACK_PAIRS {
        out.push_str(&format!(
            "### {}\n\n{}\n\n{}\n{}\n",
            pair.heading,
            pair.why,
            fence(
                "rust",
                "out/rust/src/lib.rs",
                &excerpt(&rust, pair.rust.0, Some(pair.rust.1))?
            ),
            fence(
                "go",
                "out/go-from-rust/series.go",
                &excerpt(&go_from_rust, pair.go.0, Some(pair.go.1))?
            ),
        ));
    }
    out.push_str(&format!(
        "## Checked by the harness\n\n\
         ```shell-session\n$ codespec harness {EXAMPLE} --seeds 200 --commands 300\n```\n\n\
         The [harness](/docs/concepts/harness) runs the original Go, the Go output, the Rust \
         output and the Go written from the Rust output on the `series` trace: 200 seeded traces \
         of 300 commands that push `int64` extremes, ask for non-positive windows and steps, and \
         call every function. All four must print the same bytes; `task examples-series` runs it \
         in `task check`.\n\n\
         <details>\n<summary>The whole Go output</summary>\n\n{}\n</details>\n\n\
         <details>\n<summary>The whole Rust output</summary>\n\n{}\n</details>\n\n\
         <details>\n<summary>The whole Go written from the Rust output</summary>\n\n{}\n</details>\n\n\
         ## What this does not show\n\n\
         - Slice expressions (`s[i:j]`), sized `make`, labeled `break` and `continue`, \
         `fallthrough` and `goto` are read as `foreign` statements with a refusal; the package \
         avoids them.\n\
         - The Rust read here is the Rust writer's own output. Hand-written Rust is read in \
         [`inventory`]({TREE}/examples/inventory); Rust outside the reader's subset, a closure \
         for one, is kept as `foreign` with a refusal.\n",
        fence("go", GO, &go),
        fence("rust", RUST, &rust),
        fence("go", GO_FROM_RUST, &go_from_rust),
    ));
    Ok(out)
}

/// The page and the diagram.
pub fn outputs(root: &Path) -> Result<Vec<(&'static str, String)>> {
    let passed = module(root, PASSED)?;
    Ok(vec![
        (PAGE, page(root)?),
        (
            DIAGRAM,
            diagram::svg(
                &shown(&passed),
                "Steps, EvenSum, Reverse and Residue of the cir/1 module series after codespec pass",
            ),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn the_series_page_pairs_each_go_construct_with_its_rust_and_draws_the_loops() {
        let generated = outputs(&root()).unwrap();
        let page = &generated[0].1;
        for want in [
            "```go title=\"examples/series/source/series.go\"",
            "func Steps(n int) (int, error) {",
            "pub fn steps(mut n: i64) -> Result<i64, Error> {",
            "i = i.wrapping_add(1i64);\n                        continue;",
            "let __value0 = self.values[j as usize];",
            "let __tag = v.wrapping_rem(3i64);",
            "\"k\": \"for\"",
        ] {
            assert!(page.contains(want), "missing {want:?}");
        }
        let svg = &generated[1].1;
        assert!(svg.starts_with("<svg "));
        for want in [
            "Steps", "EvenSum", "Reverse", "Residue", "break", "continue",
        ] {
            assert!(svg.contains(want), "the diagram has no {want:?}");
        }
        assert!(
            !svg.contains("func Series.MaxWindow"),
            "the diagram draws only the four"
        );
        assert_eq!(svg, &outputs(&root()).unwrap()[1].1);
    }

    #[test]
    fn the_series_page_shows_the_rust_output_read_back_and_written_as_go() {
        let page = page(&root()).unwrap();
        for want in [
            "## Rust → `cir/1` → Go",
            "codespec read --lang rust examples/series/out/rust",
            "```rust title=\"out/rust/src/lib.rs\"\n    pub fn push(&mut self, v: i64) -> i64 {",
            "```go title=\"out/go-from-rust/series.go\"\nfunc (s *Series) Push(v int64) int64 {",
            "return int64(len(s.values))",
            "func New() Series {",
            "examples/series/series.rust-read.cir.json",
            "the original Go, the Go output, the Rust output and the Go written from the Rust \
             output",
        ] {
            assert!(page.contains(want), "missing {want:?}");
        }
        assert!(
            !page.contains("this example runs Go → `cir/1` → Rust only"),
            "the page still says the Rust output is not read back"
        );
    }
}
