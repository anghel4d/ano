# ano ECS — the world store beneath the language

**Status: working blueprint, 2026-07-02.** This is the design of Anoptic's world store: the C-side ECS that ano scripts against. It is a commitment, not a survey. The BQN files under `demos/` are the reference semantics. The store is those shapes in bits. An implementation in `src/` is verified against the BQN post-states by differential testing (`demos/demos.md`). We target C23. Dialect notes are in §14.

## 0. Position

A traditional ECS (Flecs, EnTT, Bevy) is a scheduler's data structure. It exists so per-entity update functions iterate fast. Ano never iterates entities. Every statement is a column transformation: build a mask with vector compares, scatter an effect under it. So the store's client is a query executor, not a system scheduler. The correct lineage is not Flecs. It is kdb+, Arrow, and the APL runtimes: a column store with validity bitmaps, vectorized kernels over fixed-size pages, and copy-on-write snapshots for the time axis. The host's C systems (physics, render, input) live outside. They write their own columns and hand ano a sealed snapshot at ingest. The relationship is exactly q to kdb+. The ECS is the database. Ano is its resident query language. This document is the storage engine spec.

The design rule throughout: every mechanism below is the physical form of a law the spec already states. Where foundations.md proves something, we delete machinery. The tier theorems are storage classes. The `;` commutation law is the parallelism license. Totality by construction means the load-time verifier checks footprints and nothing else. Mathematical consistency is what lets the engine be small.

## 1. Law to mechanism

Each row is a spec commitment and the storage decision it forces. The rest of the document elaborates the right column.

| law (where) | mechanism (§) |
|---|---|
| selection is a mask; masks have no multiplicity (§5, foundations §1) | the bitmap is the engine's working currency; idempotent scatter is bitmap OR (§4) |
| a bare component name is the set of carriers (§1) | presence bitmap per column; a valueless tag *is* its bitmap, 1 bit/slot (§3) |
| one hop is one indexed read (§5) | entity ID = global slot index; `rel.Comp` is a direct gather, no row indirection ever (§2, §7) |
| left-join-null: dangling/absent drops the row (§5, ex44) | sentinel + generation mismatch clear the mask bit; no fault path (§7) |
| one statement = one gather-effect-scatter barrier (§10) | COW pages give free pre-state; effects merge in a typed delta buffer; one commit (§5) |
| `;`-effects must commute under merge laws (§10, §11) | the effect buffer is a value in a commutative monoid, exact by wide-lane accumulation (§5, §6) |
| archetype index: presence per archetype, not per key (Technical Explanation) | page summaries: presence-AND skips dead pages in O(1); archetype is emergent page locality, not layout (§4) |
| structural effects migrate archetypes (§9) | `+Comp` is a bitmap OR — migration is a bit flip, never a row move (§5) |
| γ is primitive; fold identity or row-fail (foundations §6.1, §6.3) | scatter-reduce over functional inverses, CSR fibers, implicit stencils; reducer registry with identity flags (§7, §10) |
| Tier 1 keys are regenerable (foundations §2.1) | lattice columns: no allocator, no generations, no presence bitmap — presence is total by construction (§9) |
| Tier 2 law: Sym(I)-equivariance, value-only ties (foundations §3) | mask-aligned scatters only; dense-rank kernel; sort-act-unsort runs in compressed space (§8) |
| Tier 3 law: naturality in V (foundations §4) | handle columns the engine may only memcpy, reindex, select, dispatch (§9) |
| determinism and replay (Intro, ano-time.md) | fixed-point state, saturate-once-at-scatter, canonical spawn order, COW tick partitions (§6, §11) |
| rules share one barrier per tick; conflicts rejected at install (§11) | rule registry with `_BitInt` footprints; pairwise commute check at install; dirty-page incrementality (§10, §11) |
| readonly bindings: hot host state, predicated on, never written (Binding types) | write-footprint-empty columns with double-buffered page directories swapped at ingest (§11) |

## 2. Keys: three storage classes from the tier theorems

The generable-vs-nominal asymmetry (foundations §2.1) is an allocator decision, not philosophy. Three classes fall out. Each theorem deletes a mechanism from one of them.

**Nominal keys (Tier 2).** An entity is a slot in one global index space shared by every record column. The ID carries a generation. Relationship columns store IDs as data, so the hop detects staleness with one compare against `gen[slot]`. No lookup structure. Slot reuse happens only at tick seal, from a slot-sorted free list. Allocation order is a pure function of the statement log (§11).

```c
// ano_id.h — nominal key. Invariants: NIL is all-ones; gen 0 never allocated; slot indexes every record column directly.
typedef union ano_id {
    uint64_t bits;
    struct { uint32_t slot; uint32_t gen; };
} ano_id;
static_assert(sizeof(ano_id) == 8);
constexpr ano_id ANO_NIL = { .bits = UINT64_MAX };

static inline bool id_live(const uint32_t *gen_col, ano_id x) {
    return x.bits != ANO_NIL.bits && gen_col[x.slot] == x.gen;
}
```

The union is deliberate. Relationship columns store `bits` and compare in one 64-bit op. The hop unpacks `slot` for the gather. C blesses reading a member other than the one last written. The bytes reinterpret (C23 6.5.2.3). We use that liberally, and §9 makes it load-bearing.

**Generable keys (Tier 1).** A lattice cell's key is its coordinate, recomputable from the shape (`I = ↕shape`). So a lattice column carries no allocator, no generations, no free list, and no presence bitmap. A `w×h` column is exactly `w*h` cells. Presence is total. The theorem that rank-changing ops are free over space is, physically, the fact that there is nothing to leak. Dropping cells drops bytes. `↕` remints the index whenever wanted. The header carries what the tier actually demands: shape, the frame `(o, S)`, the boundary policy (foundations §2.4).

**Opaque values (Tier 3).** The column stores fixed-width handles into host-owned pools. Naturality in V says every admitted map is a reindexing. A handle supports exactly that: memcpy, gather, mask, dispatch. The type enforces the theorem. The engine has no accessor that dereferences a handle.

## 3. Columns and pages

A component is one column over the global slot space, stored as a directory of fixed-size pages. The page is the universal granule: the unit of COW, of dirtiness, of parallelism, of the mask hierarchy, and of the vectorized kernels.

```c
// ano_page.h — geometry. 4096 slots/page: mask page = 512 B, i64 page = 32 KB, sym page = 16 KB.
enum : uint32_t {
    PAGE_SLOTS = 4096,
    PAGE_WORDS = PAGE_SLOTS / 64,           // 64 mask words
    PAGE_SHIFT = 12, PAGE_LO = PAGE_SLOTS - 1,
};

typedef enum : uint8_t { T_TAG, T_I64, T_Q, T_SYM, T_ID, T_F64, T_H64 } ano_elt;
// T_TAG: presence bitmap only, no value bytes. T_Q: 48.16 fixed point (§6). T_SYM: interned u32.
// T_ID: relationship. T_F64: readonly host columns only (§6). T_H64: opaque handle (§9).

typedef struct ano_page {
    _Atomic uint32_t rc;      // COW sharing across tick directories
    uint32_t tick;            // tick that opened this page for writing — the dirty bit (§11)
    alignas(64) unsigned char bytes[];
} ano_page;

typedef struct ano_col {
    ano_elt elt; uint8_t tier; uint16_t comp;   // registry index
    uint32_t npages;
    ano_page **dir;           // NULL entry = no slot in this page carries the component
    struct ano_mask present;  // which slots carry it — this IS the component mask of §1
} ano_col;
```

Element access is `dir[slot >> PAGE_SHIFT]->bytes` at `slot & PAGE_LO`. A NULL directory entry doubles as storage elision for a component absent from a whole page. A component carried by 1% of the world pays pages only where its carriers cluster. And they do cluster: spawns batch by statement and allocation is a bump (§5), so entities minted together land together. This recovers the archetype as **emergent locality**. The page's live component set plays the role Flecs gives the archetype table, but nothing enforces it, so nothing ever migrates. `Plot & … , +Planted` across a thousand plots is a bitmap OR over a handful of pages. Under an archetype store it is a thousand row moves. The farm interlude is the benchmark that decides this argument.

Compound components (`pos`) are SoA. The registry maps `pos.x`, `pos.y` to sibling columns sharing one presence bitmap. Field projection (dot role 3) resolves to a column handle at plan time, zero runtime cost.

Symbols intern once, globally. `T_SYM` cells are u32 indices. `` Faction == `Bandit `` is an integer compare. The intern table is append-only within a run and serialized with saves, so symbol identity is replay-stable.

Why not sparse sets (EnTT): the sparse→dense indirection puts a dependent load on every hop, and two components' dense arrays share no index space, so `Nord & TwoHanded > 60` cannot be a bitmap AND. Why not archetype tables (Flecs, Bevy): the hop becomes ID→record→(table,row)→column, two indirections against the spec's one indexed read. Structural effects become migration storms. Grade, scan, and reshape want one flat column, not a column shattered across tables. Both designs optimize entity iteration. Ano does not iterate entities.

## 4. Masks: the working currency

A selection is a mask (foundations §1: the subobject inclusion, never an endofunction). Our mask is two-level: a summary bitmap over pages, and a 512-byte bitmap per live page, with cached popcounts. All of Part I of the spec evaluates into this one type. Every effect consumes it.

```c
// ano_mask.h — two-level bitmap over the slot space.
// Invariants: sum bit p set iff page[p] non-NULL and pop[p] > 0; total = Σ pop; NULL page ≡ all-zero.
typedef struct ano_mask {
    uint64_t *sum;            // ⌈npages/64⌉ summary words
    uint64_t **page;          // 64-word bitmaps, arena-allocated per statement
    uint16_t *pop;            // per-page popcount (≤ 4096)
    uint32_t npages; uint64_t total;
} ano_mask;
```

The mask algebra is word-wise AND/OR/ANDNOT with summary short-circuit. `Nord & TwoHanded > 60` first ANDs two presence summaries. Pages where either component is empty vanish in one word op. Then it ANDs the per-page bitmaps of survivors. Only then does the value compare run, only on surviving pages. This delivers the archetype index's promise ("presence once per archetype, not per key") at page granularity by predicate pushdown, the same trick as a column database's zone maps. We order conjuncts by estimated selectivity: presence bitmaps first (free), then integer compares, then hops (they gather), then folds.

The per-page popcount picks the kernel regime, DuckDB's selection-vector switch. Below ~1/16 density a kernel iterates set bits (`stdc_trailing_zeros` over `b &= b-1`). Above it, kernels run the full page branchless and let the mask gate lanes. Both regimes appear in §5's scatter.

Value predicates write mask bits with vector compares:

```c
// cmp_gt_i64: one page of `col > s` into out bits, called only on pages the presence join kept.
// in: v = page cells, s = scalar. out: 64 mask words. invariant: bits beyond live slots are 0 (caller trims tail).
static void cmp_gt_i64(const int64_t v[static PAGE_SLOTS], int64_t s, uint64_t out[static PAGE_WORDS]) {
    for (uint32_t k = 0; k < PAGE_WORDS; k++) {
        uint64_t w = 0;
        for (uint32_t j = 0; j < 64; j++) w |= (uint64_t)(v[k*64 + j] > s) << j;   // vectorizes to cmpgt+movmsk
        out[k] = w;
    }
}
```

Masks have no multiplicity. The set hop's image (`Frenzy.targets'`, ex12) is bits OR'd into a mask. An entity reached twice is one bit. The effect lands once. The idempotent-scatter law costs nothing to enforce because the representation cannot express multiplicity. In-degree, when wanted, is a γ fold (§7), exactly as the spec routes it.

