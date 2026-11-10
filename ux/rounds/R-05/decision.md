# R-05 decision: Are the sidebar's group names set at 14 px in sentence case instead of 10.9 px uppercase kickers?
Finding: UXF-020. Target metric: font size of the level-1 sidebar category labels (`ux/tests/l2-findings.spec.ts::UXF-020`), now 10.88 px, expected 14 px.
Yes: `website/src/css/custom.css` sets the six group names (Start here, Guides, Concepts, Examples, Reference, Project) at 0.875rem, normal letter spacing, no uppercase; colour and weight stay the theme's. shot: shots/decision-yes.png
No:  the `@beyond10x/docs-system` kicker style stays: 0.68rem, uppercase, 0.14em letter spacing, muted. shot: shots/decision-no.png
Default if unanswered by the next round: yes, because since R-03 only the open group shows its pages, so the other five names are the reader's only way into Guides, Concepts, Examples, Reference and Project, and the 14 px floor holds for every other control.
Answer:
