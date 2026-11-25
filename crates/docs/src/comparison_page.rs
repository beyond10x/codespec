//! The page `website/docs/comparison.md`, written from `website/data/comparison.json`
//! ([`crate::comparison`]). Nothing on it is written by hand: the method and the dimension
//! definitions, the matrix split by dimension group, where the assessed tool stands out and where
//! it is behind, the caveats, and for every cell its note and source.
//!
//! A cell of the matrix links to its note, which carries the source. A citation in a finding's
//! text (`[^tool-DIM]`) links to the note of that cell; a file citation (`[@path:line]`) links to
//! the file at the assessed revision, on the line.

use std::fmt::Write;

use crate::comparison::{Cell, Comparison, Dimension, Finding, Kind, Piece, Source, Value, pieces};
use crate::{FRONT_MATTER_HEADER, HEADER, SOURCE, yaml_string};

/// The page, relative to the repository root.
pub const PAGE: &str = "website/docs/comparison.md";

/// The repository on GitHub.
const REPO: &str = "https://github.com/beyond10x/codespec";

/// `text` as Markdown that shows it literally: what would start emphasis, code, a link, HTML or a
/// table cell is escaped.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '>' | '|' | '~' | '{' | '}'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `url` as a Markdown link destination.
fn destination(url: &str) -> String {
    url.replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('<', "%3C")
        .replace('>', "%3E")
}

/// The host of an `https` URL, which is the link text of a source.
fn host(url: &str) -> &str {
    url.strip_prefix("https://")
        .unwrap_or(url)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(url)
}

fn anchor(tool: &str, dimension: &str) -> String {
    format!("fn-{tool}-{}", dimension.to_lowercase())
}

/// A file of the repository at `git_ref`, on its lines, as a link.
fn blob(git_ref: &str, path: &str, line: u32, end: Option<u32>) -> String {
    let lines = end.map_or(format!("L{line}"), |e| format!("L{line}-L{e}"));
    format!("{REPO}/blob/{git_ref}/{path}#{lines}")
}

fn lines(line: u32, end: Option<u32>) -> String {
    end.map_or(line.to_string(), |e| format!("{line}-{e}"))
}

/// The source of `cell` as a link, or `None`.
fn source_link(c: &Comparison, cell: &Cell) -> Option<String> {
    Some(match cell.source.as_ref()? {
        Source::Url(u) => format!("[{}](<{}>)", plain(host(&u.url)), destination(&u.url)),
        Source::Repo(r) => {
            let git_ref = r.git_ref.as_deref().unwrap_or(&c.subject.commit);
            let name = format!("{}:{}", r.path, lines(r.line, r.end));
            let on = r
                .git_ref
                .as_deref()
                .map_or(String::new(), |b| format!(" on branch `{b}`, not on main"));
            format!(
                "[{}](<{}>){on}",
                plain(&name),
                destination(&blob(git_ref, &r.path, r.line, r.end))
            )
        }
    })
}

/// A finding's text with its citations turned into links.
fn finding_text(c: &Comparison, text: &str) -> String {
    let mut out = String::new();
    for piece in pieces(text) {
        match piece {
            Piece::Text(t) | Piece::Broken(t) => out.push_str(t),
            Piece::Cell(tool, dim) => {
                let name = c
                    .tools
                    .iter()
                    .find(|t| t.id == tool)
                    .map_or(tool, |t| t.name.as_str());
                let _ = write!(
                    out,
                    "<sup>[{} {}](#{})</sup>",
                    plain(name),
                    plain(dim),
                    anchor(tool, dim)
                );
            }
            Piece::Cite(path, line, end) => {
                let _ = write!(
                    out,
                    "[{}](<{}>)",
                    plain(&format!("{path}:{}", lines(line, end))),
                    destination(&blob(&c.subject.commit, path, line, end))
                );
            }
        }
    }
    out
}

fn findings(c: &Comparison, out: &mut String, list: &[Finding]) {
    for (n, f) in list.iter().enumerate() {
        let _ = writeln!(
            out,
            "{}. **{}.** {}",
            n + 1,
            f.title.trim_end_matches('.'),
            finding_text(c, &f.text)
        );
    }
    out.push('\n');
}

fn decision(d: &Dimension) -> String {
    let Some(dec) = &d.decision else {
        return "text, not a verdict".to_owned();
    };
    let mut parts = vec![format!("**yes**: {}", plain(&dec.yes))];
    if let Some(p) = &dec.partial {
        parts.push(format!("**partial**: {}", plain(p)));
    }
    parts.push(format!("**no**: {}", plain(&dec.no)));
    parts.join(" ")
}