## 5. The barrier: gather, compute, scatter

One statement is one transaction against pre-state. Two mechanisms implement the barrier, and both share their machinery with the time axis (§11). COW pages make pre-state free to read. A typed delta buffer makes the scatter a single merged commit.

**Gather.** Reads never copy. Pre-state is whatever the sealed directories say. Within a statement the open tick's pages are pre-state too, because the statement's own writes haven't committed. Column expressions evaluate in **compressed space**. The mask compresses selected cells into dense scratch vectors (BQN's `mask⊸/`). Kernels run dense. Results align to the mask's popcount. This executes the `⌾(mask⊸/)` idiom from every demo literally: gather, act dense, scatter back through the same mask.

**The effect buffer is a value, not a command list.** Flecs replays deferred commands in order. Our buffer merges. Per (column, op-class) the buffer holds a delta: lazily materialized pages of accumulator lanes. Each `;`-batched effect folds its operand into the delta under its mask. `;` is the effect algebra's ⊕ performed at buffer time. The commit is order-free by construction, not by discipline. We never check the spec's commutation law (§10) at commit. It is the buffer's data structure.

```c
// ano_fx.h — effect records and the delta.
typedef enum : uint16_t {   // op classes; one class per (column, barrier), SET admits disjoint masks
    FX_ADD = 1u<<0, FX_MUL = 1u<<1, FX_MIN = 1u<<2, FX_MAX = 1u<<3,
    FX_OR  = 1u<<4, FX_AND = 1u<<5, FX_SET = 1u<<6,
    FX_TAG = 1u<<7, FX_UNTAG = 1u<<8, FX_KILL = 1u<<9, FX_SPAWN = 1u<<10,
} ano_fxop;

typedef struct ano_fx {
    uint16_t comp; ano_fxop op;
    const ano_mask *sel;                       // the saved mask — never a re-gather (§10, ex49)
    bool is_vec;
    union { int64_t scalar; const int64_t *vec; };   // broadcast, or dense vector of length sel->total
} ano_fx;

typedef struct ano_delta {                     // per (comp, opclass), pages allocated on first touch
    _BitInt(128) *acc;                          // wide lanes: the merge monoid is EXACTLY associative (§6)
    ano_mask touched;
} ano_delta;
```

**Scatter.** At statement end the buffer commits. For each delta we open the target pages copy-on-write, apply `cell ⊕ acc` under the touched mask, and saturate or reduce to storage width once (§6). The canonical ex1 kernel, both regimes:

```c
// fx_scatter_add_i64: commit an additive delta into a column, page-at-a-time under COW.
// in: c open for tick now, d = delta, sel = touched mask. out: none.
// invariant: off-mask cells bit-identical to pre-state (demos/effects/13); each cell written once per barrier.
void fx_scatter_add_i64(ano_col *c, const ano_delta *d, uint32_t now) {
    for (uint32_t p = 0; p < d->touched.npages; p++) {
        if (!(d->touched.sum[p >> 6] >> (p & 63) & 1)) continue;
        int64_t *v = (int64_t *)col_open_page(c, p, now)->bytes;     // COW iff shared or stale (§11)
        const uint64_t *w = d->touched.page[p];
        const _BitInt(128) *a = delta_page(d, p);                    // lane page, materialized on first touch
        if (d->touched.pop[p] < PAGE_SLOTS / 16) {
            for (uint32_t k = 0; k < PAGE_WORDS; k++)
                for (uint64_t b = w[k]; b; b &= b - 1) {
                    uint32_t j = k*64 + stdc_trailing_zeros(b);
                    v[j] = sat_i64(a[j] + v[j]);                     // saturate ONCE, at the boundary
                }
        } else {
            for (uint32_t j = 0; j < PAGE_SLOTS; j++) {              // dense regime: branchless, SIMD-friendly
                _BitInt(128) hit = (w[j >> 6] >> (j & 63)) & 1;
                v[j] = sat_i64(v[j] + a[j] * hit);
            }
        }
    }
}
```

