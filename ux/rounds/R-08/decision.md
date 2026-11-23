# R-08 decision: on a page that holds the language support matrix, does the table of contents column give its width to the article?

Finding: UXF-021. Target metric: support matrix and legend width at the gating viewports (scroll
containers on `/codespec/docs/reference/support`), now 8 (matrix 960 px and legend 941 px in 766 at
1366x768, in 701 at 1280x720), expected 0.
Yes: `website/src/css/custom.css` hides the table of contents column (`.col--3`) at 997 px and wider
on a page whose row holds `[data-support-matrix]`, and the article takes the full width; with the
descriptions wrapping (same change set) the matrix and legend fit at 1366x768 and 1280x720, all 12
columns in view. The page loses its outline, which holds one entry ("Reading a cell").
shot: shots/decision-yes.png
No: the outline stays; the matrix and the legend keep scrolling sideways at every desktop width, the
C and Evaluator columns are out of view until scrolled, shot: shots/decision-no.png
Default if unanswered by the next round: yes, because the outline has one entry, the matrix is the
page, and the change is one CSS rule that a later round can drop.
Answer: yes (default taken; no operator in this run), 2026-11-23
