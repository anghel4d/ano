# ano JIT — the execution engine, and why the racebike is not LuaJIT

**Status: design note, 2026-07-05.** This orbits ano-sky.md and ano-ecs.md. The sky note argues why every statement lands in the one fragment where static proof beats dynamic guessing, and grades the payoff (bandwidth-optimal columnar, the affine schedule, the machine-as-world end). The ECS note specifies the store the engine runs on. This note is the engine itself: how ano source becomes machine code, what the backend is, and why the fast path is a different genre from the one people reach for when they say "make it fast like LuaJIT." A position, not a survey.

## What Lua actually answers

Two implementations, two answers. Reference Lua (PUC-Rio) is a bytecode interpreter: source compiles to bytecode for a register-based VM (the 5.0 innovation, register not stack, fewer ops per operation), and a hand-tuned C dispatch loop runs it. No native code. LuaJIT is the racebike, and a separate thing: an assembly interpreter under a tracing JIT. It watches for hot loops, records the linear operation sequence of one trip through the loop straight across function calls, lowers the trace to SSA, optimizes hard (narrowing, allocation sinking, loop-invariant hoisting), and emits machine code guarded by checks that deoptimize back to the interpreter when the trace's assumptions break. NaN-boxing keeps every value in 64 bits with no heap box. It is among the fastest dynamic-language implementations ever built, and it is roughly one generational engineer's decade.

## The racebike is a different genre

Tracing is the wrong tool for ano, for a precise reason. LuaJIT's whole war is recovering structure that a scalar, dynamically-typed, mutable, aliased language destroyed: it traces because the source hid the types and the parallelism, so it must discover them at runtime and guard the guess. Ano is the inverse language. A statement is already bulk: gather against pre-state, stage an effect buffer, scatter at the barrier (ano-ecs.md §10, ano-sky.md). The barrier is not decoration. Because every effect reads only pre-state, the statement is a map/filter/scatter with no loop-carried dependency except the final scatter, which is exactly the fusion and alias-freedom LuaJIT spends its transistor budget guessing back. Ano has it by construction, statically, before the program runs.

So the genre is not the tracing JIT. It is the query and array compilers: kdb+/q, HyPer's data-centric query compilation (Neumann, "Efficiently Compiling Efficient Query Plans for Modern Hardware"), the array-language compilers (Futhark, Halide, TACO), and the affine stack (Polly, Pluto, ISL). Ano's speed ceiling is set by memory bandwidth over columns and by how well the gather-effect-scatter fuses, not by dispatch overhead over scalars. LuaJIT is the wrong idol precisely because it is the champion of the workload ano does not have.

## The current ceiling

Today anoc transpiles ano to BQN and CBQN interprets the BQN (src/, GRAMMAR.md). CBQN is a fast array interpreter, but it is still an interpreter: it dispatches array primitives and materializes intermediates, a fresh array for the mask, one for each effect temp, one for the commit. That materialization is the ceiling. The differential-tested BQN twins are the reference semantics and stay so. The engine's job is to compute those same post-states without walking a primitive-dispatch loop and without allocating the intermediates.

## The build

The racebike is a specific stack, and none of it is a research problem. It is backend engineering the front already earns the right to.

A typed IR for the gather-effect-scatter. anoc emits BQN text, which is the wrong substrate to optimize. The engine needs an SSA-shaped array/relational IR carrying column types, presence, and the barrier boundary, so fusion and scheduling are transforms over it rather than string manipulation.

Fusion of the whole statement into one pass. Instead of materializing the mask, then the temps, then the commit, generate a single loop over selected rows that reads the columns it touches, computes, and scatters. This is deforestation and operator fusion, and the barrier is the soundness proof: no pipeline breaker inside a statement except the scatter. The data-centric playbook (Neumann, HyPer) is the direct model: keep values in registers across operators until a breaker, one tight loop per pipeline, and a selection-plus-effect statement is usually one pipeline.