**Structural effects** stage in the same buffer but commit in a canonical class order after value effects: tag OR / UNTAG ANDNOT, then relationship writes, then kills, then spawns. Kills clear presence across the slot's columns (bulk ANDNOT per column, summary-gated), bump `gen[slot]`, and queue the slot for the seal-time free list. Spawns mint from a bump cursor. That is ex22's `new ← (1+⌈´keys)+↕+´count` verbatim, ordered by (statement, selected slot, replicate index), with `ckd_add` guarding the total. Spawning last means a barrier's minted rows are untouchable by that barrier's own saved masks. That is the ex49 semantics: the ghosts survive the despawn because the saved mask predates them.

`+Comp` deserves emphasis. It is presence-OR plus, for valued components, default-fill of newly present cells. No row moves. No table migration. No archetype graph. The spec's "adding a component migrates the entity to a new archetype" holds in the only sense that matters, the presence relation, at bitmap cost.

## 6. Numbers: the merge laws pick the arithmetic

IEEE float addition is not associative. A store carrying gameplay state in f64 makes the `;` law and the rule-barrier merge a fiction, and parallel commit a nondeterminism engine. The consistency argument runs forward: the language's laws are theorems over integers and fixed point and falsehoods over floats. So Tier-2 mutable numeric columns are `T_I64` or `T_Q` (48.16 fixed point). `T_F64` is admitted only on readonly host columns, which sit outside the replay guarantee by the Binding-types paragraph anyway.

```c
// ano_q.h — 48.16 fixed point. ±1.4e14 world units at 1/65536 resolution; deterministic on every target.
typedef int64_t ano_q;
constexpr ano_q Q_ONE = 1 << 16;

static inline ano_q q_mul(ano_q a, ano_q b) {        // full 128-bit product, truncate: exact per pair
    return (ano_q)(((_BitInt(128))a * b) >> 16);
}
static inline int64_t sat_i64(_BitInt(128) x) {      // storage-width clamp, applied once at scatter
    return x > INT64_MAX ? INT64_MAX : x < INT64_MIN ? INT64_MIN : (int64_t)x;
}
```

`_BitInt(128)` is load-bearing twice. First, the delta accumulators (§5). Saturating add is commutative but not associative (`(a ⊞ big) ⊞ −big ≠ a ⊞ (big ⊞ −big)`), so saturating per effect would make the merge order-dependent. Instead operands accumulate in 128-bit lanes, where plain addition is exactly associative and commutative because no sane barrier overflows 128 bits. Saturation happens once at the storage boundary. The merge monoid is a real monoid. The `;` law and the rule-set merge are true statements about the implementation. Second, `q_mul`'s intermediate product, which makes fixed-point multiply exact per pair with no double-rounding path.

The exactness ledger gates what merges freely. ADD (wide lanes), MIN, MAX, OR, AND are exactly associative-commutative. Effects in these classes merge freely across `;` and across standing rules. SET merges only under pairwise-disjoint masks, checked from the buffer's masks. MUL under truncation is exact per pair but not associative in composition. So overlapping MUL effects on one cell in one barrier are rejected. Compose the multipliers in the script. Disjoint-mask MULs merge fine. This is the spec's "overlapping writes are accepted only when the effect algebra proves a deterministic merge" with the proofs discharged by arithmetic class.

Host callbacks that feed gameplay state (`fib`, `polar`, noise) must be deterministic per seed and target-independent: integer or fixed-point implementations, CORDIC or tables for the trig, never libm. `T_F64` readonly columns may be predicated on. The ingest snapshot (§11) makes host float churn invisible mid-tick, which is all the determinism claim needs.

## 7. Relationships: one span, three physical forms

A relationship is a span `I ← E → J`: edges with a source leg and a target leg. The registry picks the physical form from the declaration. All three forms answer the same two questions: the hop (gather along the functional direction) and γ (fold over fibers).

**Functional relationship** (`mentor`, `pen`): a `T_ID` column. The hop is the demo's `mentor⊏twoHanded` as one gather with liveness woven in:

```c
// hop_gather_i64: rel.Comp over sel → compressed values + surviving mask (left-join-null, ex8/ex44).
// in: rel T_ID column, comp column + presence, gen column, sel. out: out[0..k) dense, alive ⊆ sel.
// invariant: k = alive.total; a NIL or stale link or target lacking comp clears the bit, never faults.
size_t hop_gather_i64(const ano_col *rel, const ano_col *comp, const uint32_t *gen,
                      const ano_mask *sel, int64_t *out, ano_mask *alive);
```

Per selected slot: load `ano_id x`, test `id_live(gen, x)`, test `comp->present` at `x.slot`, gather `comp[x.slot]`. Failures drop the mask bit. The left-join-null rule is a bit clear. `mentor.mentor.Dead` chains by feeding `alive` back in as `sel`. On AVX-512 the whole test-and-gather is a masked `vpgatherqq`. Scalar code is the fallback, not the design.

