//! `codespec measure`: per artefact, the functions read and refused and the refusals by category.
//!
//! A hand-built module gives the exact text and JSON reports; the lending fixture's report is the
//! checked-in `adapters/groovy/testdata/lending.measure.txt`, and its numbers are the ones
//! `adapters/groovy/testdata/lending/EXPECTED.md` states.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use codespec_cli::measure::{json, measure, text};
use codespec_model::Module;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn codespec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(args)
        .current_dir(root())
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    assert!(
        o.status.success(),
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// Two files: `a/Cart.groovy` with a read method, a method holding a `foreign` inside an `if` and a
/// field refusal (domain metadata); `b/util.go` with a read function and a refusal whose reason
/// carries no category.
const TWO_FILES: &str = r#"{
  "format": "cir/1",
  "name": "shop",
  "reader": {"language": "groovy", "name": "hand", "version": "0"},
  "types": [],
  "funcs": [
    {
      "name": "total",
      "exported": true,
      "recv": {"name": "c", "t": {"k": "ptr", "elem": {"k": "named", "name": "Cart"}}},
      "params": [],
      "results": [],
      "body": [{"k": "return", "results": [], "pos": "a/Cart.groovy:4"}],
      "pos": "a/Cart.groovy:3"
    },
    {
      "name": "names",
      "exported": true,
      "recv": {"name": "c", "t": {"k": "ptr", "elem": {"k": "named", "name": "Cart"}}},
      "params": [],
      "results": [],
      "body": [
        {
          "k": "if",
          "cond": {"k": "lit", "value": "true", "t": {"k": "bool"}},
          "then": [
            {
              "k": "foreign",
              "lang": "groovy",
              "src": "items.collect { it.name }",
              "reason": "closure: a closure passed to collect",
              "pos": "a/Cart.groovy:8"
            }
          ],
          "pos": "a/Cart.groovy:7"
        }
      ],
      "pos": "a/Cart.groovy:6"
    },
    {
      "name": "run",
      "exported": false,
      "params": [],
      "results": [],
      "body": [],
      "pos": "b/util.go:3"
    }
  ],
  "layout": [],
  "refusals": [
    {"pos": "a/Cart.groovy:2", "what": "Cart.mapping", "reason": "gorm: mapping metadata mapping"},
    {"pos": "a/Cart.groovy:8", "what": "items.collect { it.name }", "reason": "closure: a closure passed to collect"},
    {"pos": "b/util.go:5", "what": "go func() {}", "reason": "goroutines are outside cir/1"}
  ]
}
"#;

const TWO_FILES_TEXT: &str = "\
file           read  refused  closure  dynamic  gorm  groovy-semantics  spring  type  uncategorised  unresolved  unsupported
a/Cart.groovy     1        1        1        0     1                 0       0     0              0           0            0
b/util.go         1        0        0        0     0                 0       0     0              1           0            0
total             2        1        1        0     1                 0       0     0              1           0            0
read 2 of 3 functions, 66.7%
";

const TWO_FILES_JSON: &str = r#"{
  "format": "codespec-measure/1",
  "module": "shop",
  "files": [
    {
      "file": "a/Cart.groovy",
      "read": [
        "Cart.total"
      ],
      "refused": [
        "Cart.names"
      ],
      "refusals": [
        {
          "category": "closure",
          "count": 1
        },
        {
          "category": "gorm",
          "count": 1
        }
      ]
    },
    {
      "file": "b/util.go",
      "read": [
        "run"
      ],
      "refused": [],
      "refusals": [
        {
          "category": "uncategorised",
          "count": 1
        }
      ]
    }
  ],
  "total": {
    "functions": 3,
    "read": 2,
    "refused": 1,
    "refusals": [
      {
        "category": "closure",
        "count": 1
      },
      {
        "category": "gorm",
        "count": 1
      },
      {
        "category": "uncategorised",
        "count": 1
      }
    ],
    "share_read": "66.7%"
  }
}
"#;

