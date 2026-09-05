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


## Implementation

The essay above is the author's design motivation. Its C-engine terminology is historical; Steel and Kore are the Rust reference implementation.

Steel emits BQN for CBQN. Registry declarations supply data and callable contracts. Effects read pre-state and publish at barriers.

A general proof-carrying rewrite registry, Haskell embedding, and machine-as-world compiler are not implemented. Trust declarations and successful examples do not grant rewrite authority. The required proof-to-compiler bridge is recorded in [foundations.md](../proofs/foundations.md).

Current behavior belongs in [the language reference](ano-language.md). This essay establishes no measured speedup or native-backend guarantee.
