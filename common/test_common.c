/* Smoke tests for the ported arena and string module: allocation seams, UTF-8 handling,
 * collation, and filename ordering used by kore. */

#include <stdio.h>
#include <string.h>

#include "anoptic_memory.h"
#include "anoptic_strings.h"
#include "anoptic_strings_utf.h"

static int failures = 0;
#define CHECK(cond, msg) do { \
    if (!(cond)) { printf("FAIL: %s (%s:%d)\n", (msg), __FILE__, __LINE__); failures++; } \
} while (0)

static void test_arena(void)
{
    ano_arena_t *a = ano_arena_new(0);
    CHECK(a != NULL, "arena_new");
    char *p = ano_arena_alloc(a, 5);
    CHECK(p != NULL && ((uintptr_t)p & 15u) == 0, "alloc is 16-aligned");
    memcpy(p, "hello", 5);
    char *q = ano_arena_realloc(a, p, 4096);
    CHECK(q != NULL && memcmp(q, "hello", 5 - 1) == 0 && memcmp(q, "hello", 5) == 0, "realloc keeps bytes");
    ano_arena_free(a, q);
    char *r = ano_arena_alloc(a, 8);
    CHECK(r == q, "free of the newest block pops; the bytes reuse");
    char *z = ano_arena_zalloc(a, 64);
    int zero = 1;
    for (int i = 0; i < 64; i++) zero &= z[i] == 0;
    CHECK(zero, "zalloc zero-fills");
    // interior free is a no-op, interior realloc copies
    char *keep = ano_arena_alloc(a, 24);
    memcpy(keep, "abcdefghijklmnopqrstuvwx", 24);
    ano_arena_alloc(a, 8);                          // keep is no longer newest
    ano_arena_free(a, keep);                        // must not disturb the region
    char *grown = ano_arena_realloc(a, keep, 200);
    CHECK(grown != NULL && grown != keep && memcmp(grown, "abcdefghijklmnopqrstuvwx", 24) == 0,
          "interior realloc copies to a fresh block");
    // an oversized request gets its own chunk
    char *big = ano_arena_alloc(a, 300 * 1024);
    CHECK(big != NULL, "oversized alloc");
    ano_arena_reset(a);
    CHECK(ano_arena_used(a) == 0, "reset empties");
    CHECK(ano_arena_alloc(a, 16) != NULL, "arena usable after reset");
    ano_arena_destroy(a);
    CHECK(ano_arena_alloc(NULL, 8) == NULL, "NULL arena refuses");
}

static void test_strings(void)
{
    ano_arena_t *heap LOCALARENAATTR = ano_arena_new(0);
    anostr_t s = anostr_from_cstr(heap, "hello");
    CHECK(anostr_is_inline(s) && anostr_len(s) == 5, "short strings inline");
    anostr_t l = anostr_from_cstr(heap, "a considerably longer string that cannot inline");
    CHECK(!anostr_is_inline(l), "long strings live in the arena");
    CHECK(anostr_eq(l, anostr_from_cstr(heap, "a considerably longer string that cannot inline")), "eq across copies");
    CHECK(anostr_compare(anostr_lit("abc"), anostr_lit("abd")) < 0, "compare is byte order");
    CHECK(anostr_find(l, anostr_lit("longer"), 0) == 15, "find");

    anostr_builder_t b = anostr_builder_make(heap, 0);
    anostr_builder_appendf(&b, "%d-%s", 42, "canonical");
    anostr_builder_append_cstr(&b, "-masked-update");
    anostr_t built = anostr_freeze(&b);
    CHECK(anostr_eq(built, anostr_lit("42-canonical-masked-update")), "builder + freeze");

    anostr_intern_t *t = anostr_intern_make(heap);
    anostr_sym k1 = anostr_intern(t, anostr_lit("kin"));
    anostr_sym k2 = anostr_intern(t, anostr_from_cstr(heap, "kin"));
    CHECK(k1 == k2 && k1 != ANOSTR_SYM_NONE, "intern dedupes across variants");
}

static void test_utf(void)
{
    anostr_t ja = anostr_lit("北と両手");
    CHECK(anostr_rune_count(ja) == 4, "rune count over kanji/kana");
    CHECK(anostr_utf8_valid(ja), "valid utf-8");
    size_t i = 0;
    CHECK(anostr_rune_next(ja, &i) == 0x5317u && i == 3, "decode 北");
    anostr_t bad = anostr_lit("a\xC3(z");
    CHECK(!anostr_utf8_valid(bad), "malformed detected");
    i = 1;
    CHECK(anostr_rune_next(bad, &i) == ANORUNE_REPLACEMENT && i == 2, "malformed decodes U+FFFD, advances 1");
    CHECK(anorune_to_upper('a') == 'A' && anorune_to_lower(0x0141u) == 0x0142u, "case mapping");
    CHECK(anorune_is_letter(0x3042u), "あ is a letter");
}

static void test_collation(void)
{
    CHECK(anostr_collate(anostr_lit("Äpfel"), anostr_lit("Zebra")) < 0, "Äpfel < Zebra");
    CHECK(anostr_collate(anostr_lit("resume"), anostr_lit("résumé")) < 0, "resume < résumé");
    CHECK(anostr_collate(anostr_lit("apple"), anostr_lit("Apple")) < 0, "case is level three");
    CHECK(anostr_collate(anostr_lit("あ"), anostr_lit("い")) < 0, "gojuon order");
    CHECK(anostr_eq_base(anostr_lit("Ålesund"), anostr_lit("alesund")), "base-letter equality");
    CHECK(anostr_find_base(anostr_lit("def Kin = moore"), anostr_lit("kin"), 0) == 4, "base-letter find");

    // After removing extensions, the ASCII stem must sort before its -nihongo twin.
    anostr_t a = anostr_lit("01-canonical-masked-update");
    anostr_t b = anostr_lit("01-canonical-masked-update-nihongo");
    CHECK(anostr_collate(a, b) < 0, "stem precedes its -nihongo conjugate");

    anostr_t items[4] = {
        anostr_lit("10-conways/a"), anostr_lit("1-selection/z"),
        anostr_lit("1-selection/a-nihongo"), anostr_lit("1-selection/a"),
    };
    anostr_sort(items, 4);
    CHECK(anostr_eq(items[0], anostr_lit("1-selection/a")) &&
          anostr_eq(items[1], anostr_lit("1-selection/a-nihongo")) &&
          anostr_eq(items[2], anostr_lit("1-selection/z")) &&
          anostr_eq(items[3], anostr_lit("10-conways/a")), "sort: prefix rule + path order");
}

int main(void)
{
    test_arena();
    test_strings();
    test_utf();
    test_collation();
    if (failures == 0) printf("ok common — arena, strings, utf, collation\n");
    return failures ? 1 : 0;
}
