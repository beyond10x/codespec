//! The Grails 2.5.6 lending fixture under `adapters/groovy/testdata/lending`.
//!
//! Every function declared in a domain class or a service carries a trailing marker,
//! `// expect: read`, `// expect: refused` or `// expect: refused <category>`, and `EXPECTED.md`
//! states per artefact what those markers add up to. The grammar is in `EXPECTED.md`. The second
//! test holds the construct inventory the Grails reader stories rely on.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CATEGORIES: [&str; 8] = [
    "dynamic",
    "closure",
    "groovy-semantics",
    "type",
    "unresolved",
    "gorm",
    "spring",
    "unsupported",
];

const READ_DIRS: [&str; 2] = ["grails-app/domain", "grails-app/services"];

const KEYWORDS: [&str; 15] = [
    "if",
    "else",
    "for",
    "while",
    "switch",
    "catch",
    "return",
    "new",
    "try",
    "synchronized",
    "throw",
    "assert",
    "case",
    "do",
    "finally",
];

const MODIFIERS: [&str; 7] = [
    "public",
    "private",
    "protected",
    "static",
    "final",
    "abstract",
    "synchronized",
];

fn lending() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/groovy/testdata/lending")
}

fn collect_groovy(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("read directory {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_groovy(&path, out);
        } else if path.extension().is_some_and(|x| x == "groovy") {
            out.push(path);
        }
    }
}

/// Whether a `.groovy` file, by its path relative to the application, is an artefact the Grails
/// reader reads: any file under `grails-app/domain`, a `*Service.groovy` under
/// `grails-app/services`.
fn is_read_artefact(rel: &str) -> bool {
    rel.starts_with("grails-app/domain/")
        || (rel.starts_with("grails-app/services/") && rel.ends_with("Service.groovy"))
}

