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

/// Story release-musl-static: the release workflow builds the musl target, runs the static check
/// on its archive, builds every target with `--target`, and counts the musl archive in SHA256SUMS.
#[test]
fn the_release_builds_and_checks_a_musl_archive() {
    let workflow = repo_file(".github/workflows/release-build.yml");
    for needle in [
        "- target: x86_64-unknown-linux-musl",
        "sh scripts/release-static-check.sh archive \"dist/codespec-$GITHUB_REF_NAME-$TARGET.tar.gz\" \"$GITHUB_REF_NAME\"",
        "--target \"$TARGET\"",
        "test \"${#archives[@]}\" -eq 5",
    ] {
        assert!(
            workflow.contains(needle),
            "release-build.yml lost `{needle}`"
        );
    }
    let check = workflow
        .split("- name: Every binary of the musl archive is static")
        .nth(1)
        .expect("the static check step");
    assert!(
        check
            .lines()
            .take(3)
            .any(|l| l.contains("if: matrix.target == 'x86_64-unknown-linux-musl'")),
        "the static check step runs on every target"
    );
}

fn static_check_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Adversary case for story release-musl-static. `release-static-check.sh static` says it exits 0
/// only when every file is a static ELF executable. A file whose ELF header reads but whose program
/// and section headers are cut off (a truncated copy of this test's own, dynamically linked,
/// executable) is not one: `readelf -lW` and `-dW` fail, the `$(... | sed ...)` pipelines hide
/// that, `interp` and `needed` stay empty and the file passes.
#[test]
fn static_check_refuses_an_elf_file_readelf_cannot_read() {
    let root = static_check_root();
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("static-check-truncated");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let whole = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let cut = work.join("truncated");
    std::fs::write(&cut, &whole[..200]).unwrap();
    let out = std::process::Command::new("sh")
        .arg(root.join("scripts/release-static-check.sh"))
        .arg("static")
        .arg(&cut)
        .output()
        .unwrap();
    assert_ne!(
        out.status.code(),
        Some(0),
        "a truncated ELF file passed `static`: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `archive` unpacks into `${RUNNER_TEMP:-$PWD/target}/release-static-check.XXXXXX`. In a checkout
/// that has no `target/` yet and no RUNNER_TEMP (a fresh clone, the way a reader of docs/release.md
/// runs the script) `mktemp -d` fails before anything is checked. The script, copied into a tree
/// with no `target/`, must still reach its own verdict on an archive with no ELF file.
#[test]
fn archive_mode_runs_in_a_checkout_without_a_target_directory() {
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("static-check-no-target");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(work.join("scripts")).unwrap();
    std::fs::copy(
        static_check_root().join("scripts/release-static-check.sh"),
        work.join("scripts/release-static-check.sh"),
    )
    .unwrap();
    std::fs::write(work.join("README.md"), "x").unwrap();
    let tar = std::process::Command::new("tar")
        .arg("-czf")
        .arg(work.join("a.tar.gz"))
        .arg("-C")
        .arg(&work)
        .arg("README.md")
        .status()
        .unwrap();
    assert!(tar.success());
    let out = std::process::Command::new("sh")
        .arg(work.join("scripts/release-static-check.sh"))
        .arg("archive")
        .arg(work.join("a.tar.gz"))
        .arg("0.0.0")
        .env_remove("RUNNER_TEMP")
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("no ELF file at its top level"),
        "archive mode died before its own check: {err}"
    );
}
