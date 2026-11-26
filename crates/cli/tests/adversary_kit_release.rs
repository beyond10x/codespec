//! Adversary case for story conformance-kit-release.
//!
//! The adapter guide tells an author outside the repository to take the conformance kit from the
//! release and run `codespec conform`, which needs `codespec-run-<lang>` in `--adapters` (and, for a
//! reader, `codespec-write-go` and `codespec-run-go`). The release must then ship a runner beside
//! the kit, or the kit cannot be run with what the release contains.

use std::path::{Path, PathBuf};

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

/// The Package step's `tar -czf "dist/$package.tar.gz"` command, its continuation lines joined.
fn package_tar_command(workflow: &str) -> String {
    let mut lines = workflow
        .lines()
        .skip_while(|l| !l.contains("tar -czf \"dist/$package.tar.gz\""));
    let mut command = String::new();
    for line in lines.by_ref() {
        let line = line.trim();
        match line.strip_suffix('\\') {
            Some(part) => command.push_str(part),
            None => {
                command.push_str(line);
                break;
            }
        }
    }
    assert!(!command.is_empty(), "no Package tar line");
    command
}

/// The text of the workflow step named `name`, up to the next step.
fn step<'a>(workflow: &'a str, name: &str) -> &'a str {
    let start = workflow
        .find(&format!("- name: {name}"))
        .unwrap_or_else(|| panic!("no step {name}"));
    let rest = &workflow[start + 2..];
    let end = rest.find("\n      - ").unwrap_or(rest.len());
    &rest[..end]
}

/// Story release-ships-go-summaries: every platform archive holds `summaries/*.json` under
/// `share/codespec/summaries/`, and the release build checks the packed archive with
/// `scripts/release-summaries-check.sh`; the conformance archive the same under its directory.
#[test]
fn every_release_archive_ships_the_summaries_and_checks_it() {
    let workflow = repo_file(".github/workflows/release-build.yml");
    let package = step(&workflow, "Package the binaries");
    assert!(
        package.contains("mkdir -p \"dist/$package/share/codespec/summaries\"")
            && package.contains("cp summaries/*.json \"dist/$package/share/codespec/summaries/\""),
        "the Package step does not copy summaries/*.json under share/codespec/summaries/:\n{package}"
    );
    let tar = package_tar_command(&workflow);
    assert!(
        tar.split_whitespace().any(|w| w == "share"),
        "the platform archive does not hold share/: {tar}"
    );
    assert!(
        package.contains("sh scripts/release-summaries-check.sh \"dist/$package.tar.gz\""),
        "no check of the packed archive in the Package step:\n{package}"
    );
    let taskfile = repo_file("Taskfile.yml");
    let kit = &taskfile[taskfile
        .find("\n  conformance-kit:")
        .expect("conformance-kit")..];
    let kit = &kit[..kit[1..].find("\n  conformance-cases:").expect("next task")];
    assert!(
        kit.contains("cp summaries/*.json \"$scratch/stage/$name/share/codespec/summaries/\""),
        "conformance-kit does not stage summaries/*.json:\n{kit}"
    );
    assert!(
        kit.contains("sh scripts/release-summaries-check.sh \"$archive\" \"$name/\""),
        "conformance-kit does not check its archive:\n{kit}"
    );
}

/// The archive check, `scripts/release-summaries-check.sh`, passes an archive holding both files
/// of `summaries/` and fails, naming it, when either is missing, flat or under a directory.
#[test]
fn the_archive_check_fails_when_either_summary_file_is_missing() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let files = ["go-std.effects.json", "go-linux-amd64.barrier.json"];
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("release-summaries-check");
    let _ = std::fs::remove_dir_all(&work);
    let pack = |name: &str, dir: &str, keep: &[&str]| -> PathBuf {
        let stage = work.join(name);
        let share = stage.join(dir).join("share/codespec/summaries");
        std::fs::create_dir_all(&share).unwrap();
        std::fs::write(stage.join(dir).join("codespec"), "").unwrap();
        for f in keep {
            std::fs::copy(root.join("summaries").join(f), share.join(f)).unwrap();
        }
        let archive = work.join(format!("{name}.tar.gz"));
        let top = if dir.is_empty() {
            vec!["codespec", "share"]
        } else {
            vec![dir.trim_end_matches('/')]
        };
        let tar = std::process::Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&stage)
            .args(&top)
            .status()
            .unwrap();
        assert!(tar.success());
        archive
    };
    let check = |archive: &Path, dir: &str| {
        std::process::Command::new("sh")
            .arg(root.join("scripts/release-summaries-check.sh"))
            .arg(archive)
            .arg(dir)
            .output()
            .unwrap()
    };
    for dir in ["", "codespec-conformance-0.0.0/"] {
        let whole = pack(&format!("whole-{}", dir.len()), dir, &files);
        let ok = check(&whole, dir);
        assert_eq!(
            ok.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&ok.stderr)
        );
        for (i, gone) in files.iter().enumerate() {
            let keep: Vec<&str> = files.iter().copied().filter(|f| f != gone).collect();
            let short = pack(&format!("without-{i}-{}", dir.len()), dir, &keep);
            let failed = check(&short, dir);
            let err = String::from_utf8_lossy(&failed.stderr);
            assert_eq!(failed.status.code(), Some(1), "{gone} missing: {err}");
            assert!(
                err.contains(&format!("missing {dir}share/codespec/summaries/{gone}")),
                "{err}"
            );
        }
    }
    let flat = pack("flat-checked-as-kit", "", &files);
    assert_eq!(
        check(&flat, "codespec-conformance-0.0.0/").status.code(),
        Some(1),
        "a flat archive does not hold the files under the kit's directory"
    );
}
