//! The status data, `website/data/status.json` (`b10x-status/1`), from [`CAPABILITIES`].
//!
//! A shipped capability names the check that holds it, as `file::name`: a Rust test
//! (`fn name(`), a Go test (`func name(`), a `Taskfile.yml` task the gate runs (`  name:`), or a
//! conformance report under `website/data/conformance/` as `file::conforms`, which holds while the
//! report, read strictly, says `conforms: true`.
//! Generation fails when that file has no such function or task, so a claim cannot outlive the
//! check it rests on. A capability that is not shipped names none.

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::Result;

/// Where the status data lands, relative to the repository root.
pub const DATA: &str = "website/data/status.json";

/// The release the list describes.
pub const VERSION: &str = "0.4.0";

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
        detail: "codespec-read-rust lifts a crate's src/lib.rs into cir/1 with syn and its own name and type resolution for the cir/1 subset; the Rust writer's orders output reads back to the passed document but for three named exceptions.",
        status: "shipped",
        evidence: Some(
            "adapters/rust-read/tests/reader.rs::orders_rust_output_reads_back_to_the_passed_document_but_for_named_exceptions",
        ),
        href: "/docs/concepts/pipeline",
    },
    Capability {
        area: "Readers",
        label: "Rust outside the subset is kept, not guessed",
        detail: "A trait object, a generic function, a closure, unsafe or an integer literal nothing types is kept as a foreign Rust statement with a refusal at its line.",
        status: "shipped",
        evidence: Some(
            "adapters/rust-read/tests/reader.rs::a_closure_is_kept_as_foreign_with_a_refusal_at_its_line",
        ),
        href: "/docs/concepts/pipeline#refusals",
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
        label: "inventory",
        detail: "A hand-written Rust crate read, passed and written as Go and as Rust; the original, the Go output and the Rust output agree on 200 traces of 300 commands.",
        status: "shipped",
        evidence: Some("Taskfile.yml::examples-inventory"),
        href: "/docs/concepts/harness",
    },
    Capability {
        area: "Examples",
        label: "series",
        detail: "An algorithmic Go package: for loops with break and continue, switch, slices and integer arithmetic that wraps on overflow. Read, passed, written as Go and as Rust with no refusal; the original, the Go output and the Rust output agree on 200 traces of 300 commands.",
        status: "shipped",
        evidence: Some("Taskfile.yml::examples-series"),
        href: "/docs/concepts/harness",
    },
    Capability {
        area: "Interfaces",
        label: "codespec command line",
        detail: "validate, schema, features, read, pass, write, conform and harness.",
        status: "shipped",
        evidence: Some(
            "crates/cli/tests/harness.rs::the_three_implementations_agree_on_a_few_seeds",
        ),
        href: "/docs/reference/cli",
    },
    Capability {
        area: "Conformance",
        label: "Go reader conforms to the kit",
        detail: "codespec conform runs codespec-read-go over its corpus, one Go source per claimed feature: each runs to the case's expected output, reads twice to the same bytes, uses its feature outside a foreign statement, and written as Go runs to the expected output again.",
        status: "shipped",
        evidence: Some("website/data/conformance/reader-go.json::conforms"),
        href: "/docs/guides/add-an-adapter",
    },
    Capability {
        area: "Conformance",
        label: "Rust reader conforms to the kit",
        detail: "codespec conform runs codespec-read-rust over its corpus, one Rust crate per claimed feature, through the same four steps as the Go reader.",
        status: "shipped",
        evidence: Some("website/data/conformance/reader-rust.json::conforms"),
        href: "/docs/guides/add-an-adapter",
    },
    Capability {
        area: "Conformance",
        label: "Java reader conforms to the kit",
        detail: "codespec conform runs codespec-read-java over its corpus, one Java source per claimed feature, through the same four steps as the Go reader; codespec-run-java runs each source to the case's expected output first.",
        status: "shipped",
        evidence: Some("website/data/conformance/reader-java.json::conforms"),
        href: "/docs/guides/add-an-adapter",
    },
    Capability {
        area: "Conformance",
        label: "Groovy reader conforms to the kit",
        detail: "codespec conform runs codespec-read-groovy over its corpus, the kit's Java sources compiled as Groovy 2.4, through the same four steps as the Go reader; codespec-run-groovy runs each source to the case's expected output first.",
        status: "shipped",
        evidence: Some("website/data/conformance/reader-groovy.json::conforms"),
        href: "/docs/guides/add-an-adapter",
    },
    Capability {
        area: "Conformance",
        label: "Go writer conforms to the kit",
        detail: "codespec conform writes every case of the kit with codespec-write-go, twice to the same bytes, and runs it with codespec-run-go: every claimed feature gives the expected output, every unclaimed one is refused or right, and the boundary cases pass.",
        status: "shipped",
        evidence: Some("website/data/conformance/writer-go.json::conforms"),
        href: "/docs/guides/add-an-adapter",
    },
    Capability {
        area: "Conformance",
        label: "Rust writer conforms to the kit",
        detail: "codespec conform writes every case of the kit with codespec-write-rust and runs it with codespec-run-rust, as for the Go writer; a module without facts is refused naming codespec pass.",
        status: "shipped",
        evidence: Some("website/data/conformance/writer-rust.json::conforms"),
        href: "/docs/guides/add-an-adapter",
    },
];

