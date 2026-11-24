//! What follows the span edits, pure: `gofmt`'s output turned into text edits merged with the
//! span edits ([`merge_whitespace`], `docs/code-edit-language.md` § 6.1), and the build commands
//! of `--build` (§ 7.4) with the part of their output a refusal prints. [`crate::run`] starts the
//! processes.
//!
//! `gofmt` runs on a Go file that was `gofmt`-clean when read. The span-edited text and `gofmt`'s
//! output must have the same bytes outside whitespace; walking both in step, each whitespace run
//! that differs becomes one text edit on the original file, and a run touching a span edit joins
//! it. So the diff stays made from edits and the bytes written are the edits applied.

use std::time::Duration;

use crate::change::{TextEdit, apply};

/// The longest one build command may run.
pub const BUILD_TIMEOUT: Duration = Duration::from_secs(600);

/// The lines of a failed build's output a refusal prints.
pub const BUILD_LINES: usize = 20;

/// The commands `--build` runs, in order, in the copy of each root of `lang`: Go `go build ./...`
/// and `go vet ./...` (test files included), Rust `cargo check --all-targets`. Empty for a
/// language without one.
pub fn build_commands(lang: &str) -> &'static [&'static [&'static str]] {
    match lang {
        "go" => &[&["go", "build", "./..."], &["go", "vet", "./..."]],
        "rust" => &[&[
            "cargo",
            "check",
            "--all-targets",
            "--quiet",
            "--message-format",
            "short",
        ]],
        _ => &[],
    }
}

