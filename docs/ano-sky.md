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

The Sky Registry has two rows. A reducer registers an accumulator step and whatever laws it actually has. Identity licenses a value for the empty fold, associativity licenses unordered regrouping over a fixed traversal, and associativity plus commutativity license parallel/unordered execution that may discard traversal order; exact left accumulation on a declared order needs none of those proofs beyond a compatible step. These are capabilities pushed into the syntax, load-bearing, never guessed. And the merge certificates (§10, §11), disjoint footprints, the effect algebra, the guard-complement clause, are commutativity proofs injected into the compiler that license execution freedom. Naming the shelf tells you what else goes on it: commutativity certificates for registered verbs, associativity witnesses that license parallel scan trees without changing their declared order, rewrite laws like fold fusion, σ-pushdown, mask algebra.

The failure modes are not symmetric. A wrong axiom corrupts one gather; a wrong theorem silently miscompiles every statement it touches. The ruling is binary rewrite authority. Trusted host declarations and property tests may admit or diagnose Ground behavior, but no confidence grade grants a rewrite. A Sky law is usable only after a machine checker mints its sealed rewrite capability.

## The certificate boundary

A certificate is content-addressed by law identity and version, exact typed operation signature and endpoints, schema fingerprint, normalized proposition digest, proof-checker identity and version, and proof payload. Registration reruns the small checker and the optimizer receives only the resulting sealed `Rewrite<Law>`, never raw trust metadata. Cache keys contain the same tuple; schema replacement, signature drift, proposition drift, or checker-version drift invalidates the capability. Property tests remain valuable counterexample search and BQN may remain an explanatory witness, but neither can mint `Rewrite<Law>`.

## Communion is optional

The Ground Registry is mandatory: a script can name nothing without it. The Sky above the prelude is optional. The compiler's built-in fold identities and merge families are checked and sealed at build time; no prover runs on a script's hot path. Extending the table with a certified verb, scan law, or fusion rewrite pays the checker once at registration, and every schema-matching script thereafter reuses the sealed capability. Executing an uncertified operation remains mundane; only optimization freedom is withheld.

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

The verification ladder is honest about what each rung buys. Vanilla GHC checks the embedding's schema indices and constructs candidate obligations. LiquidHaskell can discharge value-dependent refinements such as guard complement. The full contents of `proofs/foundations.md` need a dependent prover. These rungs are proof-construction and counterexample tools; production rewrite authority appears only when their result is translated to and accepted by the canonical certificate checker.

The genre is Accelerate and Feldspar: embedded array languages in Haskell, typed deep embeddings with fast native backends. Ano's seat in the genre is exact: Accelerate is pure over a dead array, ano is pure over a live world with a commit barrier. The embedding admits agreement properties between the Haskell denotation and each backend.

## The hypothesis: the machine as a registered world

Status: hypothesis, kept falsifiable. Nothing in this section changes the surface grammar.

An x64 machine is columnar data plus a step function: a code column the processor gathers from, a register column it scatters into, memory as the bulk store. This is not a metaphor and not this note's invention. It is the standard formalization: Sail (the official Arm and RISC-V formal specs), the K framework's x86-64, the ACL2 models all define the machine as a record of arrays with a transition function. What is unclaimed is the direction of use.

The correspondence with the microarchitecture is tighter than the ISA suggests. An out-of-order core already runs ano's evaluation model: register renaming makes every in-flight instruction read pure pre-state (SSA in silicon), the store buffer is the effect buffer with writes staged and invisible until commit, and retirement is the barrier. The out-of-order apparatus exists because the ISA over-sequences: it erases commutativity into a linear instruction stream and the core spends its transistor budget guessing it back. The same disease as IO's linear bind, the same heroic-runtime cure. Ano's contract is weaker than the ISA's, commutativity not total order, so it is cheaper to honor. The hardware already runs ano's model. The sequential ISA is the fiction maintained over it.

What proving buys, and what it does not. Barrier semantics makes intra-statement aliasing hazards statically absent, the thing autovectorizers burn their budget failing to prove in C, and SoA layout is native, so dependence and layout are provable and the compiler emits wide column operations without guessing. Latency is not provable: it is data-dependent (the cache miss), and Itanium died proving schedules statically. Claim the alias-freedom prize, and leave latency to the core.

The code column. Von Neumann means code is data, but literal self-modifying code dies in the pipeline (instruction-cache invalidation, thousand-cycle penalties). The usable form is JIT synthesis into fresh columns through the accepted-plan boundary. The surface does not grow runtime quotation for it: `eval` stops at the compile-time literal splice. Church reduction as a columnar transform has an existence proof in the Reduceron, an FPGA graph-reduction machine that runs template instantiation as wide parallel memory operations, β-reduction lowered to gather/scatter.

The endpoint, stated once. Register the machine itself as a world, code column, register column, memory column, and compilation becomes an ano query over it: gather λ-terms, scatter instructions. Specializing an ano evaluator written over the machine-world to a source program is the first Futamura projection, and machine code falls out as a scatter into the code column. The pieces are each established; the composition remains the bet. The first falsifiable gate is the Haskell embedding above, followed by an agreement proof against the accepted-plan denotation. Failure ends this backend experiment and changes no Ano semantics.

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

Doctrine, two registries, axioms below and proof-carrying rewrites above: adopted. The canonical certificate tuple and binary authority rule are fixed; its checker and Steel bridge remain implementation work tracked by `proofs/foundations.md`. The Haskell embedding is the first falsifiable prototype. The machine as a registered world remains a research backend hypothesis, never a language roadmap.

## The speed claim, graded

Three claims, three grades.

Grade one, established: bandwidth-optimal columnar execution. Dense column sweeps are memory-bound: the wall is roughly ten billion 8-byte elements per second per socket, and no compiler exceeds it. The game is the fraction of the ceiling you hit: naive row-oriented code lands at 5-10%, columnar code at 80-90%, and that gap is the kdb+ 10-100x. Ano sits at the ceiling by construction: SoA native, no presence test down a column, statements as pre-fused kernels. Not a bet. q cashed it.

Grade two, established mathematics with shipping silicon: provable scheduling over the certified affine fragment. The polyhedral model computes legal schedules when loop domains, access maps, and dependences are affine. Ano's barrier removes read-after-write visibility inside one statement, but it does not by itself prove alias freedom, injective destinations, affine gathers, or deterministic collisions. Those facts must come from habitat layouts, lineage maps, destination maps, and merge witnesses in the domain-and-lineage IR. An affine lattice placement `χ(k) = o + β(k)` is geometry; it helps scheduling only when the actual storage accesses are affine in the same coordinates. The eligible subset may lower to SIMD, GPU, or a static accelerator; irregular relation spans and host callables remain outside it. A statement resembles a kernel launch only after plain writes prove disjoint destinations and colliding writes supply the commutative reduction an atomic or segmented fold implements. The compiler may prove that fragment. It must refuse or choose a more general lowering outside it.

Grade three, hypothesis: the full loop, the machine as world, compilation as a query, the Futamura end. Nothing above proves it. Everything above makes it non-crazy.

The headline: the grammar confines every statement to the one fragment of computation where the evidence says static proof beats dynamic guessing, and the fragment's mathematics (polyhedral), economics (kdb+), and hardware (Groq, TPU, the GPU) all pre-exist. The engine is not new. The notation may be.