/// Whether `source`, the text of `file`, defines the check `name`.
fn defines(file: &str, source: &str, name: &str) -> bool {
    if file.ends_with(".json") {
        // Checked by `report_conforms` before this is reached.
        name == "conforms"
    } else if file.ends_with("Taskfile.yml") {
        source.lines().any(|line| line == format!("  {name}:"))
    } else if file.ends_with(".go") {
        source.contains(&format!("func {name}("))
    } else {
        source.contains(&format!("fn {name}("))
    }
}

/// A report as evidence: `file::conforms`, a `codespec-conformance-report/1` read strictly that
/// says `conforms: true`.
fn report_conforms(file: &str, source: &str, name: &str) -> Result<()> {
    if name != "conforms" {
        return Err(format!(
            "{file}::{name}: a report is evidence as `{file}::conforms`"
        ));
    }
    let report = codespec_conformance::conform::read_report(source.as_bytes())
        .map_err(|e| format!("{file}: {e}"))?;
    if !report.conforms {
        return Err(format!(
            "{file}: {} {} does not conform",
            report.adapter.role, report.adapter.lang
        ));
    }
    Ok(())
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
                if file.ends_with(".json") {
                    report_conforms(file, &source, name).map_err(|e| format!("{label}: {e}"))?;
                }
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
        assert!(document.contains("\"version\": \"0.4.0\""));
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
    fn a_conformance_report_is_evidence_only_while_it_says_the_adapter_conforms() {
        let dir = root().join("target/codespec-docs-test/report-evidence");
        fs::create_dir_all(&dir).unwrap();
        let current = fs::read(root().join("website/data/conformance/writer-go.json")).unwrap();
        let mut report = codespec_conformance::conform::read_report(&current).unwrap();
        assert!(report.conforms);
        report.conforms = false;
        fs::write(
            dir.join("failing.json"),
            codespec_conformance::conform::report_json(&report),
        )
        .unwrap();
        let failing = capability(
            "shipped",
            Some("target/codespec-docs-test/report-evidence/failing.json::conforms"),
        );
        let error = document_for(root(), &[failing]).unwrap_err();
        assert!(error.contains("does not conform"), "{error}");
        let passing = capability(
            "shipped",
            Some("website/data/conformance/writer-go.json::conforms"),
        );
        assert!(document_for(root(), &[passing]).is_ok());
        let other_field = capability(
            "shipped",
            Some("website/data/conformance/writer-go.json::summary"),
        );
        assert!(document_for(root(), &[other_field]).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_workspace_version_is_the_one_the_status_names() {
        let manifest = fs::read_to_string(root().join("Cargo.toml")).unwrap();
        assert!(manifest.contains(&format!("\nversion = \"{VERSION}\"\n")));
    }
}
