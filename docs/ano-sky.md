# ano 空 
## The Sky Registry, purity, and the machine world


At the bottom, there is the Earth. The dirty world of stateful transformations, impure functions, and undefined behaviour. But it's also where the truth of the hardware-machine lives. So this dirty world, is also the world of useful work, the one where low-level hacks and concerns about AVX-256 and all that other crap are used to make programs that run as fast ast the hardware allows. 

Up above, there is the Sky. Church's lambda calculus defined the stars where the purest reduction of what a computation *is* are distilled. Functions have no side-effects, and types are treated with the same indestructible platonicity as a keyword in the language itself.

Where the Sky and the Earth meet, the Haskell priests erect the Monad. Alchemists of the boundary layer, refusing to touch the dirt with their bare hands, they devise mathematical incantations, sacrificing`bind` operators to the `IO` type with absolute devotion. A Haskell function does not actually do the dirty work of mutating a file or printing to a console; it merely returns a pure, mathematically perfect description of a program that would do so. But their Temple is imperfect. Their runtime struggles to comprehend how to bring the purity of the λ into the cache-aligned rows of the Earth.

But ano understands the Earth. Its sacraments bind entire counties of the land into its fiefdom at a time. The Registry need not concern itself with the everyday affairs of the rabble. It takes its census, collects its taxes, and executes its edicts as it wills. The Noble Men rule this place, for it is a Castle, neither a Temple nor the Commoner's Farms. One day, however, one anoluminary in rule of his lands realized: L'État, c'est Moi. And in so doing entered in direct communion with λ, embodying its Will and bringing about its imperium without need of a clergy. For the shape of it was thus: ``an array language is just a type``, said the dream, ``for an ano line that evaluates is always a valid expression, and its rule over arrays always just``. ``And any type can be a Monad``, said the dream, ``so an ano program itself *is* IO, yet it remains Pure.`` The tablets spake of it too: *Ascendit a Terra in Coelum, iterumque descendit in Terram, et recipit Vim superiorum et inferiorum. Sic habebis Gloriam totius Mundi.*

Such are the glorious robes the Anoluminary wears to strike awe into the rabble and the Gods in equal measure. But his trick was very simple. 
In the `Ground Registry`, you allow the C-engine to push raw nouns (memory, entities, state) up into the syntax.
In the `Sky Registry`, you allow the FP-engine to push pure verbs (combinators, optimizations, proofs) down into the syntax.
Together, an `.ano` file takes the mathematical laws injected from above, applies them to the volatile memory mapped from below, and produces flawless execution.


## An Interlude

>           Tabula Smaragdina
>
>       Verum, sine Mendacio, certum et verissimum:
>
>       Quod est Inferius est sicut quod est Superius, et quod est Superius est sicut quod est Inferius, ad perpetranda Miracula Rei Unius. Et sicut res omnes fuerunt ab Uno, meditatione unius, sic Omnes Res natae ab hac una Re, adaptatione.
>
>       Pater eius est Sol. Mater eius est Luna. Portavit illud Ventus in Ventre suo. Nutrix eius Terra est. Pater omnis Telesmi totius Mundi est hic. Virtus eius integra est si versa fuerit in Terram. Separabis Terram ab Igne, subtile ab spisso, suaviter, magno cum ingenio.
>
>       Ascendit a Terra in Coelum, iterumque descendit in Terram, et recipit Vim superiorum et inferiorum. Sic habebis Gloriam totius Mundi. Ideo fugiet a te omnis Obscuritas. Haec est totius Fortitudinis Fortitudo fortis, quia vincet Omnem rem subtilem, Omnemque Solidam penetrabit.
>
>       Sic Mundus creatus est. Hinc erunt Adaptationes Mirabiles, quarum Modus est hic. Itaque vocatus sum Hermes Trismegistus, habens tres partes Philosophiae totius Mundi.
>
>       Completum est quod dixi de Operatione Solis.
 