Schema specialization, ahead of time. Every type LuaJIT infers at runtime under guards, ano knows statically from the registry and the host ECS: column types, presence, cardinalities (ano-ecs.md §11). So the engine does at compile time, once, what LuaJIT does speculatively on every trace: no guards, no deopt, no type check in the hot loop. The mental model is a database plan compiler specialized to a schema, not a tracing JIT.

MLIR as the backend. Do not hand-roll a native backend; that path requires being Mike Pall. MLIR is the tractable form of the insane engineering here, because its dialects are almost a description of what ano already is: linalg and affine for the bulk and spatial kernels, vector for SIMD, gpu for kernel launch, and the affine dialect carries polyhedral scheduling native. LLVM is the fallback for the scalar tail. The Part IV frame law `pos = φ(k) = o + S·k` is literally an affine access function, which is the schedulability condition that dialect wants (ano-sky.md, Grade two).

A runtime. The column store (ano-ecs.md), a page allocator, a barrier scheduler that owns the commit, and an execution substrate: a thread pool for CPU, a queue for GPU. Because the semantics never names an order inside a barrier, the merge laws are commutative atomics and one statement is either an AVX-512 masked sweep or a CUDA kernel launch, unchanged (ano-sky.md, Grade two; Accelerate walked this).

SIMD and GPU are the ceiling, not a JIT feature. Mask-and-scatter maps straight onto the ISA: AVX-512 mask registers are selection masks, vgather and vscatter are the hardware ops. For a large world a statement is a kernel and scatter-under-mask is a native GPU primitive. This is the ceiling scalar Lua can never reach, because the work is bandwidth-and-SIMD bound, not dispatch bound.

## JIT or AOT

The word JIT in the title is aspirational, and read precisely. The near-term engine is ahead-of-time: compile each statement to native code against the known schema, dispatch on the world. A JIT earns its keep only where shape is unknown until runtime: world cardinality, the actual mask size a standing rule fires on, statistics that shift across ticks. That is the adaptive-query-execution analog, recompile a hot statement when its cardinality profile moves, and it is the only LuaJIT-shaped part of the design, but it specializes on data shape, not on types, which are already static. The full JIT end, the machine registered as a world and compilation as an ano query over it, the Futamura projection, is ano-sky.md's Grade three: a hypothesis, not this note's promise.

## Why this is easier than LuaJIT

The encouraging accounting, held under the correction it deserves. A production array or query JIT is person-decades in the genre (kdb+, HotSpot, LuaJIT). The effort is real, and it is mostly backend plumbing: the IR, the MLIR lowering, the runtime, the cost model that picks fuse boundaries and CPU versus GPU. But the analysis that makes LuaJIT hard, dependence analysis, alias analysis, type recovery, is discharged in ano by construction. LuaJIT spends its genius fighting the language to recover parallelism the source destroyed. Ano's grammar hands the compiler the affine access functions and a barrier that proves fusion is legal. The hard science is pre-paid by the notation. What remains is engineering, a lot of it, but not invention.

The grades are ano-sky.md's, not restated here: bandwidth-optimal columnar is established and cashed by q (Grade one); the polyhedral schedule over the affine fragment is established mathematics with shipping silicon, Groq and the TPU and the GPU (Grade two); the machine-as-world, compiler-as-a-statement end is the hypothesis (Grade three). This note commits only to the obvious part: the engine that lowers a statement past BQN into a fused, schema-specialized, vectorized kernel is Grade-one-and-two engineering, and it does not exist yet only because it has not been built.

## First falsifiable step

Match the sky note's discipline and name the smallest thing that would prove the ceiling is reachable. Take one representative statement, a masked column update over a real registry, and lower it, not to BQN, but through a typed IR to one fused native kernel: mask compare, compute, masked scatter, no materialized intermediates. Measure it against CBQN on the same post-state, on a world large enough that bandwidth dominates startup. Grade one predicts the fused kernel sits near the memory ceiling and CBQN well under it. If that gap shows up on one statement, the racebike is an engineering schedule, not a research question. The demo twins are the correctness oracle throughout: the kernel is right when its post-state matches the BQN.
