/* SPDX-FileCopyrightText: 2026 Anoptic Game Engine Authors
 *
 * SPDX-License-Identifier: LGPL-3.0 */
/*  == Anoptic Game Engine v0.0000001 == */

// The bump arena behind anoptic_memory.h. Chunked singly-linked regions; allocations
// bump a cursor in the head chunk. Every block is preceded by a 16-byte header holding
// its payload size, so realloc of an arbitrary block knows how much to copy. The head
// chunk's most recent block is tracked, and only it reallocs or frees in place.

#include "anoptic_memory.h"

#include <stdint.h>
#include <string.h>

#define ARENA_ALIGN   16u
#define ARENA_DEFAULT (64u * 1024u)

typedef struct ano_chunk_t {
    struct ano_chunk_t *next;
    size_t cap, used;       // payload bytes: capacity of and cursor into data[]
    _Alignas(ARENA_ALIGN) char data[];
} ano_chunk_t;

struct ano_arena_t {
    ano_chunk_t *head;      // newest chunk; all allocation lands here
    size_t chunkSize;       // payload size for regular chunks
    char  *last;            // most recent block (in head), else NULL
    size_t total;           // payload bytes handed out
};

// n rounded up to the block grain; SIZE_MAX marks overflow.
static size_t align_up(size_t n)
{
    if (n > SIZE_MAX - ARENA_ALIGN)
        return SIZE_MAX;
    return (n + ARENA_ALIGN - 1) & ~(size_t)(ARENA_ALIGN - 1);
}

static size_t block_size(const char *p)
{
    size_t n;
    memcpy(&n, p - ARENA_ALIGN, sizeof n);
    return n;
}

static void block_set_size(char *p, size_t n)
{
    memcpy(p - ARENA_ALIGN, &n, sizeof n);
}

ano_arena_t *ano_arena_new(size_t chunkHint)
{
    ano_arena_t *a = calloc(1, sizeof *a);
    if (a == NULL)
        return NULL;
    a->chunkSize = chunkHint ? align_up(chunkHint) : ARENA_DEFAULT;
    return a;
}

void ano_arena_destroy(ano_arena_t *a)
{
    if (a == NULL)
        return;
    for (ano_chunk_t *c = a->head; c != NULL; ) {
        ano_chunk_t *next = c->next;
        free(c);
        c = next;
    }
    free(a);
}

void ano_arena_reset(ano_arena_t *a)
{
    if (a == NULL)
        return;
    ano_chunk_t *keep = a->head;    // newest = largest-or-equal; the natural survivor
    if (keep != NULL) {
        for (ano_chunk_t *c = keep->next; c != NULL; ) {
            ano_chunk_t *next = c->next;
            free(c);
            c = next;
        }
        keep->next = NULL;
        keep->used = 0;
    }
    a->last = NULL;
    a->total = 0;
}

void *ano_arena_alloc(ano_arena_t *a, size_t n)
{
    if (a == NULL)
        return NULL;
    if (n == 0)
        n = 1;
    size_t body = align_up(n);
    if (body == SIZE_MAX || body > SIZE_MAX - ARENA_ALIGN)
        return NULL;
    size_t need = ARENA_ALIGN + body;   // header + payload
    if (a->head == NULL || a->head->cap - a->head->used < need) {
        size_t cap = need > a->chunkSize ? need : a->chunkSize;
        if (cap > SIZE_MAX - sizeof(ano_chunk_t))
            return NULL;
        ano_chunk_t *c = malloc(sizeof *c + cap);
        if (c == NULL)
            return NULL;
        c->next = a->head;
        c->cap = cap;
        c->used = 0;
        a->head = c;
    }
    char *p = a->head->data + a->head->used + ARENA_ALIGN;
    block_set_size(p, n);
    a->head->used += need;
    a->last = p;
    a->total += n;
    return p;
}

void *ano_arena_zalloc(ano_arena_t *a, size_t n)
{
    void *p = ano_arena_alloc(a, n);
    if (p != NULL)
        memset(p, 0, n);
    return p;
}

void *ano_arena_realloc(ano_arena_t *a, void *p, size_t n)
{
    if (a == NULL)
        return NULL;
    if (p == NULL)
        return ano_arena_alloc(a, n);
    if (n == 0)
        n = 1;
    size_t old = block_size(p);
    // In-place when p is the newest block and its chunk holds the new size.
    if ((char *)p == a->last) {
        size_t body = align_up(n);
        size_t off = (size_t)((char *)p - a->head->data);   // payload offset in head
        if (body != SIZE_MAX && off + body <= a->head->cap) {
            a->head->used = off + body;
            block_set_size(p, n);
            a->total += n > old ? n - old : 0;
            a->total -= old > n ? old - n : 0;
            return p;
        }
    }
    if (n <= old) {         // shrink of an interior block: the bytes already suffice
        block_set_size(p, n);
        return p;
    }
    void *fresh = ano_arena_alloc(a, n);
    if (fresh == NULL)
        return NULL;
    memcpy(fresh, p, old);
    return fresh;           // the old block stays in the region until reset/destroy
}

void ano_arena_free(ano_arena_t *a, void *p)
{
    if (a == NULL || p == NULL || (char *)p != a->last)
        return;             // region granularity: only the newest block pops
    size_t n = block_size(p);
    a->head->used = (size_t)((char *)p - a->head->data) - ARENA_ALIGN;
    a->total -= n;
    a->last = NULL;
}

size_t ano_arena_used(const ano_arena_t *a)
{
    return a == NULL ? 0 : a->total;
}

void ano_arena_release(ano_arena_t **in)
{
    if (in != NULL) {
        ano_arena_destroy(*in);
        *in = NULL;
    }
}