The **inverse read** of a functional relationship (`livestock'` = fibers of `pen`) never materializes fibers. γ over it is a scatter-reduce, the counting-sort fold, one pass. It is `demos/gamma/36-farm-gamma.bqn`'s `+´¨ (pen∾≠pens) ⊔ cattle` without the ⊔:

```c
// gamma_count: Pen , Headcount = #/ (livestock' & Cattle). acc indexed by target slot.
// in: key = pen T_ID column, fsel = animal-side mask (present(pen) & Cattle), tsel = pen-side selection.
// out: acc[t] for t in tsel. invariant: acc starts at the fold's identity (0 for #/); empty fibers stay identity,
// so pen 3 reads 0 (demo line 14) — and a reducer with NO identity instead clears t from tsel (row-fail, foundations §6.1).
void gamma_count(const ano_id *key, const ano_mask *fsel, ano_mask *tsel, int64_t *acc);
```

**Set-valued forward relationship** (`targets`, explicit adjacency): CSR, an offsets column on sources plus a flat `T_ID` edge array. The fiber at a source is a slice. The image (`targets'` in source position) is bits OR'd from edge targets into a mask. Forward γ is a per-source slice fold. CSR rebuilds or patches at the barrier that edits it. Edge edits are structural effects with the edge array as their footprint, so the rule machinery already serializes them. Patch vs rebuild is an open tuning question (§15).

**Implicit relationship** (the stencil: `neighbors`, `prev`, `neighbor(clamp)`): no storage. The fiber comes from the lattice shape plus the registered stencil and boundary policy. γ over it is shift-and-accumulate. The demo's `S ← »+«+»˘+«˘` becomes four strided adds:

```c
// stencil4_sum_q: Σ over the 4-neighbor fiber, boundary-shrunk (fiber loses out-of-grid legs; demos/gamma/36).
// in: g = h×w lattice cells. out: sum per cell, cnt per cell (fiber size: 2 at corners, 3 at edges, 4 inside).
// invariant: matches the flattened-edge γ cross-check in the demo cell for cell.
void stencil4_sum_q(const ano_q *g, uint32_t h, uint32_t w, ano_q *sum, int32_t *cnt);
```

`avg/ neighbors'.Moisture` is then fold-and-finish (foundations §6.3): sum and count in one pass, one fixed-point divide at the end. The no-identity rule is honest because `cnt` is right there to test. The three physical forms are one algebraic object. That is why the surface needs no syntax per form. The registry's declaration picks the kernel. `fold/ rel'…` compiles to scatter-reduce, slice-fold, or stencil as the span's representation implies.

## 8. Order: grade, rank, scan, top-k

Grade runs in compressed space. Gather `(key, slot)` pairs dense, LSD radix sort on the key. Integer and fixed-point keys make this exact and fast, no float-key cleverness needed given §6. The permutation is the sorted slot order. Stable ties come free from LSD radix. Write-backs then deliberately discard them.

Rank write-backs are value-only (foundations §3.4). Dense rank gives equal keys equal ranks. That is the whole repair that makes `Unit , Slot = rank(Initiative)` equivariant. After the radix pass it is one run-length scan:

```c
// dense_rank: value-only rank from sorted (key, slot) pairs, ex18/ex38: rank ↩ (∧⍷gold)⊐gold.
// in: pairs sorted by key, n = count. out: rank[slot] dense, ties share a rank.
// invariant: rank depends on the value multiset alone — Sym(I)-equivariant by foundations §3.4.
void dense_rank(const ano_kv *pairs, size_t n, int64_t *rank_by_slot) {
    int64_t r = -1; int64_t prev;
    for (size_t i = 0; i < n; i++) {
        if (i == 0 || pairs[i].key != prev) { r++; prev = pairs[i].key; }
        rank_by_slot[pairs[i].slot] = r;
    }
}
```

`top k (grade desc …)` truncates the sorted pairs and ORs the k slots into a mask. That composes ex19's pipeline `(/enemy) ⊏˜ 5↑⍒enemy/threat` into one pass, since the gather already carried world slots through the sort. Scan (`+\ … along`) is the same shape: gather in the declared order (`along` is an index sequence, `⊏` not a mask, per ex17), run the sequential accumulate dense, scatter back. The Tier-2 conjugation theorem (foundations §3.5) is implemented literally as this sort-act-unsort sandwich. The store never offers an entity scan without an order argument because the plan compiler has no instruction for it. The obstruction is grammatical, then physical.

## 9. Space and the opaque, or: the union is the view functor

**Tier 1.** A lattice column is a dense rank-2 (or rank-n) array with a header: shape, frame, boundary.

```c
// ano_lat.h — lattice column. No allocator, no gens, no presence: the tier theorem deleted them (§2).
typedef struct ano_frame { ano_q ox, oy, sx, sy; } ano_frame;   // pos = o + S·k, foundations §2.4
typedef enum : uint8_t { B_SHRINK, B_CLAMP, B_WRAP, B_ZERO } ano_bound;
typedef struct ano_lat {
    ano_elt elt; ano_bound bound;
    uint32_t rank; uint32_t shape[2];
    ano_frame frame;                    // fixed by @scope at plan time; the counter check is compile-time (units erase)
    ano_page **dir;                     // row-major, same page machinery as §3 — COW and history for free
} ano_lat;
```

Generators cost nothing. `8 8 & (x + y) % 2 == 0` never materializes coordinate columns. `x` and `y` are affine functions of the cell index, computed in registers per page. Shifts are strided copies parameterized by `bound`. `scan2(+)` (the summed-area table, ex30 Version B) is two axis passes. Reshape (`to`) and reduce change shape freely because the output index is as definable as the input's. That is the §2 theorem again, now as the absence of bookkeeping in the reshape kernel. The frame and counter checks run at plan time and erase. The runtime never sees a unit.

**Tier 3.** A handle column is `T_H64`: an opaque 64-bit value the engine moves but never reads. Dispatch is the registry's envelope: a host function typed over (selection, handle column, output footprint), invoked per barrier with the compressed selection. The host writes an ordinary Tier-2 column back through the same delta buffer as everyone else. `Hostile , shortestPath via Adj` commits under the same laws as `Gold += 1000`.

The tier pun (foundations §1, §4.2) gets a physical spelling. The same page bytes under two members of a union are two views. Selecting the member is the functor that asserts or strips meaning:

```c
// The view pun: one page of bytes, two tiers. Reading the other member is defined byte reinterpretation (C23 6.5.2.3).
typedef union adj_page {
    uint64_t rows[PAGE_SLOTS];    // V₀ = 𝔹^64 rows: Tier 1 — bit j of rows[i]; OutDeg = +/ Adj@row is stdc_count_ones
    ano_h64  graph[PAGE_SLOTS];   // V = Graph: Tier 3 — no engine op reads it; dispatch only (ex39)
} adj_page;
```

Nothing converts. Nothing copies. The registry records which view each operation is allowed. The plan compiler admits `+/ Adj@row` under `rows` and routes `shortestPath` to dispatch under `graph`. The claim that tier is a property of the (operation, view) pair compiles to a union member access.

## 10. The registry

Registration is the compile-time contract between host and language (Binding types, Technical Explanation). Everything the plan compiler and the install checker need is a table of plain values, built with designated initializers and frozen `constexpr` where the component set is static.

```c
// ano_reg.h — descriptors. The component universe is capped at 128 per world: footprints are one _BitInt.
typedef _BitInt(128) ano_compset;               // (need & ~have) == 0 is the subset test, one op

