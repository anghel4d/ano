# Execution backend

Steel currently emits BQN text. `steel --run` executes it with CBQN; [steel/src/main.rs](../steel/src/main.rs) owns that path and [steel/src/emit.rs](../steel/src/emit.rs) generates the program.

This repository does not contain an implemented native Ano JIT, AOT backend, MLIR pipeline, GPU runtime, or schema-specialized native kernel. It makes no measured performance claim about such a backend.

The bytecode/JIT boundary remains a design constraint. A future backend must preserve the accepted language semantics and pass the corresponding positive and refusal tests. A BQN sibling is not an independent oracle.

The authored research perspective is retained in [ano-sky.md](ano-sky.md). Concrete open work belongs in [todo/TODO.md](../todo/TODO.md), rather than a second backend roadmap here.
