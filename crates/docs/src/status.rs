//! The status data, `website/data/status.json` (`b10x-status/1`), from [`CAPABILITIES`].
//!
//! A shipped capability names the check that holds it, as `file::name`: a Rust test
//! (`fn name(`), a Go test (`func name(`) or a `Taskfile.yml` task the gate runs (`  name:`).
//! Generation fails when that file has no such function or task, so a claim cannot outlive the
//! check it rests on. A capability that is not shipped names none.

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::Result;

/// Where the status data lands, relative to the repository root.
pub const DATA: &str = "website/data/status.json";

/// The release the list describes.
pub const VERSION: &str = "0.1.0";

/// The date the list below was last checked against the tree.
const AS_OF: &str = "2026-11-06";

/// One capability on the status page.
pub struct Capability {
    pub area: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    /// `shipped`, `decided` or `planned`.
    pub status: &'static str,
    /// For a shipped capability, the check that holds it: `path::name`.
    pub evidence: Option<&'static str>,
    pub href: &'static str,
}

/// Every capability the site claims, in the order the status table lists them.
pub const CAPABILITIES: &[Capability] = &[
    Capability {
        area: "Model",
        label: "cir/1, defined once in Rust",
        detail: "Types, declarations, statements and expressions as tagged nodes; the JSON Schema is generated from the model and drift-checked.",
        status: "shipped",
        evidence: Some(
            "crates/model/tests/model.rs::the_checked_in_schema_is_the_one_the_model_generates",
        ),
        href: "/docs/reference/cir-1",
    },
    Capability {
        area: "Model",
        label: "Strict reading",
        detail: "An unknown field, an unknown kind, an expression without a type and any format other than cir/1 are refused.",
        status: "shipped",
        evidence: Some("crates/model/tests/model.rs::an_expression_without_a_type_is_refused"),
        href: "/docs/concepts/cir-1",
    },
    Capability {
        area: "Model",
        label: "Canonical encoding",
        detail: "One byte sequence per document; codespec validate --canonical refuses any other.",
        status: "shipped",
        evidence: Some(
            "crates/model/tests/model.rs::every_example_document_reads_and_is_canonical",
        ),
        href: "/docs/concepts/cir-1",
    },
    Capability {
        area: "Readers",
        label: "Go reader on go/types",
        detail: "codespec-read-go lifts one Go package; reading twice gives the same bytes, equal to the checked-in document of every example.",
        status: "shipped",
        evidence: Some("Taskfile.yml::examples-check"),
        href: "/docs/concepts/pipeline",
    },
    Capability {
        area: "Readers",
        label: "Foreign statements and refusals",
        detail: "A construct cir/1 cannot express (a goroutine) is kept verbatim as a foreign statement and listed as a refusal; the Rust writer then refuses the function holding it.",
        status: "shipped",
        evidence: Some(
            "adapters/rust/tests/writer.rs::orders_refuses_exactly_audit_at_its_goroutine",
        ),
        href: "/docs/concepts/pipeline#refusals",
    },
    Capability {
        area: "Readers",
        label: "Rust reader on syn",
        detail: "codespec-read-rust: Rust to cir/1 with its own name and type resolution for the cir/1 subset, so Rust to cir/1 to Go runs under the harness. Decided for 0.1.0, in progress; not in this tree yet.",
        status: "decided",
        evidence: None,
        href: "/docs/vision#milestones",
    },
    Capability {
        area: "Passes",
        label: "lookup-guard",
        detail: "A comma-ok map lookup followed by an early-return guard becomes one lookup_else statement.",
        status: "shipped",
        evidence: Some(
            "crates/passes/tests/passes.rs::lookup_guard_rewrites_a_guarded_comma_ok_lookup",
        ),
        href: "/docs/concepts/pipeline#passes-and-facts",
    },
    Capability {
        area: "Passes",
        label: "Seven facts",
        detail: "shared, messages, fallible, comma_ok, read_only, displays and unordered_iteration, computed once per module for every writer.",
        status: "shipped",
        evidence: Some(
            "crates/passes/tests/passes.rs::orders_gets_its_lookups_rewritten_and_its_facts",
        ),
        href: "/docs/concepts/pipeline#passes-and-facts",
    },
    Capability {
        area: "Passes",
        label: "Passes reach their fixed point in one run",
        detail: "codespec pass over its own output gives the same bytes.",
        status: "shipped",
        evidence: Some(
            "crates/passes/tests/passes.rs::passes_run_twice_give_the_same_bytes_as_once",
        ),
        href: "/docs/concepts/pipeline#passes-and-facts",
    },
    Capability {
        area: "Writers",
        label: "Go writer on go/ast and go/printer",
        detail: "codespec-write-go writes gofmt-clean, vetted Go; the reader reads it back to the same cir/1.",
        status: "shipped",
        evidence: Some("adapters/go/cmd/codespec-write-go/write_test.go::TestOrdersFixedPoint"),
        href: "/docs/concepts/pipeline#writers",
    },
    Capability {
        area: "Writers",
        label: "Rust writer on syn and prettyplease",
        detail: "codespec-write-rust writes a Cargo crate from the module and its facts; the crate builds without warnings.",
        status: "shipped",
        evidence: Some("adapters/rust/tests/writer.rs::the_written_crate_builds_without_warnings"),
        href: "/docs/guides/translate-go-to-rust",
    },
    Capability {
        area: "Writers",
        label: "Writers decide nothing the facts do not say",
        detail: "A module without facts is refused whole by the Rust writer, naming the pass to run.",
        status: "shipped",
        evidence: Some(
            "adapters/rust/tests/writer.rs::a_module_without_facts_is_refused_naming_the_pass",
        ),
        href: "/docs/concepts/pipeline#writers",
    },
    Capability {
        area: "Harness",
        label: "Byte-for-byte comparison on seeded traces",
        detail: "The original Go, the Go output and the Rust output of orders agree on 200 traces of 300 commands.",
        status: "shipped",
        evidence: Some("Taskfile.yml::harness"),
        href: "/docs/concepts/harness",
    },
    Capability {
        area: "Harness",
        label: "A planted change is caught",
        detail: "A one-token change in the Rust output exits 1 naming the seed, the implementation and the first differing line.",
        status: "shipped",
        evidence: Some(
            "crates/cli/tests/harness.rs::a_planted_one_token_change_in_the_rust_output_fails_with_seed_rust_and_first_line",
        ),
        href: "/docs/concepts/harness",
    },
    Capability {
        area: "Harness",
        label: "Traces pinned across platforms",
        detail: "SplitMix64 and integer arithmetic only: one seed gives the same trace bytes everywhere.",
        status: "shipped",
        evidence: Some("crates/harness/tests/harness.rs::a_trace_is_pinned_across_platforms"),
        href: "/docs/concepts/harness#traces",
    },
    Capability {
        area: "Examples",
        label: "orders",
        detail: "A Go order service read, passed, written as Go and as Rust, and compared under the harness.",
        status: "shipped",
        evidence: Some("Taskfile.yml::examples-rust"),
        href: "/docs/getting-started",
    },
    Capability {
        area: "Examples",
        label: "greeter",
        detail: "A Go greeting service written as Go and as Rust with no refusal; the showcase page is generated from its files.",
        status: "shipped",
        evidence: Some("Taskfile.yml::examples-greeter"),
        href: "/docs/examples/greeter",
    },
    Capability {
        area: "Examples",
        label: "An algorithmic example",
        detail: "Loops, switch, slices and arithmetic, held to the same criteria as orders.",
        status: "planned",
        evidence: None,
        href: "/docs/vision#milestones",
    },
    Capability {
        area: "Interfaces",
        label: "codespec command line",
        detail: "validate, schema, read, pass, write and harness.",
        status: "shipped",
        evidence: Some(
            "crates/cli/tests/harness.rs::the_three_implementations_agree_on_a_few_seeds",
        ),
        href: "/docs/reference/cli",
    },
];