fn group(c: &Comparison, out: &mut String, id: &str, name: &str) {
    let dims: Vec<&Dimension> = c.dimensions.iter().filter(|d| d.group == id).collect();
    let _ = writeln!(out, "## {name}\n");
    let _ = writeln!(out, "### Dimensions\n");
    out.push_str("| id | dimension | question | how a cell is decided |\n|---|---|---|---|\n");
    for d in &dims {
        let _ = writeln!(
            out,
            "| [{}](#dim-{}) | {} | {} | {} |",
            d.id,
            d.id.to_lowercase(),
            plain(&d.name),
            plain(&d.definition),
            decision(d)
        );
    }
    out.push('\n');
    let status: Vec<&&Dimension> = dims.iter().filter(|d| d.kind == Kind::Status).collect();
    if !status.is_empty() {
        let _ = writeln!(out, "### Matrix\n");
        out.push_str("| tool |");
        for d in &status {
            let _ = write!(out, " [{}](#dim-{}) |", d.id, d.id.to_lowercase());
        }
        out.push_str("\n|---|");
        for _ in &status {
            out.push_str("---|");
        }
        out.push('\n');
        for t in &c.tools {
            let _ = write!(out, "| {} |", plain(&t.name));
            for d in &status {
                match c
                    .cells
                    .iter()
                    .find(|x| x.tool == t.id && x.dimension == d.id)
                {
                    Some(cell) => {
                        let _ =
                            write!(out, " [{}](#{}) |", cell.value.word(), anchor(&t.id, &d.id));
                    }
                    None => out.push_str(" |"),
                }
            }
            out.push('\n');
        }
        out.push('\n');
    }
    let _ = writeln!(out, "### Notes and sources\n");
    for d in &dims {
        let _ = writeln!(
            out,
            "#### {} {} {{#dim-{}}}\n",
            d.id,
            plain(&d.name),
            d.id.to_lowercase()
        );
        for t in &c.tools {
            let Some(cell) = c
                .cells
                .iter()
                .find(|x| x.tool == t.id && x.dimension == d.id)
            else {
                continue;
            };
            let verdict = if cell.value == Value::Info {
                String::new()
            } else {
                format!(" {}:", cell.value.word())
            };
            let source = source_link(c, cell).map_or(String::new(), |s| format!(" {s}"));
            let _ = writeln!(
                out,
                "- <a id=\"{}\"></a>**{}**{verdict} {}{source}",
                anchor(&t.id, &d.id),
                plain(&t.name),
                plain(&cell.note)
            );
        }
        out.push('\n');
    }
}

