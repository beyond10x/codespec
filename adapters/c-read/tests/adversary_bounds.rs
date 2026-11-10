//! Adversary case for the limits of `codespec-read-c`: the JSON limit is per `.c` file and every
//! file's tree is kept until the module is assembled, so a directory of small `.c` files that each
//! include one header holds that header's tree once per file. 16 files of 19 bytes and a 600 kB
//! header are far inside the 16 MiB source limit.

use std::path::Path;
use std::process::Command;

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

/// The address space the reader is given: about ten times what it needs for one of the files.
const ADDRESS_SPACE_KIB: u64 = 3 << 20;

#[test]
fn sixteen_files_including_one_header_stay_within_a_fixed_memory_budget() {
    let work =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/c-read-tests/adversary-bounds");
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join("multiply");
    std::fs::create_dir_all(&dir).expect("source dir");
    let header: String = (0..60_000).map(|i| format!("int v{i};\n")).collect();
    std::fs::write(dir.join("big.h"), header).expect("write big.h");
    for i in 0..16 {
        std::fs::write(dir.join(format!("f{i}.c")), "#include \"big.h\"\n").expect("write .c");
    }
    let o = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -c 0; ulimit -v {ADDRESS_SPACE_KIB}; exec \"$0\" \"$1\""
        ))
        .arg(READ_C)
        .arg(&dir)
        .output()
        .expect("run codespec-read-c");
    let stderr = String::from_utf8_lossy(&o.stderr);
    let refusal = o.status.code() == Some(1)
        && stderr.starts_with("codespec-read-c: ")
        && stderr.lines().count() == 1;
    assert!(
        o.status.success() || refusal,
        "exit 0 or a one-line refusal, got {}:\n{}",
        o.status,
        stderr.chars().take(600).collect::<String>()
    );
}
