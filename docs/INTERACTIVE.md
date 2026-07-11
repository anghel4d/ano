# INTERACTIVE — the ano editor.

Transcription of the author's notebook pages (2026-07-07 the editor and the registry, 2026-07-08 the data model). The ask, verbatim: an ano editor — "get me a harness for this lang!" — integrated into anoptic_engine as a debug panel. Rendering candidates from the margin: a space, or ncurses. The registry plan (REGFIX.md) and the data-model plan (DATAMODEL.md) are executed — PATCHES.md snapshot 26w28b is the record, `.archive/` holds the plans; this file holds the pictures.

## The panel and the commit loop

```
┌─ ano editor ───────────────────────────────────────┐    ┌──────────────┐
│ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │    │  .reg files  │◄───────┐
│                                                    │    └──────┬───────┘        │
│                 columns         relations          │           │ read in        │
│                 race      gold  twohanded  stamina │           ▼                │
│  ┌ 1 0 3 0 ┐    Nord       300     60        100   │    ┌───────────────┐       │
│  │ 0 1 0 4 │    Khajit     400     20        110   │◄───│ binary tables │       │ mv
│  │ 3 0 1 0 │    NORD        57     28        110   │───►│   in-memory   │       │ commit
│  │ 0 4 0 1 │    Imperial  1000     31        100   │    └───────┬───────┘       │
│  └ 1 1 1 0 ┘    Nord       800     50         90   │            │ write out     │
│       ▲                                            │            ▼               │
│  space? ncurses?                                   │    ┌───────────────┐       │
├────────────────────────────────────────────────────┤    │ .reg (staged) │───────┘
│ > Nord & Gold > 50 , Gold += 500                 ▷ │    └───────────────┘
└────────────────────────────────────────────────────┘
```

The world as a spreadsheet: the relation matrix beside the column store, an ano REPL line at the bottom running statements against the live tables. The loop to the right is the whole persistence story — registries read into binary in-memory tables, written out to a staged file, committed by rename. mv is atomic, so saves are crash-safe; a save file is a registry dump.

The panel's own data draws the case contract: `Gold` at the prompt folds to the `gold` column — names are always case-insensitive — while the race values `Nord` and `NORD` stay distinct — values never fold.

## The registry, three layers

```
┌─────────────────────────────────────────────────────────────┐
│ C API             structs at compile time — authoritative;  │
│                   anostr_t-style checked value types        │
├─────────────────────────────────────────────────────────────┤
│ storage files     .reg / .anoreg — data at rest:            │
│                   saves, arcade user data                   │
├─────────────────────────────────────────────────────────────┤
│ in-memory         virtual ◄──────────────────► direct ECS   │
│ representation    (anoc arena tables)   (anoptic_engine)    │
└─────────────────────────────────────────────────────────────┘
```

One API, three layers. `.reg` text is one frontend of the C API, not the API; the in-memory representation runs the full span from anoc's private arena tables to the engine's live store.

## The staged file, zoomed

```
╭╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╮
┆  column definitions                 ┆
┆  alias bindings                     ┆   directives: as · ja · role
┆  function bindings (function ptrs)  ┆
┆  spatial lattice definitions        ┆
┆  enums                              ┆
┆                                     ┆
┆  ⟨ actual columnar data & arrays ⟩  ┆
╰╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╯
```

One file carries schema, vocabulary, and data: definitions and bindings above, the columnar payload below.

## The entry taxonomy

```
columns       →  classic ECS, discrete number of columns
spaces        →  configuring basis vectors, dimensionality, lattices
arrays        →  any other array or matrix types
extern func   →  struct { ano_fptr, attributes }, strongly typed
net           →  struct { socket, format }
```

## The data model — the ladder

From the second page, titled "あの ano lang full syntax list". The flagship line, annotated:

```
Nord  &  Gold > 50 ,  Gold += 500
─┬──     ─┬──   ┬─         ┬─
 │        │     │          └─ or just +, can be inferred analytically
 │        │     └─ value of gold
 │        └─ column name
 ├─ Option A: Nord is just a tag an entity can have. 1d.
 └─ Option B: Nord is an item of a Race column, wired up "as" tag in the registry.
```

The ladder, with the two space sketches (a curved manifold, a lattice cross) at its left end:

```
        change of basis          one col
 spaces ◄─────────────► matrices ─────► arrays ◄────► columns ◄────► relations ─────► tags
   nd                     n×m          dense 1d      dense 1d       sparse 1d       points
```

Dimensionality, in the author's reading: nd → 3d → dense 1d (numerical arrays, record columns) → sparse 1d (relations-as-indexes belonging to an entity, row to row) → points (tags, sparse or dense, enum values). Spaces and transformations can be represented as matrices; a matrix column is an array; an array indexed by entities is a column; a column of row indexes is a relation; a relation or column collapsed to membership is a tag.

The right half of the ladder, walked on live data:

```
                              Gold        Master        tags
 ┌ 0 8 9 0 ┐   ┌ 0 ┐   [0]    400         [2] ──┐       Nord
 │ 1 1 6 1 │   │ 1 │   [1]    600          /    │       Khajiit
 │ 4 2 1 2 │   │ 2 │   [2]    200 ◄─────────────┘       Nord
 │ 6 3 1 3 │   │ 3 │   [3]    150         [4] ──┐       Nord
 └ 5 0 1 4 ┘   └ 4 ┘   [4]    221 ◄─────────────┘       Orc
```

Master stores row indexes — a sparse relation, `/` is none, each arrow one hop. The tags column is Option B drawn out: race values as points, one per row, read as tags. The plan that coheres all of this with the shipped semantics is DATAMODEL.md.
