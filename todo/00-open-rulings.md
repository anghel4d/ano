# 00 — open rulings: the residue, Q/A

Only unanswered decisions remain here. Q16 needs a ruling. No ruled-but-unbuilt item lives in this file.

Q16. The debug channel emits FIBER and RELATION reports. FIBER reports include only rows kept by the predicate. RELATION reports scan the whole column. A rejected row can therefore report a dead link, and a hop used in both predicate and effect can report the same dead link twice. Whole-column reporting matches column evaluation but is noisy. Rule: should RELATION reports filter to selected rows like FIBER reports?

Status. Ruling pending. Steel implements whole-column RELATION reports. Keeping that scope needs no core change. Filtering or deduplication needs implementation and trace tests.