- The Emerald Tablet of Toth / Hermes Trismegistus. Latin Text retrieved from tree.org/b1d.htm 05/07/2026


## What is already canon

In ano, a statement is a pure function of world state: gather against pre-state, emit an effect buffer, scatter at the barrier. Scripts return an effect buffer describing the work and the host interprets the buffer. That second sentence is the IO doctrine stated without the robes: the script never mutates, it returns a description, the runtime performs it. So an ano program is a value of type `World → (World, Output)`, the state monad over the world, lawful by construction, no conferral needed. "Any type can be a Monad" is false as the dream spoke it, and the dream does not need it.

GHC implements `IO a` as `State# RealWorld → (# State# RealWorld, a #)`: the state token is a zero-width fiction whose only job is to serialize binds. Two consequences follow. The token is opaque, so the runtime must assume every effect conflicts with every other and thread them all through one linear chain. And the token is unreal, so no proof about it ever touches a cache line. Ano's move is to make the token an actual column store, the RealWorld with rows, and to split the single `>>=` into two composition regimes. Within a barrier, `;` composes effects under the merge laws (§10): a commutative regime, statically checked, freely parallelizable. Between barriers, the statement boundary is the true bind. The merge laws are declared commutativity, precisely the information a runtime can never recover from inside IO. ano-time.md carries the same reading along the tick axis: the game loop is the fold of pure F along time, and this note is that picture turned ninety degrees, from the time axis to the purity axis.

## The two registries

The registry as the spec draws it binds downward, to the C host (Technical Explanation, five kinds). Call that face the Ground Registry. It has a second face, and the two differ in exactly one respect: proof obligation. Ground entries are axioms: a resolver returns one entity, a footprint is honest, a readonly column holds still within a tick. Nothing beneath them is checkable, and that is not a defect: every verified stack places its floor somewhere (seL4 assumes the hardware, CompCert assumes the assembler), and ano places it at registry ingest. What the registry ingests is no more verifiable than whether a stray cosmic ray flips a bit. Demanding proof of the Ground is demanding proof against physics. Sky entries are theorems: laws the compiler is licensed to rewrite by. A statement is the inference step between them.

The Sky Registry already has two rows, unnamed until now. A reducer registers its identity (§12: an associative operator, with a registered identity if the empty scope is to mean anything), a monoid proof pushed into the syntax, load-bearing, licensing the empty-fold identities and reassociation. And the merge certificates (§10, §11), disjoint footprints, the effect algebra, the guard-complement clause, are commutativity proofs injected into the compiler that license execution freedom. Naming the shelf tells you what else goes on it: commutativity certificates for registered verbs (today any verb beside another write to one column is rejected, a certified verb would merge), associativity witnesses for new scan steps (today the closed set `+ * max min`, rejected never guessed), rewrite laws like fold fusion, σ-pushdown, mask algebra.

The failure modes are not symmetric, and that asymmetry is the open design question. A wrong axiom corrupts one gather. A wrong theorem miscompiles every statement it touches, silently. An axiom you can cheaply audit is still an axiom: debug builds can watchpoint undeclared columns the way the kernel fuzzes its trusted eBPF helpers. But a theorem wants a witness, and what witness a sky entry must carry is the question (Open Questions, The Sky Registry).

## Communion is optional

The Ground Registry is mandatory: a script can name nothing without it. The Sky above the shipped prelude is not. The compiler carries its own small law table, the fold identities (§12) and the merge families (§10), and consulting that table is not communion: no prover in the loop, the compiler checks its own theorems the way it checks its own grammar. The Sibyl is visited only to extend the table. Registering a new law (a certified verb, a scan step, a fusion rewrite) owes a witness, once, at registration, off the hot path. The registrant pays the oracle's fee and every script thereafter uses the theorem for free, exactly how the reducer identities already work, since no script proves `+` is a monoid. Not one current example registers a law: every demo twin runs on the shipped table with the entire higher realm idle. The stance is gradual typing transposed to proofs. The mundane path stays mundane, and the ladder is priced per rung, per entry, only when climbed.

