//! A unified diff made from the edits themselves, not by a diff algorithm
//! (`docs/code-edit-language.md` § 8): per file `--- a/<path>` and `+++ b/<path>`, then a hunk
//! around each edited run of lines with three lines of context; runs whose context would touch
//! share a hunk. `git apply --check` accepts it.

use crate::change::{TextEdit, apply};

/// Lines of context around each change.
pub const CONTEXT: usize = 3;

/// The diff of `edits` (sorted by file, then start, not overlapping), each file's bytes given by
/// `source` by the edit's path. An error names a file that cannot be read, or an edit outside
/// it.
pub fn unified(
    edits: &[TextEdit],
    source: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<String, String> {
    let mut out = String::new();
    let mut files: Vec<&str> = edits.iter().map(|e| e.file.as_str()).collect();
    files.sort();
    files.dedup();
    for file in files {
        let bytes = source(file).map_err(|e| format!("{file}: {e}"))?;
        let mine: Vec<&TextEdit> = edits.iter().filter(|e| e.file == file).collect();
        if let Some(e) = mine.iter().find(|e| e.end > bytes.len() || e.start > e.end) {
            return Err(format!(
                "{file}: edit {}..{} outside its {} bytes",
                e.start,
                e.end,
                bytes.len()
            ));
        }
        out.push_str(&format!("--- a/{file}\n+++ b/{file}\n"));
        out.push_str(&file_hunks(&bytes, &mine));
    }
    Ok(out)
}

/// The lines of `bytes`: each with its line break, the last without when the file ends without
/// one.
fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split_inclusive(|b| *b == b'\n').collect()
}

/// The hunks of one file.
fn file_hunks(bytes: &[u8], edits: &[&TextEdit]) -> String {
    let old = lines(bytes);
    let mut starts = Vec::with_capacity(old.len() + 1);
    let mut at = 0;
    for l in &old {
        starts.push(at);
        at += l.len();
    }
    starts.push(at);
    // The line (0-based) holding byte `pos`.
    let line_of = |pos: usize| starts.partition_point(|&s| s <= pos).saturating_sub(1);
    // Each edit's run of old lines, `first..=last`.
    let mut runs: Vec<(usize, usize, Vec<&TextEdit>)> = Vec::new();
    for e in edits {
        let first = line_of(e.start);
        let last = if e.end > e.start {
            line_of(e.end - 1)
        } else {
            first
        };
        match runs.last_mut() {
            Some((_, l, es)) if first <= *l => {
                *l = (*l).max(last);
                es.push(e);
            }
            _ => runs.push((first, last, vec![e])),
        }
    }
    // Group runs whose context touches into hunks.
    let mut hunks: Vec<Vec<(usize, usize, Vec<&TextEdit>)>> = Vec::new();
    for run in runs {
        match hunks.last_mut() {
            Some(h) if run.0 <= h.last().expect("a hunk has a run").1 + 2 * CONTEXT + 1 => {
                h.push(run)
            }
            _ => hunks.push(vec![run]),
        }
    }
    let mut out = String::new();
    let mut delta: i64 = 0;
    for h in hunks {
        let from = h[0].0.saturating_sub(CONTEXT);
        let to = (h.last().expect("a hunk has a run").1 + CONTEXT).min(old.len().saturating_sub(1));
        let mut body = String::new();
        let (mut old_n, mut new_n) = (0usize, 0usize);
        let mut line = from;
        for (first, last, es) in &h {
            while line < *first {
                push_line(&mut body, ' ', old[line]);
                old_n += 1;
                new_n += 1;
                line += 1;
            }
            for l in &old[*first..=*last] {
                push_line(&mut body, '-', l);
                old_n += 1;
            }
            let base = starts[*first];
            let shifted: Vec<TextEdit> = es
                .iter()
                .map(|e| TextEdit {
                    file: e.file.clone(),
                    start: e.start - base,
                    end: e.end - base,
                    replacement: e.replacement.clone(),
                })
                .collect();
            let refs: Vec<&TextEdit> = shifted.iter().collect();
            let text = apply(&bytes[base..starts[*last + 1]], &refs);
            for l in lines(&text) {
                push_line(&mut body, '+', l);
                new_n += 1;
            }
            line = *last + 1;
        }
        while line <= to {
            push_line(&mut body, ' ', old[line]);
            old_n += 1;
            new_n += 1;
            line += 1;
        }
        let new_from = (from as i64 + delta) as usize;
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(from, old_n),
            range(new_from, new_n)
        ));
        out.push_str(&body);
        delta += new_n as i64 - old_n as i64;
    }
    out
}

/// A hunk range: 1-based start and count; an empty range starts at the line before.
fn range(from: usize, n: usize) -> String {
    if n == 0 {
        format!("{from},0")
    } else {
        format!("{},{n}", from + 1)
    }
}

/// One diff line; a line without a break gets git's marker.
fn push_line(out: &mut String, mark: char, line: &[u8]) {
    out.push(mark);
    out.push_str(&String::from_utf8_lossy(line));
    if !line.ends_with(b"\n") {
        out.push_str("\n\\ No newline at end of file\n");
    }
}