/// The page for `c`.
pub fn page(c: &Comparison) -> String {
    let tools = c.tools.len();
    let others = tools - 1;
    let status = c
        .dimensions
        .iter()
        .filter(|d| d.kind == Kind::Status)
        .count();
    let subject = c
        .tools
        .iter()
        .find(|t| t.id == c.subject.tool)
        .map_or(c.subject.tool.as_str(), |t| t.name.as_str());
    let short = &c.subject.commit[..8];
    let mut out = String::new();
    let _ = write!(
        out,
        "---\n{FRONT_MATTER_HEADER}\ntitle: \"Comparison with other tools\"\nsidebar_label: \"Comparison with other tools\"\nsidebar_position: 6\ndescription: {}\nlede: {}\nsource: {}\nsource_url: {}\ncustom_edit_url: null\n---\n\n{HEADER}\n\n# Comparison with other tools\n\n",
        yaml_string(&format!(
            "{} dimensions asked of codespec and {others} other code analysis tools, one cited cell for each, assessed {}.",
            c.dimensions.len(),
            c.assessed
        )),
        yaml_string(&format!(
            "Every cell about another tool cites a page of its own documentation, repository or paper; every cell about codespec cites a file and line of this repository. Assessed {}.",
            c.assessed
        )),
        yaml_string(
            "website/data/comparison.json, a codespec-comparison/1 document checked and rendered by codespec-docs"
        ),
        yaml_string(&format!("{SOURCE}/website/data/comparison.json")),
    );
    let _ = write!(
        out,
        "Assessed **{}**: {} dimensions ({status} with a verdict, {} informational), {tools} columns ({others} other tools and {subject}), {} cells. codespec cells cite its repository at revision [`{short}`]({REPO}/tree/{}) (version {}); every other cell cites the page it was read from, read on {}.\n\n",
        c.assessed,
        c.dimensions.len(),
        c.dimensions.len() - status,
        c.cells.len(),
        c.subject.commit,
        c.subject.version,
        c.assessed,
    );
    let _ = write!(
        out,
        "The assessment is data: [`website/data/comparison.json`]({SOURCE}/website/data/comparison.json), format `codespec-comparison/1`, with its JSON Schema [`schemas/comparison-1.schema.json`]({SOURCE}/schemas/comparison-1.schema.json). `task docs-check` reads the file strictly into the types the schema is generated from, refuses a cell without a source, checks that every cited file and line of this repository exists, checks every \"only\" below against the cells, and fails when this page differs from what the file renders.\n\n"
    );

    let _ = writeln!(out, "## How to read it\n");
    out.push_str("Each cell is one of:\n\n");
    for v in &c.values {
        let _ = writeln!(out, "- **{}**: {}", v.id.word(), plain(&v.rule));
    }
    let _ = write!(
        out,
        "\n**Only**, on this page, means: of the {tools} columns of this matrix, on the cells shown, on {}. Each statement below about who has **yes** is checked against the cells by `task docs-check`, which fails when the cells give `yes` to another column. An **unknown** cell may hide a capability, so a count of cells is a reading aid and not a score: the dimensions are not equally important.\n\n",
        c.assessed,
    );

    let _ = writeln!(out, "## Where codespec stands out\n");
    findings(c, &mut out, &c.stands_out);
    let _ = writeln!(out, "## Where codespec is behind\n");
    findings(c, &mut out, &c.behind);
    let _ = writeln!(out, "## Caveats\n");
    for caveat in &c.caveats {
        let _ = writeln!(out, "- {caveat}");
    }
    out.push('\n');

    let _ = writeln!(out, "## The tools\n");
    out.push_str("| tool | vendor | edition | assessed | licence | pages that did not load |\n|---|---|---|---|---|---|\n");
    for t in &c.tools {
        let edition = t.edition.as_deref().map_or(String::new(), plain);
        let version = t
            .version
            .as_deref()
            .map_or(String::new(), |v| format!(" {}", plain(v)));
        let _ = writeln!(
            out,
            "| {}{version} | {} | {edition} | {} | {} | {} |",
            plain(&t.name),
            plain(&t.vendor),
            t.assessed,
            plain(&t.license),
            t.unread.len()
        );
    }
    out.push_str("\nNo cell cites a page that did not load; the check refuses one that does.\n\n");
    let unread: Vec<_> = c.tools.iter().filter(|t| !t.unread.is_empty()).collect();
    if !unread.is_empty() {
        out.push_str("<details>\n<summary>The pages that did not load</summary>\n\n");
        for t in unread {
            let _ = writeln!(out, "**{}**\n", plain(&t.name));
            for url in &t.unread {
                let _ = writeln!(out, "- <{}>", destination(url));
            }
            out.push('\n');
        }
        out.push_str("</details>\n\n");
    }

    for g in &c.groups {
        group(c, &mut out, &g.id, &g.name);
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_shown_literally() {
        assert_eq!(
            plain("a_b *c* [d](e) <f> | `g`"),
            "a\\_b \\*c\\* \\[d\\](e) \\<f\\> \\| \\`g\\`"
        );
        assert_eq!(
            destination("https://x.org/a(b) c"),
            "https://x.org/a%28b%29%20c"
        );
        assert_eq!(host("https://docs.semgrep.dev/a/b?c"), "docs.semgrep.dev");
    }

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Every link of the page to a note ends on a note, and each cell has exactly one note.
    #[test]
    fn every_cell_has_one_note_and_every_link_to_one_lands() {
        let c = crate::comparison::check(&root()).unwrap();
        let page = page(&c);
        for cell in &c.cells {
            let id = format!("<a id=\"{}\"></a>", anchor(&cell.tool, &cell.dimension));
            assert_eq!(page.matches(&id).count(), 1, "{id}");
        }
        let mut links = 0;
        for part in page.split("](#fn-").skip(1) {
            let target = part.split(')').next().unwrap();
            assert!(
                page.contains(&format!("<a id=\"fn-{target}\"></a>")),
                "no note for #fn-{target}"
            );
            links += 1;
        }
        assert!(links > c.cells.len() / 2, "{links}");
    }

    /// The page is reachable: from the README, the sidebar and the navigation of the site.
    #[test]
    fn the_page_is_linked_from_the_readme_and_the_site() {
        let read = |rel: &str| std::fs::read_to_string(root().join(rel)).unwrap();
        assert!(read("README.md").contains("/docs/comparison)"));
        assert!(read("website/sidebars.ts").contains("'comparison'"));
        let config = read("website/docusaurus.config.ts");
        assert_eq!(config.matches("to: '/docs/comparison'").count(), 2);
    }
}