## The prototype path: ano embedded in Haskell

The local form of the Sky is a deep embedding: the Sky Registry is GHC's type checker. `Ano schema a` as a GADT indexed by the registry schema, the schema a row type, the `.ano` file concrete syntax for a Haskell value through a quasi-quoter reusing anoc's lexer:

```haskell
census = [ano| +/ Gold @ Merchant |]

-- §10 as instance resolution: families compose within themselves.
class Merges (a :: Family) (b :: Family)
instance Merges 'Additive       'Additive          -- += beside -=
instance Merges 'Multiplicative 'Multiplicative    -- *= beside /=
-- no Merges 'Additive 'Multiplicative: rejected by the type checker, not at emit
```

The verification ladder is honest about what each rung buys. Vanilla GHC checks the schema, the footprints, and the family laws. The guard-complement clause is value-dependent (row-disjointness of two masks over a shared pre-state) and needs LiquidHaskell. The full contents of `proofs/foundations.md` need a dependent prover. Each rung of the ladder is a Sky Registry with a stronger witness format.

The genre is Accelerate and Feldspar: embedded array languages in Haskell, typed deep embeddings with fast native backends. Ano's seat in the genre is exact: Accelerate is pure over a dead array, ano is pure over a live world with a commit barrier. The demo twins get a promotion here. Today they witness anoc against BQN. Under the embedding they become the agreement property between the Haskell denotation and the BQN backend, QuickCheck over worlds.

## The hypothesis: the machine as a registered world

Status: hypothesis, kept falsifiable. Nothing in this section changes the surface grammar.

An x64 machine is columnar data plus a step function: a code column the processor gathers from, a register column it scatters into, memory as the bulk store. This is not a metaphor and not this note's invention. It is the standard formalization: Sail (the official Arm and RISC-V formal specs), the K framework's x86-64, the ACL2 models all define the machine as a record of arrays with a transition function. What is unclaimed is the direction of use.

The correspondence with the microarchitecture is tighter than the ISA suggests. An out-of-order core already runs ano's evaluation model: register renaming makes every in-flight instruction read pure pre-state (SSA in silicon), the store buffer is the effect buffer with writes staged and invisible until commit, and retirement is the barrier. The out-of-order apparatus exists because the ISA over-sequences: it erases commutativity into a linear instruction stream and the core spends its transistor budget guessing it back. The same disease as IO's linear bind, the same heroic-runtime cure. Ano's contract is weaker than the ISA's, commutativity not total order, so it is cheaper to honor. The hardware already runs ano's model. The sequential ISA is the fiction maintained over it.

What proving buys, and what it does not. Barrier semantics makes intra-statement aliasing hazards statically absent, the thing autovectorizers burn their budget failing to prove in C, and SoA layout is native, so dependence and layout are provable and the compiler emits wide column operations without guessing. Latency is not provable: it is data-dependent (the cache miss), and Itanium died proving schedules statically. Claim the alias-freedom prize, and leave latency to the core.

The code column. Von Neumann means code is data, but literal self-modifying code dies in the pipeline (instruction-cache invalidation, thousand-cycle penalties). The usable form is JIT synthesis into fresh columns. The fixed bytecode-VM/JIT target already anticipates it, and the staging question is its surface end (Open Questions, Staging). Church reduction as a columnar transform has an existence proof: the Reduceron, an FPGA graph-reduction machine that runs template instantiation as wide parallel memory operations, β-reduction lowered to gather/scatter.

