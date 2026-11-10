# Heuristic checklist

Regression assets of S3 findings that no test measures (ui-iteration-loop § 5). Each round's
heuristic review reads every line against the site and files a finding for a line that fails.

| line | finding | check |
|---|---|---|
| Each claim is stated on one page and linked from the others. | UXF-009, UXF-017, UXF-022, UXF-027 | The pipeline diagram, "no language model", ESS, the trace counts and the release archive's contents appear on one page each; the home page, index, vision and status link to it. `crates/docs/src/main.rs::each_claim_is_on_one_docs_page` reads every hand-written page. |
| A page has one name in the navbar, sidebar, H1, title, footer and search. | UXF-011, UXF-018 | `crates/docs/src/main.rs::every_page_has_one_name` holds title, sidebar label and H1, `the_overview_names_pages_as_the_sidebar_does` the overview's links; the footer and navbar labels in `website/docusaurus.config.ts` are read against the sidebar by eye. Decided exception (R-06, no): the navbar's section items "Documentation" and "Reference" open one page each, and search shows "Reference › Reference". |
| No code block over 60 lines is open by default on a generated page. | UXF-013 | `crates/docs/src/main.rs::no_generated_page_shows_a_long_code_block_open`. A reference page's length is its content and is not folded. |