/// Every artefact the reader reads, as (path relative to the fixture, text), in byte order of the
/// path.
fn read_artefacts() -> Vec<(String, String)> {
    let root = lending();
    let mut paths = Vec::new();
    for dir in READ_DIRS {
        collect_groovy(&root.join(dir), &mut paths);
    }
    let mut files: Vec<(String, String)> = paths
        .iter()
        .filter_map(|p| {
            let rel = p
                .strip_prefix(&root)
                .expect("under the fixture")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if !is_read_artefact(&rel) {
                return None;
            }
            let text =
                fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            Some((rel, text))
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no .groovy file under {}",
        root.display()
    );
    files
}

#[derive(Debug, Clone, PartialEq)]
enum Marker {
    Read,
    RefusedFunction,
    Refused(String),
}

const MARK: &str = "// expect:";

fn marker(line: &str) -> Result<Option<Marker>, String> {
    let Some(at) = line.find(MARK) else {
        return Ok(None);
    };
    let rest = line[at + MARK.len()..].trim();
    let words: Vec<&str> = rest.split_whitespace().collect();
    match words.as_slice() {
        ["read"] => Ok(Some(Marker::Read)),
        ["refused"] => Ok(Some(Marker::RefusedFunction)),
        ["refused", category] if CATEGORIES.contains(category) => {
            Ok(Some(Marker::Refused((*category).to_string())))
        }
        ["refused", category] => Err(format!(
            "`{category}` is not a refusal category; the closed list is {}",
            CATEGORIES.join(", ")
        )),
        _ => Err(format!("`{MARK} {rest}` is not a marker")),
    }
}

/// The code of a line: everything before its first `//`.
fn code_of(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

fn is_identifier(word: &str) -> bool {
    let mut chars = word.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

const PRIMITIVES: [&str; 10] = [
    "def", "void", "boolean", "byte", "char", "short", "int", "long", "float", "double",
];

/// The code after the annotations that open it, `@Name` or `@Name(arguments)`.
fn after_annotations(mut code: &str) -> &str {
    loop {
        code = code.trim_start();
        let Some(rest) = code.strip_prefix('@') else {
            return code;
        };
        let name_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.'))
            .unwrap_or(rest.len());
        let rest = &rest[name_end..];
        let Some(args) = rest.strip_prefix('(') else {
            code = rest;
            continue;
        };
        let mut depth = 1;
        let mut end = None;
        for (i, c) in args.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        match end {
            Some(i) => code = &args[i + 1..],
            None => return "",
        }
    }
}

/// Whether a word is a Groovy return type: `def`, `void`, a primitive, or a class name, simple or
/// qualified, its last segment capitalised, with type arguments or array brackets.
fn is_return_type(word: &str) -> bool {
    if PRIMITIVES.contains(&word.trim_end_matches("[]")) {
        return true;
    }
    let base = word.split(['<', '[']).next().unwrap_or("");
    let last = base.rsplit('.').next().unwrap_or("");
    base.split('.').all(is_identifier) && last.starts_with(|c: char| c.is_ascii_uppercase())
}

/// The name a method declaration line declares, `def`, typed or untyped with a modifier only:
/// annotations, modifiers, a return type and a name before the first `(`, no `=` there, not a
/// statement keyword. A command expression (`println describe(tool)`) has no return type and is
/// not a declaration.
fn declared_name(code: &str) -> Option<String> {
    let code = after_annotations(code.trim_start().trim_start_matches('}'));
    let head = &code[..code.find('(')?];
    if head.contains('=') || head.contains('"') || head.contains('\'') {
        return None;
    }
    let words: Vec<&str> = head.split_whitespace().collect();
    if words.first().is_none_or(|w| KEYWORDS.contains(w)) {
        return None;
    }
    let modified = words.iter().any(|w| MODIFIERS.contains(w));
    let words: Vec<&str> = words
        .into_iter()
        .filter(|w| !MODIFIERS.contains(w))
        .collect();
    let (name, before) = words.split_last()?;
    if !is_identifier(name) {
        return None;
    }
    match before.first() {
        None if modified => Some((*name).to_string()),
        Some(t) if is_return_type(t) => Some((*name).to_string()),
        _ => None,
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

#[derive(Debug, Default, PartialEq)]
struct Counts {
    read: Vec<String>,
    refused: Vec<String>,
    categories: BTreeMap<String, usize>,
}

struct Open {
    name: String,
    indent: usize,
    line: usize,
    verdict: Marker,
    body_refused: bool,
}

fn close(path: &str, f: Open, counts: &mut Counts, problems: &mut Vec<String>) {
    let at = format!("{path}:{}", f.line);
    match (&f.verdict, f.body_refused) {
        (Marker::Read, true) => problems.push(format!(
            "{at}: `{}` is marked read but a line in its body is marked refused",
            f.name
        )),
        (Marker::RefusedFunction, false) => problems.push(format!(
            "{at}: `{}` is marked refused with no category and no line in its body is marked refused",
            f.name
        )),
        _ => {}
    }
    if f.verdict == Marker::Read && !f.body_refused {
        counts.read.push(f.name);
    } else {
        counts.refused.push(f.name);
    }
}

/// What the markers of one artefact add up to; every malformed or missing marker is a problem.
fn count(path: &str, text: &str, problems: &mut Vec<String>) -> Counts {
    let mut counts = Counts::default();
    let mut open: Option<Open> = None;
    let mut whole_refused = false;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let mark = match marker(line) {
            Ok(m) => m,
            Err(e) => {
                problems.push(format!("{path}:{n}: {e}"));
                None
            }
        };
        let code = code_of(line);
        if let Some(f) = open.take() {
            if code.trim_start().starts_with('}') && indent(line) == f.indent {
                close(path, f, &mut counts, problems);
            } else {
                open = Some(f);
            }
        }
        if code.trim_start().starts_with("import ")
            && mark == Some(Marker::Refused("unresolved".into()))
        {
            whole_refused = true;
        }
        if let Some(name) = declared_name(code) {
            if let Some(f) = &open {
                problems.push(format!(
                    "{path}:{n}: `{name}` is declared inside `{}` (line {})",
                    f.name, f.line
                ));
            }
            if !code.trim_end().ends_with('{') {
                problems.push(format!(
                    "{path}:{n}: the declaration of `{name}` must open its body on its own line"
                ));
            }
            match &mark {
                None => problems.push(format!(
                    "{path}:{n}: the declaration of `{name}` has no `{MARK}` marker"
                )),
                Some(verdict) => {
                    open = Some(Open {
                        name,
                        indent: indent(line),
                        line: n,
                        verdict: verdict.clone(),
                        body_refused: false,
                    });
                }
            }
        } else if mark == Some(Marker::RefusedFunction) {
            problems.push(format!(
                "{path}:{n}: `{MARK} refused` without a category belongs on a declaration line"
            ));
        }
        if let Some(Marker::Refused(category)) = &mark {
            *counts.categories.entry(category.clone()).or_default() += 1;
            if let Some(f) = open.as_mut().filter(|f| f.line != n) {
                f.body_refused = true;
            }
        }
    }
    if let Some(f) = open {
        problems.push(format!(
            "{path}:{}: the body of `{}` is not closed by a `}}` at its indentation",
            f.line, f.name
        ));
    }
    if whole_refused {
        return Counts {
            read: Vec::new(),
            refused: Vec::new(),
            categories: BTreeMap::from([("unresolved".to_string(), 1)]),
        };
    }
    counts.read.sort();
    counts.refused.sort();
    counts
}

fn names(cell: &str) -> Vec<String> {
    let cell = cell.trim();
    if cell == "none" {
        return Vec::new();
    }
    let mut v: Vec<String> = cell
        .split(',')
        .map(|s| s.trim().trim_matches('`').to_string())
        .collect();
    v.sort();
    v
}

fn categories(cell: &str) -> Result<BTreeMap<String, usize>, String> {
    let cell = cell.trim();
    let mut out = BTreeMap::new();
    if cell == "none" {
        return Ok(out);
    }
    for part in cell.split(',') {
        let (category, n) = part
            .trim()
            .rsplit_once(' ')
            .ok_or_else(|| format!("`{part}` is not `<category> <count>`"))?;
        if !CATEGORIES.contains(&category) {
            return Err(format!("`{category}` is not a refusal category"));
        }
        let n: usize = n
            .parse()
            .map_err(|_| format!("`{n}` is not a count in `{part}`"))?;
        if out.insert(category.to_string(), n).is_some() {
            return Err(format!("`{category}` is listed twice in `{cell}`"));
        }
    }
    Ok(out)
}

/// A table row: read, refused, then one count per category in the order of `CATEGORIES`.
fn row(counts: &Counts) -> Vec<usize> {
    let mut v = vec![counts.read.len(), counts.refused.len()];
    v.extend(
        CATEGORIES
            .iter()
            .map(|c| counts.categories.get(*c).copied().unwrap_or(0)),
    );
    v
}

struct Expected {
    sections: BTreeMap<String, Counts>,
    rows: BTreeMap<String, Vec<usize>>,
}

fn parse_expected(text: &str, problems: &mut Vec<String>) -> Expected {
    let mut sections: BTreeMap<String, Counts> = BTreeMap::new();
    let mut rows = BTreeMap::new();
    let mut current: Option<String> = None;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if let Some(path) = line.strip_prefix("### `").and_then(|s| s.strip_suffix('`')) {
            sections.insert(path.to_string(), Counts::default());
            current = Some(path.to_string());
        } else if let Some(cells) = line.strip_prefix('|').and_then(|s| s.strip_suffix('|')) {
            let cells: Vec<&str> = cells.split('|').map(str::trim).collect();
            let key = cells[0].trim_matches('`');
            if key == "file" || key.starts_with("---") {
                continue;
            }
            let nums: Result<Vec<usize>, _> = cells[1..].iter().map(|c| c.parse()).collect();
            match nums {
                Ok(v) if v.len() == 2 + CATEGORIES.len() => {
                    rows.insert(key.to_string(), v);
                }
                _ => problems.push(format!(
                    "EXPECTED.md:{n}: a row needs {} counts: {line}",
                    2 + CATEGORIES.len()
                )),
            }
        } else if let (Some(path), Some(item)) = (&current, line.strip_prefix("- ")) {
            let entry = sections.get_mut(path).expect("section exists");
            if let Some(v) = item.strip_prefix("read: ") {
                entry.read = names(v);
            } else if let Some(v) = item.strip_prefix("refused: ") {
                entry.refused = names(v);
            } else if let Some(v) = item.strip_prefix("refusals: ") {
                match categories(v) {
                    Ok(c) => entry.categories = c,
                    Err(e) => problems.push(format!("EXPECTED.md:{n}: {e}")),
                }
            }
        }
    }
    Expected { sections, rows }
}

#[test]
fn every_lending_function_carries_a_marker_and_expected_md_states_their_totals() {
    let files = read_artefacts();
    let mut problems = Vec::new();
    let mut actual: BTreeMap<String, Counts> = BTreeMap::new();
    for (path, text) in &files {
        actual.insert(path.clone(), count(path, text, &mut problems));
    }
    let expected_path = lending().join("EXPECTED.md");
    let expected_text = fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", expected_path.display()));
    let expected = parse_expected(&expected_text, &mut problems);

    let mut total = vec![0; 2 + CATEGORIES.len()];
    for (path, counts) in &actual {
        let r = row(counts);
        for (t, x) in total.iter_mut().zip(&r) {
            *t += x;
        }
        match expected.sections.get(path) {
            None => problems.push(format!("EXPECTED.md has no section `### `{path}``")),
            Some(e) if e != counts => problems.push(format!(
                "EXPECTED.md section {path} says {e:?}, the markers say {counts:?}"
            )),
            Some(_) => {}
        }
        match expected.rows.get(path) {
            None => problems.push(format!("EXPECTED.md has no table row for `{path}`")),
            Some(e) if *e != r => problems.push(format!(
                "EXPECTED.md row {path} says {e:?}, the markers say {r:?}"
            )),
            Some(_) => {}
        }
    }
    for path in expected.sections.keys() {
        if !actual.contains_key(path) {
            problems.push(format!(
                "EXPECTED.md has a section for {path}, which is not read"
            ));
        }
    }
    for path in expected.rows.keys() {
        if path != "total" && !actual.contains_key(path) {
            problems.push(format!(
                "EXPECTED.md has a row for {path}, which is not read"
            ));
        }
    }
    match expected.rows.get("total") {
        None => problems.push("EXPECTED.md has no `total` row".into()),
        Some(e) if *e != total => problems.push(format!(
            "EXPECTED.md total says {e:?}, the markers say {total:?}"
        )),
        Some(_) => {}
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Marked lines: (file, line number, code, the whole line).
fn marked_lines(files: &[(String, String)]) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for (path, text) in files {
        for (i, line) in text.lines().enumerate() {
            if line.contains(MARK) {
                out.push((path.clone(), i + 1, code_of(line).to_string()));
            }
        }
    }
    out
}

fn while_with_break(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    lines.iter().enumerate().any(|(i, line)| {
        if !line.contains(MARK) || !code_of(line).trim_start().starts_with("while (") {
            return false;
        }
        let w = indent(line);
        lines[i + 1..]
            .iter()
            .take_while(|l| !(indent(l) <= w && l.trim_start().starts_with('}')))
            .any(|l| code_of(l).trim() == "break")
    })
}

/// The 0-based line ranges of method bodies, declaration line to closing brace.
fn function_spans(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if declared_name(code_of(lines[i])).is_some() {
            let w = indent(lines[i]);
            let end = (i + 1..lines.len())
                .find(|&j| indent(lines[j]) == w && code_of(lines[j]).trim_start().starts_with('}'))
                .unwrap_or(lines.len() - 1);
            spans.push((i, end));
            i = end;
        }
        i += 1;
    }
    spans
}

/// A `for` marked read over the keys of a `TreeMap` field: a field outside every method, typed
/// `TreeMap<…>` and built with `new TreeMap<>()`, the one form the shared JVM rules read as
/// `sorted_keys`.
fn for_over_treemap(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let spans = function_spans(&lines);
    let in_function = |i: usize| spans.iter().any(|&(a, b)| a <= i && i <= b);
    lines.iter().any(|line| {
        let code = code_of(line).trim_start();
        if marker(line).ok().flatten() != Some(Marker::Read) || !code.starts_with("for (") {
            return false;
        }
        let Some(close) = code.rfind(')') else {
            return false;
        };
        let header = &code["for (".len()..close];
        let iterable = header
            .rsplit_once(" : ")
            .or_else(|| header.rsplit_once(" in "))
            .map_or("", |(_, it)| it.trim());
        let var: String = iterable
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        !var.is_empty()
            && lines.iter().enumerate().any(|(i, l)| {
                let c = code_of(l).trim();
                !in_function(i)
                    && c.starts_with("TreeMap<")
                    && c.ends_with(&format!(" {var} = new TreeMap<>()"))
            })
    })
}

#[test]
fn the_lending_fixture_holds_every_construct_of_the_inventory() {
    let files = read_artefacts();
    let marked = marked_lines(&files);
    let needles = [
        ".save()",
        ".delete()",
        "Tool.get(",
        "Tool.findByName(",
        "Loan.findAllByMember(",
        "Tool.withCriteria {",
        "addToLoans(",
        "static constraints",
        "static mapping",
        "static hasMany",
        "static belongsTo",
        "def catalogService",
        "FeeService feeService",
        "methodMissing",
        "metaClass",
        ".each {",
        ".collect {",
        "?.",
        "?:",
        "switch (",
    ];
    let mut missing: Vec<String> = needles
        .iter()
        .filter(|n| !marked.iter().any(|(_, _, code)| code.contains(*n)))
        .map(|n| format!("`{n}` on a line with an `{MARK}` marker"))
        .collect();
    if !files.iter().any(|(_, t)| while_with_break(t)) {
        missing.push(format!(
            "a `while` with a `break` in its body, the `while` line marked `{MARK}`"
        ));
    }
    if !files.iter().any(|(_, t)| for_over_treemap(t)) {
        missing.push(format!(
            "a `for` marked `{MARK} read` over a `TreeMap` field built with `new TreeMap<>()`"
        ));
    }
    let file = |suffix: &str| files.iter().find(|(p, _)| p.ends_with(suffix));
    match file("/TransactionalAuditService.groovy") {
        Some((_, t))
            if t.lines()
                .any(|l| code_of(l).trim() == "import grails.transaction.Transactional") => {}
        _ => missing.push(
            "`import grails.transaction.Transactional` in TransactionalAuditService.groovy".into(),
        ),
    }
    match file("/LoanService.groovy") {
        Some((_, t))
            if t.lines().any(|l| {
                let c = code_of(l);
                !c.trim_start().starts_with("import ") && c.contains("DueDates.")
            }) => {}
        _ => missing.push("a call of `DueDates` in LoanService.groovy".into()),
    }
    let utils = lending().join("grails-app/utils/lending/DueDates.groovy");
    if !utils.is_file() {
        missing.push(format!("{}", utils.display()));
    }
    assert!(
        missing.is_empty(),
        "the lending fixture lacks:\n{}",
        missing.join("\n")
    );
}

#[test]
fn a_declaration_line_is_told_from_a_statement() {
    assert_eq!(
        declared_name("    long feeFor(int days) {"),
        Some("feeFor".into())
    );
    assert_eq!(
        declared_name("    def methodMissing(String name, args) {"),
        Some("methodMissing".into())
    );
    assert_eq!(
        declared_name("    static int dueDay(int day) {"),
        Some("dueDay".into())
    );
    assert_eq!(
        declared_name("    Map<String, Integer> perShelf(List<Tool> tools) {"),
        Some("perShelf".into())
    );
    for statement in [
        "        if (loans) {",
        "        return Tool.get(id)",
        "        total = total + fee(x)",
        "        eq('shelf', name)",
        "        tools.each { Tool t ->",
        "        long fee = cents(days)",
        "        } else if (x) {",
        "        loan.save()",
    ] {
        assert_eq!(declared_name(statement), None, "{statement}");
    }
}

#[test]
fn only_service_files_under_services_count_as_read_artefacts() {
    assert!(is_read_artefact(
        "grails-app/services/lending/LoanService.groovy"
    ));
    assert!(is_read_artefact("grails-app/domain/lending/Tool.groovy"));
    for not_read in [
        "grails-app/services/lending/FeeRules.groovy",
        "grails-app/utils/lending/DueDates.groovy",
        "grails-app/controllers/lending/ToolController.groovy",
        "src/groovy/lending/ShelfCodes.groovy",
    ] {
        assert!(!is_read_artefact(not_read), "{not_read}");
    }
}

/// Adversary cases for `story:grails-lending-fixture`; each names the property it breaks.
mod adversary {
    use super::*;

    /// Property: the marker check "fails when a `def` or typed method declaration line lacks an
    /// `// expect:` marker". Each text below declares one Groovy method with no marker; the check
    /// must report it. The Grails idiom `@Transactional(readOnly = true)` on the declaration line,
    /// an untyped method with a modifier only, and a qualified return type all pass silently.
    #[test]
    fn an_unmarked_method_declaration_is_reported_in_every_groovy_form() {
        let forms = [
            "    @Transactional(readOnly = true) def list() {",
            "    private helper(int x) {",
            "    static twice(int x) {",
            "    java.util.List<Tool> all() {",
        ];
        let silent: Vec<&str> = forms
            .iter()
            .copied()
            .filter(|decl| {
                let text = format!(
                    "package lending\n\nclass S {{\n\n{decl}\n        return null\n    }}\n}}\n"
                );
                let mut problems = Vec::new();
                let _ = count("grails-app/services/lending/S.groovy", &text, &mut problems);
                problems.is_empty()
            })
            .collect();
        assert!(
            silent.is_empty(),
            "unmarked method declarations the marker check does not see:\n{}",
            silent.join("\n")
        );
    }

    /// Property: a declaration line is told apart from a statement. A Groovy command expression
    /// (a call without parentheses whose argument is a call) is a statement, not a declaration.
    #[test]
    fn a_command_expression_statement_is_not_a_declaration() {
        for statement in [
            "        println describe(tool)",
            "        sleep delayFor(days)",
        ] {
            assert_eq!(declared_name(statement), None, "{statement}");
        }
    }

    /// Property: `EXPECTED.md` totals equal the markers. A refusals line naming one category twice
    /// is accepted with its last count, so `gorm 2, gorm 1` matches one `gorm` marker.
    #[test]
    fn expected_md_refuses_a_category_listed_twice() {
        assert!(
            categories("gorm 2, gorm 1").is_err(),
            "`gorm 2, gorm 1` was accepted as {:?}",
            categories("gorm 2, gorm 1")
        );
    }

    /// The marker of the nearest method declaration at or above a line, with its line number.
    fn enclosing(lines: &[&str], at: usize) -> Option<(usize, Option<Marker>)> {
        (0..=at).rev().find_map(|i| {
            declared_name(code_of(lines[i])).map(|_| (i + 1, marker(lines[i]).ok().flatten()))
        })
    }

    /// Property: a function marked `read` is one the reader reads. `story:groovy-reader-core`
    /// reads "as the Java reader does where the construct is shared", and
    /// `story:grails-artefacts` makes `grails-app/utils` resolve-only ("yield no struct, function
    /// or refusal"). Each line below sits, unmarked, in a function marked `read`, and holds a
    /// construct `adapters/java/src/codespec/read/Reader.java` refuses at the line named.
    #[test]
    fn no_function_marked_read_holds_a_construct_the_shared_jvm_rules_refuse() {
        let refused = [
            (
                "grails-app/domain/lending/Loan.groovy",
                "returnedDay < 0 ? today : returnedDay",
                "a conditional expression: Reader.java:2105, `?: is outside cir/1`",
            ),
            (
                "grails-app/domain/lending/Tool.groovy",
                "dailyFeeCents * days",
                "long * int widens implicitly: Reader.java:2514 and :2741, `implicit conversions are outside cir/1`",
            ),
            (
                "grails-app/services/lending/FeeService.groovy",
                "DAILY_LATE_FEE_CENTS * daysLate",
                "long * int widens implicitly: Reader.java:2514 and :2741",
            ),
            (
                "grails-app/services/lending/CatalogService.groovy",
                "for (String shelf : perShelf.keySet())",
                "a TreeMap parameter, not a local or field built with new TreeMap<>(): Reader.java:1539 and :1792",
            ),
            (
                "grails-app/services/lending/CatalogService.groovy",
                "perShelf.get(shelf)",
                "Map.get of an Integer value, absent key gives null: Reader.java:2303-2306",
            ),
            (
                "grails-app/services/lending/LoanService.groovy",
                "DueDates.dueDay(",
                "a call of a class that is not read (grails-app/utils is resolve-only): Reader.java:2273-2274",
            ),
        ];
        let files = read_artefacts();
        let mut wrong = Vec::new();
        for (path, needle, why) in refused {
            let (_, text) = files
                .iter()
                .find(|(p, _)| p == path)
                .unwrap_or_else(|| panic!("{path} is not in the fixture"));
            let lines: Vec<&str> = text.lines().collect();
            let Some(at) = lines.iter().position(|l| code_of(l).contains(needle)) else {
                panic!("{path} no longer holds `{needle}`");
            };
            let own = marker(lines[at]).ok().flatten();
            if matches!(own, Some(Marker::Refused(_))) {
                continue;
            }
            if let Some((decl, Some(Marker::Read))) = enclosing(&lines, at) {
                wrong.push(format!(
                    "{path}:{}: `{needle}` is in the function marked read at line {decl}; {why}",
                    at + 1
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "functions marked read that the shared rules refuse:\n{}",
            wrong.join("\n")
        );
    }
}