The endpoint, stated once. Register the machine itself as a world, code column, register column, memory column, and compilation becomes an ano query over it: gather λ-terms, scatter instructions. Specializing an ano evaluator written over the machine-world to a source program is the first Futamura projection, and machine code falls out as a scatter into the code column. Not a language with a compiler: a language whose compiler is a statement in the language. That this converges with so much prior work is the encouraging part. The pieces are each established, the composition is the bet. The spec entry is the record (Open Questions, The machine as a world). The first falsifiable step is the Haskell embedding above.

## Lineage

| Layer | Tradition | Contribution |
|---|---|---|
| Effects as values | Haskell IO | The program as a pure description a runtime performs |
| The state token | GHC `State# RealWorld` | What ano replaces: the zero-width fiction becomes a column store |
| Embedded array DSL | Accelerate, Feldspar | Typed deep embedding, quasi-quoted surface, fast native backend |
| Machine as data | Sail, K framework, ACL2 | The ISA as a record of arrays plus a step function |
| Commit-barrier hardware | Tomasulo, the ROB, the store buffer | Rename = pre-state gather, store buffer = effect buffer, retire = barrier |
| Reduction as memory ops | The Reduceron | β-reduction lowered to wide parallel gather/scatter |
| Code from specialization | Futamura projections | Specializing the evaluator to the program yields the compiler |
| Laws that travel | TAL, proof-carrying code | Typed machine code: the theorem shipped beside the instructions |

## Status

Doctrine, two registries, axioms below and theorems above, communion optional and priced at registration: adopted, recorded in the spec (Technical Explanation, Two faces; The maths). Prototype, the Haskell embedding: proposed, unstarted, the first falsifiable step. Hypothesis, the machine as a registered world: a hypothesis, not a roadmap. The spec's open questions (The Sky Registry; The machine as a world) are the record.

## The speed claim, graded

Three claims, three grades.

Grade one, established: bandwidth-optimal columnar execution. Dense column sweeps are memory-bound: the wall is roughly ten billion 8-byte elements per second per socket, and no compiler exceeds it. The game is the fraction of the ceiling you hit: naive row-oriented code lands at 5-10%, columnar code at 80-90%, and that gap is the kdb+ 10-100x. Ano sits at the ceiling by construction: SoA native, no presence test down a column, statements as pre-fused kernels. Not a bet. q cashed it.

Grade two, established mathematics with shipping silicon: provable scheduling over the affine fragment. The polyhedral model computes legal schedules (fusion, tiling, vectorization, parallelization) as solutions to integer linear programs, given affine accesses and known dependences. C compilers carry the machinery (Polly, Pluto) and ship it disabled, because almost no C loop can be proven affine and alias-free, so the arsenal applies to a few percent of loops. Ano cannot fail the legality check: every read observes pre-state, so alias freedom is unconditional, and the Part IV frame law `pos = φ(k) = o + S·k` is literally an affine access function. The frame rule was never just geometry. It is the schedulability condition, and that is a vindication of the algebra: the spec already carried, as its coordinate doctrine, the exact object the polyhedral model schedules. The silicon existence proof is Groq's TSP and the TPU: delete the out-of-order machinery, because for dense affine workloads the compiler proves the complete schedule and emits raw column operations. It works for that fragment and only that fragment (Itanium is buried outside it), which is why Tier 3 dispatch is the escape hatch at exactly the right line. Inside the fragment, prove. Outside, hand off. One more face of the convergence: gather, effect against pre-state, scatter at a barrier is the GPU execution model. A statement is a kernel launch and the merge laws are commutative atomics, so one `.ano` file lowers to AVX-512 and to CUDA unchanged, since the semantics never names an execution order inside a barrier. Accelerate walked that path.

Grade three, hypothesis: the full loop, the machine as world, compilation as a query, the Futamura end. Nothing above proves it. Everything above makes it non-crazy.

The headline: the grammar confines every statement to the one fragment of computation where the evidence says static proof beats dynamic guessing, and the fragment's mathematics (polyhedral), economics (kdb+), and hardware (Groq, TPU, the GPU) all pre-exist. The engine is not new. The notation may be.