typedef struct ano_compdef {
    const char *name; ano_elt elt; uint8_t tier;
    uint16_t merge;                              // admitted ano_fxop classes
    enum : uint8_t { W_MUTABLE, W_READONLY } write;   // W_READONLY: no delta may name it — checked at plan load
    uint16_t compound_of;                        // SoA parent, or 0
    ano_bound bound;                             // lattices and stencil rels
} ano_compdef;

typedef struct ano_reducer {                     // fold-and-finish (foundations §6.3)
    ano_fxop fold; bool has_id; int64_t id;
    enum : uint8_t { FIN_NONE, FIN_DIV, FIN_HOST } finish;
    uint8_t lanes;                               // avg carries {sum, count}: 2 accumulator lanes
} ano_reducer;

constexpr ano_reducer R_SUM = { .fold = FX_ADD, .has_id = true,  .id = 0, .finish = FIN_NONE, .lanes = 1 };
constexpr ano_reducer R_CNT = { .fold = FX_ADD, .has_id = true,  .id = 0, .finish = FIN_NONE, .lanes = 1 };
constexpr ano_reducer R_AVG = { .fold = FX_ADD, .has_id = false, .id = 0, .finish = FIN_DIV,  .lanes = 2 };
constexpr ano_reducer R_MAX = { .fold = FX_MAX, .has_id = false, .id = 0, .finish = FIN_NONE, .lanes = 1 };
```

`has_id` is the empty-scope switch. Identity reducers write the identity. Identity-free reducers clear the row's mask bit (row-fail). That is the left-join-null rule extended to folds, as one branch in the γ kernels, never a semantic mode. Relationship declarations carry the span form (§7): functional, inverse-of, CSR, or stencil, plus boundary policy. Host callbacks register an envelope: argument column types, output footprint, determinism class. A callback whose output feeds Tier-2 mutable state must be seeded-pure (§6).

Standing rules register compiled plans plus footprints. Installation runs the conflict check statically, since the rule set is known (spec §11):

```c
// ano_rule.h — a standing rule. Install rejects: writes ∩ READONLY ≠ ∅; or ∃ installed s with overlapping
// writes whose shared column's op classes are not jointly free-merging per §6's exactness ledger.
typedef struct ano_rule {
    uint32_t name;                               // interned; named for retraction (spec §11)
    struct ano_prog *prog;
    ano_compset reads, writes;
    uint16_t opclass[ANO_MAX_COMP];              // op class per written column, indexed sparsely
} ano_rule;
```

## 11. The tick, and time

One tick is one fold step of `state[t+1] = F(state[t])` (ano-time.md). The tick is four phases. Every phase reuses §5's machinery.

```c
// ano_tick — order is normative (spec §11): rules never race commands.
// ingest: swap host staging directories in; the sealed view of t is now what every gather reads.
// rules:  every installed rule gathers against sealed t; effect buffers merge into ONE delta (the rule barrier);
//         commit. Incrementality: a rule whose read set misses last tick's dirty pages replays its cached masks.
// commands: queued statements in program order, one barrier each (§5).
// seal:   freeze directories as partition t; sort and merge the tick's freed slots into the free list; t++.
void ano_tick(ano_world *w);
```

**COW directories are the time axis.** `col_open_page(c, p, now)` copies iff the page's `rc > 1` or `page->tick != now`, then marks it with `now`. That mark is simultaneously the COW guard, the dirty bit for rule incrementality, and the partition boundary for history. Sealing a tick snapshots the directory arrays, a few KB, never data. The consequences arrive in a bundle:

- **As-of reads.** "The same bandit as last tick" (the Identity open question's third option) is a read through partition t−1's directory: q's `aj` as a pointer swap. The store takes no position on the surface syntax. It makes the read O(1) so the language can decide freely.
- **Window folds.** `damage over last 60 ticks` iterates 60 directory entries for the pages that differ: q's `wj` at page granularity. The tick axis carries intrinsic order, so the Tier-2 no-canonical-previous obstruction dissolves along t (ano-time.md). The kernel is §8's ordered scan with the axis as the order.
- **Retention.** A ring of k directory roots, refcounted pages, plus keyframes and the statement log for everything older. Replay regenerates what retention drops. That is the Nix reading made physical. A saved game is a directory root plus a log position.
- **Sealed-partition claim.** A read against t−k cannot intersect the open barrier's writes, because COW means the open tick writes only copies and sealed pages are immutable by refcount. This is the foundations-grade statement ano-time.md asks for. It falls out of the allocator.

**Rule incrementality** is differential dataflow at page granularity, no Rete network. `dirty(t) = ⋃ pages opened during t`, per column. A rule re-evaluates only pages where `reads ∩ dirty(t−1)` is non-empty and replays cached per-page masks elsewhere. This is observationally identical to every-tick re-gather (spec §11), because a clean page provably yields last tick's mask bits. Crops spread one ring per tick, and the rule costs the ring's pages, not the field.

**Determinism checklist**, the replay contract in one place: fixed-point mutable state, wide-lane exact merges, saturation once at scatter (§6). Canonical spawn order and seal-time slot-sorted free-list recycling (§5). Slot-order iteration everywhere, pointer order nowhere. Seeded-pure host callbacks. Parallel commit only through the commutative classes (§12). Same tick-0 snapshot, same log, same trajectory. The mission file replays anywhere.

## 12. Parallelism: the `;` law is the license

The page is the morsel. Predicate evaluation and compressed-space compute parallelize embarrassingly, since pages are independent. The scatter is the interesting half, and the language already solved it. Effects merge through exactly associative-commutative monoids (§6), so per-worker partial deltas merge in any order to the same bits. The commutation law that legalizes `;` and the rule barrier is, unchanged, the proof obligation for parallel commit. γ's scatter-reduce keeps per-worker accumulator strips merged by the same monoids. Integer and fixed arithmetic make the merge tree's shape irrelevant. That is the concrete payoff of evicting floats from mutable state. We steal work over pages with deterministic merge points. The structural class serializes in canonical order, a tiny fraction of any barrier. Nothing in the parallel path is best-effort. Either an op class is in the exact ledger and parallelizes, or it is SET/MUL with a disjointness certificate, or that column's commit runs single-threaded. The determinism claim survives thread count.

## 13. The VM, and worked plans

Statements compile to plans: a short SSA program over mask and vector registers, executed page-at-a-time. This is vectorized interpretation in the DuckDB style. Dispatch overhead amortizes over 4096 slots, so the interpreter is already fast. The JIT then fuses a plan's kernels into one loop per page. It is an optimization, never a semantic change. Load-time verification is small because totality is grammatical (foundations §8). Check the plan's columns against declared footprints and write policies. Check the barrier's op classes against the exactness ledger. Done. The eBPF comparison ends here. There is no unbounded program to bound.

`Nord & TwoHanded > 60 , Gold += 1000` (ex1; `demos/effects/13`):

```
  %p = and.present   Nord, TwoHanded          ; presence ⋈: summary AND kills dead pages first
  %m = gt.i64        TwoHanded, 60  under %p  ; compare only on surviving pages
  save %m                                     ; the antecedent's saved mask (§10 continuations)
  fx.add.i64         Gold, 1000, sel=%m       ; folds into the delta's wide lanes
  commit                                      ; COW pages, saturate once — gold +↩ 1000×nord∧th>60