/// The first `n` lines of `text`, each with its line break.
pub fn first_lines(text: &str, n: usize) -> String {
    let mut out: String = text.split_inclusive('\n').take(n).collect();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// ASCII whitespace as `gofmt` writes and removes it.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}

/// A whitespace run of the edited text that `gofmt` wrote otherwise: `t` in the edited text, `f`
/// in the formatted one.
#[derive(Debug, Clone, Copy)]
struct Run {
    t: (usize, usize),
    f: (usize, usize),
}

/// The whitespace runs of `edited` that differ in `formatted`, or where the two first differ
/// outside whitespace.
fn runs(edited: &[u8], formatted: &[u8]) -> Result<Vec<Run>, String> {
    let space = |s: &[u8], mut i: usize| {
        while i < s.len() && is_space(s[i]) {
            i += 1;
        }
        i
    };
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    loop {
        let (i2, j2) = (space(edited, i), space(formatted, j));
        // A run removed or added between two identifier or number characters joins or splits
        // tokens: not a whitespace change.
        let word = |b: Option<&u8>| b.is_some_and(|c| crate::source::word_byte(*c));
        if (i2 == i) != (j2 == j)
            && i > 0
            && j > 0
            && word(edited.get(i - 1))
            && word(formatted.get(j - 1))
            && word(edited.get(i2))
            && edited.get(i2) == formatted.get(j2)
        {
            let line = 1 + edited[..i].iter().filter(|b| **b == b'\n').count();
            return Err(format!(
                "gofmt changed more than whitespace: line {line} of the edited text {} two tokens",
                if i2 == i { "splits" } else { "joins" }
            ));
        }
        if edited[i..i2] != formatted[j..j2] {
            out.push(Run {
                t: (i, i2),
                f: (j, j2),
            });
        }
        match (edited.get(i2), formatted.get(j2)) {
            (None, None) => return Ok(out),
            (a, b) if a == b => (i, j) = (i2 + 1, j2 + 1),
            _ => {
                let line = 1 + edited[..i].iter().filter(|b| **b == b'\n').count();
                // The text from the start of the word before the difference to the end of its
                // line, at most 40 bytes.
                let shown = |s: &[u8], at: usize| {
                    let start = (0..at).rev().find(|k| is_space(s[*k])).map_or(0, |k| k + 1);
                    let end = (start..s.len()).find(|k| s[*k] == b'\n').unwrap_or(s.len());
                    String::from_utf8_lossy(&s[start..end.min(start + 40)]).into_owned()
                };
                return Err(format!(
                    "gofmt changed more than whitespace: line {line} of the edited text has {:?}, \
                     gofmt's output {:?}",
                    shown(edited, i),
                    shown(formatted, j)
                ));
            }
        }
    }
}

/// One member of a group of touching changes: a span edit (its index) or a whitespace run.
#[derive(Debug, Clone, Copy)]
enum Member {
    Edit(usize),
    Run(usize),
}

/// The edits of one file after `gofmt`: `edits` (the span edits of `original`, sorted, not
/// overlapping), and `formatted`, `gofmt`'s output for `original` with `edits` applied. Each
/// whitespace run that differs is one more edit; a run touching a span edit (or touching a run
/// that does) joins it into one edit. The result applied to `original` gives `formatted`. An error
/// when the two texts differ outside whitespace.
pub fn merge_whitespace(
    original: &[u8],
    edits: &[TextEdit],
    formatted: &[u8],
) -> Result<Vec<TextEdit>, String> {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.start, e.end));
    let edited = apply(original, &sorted);
    let runs = runs(&edited, formatted)?;
    if runs.is_empty() {
        return Ok(sorted.into_iter().cloned().collect());
    }
    let file = sorted
        .first()
        .map(|e| e.file.clone())
        .ok_or("no span edit to merge gofmt's changes with")?;
    // Each span edit's range in the edited text, and the change in length it makes.
    let mut regions = Vec::with_capacity(sorted.len());
    let mut shift: isize = 0;
    for e in &sorted {
        let start = (e.start as isize + shift) as usize;
        regions.push((start, start + e.replacement.len()));
        shift += e.replacement.len() as isize - (e.end - e.start) as isize;
    }
    // An edited-text position outside every span edit (or at its border) in the original: less
    // the change of every span edit ending at or before it. None inside a span edit, whose own
    // range then covers it.
    let to_original = |p: usize| -> Option<usize> {
        if regions.iter().any(|r| r.0 < p && p < r.1) {
            return None;
        }
        let before: isize = sorted
            .iter()
            .zip(&regions)
            .filter(|(_, r)| r.1 <= p)
            .map(|(e, _)| e.replacement.len() as isize - (e.end - e.start) as isize)
            .sum();
        Some((p as isize - before) as usize)
    };
    // Members by start in the edited text, then grouped while they touch.
    let mut members: Vec<(usize, usize, Member)> = regions
        .iter()
        .enumerate()
        .map(|(k, r)| (r.0, r.1, Member::Edit(k)))
        .chain(
            runs.iter()
                .enumerate()
                .map(|(k, r)| (r.t.0, r.t.1, Member::Run(k))),
        )
        .collect();
    members.sort_by_key(|m| (m.0, m.1));
    let mut groups: Vec<(usize, usize, Vec<Member>)> = Vec::new();
    for (s, e, m) in members {
        match groups.last_mut() {
            Some(g) if s <= g.1 => {
                g.1 = g.1.max(e);
                g.2.push(m);
            }
            _ => groups.push((s, e, vec![m])),
        }
    }
    let delta = |r: &Run| (r.t.1 - r.t.0) as isize - (r.f.1 - r.f.0) as isize;
    let mut out = Vec::new();
    for (start, end, group) in groups {
        if group.iter().all(|m| matches!(m, Member::Edit(_))) {
            out.extend(group.iter().map(|m| match m {
                Member::Edit(k) => sorted[*k].clone(),
                Member::Run(_) => unreachable!("a group of span edits only"),
            }));
            continue;
        }
        // Where the group starts and ends in the formatted text: before it, only runs outside
        // it changed lengths; inside it, its own runs did.
        let mut inside = 0isize;
        let mut before = 0isize;
        for (k, r) in runs.iter().enumerate() {
            if group.iter().any(|m| matches!(m, Member::Run(j) if *j == k)) {
                inside += delta(r);
            } else if r.t.1 <= start {
                before += delta(r);
            }
        }
        let f_start = (start as isize - before) as usize;
        let f_end = (f_start as isize + (end - start) as isize - inside) as usize;
        let replacement = std::str::from_utf8(&formatted[f_start..f_end])
            .map_err(|e| format!("gofmt's output at {f_start}..{f_end}: {e}"))?
            .to_string();
        let (mut o_start, mut o_end) = (usize::MAX, 0);
        for m in &group {
            let ends = match m {
                Member::Edit(k) => [Some(sorted[*k].start), Some(sorted[*k].end)],
                Member::Run(k) => [to_original(runs[*k].t.0), to_original(runs[*k].t.1)],
            };
            for p in ends.into_iter().flatten() {
                o_start = o_start.min(p);
                o_end = o_end.max(p);
            }
        }
        out.push(TextEdit {
            file: file.clone(),
            start: o_start,
            end: o_end,
            replacement,
        });
    }
    let merged: Vec<&TextEdit> = out.iter().collect();
    if apply(original, &merged) != formatted {
        return Err("the merged edits do not give gofmt's output".into());
    }
    Ok(out)
}