/// Whether `source`, the text of `file`, defines the check `name`.
fn defines(file: &str, source: &str, name: &str) -> bool {
    if file.ends_with("Taskfile.yml") {
        source.lines().any(|line| line == format!("  {name}:"))
    } else if file.ends_with(".go") {
        source.contains(&format!("func {name}("))
    } else {
        source.contains(&format!("fn {name}("))
    }
}

/// The status document for the repository at `root`.
pub fn document(root: &Path) -> Result<String> {
    document_for(root, CAPABILITIES)
}

fn document_for(root: &Path, capabilities: &[Capability]) -> Result<String> {
    let mut items = Vec::new();
    for capability in capabilities {
        let label = capability.label;
        if !matches!(capability.status, "shipped" | "decided" | "planned") {
            return Err(format!(
                "{label}: status {} is not shipped, decided or planned",
                capability.status
            ));
        }
        match (capability.status, capability.evidence) {
            ("shipped", Some(evidence)) => {
                let (file, name) = evidence
                    .split_once("::")
                    .ok_or_else(|| format!("{evidence}: not `path::name`"))?;
                let source = fs::read_to_string(root.join(file))
                    .map_err(|e| format!("{label}: reading {file}: {e}"))?;
                if !defines(file, &source, name) {
                    return Err(format!("{label}: {file} has no test {name}"));
                }
            }
            ("shipped", None) => return Err(format!("{label}: shipped without a test")),
            (_, Some(_)) => return Err(format!("{label}: only a shipped item names a test")),
            (_, None) => {}
        }
        items.push(json!({
            "area": capability.area,
            "label": label,
            "detail": capability.detail,
            "status": capability.status,
            "href": capability.href,
        }));
    }
    let document = json!({
        "format": "b10x-status/1",
        "asOf": AS_OF,
        "version": VERSION,
        "source": "codespec-docs, from its capability list; every shipped item names the test or gate task that holds it",
        "items": items,
    });
    serde_json::to_string_pretty(&document)
        .map(|text| text + "\n")
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    fn capability(status: &'static str, evidence: Option<&'static str>) -> Capability {
        Capability {
            area: "A",
            label: "L",
            detail: "D",
            status,
            evidence,
            href: "/docs/",
        }
    }

    #[test]
    fn every_shipped_capability_rests_on_a_test_that_exists() {
        let document = document(root()).unwrap();
        assert!(document.contains("\"format\": \"b10x-status/1\""));
        assert!(document.contains("\"version\": \"0.1.0\""));
    }

    #[test]
    fn a_vanished_test_or_a_shipped_item_without_one_is_refused() {
        let gone = capability("shipped", Some("crates/model/tests/model.rs::no_such_test"));
        let error = document_for(root(), &[gone]).unwrap_err();
        assert!(error.contains("has no test no_such_test"), "{error}");
        let task = capability("shipped", Some("Taskfile.yml::no-such-task"));
        assert!(document_for(root(), &[task]).is_err());
        let go = capability(
            "shipped",
            Some("adapters/go/cmd/codespec-write-go/write_test.go::TestNoSuchThing"),
        );
        assert!(document_for(root(), &[go]).is_err());
        assert!(document_for(root(), &[capability("shipped", None)]).is_err());
        assert!(
            document_for(
                root(),
                &[capability("planned", Some("Taskfile.yml::harness"))]
            )
            .is_err()
        );
        assert!(document_for(root(), &[capability("released", None)]).is_err());
        assert!(document_for(root(), &[capability("planned", None)]).is_ok());
    }

    #[test]
    fn the_workspace_version_is_the_one_the_status_names() {
        let manifest = fs::read_to_string(root().join("Cargo.toml")).unwrap();
        assert!(manifest.contains(&format!("\nversion = \"{VERSION}\"\n")));
    }
}
