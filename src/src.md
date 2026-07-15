# src/

Status: archived C predecessor.

This tree records the first Ano-to-BQN compiler and its tests. It is not a semantic oracle, differential oracle, implementation target, or source of current design constraints. Do not port new language behavior into it.

Steel in `steel/` is the reference compiler. Kore in `kore/` is the current interactive world. CBQN is Steel's current execution backend. The language contract is `docs/ano-language.md`; the storage and lowering contract is `docs/ano-ecs.md`; the proof obligations are `proofs/foundations.md`.

The C files remain available for archaeology where an old surface form or emitter decision needs explanation. Their tests establish only what the archived implementation did.
