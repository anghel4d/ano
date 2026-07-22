# common/

The strings module is the C string type in this repository. One public value type (`anostr_t`, the 16-byte German-string layout), UTF-8 iteration and classification, DUCET collation (Latin, Greek, Cyrillic, Runic, kana; Han by code point), interning, compile-time string ids, and the builder as the only mutation path.

`anoptic_memory.h` and `ano_memory.c` provide the bump arena `ano_arena_t`: allocate from a region, one owner, free the region wholesale. Everything allocated from an arena dies with it; there are deliberately no per-object destroys. The arena keeps a per-block size header so realloc is total, and the newest block reallocs and frees in place, which is exactly the traffic the builder generates. The generated Unicode tables are `ano_unicode_tables.h` and `ano_collate_tables.h` from UCD/DUCET 17.0.0.

Pending: the collation internals' heapless scratch (sort records, tie-bulk views, key buffers) uses libc malloc, and its OOM fallback paths carry hand-written free sequences. A `_Thread_local` scratch arena reset at the top of each sort call would delete those free sites and their leak surface with no API change.

Build: `make` (objects), `make test` (the smoke battery: arena seams, string round-trips, UTF-8 totality, the collation facts kore's rail leans on). Consumers compile these sources directly with `-I common` — no library step, no deps. gnu23: `ANOSTR_SID`'s literal indexing in a constant expression is the one GNU extension, per the engine's own build policy.

SPDX headers carry the engine's LGPL-3.0 attribution verbatim as provenance; both trees are the same author's, and reconciling that with this repo's All Rights Reserved LICENSE is the author's call, flagged here rather than silently rewritten.