```

`Pen , Headcount = #/ (livestock' & Cattle)` (`demos/gamma/36`):

```
  %t = and           present(Pen), live
  %f = and.present   pen, Cattle              ; animal-side fiber filter
  %a = gamma.count   key=pen, sel=%f, id=0    ; scatter-reduce; #/ has identity ⇒ empty pens stay 0
  fx.set.i64         Headcount, %a, sel=%t    ; SET merges: single writer per cell this barrier
  commit                                      ; ⇒ 2 3 0 0 on the demo's pens
```

`Plot , Moisture = avg/ neighbors'.Moisture` (stencil γ, fold-and-finish):

```
  %s, %c = stencil4.sum.q  Moisture           ; Σ and fiber size in one pass, boundary-shrunk
  %v = div.q         %s, %c                   ; the avg finisher; %c = 0 would row-fail — impossible on a grid
  fx.set.q           Moisture, %v, sel=lattice
  commit                                      ; reads all landed pre-state: diffusion, not smearing
```

`Crop & Growth >= 100 , spawn Produce ; ~` (ex36 harvest; ex49 ordering):

```
  %m = and           present(Crop), ge.i64(Growth, 100)
  save %m
  fx.spawn           Produce, n=1, sel=%m     ; minted keys: bump cursor, canonical order
  fx.kill            sel=%m                   ; saved mask: pre-barrier slots only
  commit                                      ; kills clear presence + bump gens; spawns commit last —
                                              ; the produce survives its parent by construction
```

