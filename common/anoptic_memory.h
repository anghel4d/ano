/* SPDX-FileCopyrightText: 2026 Anoptic Game Engine Authors
 *
 * SPDX-License-Identifier: LGPL-3.0 */
/*  == Anoptic Game Engine v0.0000001 == */

// Anoptic Memory API, the ano adaptation: the engine's mimalloc heap is replaced by a
// bump arena with the same region contract — allocate from a region, free the region
// wholesale. Nothing else in the strings module changed shape: ano_arena_t stands where
// mi_heap_t stood, single-owner, and every byte allocated from an arena is valid exactly
// as long as the arena.
//
// The arena is chunked (default 64 KiB, growing to fit oversized requests), returns
// 16-aligned blocks, and keeps a size header per block so realloc is total. The most
// recent allocation reallocs and frees in place — the builder's grow loop and freeze
// shrink cost nothing — and every other free is a no-op by design (region granularity).

#ifndef ANO_COMMON_ANOPTIC_MEMORY_H
#define ANO_COMMON_ANOPTIC_MEMORY_H

#include <stddef.h>
#include <stdlib.h>
#if defined(__linux__) || defined(__APPLE__)
#include <alloca.h>
#endif

// Hardware interference sizes. Compile-time constants: _Alignas and struct layout need a
// constant, not a runtime cache query. ANO_CACHE_LINE is the true coherency line — the grain for
// data meant to share a line (packing, cache-line-granular reservation). ANO_THREAD_LINE is the
// false-sharing isolation distance — _Alignas hot per-thread atomics to it so two cores' cursors
// never collide. 128 on every target: Apple Silicon's line is 128, and x86-64's adjacent-line
// prefetcher moves the 128-byte buddy pair as one, so 64-byte separation still ping-pongs.
// Override either with -DANO_CACHE_LINE=N / -DANO_THREAD_LINE=N.
#ifndef ANO_CACHE_LINE
#if defined(__APPLE__) && defined(__aarch64__)
#define ANO_CACHE_LINE 128
#else
#define ANO_CACHE_LINE 64       // x86-64 and generic arm64
#endif
#endif
#ifndef ANO_THREAD_LINE
#define ANO_THREAD_LINE 128
#endif

typedef struct ano_arena_t ano_arena_t;

// A fresh arena whose chunks are chunkHint bytes (0 = the 64 KiB default; oversized
// allocations get their own exact chunk regardless). NULL on allocation failure.
ano_arena_t *ano_arena_new(size_t chunkHint);

// Frees every chunk and the arena itself. NULL is a no-op. Every pointer ever returned
// by this arena is dead after this call.
void ano_arena_destroy(ano_arena_t *a);

// Rewinds the arena to empty, keeping its newest (largest) chunk for reuse and freeing
// the rest. Every pointer ever returned is dead after this call.
void ano_arena_reset(ano_arena_t *a);

// n bytes, 16-aligned. NULL if a is NULL, n is 0-coerced-huge, or the chunk allocation
// fails; the arena is untouched on failure.
void *ano_arena_alloc(ano_arena_t *a, size_t n);

// ano_arena_alloc, zero-filled.
void *ano_arena_zalloc(ano_arena_t *a, size_t n);

// Total like realloc: p NULL allocates, growth copies min(old, n) bytes. The most
// recent allocation grows or shrinks in place when its chunk has room; any other p
// allocates fresh and abandons the old block to the region. NULL on failure, p intact.
void *ano_arena_realloc(ano_arena_t *a, void *p, size_t n);

// Pops p when it is the arena's most recent allocation (the bytes are reused by the
// next alloc); any other p is a no-op — the region frees wholesale, not by block.
void ano_arena_free(ano_arena_t *a, void *p);

// Payload bytes currently handed out (headers and slack excluded).
size_t ano_arena_used(const ano_arena_t *a);

// Destroys an arena at end of scope. Usage:
//     ano_arena_t *scratch LOCALARENAATTR = ano_arena_new(0);
void ano_arena_release(ano_arena_t **in);
#define LOCALARENAATTR __attribute__((__cleanup__(ano_arena_release)))

// Allocates a block of memory on the stack.
// Warning: Use with EXTREME care to not overflow!
#define ano_salloc(bytes) alloca((size_t)bytes)

#endif // ANO_COMMON_ANOPTIC_MEMORY_H