fn scratch(name: &str) -> PathBuf {
    let dir = root().join("target/measure-cli-test");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn a_hand_built_module_of_two_files_gives_the_exact_text_and_json_reports() {
    let m = Module::from_json(TWO_FILES.as_bytes()).unwrap();
    let report = measure(&m);
    assert_eq!(text(&report), TWO_FILES_TEXT);
    assert_eq!(json(&report), TWO_FILES_JSON);

    let path = scratch("two-files.cir.json");
    std::fs::write(&path, TWO_FILES).unwrap();
    let p = path.to_str().unwrap();
    assert_eq!(stdout(&codespec(&["measure", p])), TWO_FILES_TEXT);
    assert_eq!(
        stdout(&codespec(&["measure", "--format", "text", p])),
        TWO_FILES_TEXT
    );
    assert_eq!(
        stdout(&codespec(&["measure", "--format", "json", p])),
        TWO_FILES_JSON
    );
}

/// A JVM reader's `New<Class>` at its class's own line is a constructor the reader made, not a
/// declared function; a refusal of a method's declaration (a `what` ending in `(…)`, such as
/// `Cart.methodMissing(…)` or `Cart(…)`) is a refused function with no body; `Cart.hasMany` is not.
#[test]
fn a_synthesized_constructor_is_not_counted_and_a_refused_declaration_is_a_refused_function() {
    let doc = r#"{
  "format": "cir/1",
  "name": "shop",
  "reader": {"language": "groovy", "name": "hand", "version": "0"},
  "types": [
    {"k": "struct", "name": "Cart", "exported": true, "fields": [], "pos": "Cart.groovy:1"},
    {"k": "struct", "name": "Till", "exported": true, "fields": [], "pos": "Till.groovy:1"}
  ],
  "funcs": [
    {"name": "NewCart", "exported": true, "params": [], "results": [], "body": [], "pos": "Cart.groovy:1"},
    {"name": "NewTill", "exported": true, "params": [], "results": [], "body": [], "pos": "Till.groovy:4"}
  ],
  "layout": [],
  "refusals": [
    {"pos": "Cart.groovy:5", "what": "Cart.methodMissing(…)", "reason": "dynamic: methodMissing declared"},
    {"pos": "Cart.groovy:9", "what": "Cart(…)", "reason": "unsupported: a second constructor"},
    {"pos": "Cart.groovy:2", "what": "Cart.hasMany", "reason": "gorm: mapping metadata hasMany"}
  ]
}
"#;
    let report = measure(&Module::from_json(doc.as_bytes()).unwrap());
    let rows: Vec<(&str, Vec<&str>, Vec<&str>)> = report
        .files
        .iter()
        .map(|f| {
            (
                f.file.as_str(),
                f.read.iter().map(String::as_str).collect(),
                f.refused.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                "Cart.groovy",
                vec![],
                vec!["Cart(…)", "Cart.methodMissing(…)"]
            ),
            ("Till.groovy", vec!["NewTill"], vec![]),
        ]
    );
}

#[test]
fn measure_help_names_the_module_argument_the_format_option_and_what_it_counts() {
    let help = stdout(&codespec(&["measure", "--help"]));
    for want in [
        "<MODULE>",
        "a cir/1 document",
        "--format <FORMAT>",
        "codespec-measure/1",
        "counts read and refused functions per artefact by refusal category",
    ] {
        assert!(help.contains(want), "`{want}` is not in:\n{help}");
    }
    let page = std::fs::read_to_string(root().join("website/docs/reference/cli.md")).unwrap();
    for want in [
        "codespec measure",
        "counts read and refused functions per artefact by refusal category",
        "a cir/1 document",
        "--format",
    ] {
        assert!(page.contains(want), "`{want}` is not in cli.md");
    }
}

const LENDING: &str = "adapters/groovy/testdata/lending.read.cir.json";
const LENDING_MEASURE: &str = "adapters/groovy/testdata/lending.measure.txt";

#[test]
fn measuring_lending_twice_gives_the_checked_in_report() {
    let first = stdout(&codespec(&["measure", LENDING]));
    let second = stdout(&codespec(&["measure", LENDING]));
    assert_eq!(first, second, "two runs differ");
    let checked_in = std::fs::read_to_string(root().join(LENDING_MEASURE)).unwrap();
    assert_eq!(
        first, checked_in,
        "{LENDING_MEASURE} is stale; write it again with `codespec measure {LENDING}`"
    );
}

/// `EXPECTED.md`'s totals table: file (or `total`) to read, refused, then one count per category
/// in the table's own column order.
fn expected_rows() -> (Vec<String>, BTreeMap<String, Vec<usize>>) {
    let text = std::fs::read_to_string(root().join("adapters/groovy/testdata/lending/EXPECTED.md"))
        .unwrap();
    let mut header = Vec::new();
    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let Some(cells) = line.strip_prefix('|').and_then(|s| s.strip_suffix('|')) else {
            continue;
        };
        let cells: Vec<&str> = cells.split('|').map(str::trim).collect();
        if cells[0] == "file" {
            header = cells[1..].iter().map(|c| c.to_string()).collect();
        } else if !cells[0].starts_with("---") {
            let nums = cells[1..].iter().map(|c| c.parse().unwrap()).collect();
            rows.insert(cells[0].trim_matches('`').to_string(), nums);
        }
    }
    assert!(!rows.is_empty(), "EXPECTED.md has no totals table");
    (header, rows)
}

