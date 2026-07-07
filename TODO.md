# TODO

Live items only. Anything unresolved that the suites surfaced lives in `ISSUES.md`; unsettled design lives in the spec's "Open Questions, Next Steps".

1. **Consider shipping proven-affine compilation.** Every statement is affine and alias-free by grammar, so the polyhedral arsenal (fusion, tiling, vectorization, parallelization) applies to all statements with no legality analysis. A backend that proves the schedule and emits raw column instructions is the Groq/TPU bet scoped to ano's fragment; the same property gives the CUDA lowering for free. Grading and lineage: `ano-sky.md`, "The speed claim, graded". (Added 2026-07-05.)

## Closed

The 2026-07-02 design review's ten items all landed, verified by both suites green (80 .bqn witnesses, 105 .ano twins): γ as the grouped fold (§13), the recurrence entry and the honest Fibonacci gloss (Open Questions; §21), the split hop with quantifiers and the idempotent image rule (§5), the identity entry (Open Questions), the Tier 1 regenerable-key math and Tier 3 parametricity, the Tier 2 diagonal action and value-only rank ties, the repositioning (Technical Explanation: query-and-command engine, determinism/replay, totality over the eBPF framing), the lineage rows (Inform 7, OPS5/CLIPS, Rete note; Lisp row cut) and the designed なる register (§11), the precedence table and grammar appendix with desugarings, barrier granularity, and the enum sigil, and the snippet repairs plus the nihongo fixes (ゼロが/する case frame scoped to the voice split, Ikegami and Rubin cited).
