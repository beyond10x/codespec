# Heuristic checklist

Regression assets of S3 findings that no test measures (ui-iteration-loop § 5). Each round's
heuristic review reads every line against the site and files a finding for a line that fails.

| line | finding | check |
|---|---|---|
| Each claim is stated on one page and linked from the others. | UXF-009 | The pipeline diagram, "no language model", ESS and the trace counts appear on one page each; the home page, index, vision and status link to it. |
| A page has one name in the navbar, sidebar, H1, title, footer and search. | UXF-011 | `crates/docs/src/main.rs::every_page_has_one_name` holds title, sidebar label and H1; the footer and navbar labels in `website/docusaurus.config.ts` are read against the sidebar by eye. |
| No code block over 60 lines is open by default on a generated page. | UXF-013 | `crates/docs/src/main.rs::no_generated_page_shows_a_long_code_block_open`. A reference page's length is its content and is not folded. |