## 14. C23 inventory

The dialect earns its keep at specific joints. `union` punning: entity IDs (§2), effect operands (§5), the tier view functor (§9). Reading the unwritten member is the defined byte reinterpretation the view pun needs. `_BitInt(128)`: exact delta accumulation and fixed-point intermediates (§6), component-set footprints with one-op subset tests (§10). `<stdbit.h>`: `stdc_trailing_zeros` and `stdc_count_ones` are the mask kernels' inner loop. `<stdckdint.h>`: `ckd_add`/`ckd_mul` guard spawn totals and any path that leaves the wide lanes. `constexpr` objects and `static_assert`: frozen registry tables and layout proofs. `enum : type`: stable ABI for op classes and element kinds. Designated initializers: the registry is legible C. `alignas(64)`: page payloads on cacheline boundaries. `unreachable()`: kernel dispatch tails. `auto`, `typeof`: generic kernel macros without a macro language. `#embed`: mission files, board literals (ex34), and the BQN-derived fixtures baked into the conformance binary. Portability: `_BitInt(128)` is clang ≥14 anywhere and GCC 14 on the 64-bit mainline targets. That bounds the compiler floor. Everything else is vanilla C23.

## 15. Conformance, module map, open questions

**Conformance.** Every `demos/**/*.bqn` file is a fixture. The differential harness loads the demo's pre-state, runs the corresponding plan, and asserts bit-identical post-state against the BQN interpreter's: masks, columns, minted keys, and all. `check.sh` gates the BQN side. The C side gets the mirror script. The store has no semantics of its own to test. It has the demos'.

**Module map** (under `src/`): `ano_id` (§2), `ano_page`/`ano_mask` (§3–4), `ano_col` (columns + COW), `ano_fx` (delta buffer, scatter kernels), `ano_q` (arithmetic), `ano_rel` (hops, γ, CSR, stencils), `ano_ord` (radix, rank, scan, top-k), `ano_lat` (Tier 1), `ano_reg` (registry, install checks), `ano_tick` (phases, seal, history), `ano_vm` (plans, verification, interpretation), `ano_par` (morsel scheduler). The parser and plan compiler sit above this line and are the bootstrap-language question, open per CLAUDE.md. Everything below the line is this document.

**Open questions**, kept honest per repo discipline:

- Page size. 4096 slots balances mask granularity against COW copy cost. A write of one cell copies 32 KB of i64. Sub-page COW (512-slot sectors) halves waste at the cost of directory depth. Measure on the farm and a Noita-shaped material sim before freezing.
- CSR maintenance. Patch-in-place per dirty source vs rebuild-per-barrier. Rebuild is simpler and probably wins below ~10⁵ edges. The crossover needs numbers.
- A sparse column class. Paged NULL elision handles clustered sparsity. A component carried by 100 entities scattered across a million slots still pays pages. A sorted-slot side form fixes it and complicates every kernel with a second representation. Deferred until a real workload produces one.
- The per-slot signature transpose. A `_BitInt(128) sig[slot]` column (which components does e carry) would speed despawn and console inspection. It duplicates the presence bitmaps' truth, and every structural commit would maintain both. Rejected for now: despawn is bulk ANDNOT and inspection is cold. Recorded.
- Float ingress. Readonly host f64 columns sit outside replay, but a predicate over one (`physics.vel > x`) feeds a deterministic path from a nondeterministic source across machines. Either quantize at ingest (f64 → T_Q, making ingest the determinism boundary) or mark such predicates replay-tainting. Quantize-at-ingest is cleaner and costs host-integration friction. Leaning quantize.
- Sequential scan kernel. §8 implements `scan(f) along` for associative f. The spec's open recurrence question (non-associative f, write footprint disjoint from read) needs a genuinely sequential kernel. Trivial to add. Deliberately absent until the language decides. The store must not make the choice by shipping it.
- Rule retraction. The registry supports named uninstall. Whether a rule can retract itself mid-tick (stage advance, spec §11) touches the one-barrier-per-tick claim. Uninstall-at-seal is the conservative answer and the current plan.
