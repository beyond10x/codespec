//! Adversary case for story conformance-kit-release.
//!
//! The adapter guide tells an author outside the repository to take the conformance kit from the
//! release and run `codespec conform`, which needs `codespec-run-<lang>` in `--adapters` (and, for a
//! reader, `codespec-write-go` and `codespec-run-go`). The release must then ship a runner beside
//! the kit, or the kit cannot be run with what the release contains.

use std::path::PathBuf;

fn repo_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn release_binary_archive_ships_the_runner_the_guide_says_conform_needs() {
    let guide = repo_file("website/docs/guides/add-an-adapter.md");
    assert!(guide.contains("take\n`codespec-conformance-<version>.tar.gz` from the release"));
    assert!(guide.contains("`codespec-run-<lang>`"));
    assert!(guide.contains("A reader also needs `codespec-write-go` and `codespec-run-go`"));

    let workflow = repo_file(".github/workflows/release-build.yml");
    let package = workflow
        .lines()
        .find(|l| l.contains("tar -czf \"dist/$package.tar.gz\""))
        .expect("the Package step's tar line");
    assert!(
        package.contains("codespec-run-go"),
        "the release archive does not ship codespec-run-go, so `codespec conform` on the released \
         kit fails with `codespec-run-go does not exist` for every Go writer and every reader: {package}"
    );
}
