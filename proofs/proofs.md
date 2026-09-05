# Proofs

The Lean sources prove laws of explicit semantic models. They do not prove that the current compiler implements every modeled operation.

- [lean.md](lean.md): build command, module map, and trust boundary.
- [foundations.md](foundations.md): obligations between the mathematical model and implementation.
- [tick-induction.md](tick-induction.md): what the finite tick harness establishes.
- [Ano.lean](Ano.lean): imports the checked modules and negative witnesses.
- [Spatial mathematics](../docs/spatialmaths.md): domain, layout, effect, and placement model.

BQN examples are explanatory witnesses, not semantic or differential oracles. Language acceptance requires Steel/Kore behavior and refusal tests.