/// The checked-in report's table: file (or `total`) to its counts by column name.
fn measure_rows() -> BTreeMap<String, BTreeMap<String, usize>> {
    let text = std::fs::read_to_string(root().join(LENDING_MEASURE)).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(header[0], "file");
    let mut rows = BTreeMap::new();
    for line in lines.filter(|l| !l.starts_with("read ")) {
        let cells: Vec<&str> = line.split_whitespace().collect();
        let counts = header[1..]
            .iter()
            .zip(&cells[1..])
            .map(|(h, c)| (h.to_string(), c.parse().unwrap()))
            .collect();
        rows.insert(cells[0].to_string(), counts);
    }
    rows
}

#[test]
fn the_checked_in_lending_report_has_the_numbers_expected_md_states() {
    let (header, expected) = expected_rows();
    let measured = measure_rows();
    assert_eq!(
        measured.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "the report's rows are not EXPECTED.md's rows"
    );
    for (file, want) in &expected {
        let got = &measured[file];
        for (column, n) in header.iter().zip(want) {
            assert_eq!(
                got.get(column),
                Some(n),
                "{file}, {column}: EXPECTED.md says {n}"
            );
        }
        let elsewhere: usize = got
            .iter()
            .filter(|(c, _)| !header.contains(c))
            .map(|(_, n)| n)
            .sum();
        assert_eq!(elsewhere, 0, "{file}: counts in a column EXPECTED.md lacks");
    }
    let share = std::fs::read_to_string(root().join(LENDING_MEASURE)).unwrap();
    assert!(
        share.ends_with("read 14 of 38 functions, 36.8%\n"),
        "{share}"
    );
}

/// The function names per artefact, too, are the ones `EXPECTED.md` lists.
#[test]
fn the_lending_functions_read_and_refused_are_the_ones_expected_md_names() {
    let text = std::fs::read_to_string(root().join("adapters/groovy/testdata/lending/EXPECTED.md"))
        .unwrap();
    let names = |cell: &str| -> Vec<String> {
        if cell == "none" {
            return Vec::new();
        }
        let mut v: Vec<String> = cell
            .split(", ")
            .map(|s| s.trim_matches('`').to_string())
            .collect();
        v.sort();
        v
    };
    let mut expected: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    let mut current = None;
    for line in text.lines() {
        if let Some(p) = line.strip_prefix("### `").and_then(|s| s.strip_suffix('`')) {
            current = Some(p.to_string());
            expected.entry(p.to_string()).or_default();
        } else if let Some(file) = &current {
            let e = expected.get_mut(file).unwrap();
            if let Some(v) = line.strip_prefix("- read: ") {
                e.0 = names(v);
            } else if let Some(v) = line.strip_prefix("- refused: ") {
                e.1 = names(v);
            }
        }
    }
    let bytes = std::fs::read(root().join(LENDING)).unwrap();
    let report = measure(&Module::from_json(&bytes).unwrap());
    let method = |key: &String| {
        let name = key.rsplit('.').next().unwrap();
        name.strip_suffix("(…)").unwrap_or(name).to_string()
    };
    let got: BTreeMap<String, (Vec<String>, Vec<String>)> = report
        .files
        .iter()
        .map(|f| {
            let mut read: Vec<String> = f.read.iter().map(method).collect();
            let mut refused: Vec<String> = f.refused.iter().map(method).collect();
            read.sort();
            refused.sort();
            (f.file.clone(), (read, refused))
        })
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn the_java_ledger_reads_every_function_and_refuses_none() {
    let out = stdout(&codespec(&[
        "measure",
        "examples/ledger/ledger.read.cir.json",
    ]));
    let last = out.lines().last().unwrap();
    let words: Vec<&str> = last.split_whitespace().collect();
    assert_eq!(&words[..1], ["read"], "{out}");
    assert_eq!(words[1], words[3], "not every function is read: {last}");
    assert!(last.ends_with(" functions, 100.0%"), "{last}");
    let total = out.lines().find(|l| l.starts_with("total ")).unwrap();
    let counts: Vec<usize> = total
        .split_whitespace()
        .skip(1)
        .map(|c| c.parse().unwrap())
        .collect();
    assert!(counts[0] > 0, "{total}");
    assert!(counts[1..].iter().all(|&n| n == 0), "{total}");
}
