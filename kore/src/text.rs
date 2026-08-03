// UTF-8 decoding, display width, Unicode classification, DUCET and natural collation,
// base-letter search, and byte-tail repair over tables.rs.
// Keep the two decoders distinct: u8next is lenient (no overlong/surrogate/range checks);
// rune_next/rune_prev are strict (malformed = U+FFFD advance exactly 1; prev backs over
// at most 3 continuation bytes and the sequence must end exactly at the cursor).

use crate::tables::*;

pub const RUNE_REPLACEMENT: u32 = 0xFFFD;
pub const NPOS: usize = usize::MAX; // find_base miss (ANOSTR_NPOS)

// kore.c u8next, the lenient forward decoder, over a byte slice: at i >= s.len()
// return 0 without advancing (the C-string NUL sentinel); ASCII returns itself; a
// 0x80..0xBF lead yields U+FFFD advancing 1; a bad continuation yields U+FFFD advancing 1;
// value assembly masks the head with c & (0x7F >> n); NO overlong/surrogate/range checks
// (lead >= 0xF8 is treated as a 4-byte head). Drives rendering, width, word-forward,
// glyph clamping.
pub fn u8next(s: &[u8], i: &mut usize) -> u32 {
    if *i >= s.len() {
        return 0;
    }
    let c = s[*i] as u32;
    let n: usize = if c < 0x80 {
        1
    } else if c < 0xC0 {
        1
    } else if c < 0xE0 {
        2
    } else if c < 0xF0 {
        3
    } else {
        4
    };
    if n == 1 {
        *i += 1;
        return if c < 0x80 { c } else { RUNE_REPLACEMENT };
    }
    let mut v = c & (0x7F >> n);
    for k in 1..n {
        // past the end reads as the C string's NUL: fails the continuation test
        let b = if *i + k < s.len() { s[*i + k] } else { 0 };
        if (b & 0xC0) != 0x80 {
            *i += 1;
            return RUNE_REPLACEMENT;
        }
        v = (v << 6) | (b as u32 & 0x3F);
    }
    *i += n;
    v
}

// Codepoint cell width (kore.c cw): fast path c < 0x1100 -> 1; the verbatim range
// list (1100-115F 231A-231B 2B1B-2B1C 2E80-303E 3041-33FF 3400-4DBF 4E00-9FFF A000-A4CF
// AC00-D7A3 F900-FAFF FE30-FE4F FF00-FF60 FFE0-FFE6 1F300-1FAFF 20000-3FFFD) -> 2; else 1.
// Not wcwidth, not a crate — the list is contract (0x3040 is 1, 0x303F is 1).
pub fn cw(c: u32) -> i32 {
    if c < 0x1100 {
        return 1;
    }
    if (c >= 0x1100 && c <= 0x115F)
        || (c >= 0x231A && c <= 0x231B)
        || (c >= 0x2B1B && c <= 0x2B1C)
        || (c >= 0x2E80 && c <= 0x303E)
        || (c >= 0x3041 && c <= 0x33FF)
        || (c >= 0x3400 && c <= 0x4DBF)
        || (c >= 0x4E00 && c <= 0x9FFF)
        || (c >= 0xA000 && c <= 0xA4CF)
        || (c >= 0xAC00 && c <= 0xD7A3)
        || (c >= 0xF900 && c <= 0xFAFF)
        || (c >= 0xFE30 && c <= 0xFE4F)
        || (c >= 0xFF00 && c <= 0xFF60)
        || (c >= 0xFFE0 && c <= 0xFFE6)
        || (c >= 0x1F300 && c <= 0x1FAFF)
        || (c >= 0x20000 && c <= 0x3FFFD)
    {
        return 2;
    }
    1
}

// Sum of cw over u8next across the whole slice (kore.c swidth).
pub fn swidth(s: &[u8]) -> i32 {
    let mut w = 0;
    let mut i = 0;
    while i < s.len() {
        w += cw(u8next(s, &mut i));
    }
    w
}

// Strict decode core (common utf8_decode, utf.c): Some((rune, consumed 1..4)) or None
// on malformed — bare continuation / 0xF8+ lead, truncation, non-continuation follower,
// overlong, > 0x10FFFF, encoded surrogate.
fn utf8_decode(p: &[u8]) -> Option<(u32, usize)> {
    let b0 = p[0];
    if b0 < 0x80 {
        return Some((b0 as u32, 1));
    }
    let (need, mut r, min): (usize, u32, u32) = if (b0 & 0xE0) == 0xC0 {
        (1, (b0 & 0x1F) as u32, 0x80)
    } else if (b0 & 0xF0) == 0xE0 {
        (2, (b0 & 0x0F) as u32, 0x800)
    } else if (b0 & 0xF8) == 0xF0 {
        (3, (b0 & 0x07) as u32, 0x10000)
    } else {
        return None; // continuation byte or 0xF8..0xFF lead
    };
    if p.len() - 1 < need {
        return None; // truncated at end
    }
    for k in 1..=need {
        if (p[k] & 0xC0) != 0x80 {
            return None;
        }
        r = (r << 6) | (p[k] & 0x3F) as u32;
    }
    if r < min || r > 0x10FFFF || (r >= 0xD800 && r <= 0xDFFF) {
        return None; // overlong, out of range, or encoded surrogate
    }
    Some((r, 1 + need))
}

// The strict decoder core (common utf8_decode): rejects bare continuation / 0xF8+ leads,
// truncation, non-continuation followers, overlong, > 0x10FFFF, surrogates. rune_next:
// at >= len clamps i to len and returns U+FFFD; malformed advances exactly 1 with U+FFFD.
// Lives inside collation and find_base.
pub fn rune_next(s: &[u8], i: &mut usize) -> u32 {
    let at = *i;
    if at >= s.len() {
        *i = s.len();
        return RUNE_REPLACEMENT;
    }
    match utf8_decode(&s[at..]) {
        Some((r, consumed)) => {
            *i = at + consumed;
            r
        }
        None => {
            *i = at + 1;
            RUNE_REPLACEMENT
        }
    }
}

// common anostr_rune_prev (utf.c): clamp i to len; i == 0 returns U+FFFD, i stays;
// else back over at most 3 continuation bytes to a candidate lead, decode forward, and the
// candidate counts only if its sequence ends exactly at the original i; anything else means
// the byte at i-1 is malformed on its own: i -= 1, U+FFFD. Drives word-back.
pub fn rune_prev(s: &[u8], i: &mut usize) -> u32 {
    let mut at = *i;
    if at > s.len() {
        at = s.len();
    }
    if at == 0 {
        *i = 0;
        return RUNE_REPLACEMENT;
    }
    let mut start = at - 1;
    while start > 0 && at - start < 4 && (s[start] & 0xC0) == 0x80 {
        start -= 1;
    }
    if let Some((r, consumed)) = utf8_decode(&s[start..]) {
        if start + consumed == at {
            *i = start;
            return r;
        }
    }
    *i = at - 1;
    RUNE_REPLACEMENT
}

// Two-stage lookup: ANO_UC_STAGE2[ANO_UC_STAGE1[r >> 8] * 256 + (r & 0xFF)] indexes
// ANO_UC_RECORDS; r >= ANO_UC_TABLE_MAX short-circuits to record 0 (the identity).
pub fn uc_record(r: u32) -> &'static UcRecord {
    if r >= ANO_UC_TABLE_MAX {
        return &ANO_UC_RECORDS[0];
    }
    let block = ANO_UC_STAGE1[(r >> 8) as usize] as usize;
    &ANO_UC_RECORDS[ANO_UC_STAGE2[block * 256 + (r & 0xFF) as usize] as usize]
}

// General category L* within the shipped scripts; kana and kanji ARE letters.
pub fn is_letter(c: u32) -> bool {
    (uc_record(c).flags & ANO_UC_LETTER) != 0
}

// Nd: ASCII 0-9 and fullwidth digits only.
pub fn is_digit(c: u32) -> bool {
    (uc_record(c).flags & ANO_UC_DIGIT) != 0
}

// The Unicode White_Space property: space/tab/newlines, NBSP, U+3000.
pub fn is_whitespace(c: u32) -> bool {
    (uc_record(c).flags & ANO_UC_WHITESPACE) != 0
}

// One source rune's worth of CEs (common CE_QUEUE_CAP).
const CE_QUEUE_CAP: usize = 64;

fn ce_primary(ce: u32) -> u32 {
    ce >> 16
}
fn ce_secondary(ce: u32) -> u32 {
    (ce >> 5) & 0x7FF
}
fn ce_tertiary(ce: u32) -> u32 {
    ce & 0x1F
}

// NFD lookup (common decomp_lookup, collate.c): bsearch ANO_DECOMP_CP (BMP-only),
// span = offset << 3 | len into ANO_DECOMP_POOL; None when the rune decomposes to itself.
fn decomp_lookup(cp: u32) -> Option<&'static [u16]> {
    if cp >= 0x10000 {
        return None; // tables are BMP-only
    }
    match ANO_DECOMP_CP.binary_search(&(cp as u16)) {
        Ok(idx) => {
            let span = ANO_DECOMP_SPAN[idx];
            let off = (span >> 3) as usize;
            let len = (span & 0x7) as usize;
            Some(&ANO_DECOMP_POOL[off..off + len])
        }
        Err(_) => None,
    }
}

// One code point's CEs appended to q (common ce_push_cp, collate.c): stage1/stage2 ->
// ANO_CE_SPANS (offset << 4 | len into ANO_CE_POOL), span index 0 = unlisted; unlisted
// (Han included, >= 0x10000) take UCA implicit weights, code point order after every
// listed primary.
fn ce_push_cp(q: &mut [u32; CE_QUEUE_CAP], mut qn: usize, cp: u32) -> usize {
    if cp < 0x10000 {
        let block = ANO_CE_STAGE1[(cp >> 8) as usize] as usize;
        let span_idx = ANO_CE_STAGE2[block * 256 + (cp & 0xFF) as usize];
        if span_idx != 0 {
            let span = ANO_CE_SPANS[span_idx as usize];
            let off = (span >> 4) as usize;
            for k in 0..(span & 0xF) as usize {
                q[qn] = ANO_CE_POOL[off + k];
                qn += 1;
            }
            return qn;
        }
    }
    let hi16 = 0xFBC0u32 + (cp >> 15);
    let lo16 = (cp & 0x7FFF) | 0x8000;
    q[qn] = hi16 << 16 | 0x20 << 5 | 0x2;
    qn += 1;
    q[qn] = lo16 << 16;
    qn += 1;
    qn
}

// One rune's CEs, decomposition included (common ce_push_rune, collate.c).
fn ce_push_rune(q: &mut [u32; CE_QUEUE_CAP], qn: usize, r: u32) -> usize {
    if let Some(d) = decomp_lookup(r) {
        let mut n = qn;
        for &cp in d {
            n = ce_push_cp(q, n, cp as u32);
        }
        return n;
    }
    ce_push_cp(q, qn, r)
}

// Streams the collation elements of s, refilled one strict-decoded source rune at a time
// (common ce_iter_t, collate.c).
struct CeIter<'a> {
    s: &'a [u8],
    i: usize,
    qn: usize,
    qk: usize,
    q: [u32; CE_QUEUE_CAP],
}

impl<'a> CeIter<'a> {
    fn new(s: &'a [u8]) -> Self {
        CeIter {
            s,
            i: 0,
            qn: 0,
            qk: 0,
            q: [0; CE_QUEUE_CAP],
        }
    }

    fn ce_next(&mut self) -> Option<u32> {
        while self.qk == self.qn {
            if self.i >= self.s.len() {
                return None;
            }
            self.qk = 0;
            let r = rune_next(self.s, &mut self.i);
            self.qn = ce_push_rune(&mut self.q, 0, r);
        }
        let ce = self.q[self.qk];
        self.qk += 1;
        Some(ce)
    }

    // The next nonzero weight of the given level; None once the string is exhausted.
    fn next_weight(&mut self, level: i32) -> Option<u32> {
        while let Some(ce) = self.ce_next() {
            let v = match level {
                0 => ce_primary(ce),
                1 => ce_secondary(ce),
                _ => ce_tertiary(ce),
            };
            if v != 0 {
                return Some(v);
            }
        }
        None
    }
}

// One level's comparison (common collate_level, collate.c): streaming both sides'
// nonzero weights; the exhausted side sorts first — the prefix rule, PER LEVEL.
fn collate_level(a: &[u8], b: &[u8], level: i32) -> i32 {
    let mut ia = CeIter::new(a);
    let mut ib = CeIter::new(b);
    loop {
        let wa = ia.next_weight(level);
        let wb = ib.next_weight(level);
        match (wa, wb) {
            (None, None) => return 0,
            (Some(_), None) => return 1,
            (None, Some(_)) => return -1,
            (Some(x), Some(y)) => {
                if x != y {
                    return if x < y { -1 } else { 1 };
                }
            }
        }
    }
}

// DUCET collation (common anostr_collate, collate.c): byte-equal short-circuits 0;
// three passes (primary, secondary, tertiary) each streaming both strings' nonzero weights
// of that level, exhausted side first PER LEVEL (the prefix rule); ties break by byte order
// (memcmp sign, shorter-first). The CE iterator per rune: NFD via bsearch ANO_DECOMP_CP
// (BMP-only; span = offset << 3 | len into ANO_DECOMP_POOL); CE map via
// ANO_CE_STAGE2[ANO_CE_STAGE1[cp >> 8] * 256 + (cp & 0xFF)] -> ANO_CE_SPANS
// (offset << 4 | len into ANO_CE_POOL), span 0 = unlisted; unlisted (Han, >= 0x10000) take
// UCA implicit weights: hi = 0xFBC0 + (cp >> 15), lo = (cp & 0x7FFF) | 0x8000, CEs
// (hi<<16 | 0x20<<5 | 0x2) and (lo<<16). Queue cap 64. CE fields: primary ce>>16,
// secondary (ce>>5)&0x7FF, tertiary ce&0x1F. Returns memcmp-sign int.
pub fn collate(a: &[u8], b: &[u8]) -> i32 {
    if a == b {
        return 0; // byte-equal is collate-equal, skip the stream
    }
    for level in 0..3 {
        let c = collate_level(a, b, level);
        if c != 0 {
            return c;
        }
    }
    // byte order makes the order total (memcmp sign, shorter-first)
    match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

// kore.c collate_natural: maximal ASCII digit runs ('0'..'9' only) compare as
// numbers — leading zeros stripped keeping at least one digit, then by digit count, then
// bytes; equal numeric value continues the walk. A digit run against a non-digit at the
// same position falls back to straight collate on both whole remainders. Non-digit
// stretches collate by DUCET. Exhaustion: the longer string sorts later.
pub fn collate_natural(x: &[u8], y: &[u8]) -> i32 {
    let (lx, ly) = (x.len(), y.len());
    let (mut i, mut j) = (0usize, 0usize);
    while i < lx && j < ly {
        let xd = x[i].is_ascii_digit();
        let yd = y[j].is_ascii_digit();
        // digit against non-digit: straight DUCET on the remainders decides
        if xd != yd {
            return collate(&x[i..], &y[j..]);
        }
        if xd {
            let (mut si, mut sj) = (i, j);
            while i < lx && x[i].is_ascii_digit() {
                i += 1;
            }
            while j < ly && y[j].is_ascii_digit() {
                j += 1;
            }
            while si + 1 < i && x[si] == b'0' {
                si += 1;
            }
            while sj + 1 < j && y[sj] == b'0' {
                sj += 1;
            }
            let (nx, ny) = (i - si, j - sj);
            if nx != ny {
                return if nx < ny { -1 } else { 1 };
            }
            match x[si..i].cmp(&y[sj..j]) {
                std::cmp::Ordering::Less => return -1,
                std::cmp::Ordering::Greater => return 1,
                std::cmp::Ordering::Equal => {} // equal value (01 vs 1): run on, the caller's byte tiebreak settles it
            }
        } else {
            let (si, sj) = (i, j);
            while i < lx && !x[i].is_ascii_digit() {
                i += 1;
            }
            while j < ly && !y[j].is_ascii_digit() {
                j += 1;
            }
            let c = collate(&x[si..i], &y[sj..j]);
            if c != 0 {
                return c;
            }
        }
    }
    if i < lx {
        1
    } else if j < ly {
        -1
    } else {
        0
    }
}

// Primary-weight-only prefix match (common anostr_starts_base, collate.c): streams
// both primary sequences; prefix exhausting first is a match (empty needle matches).
pub fn starts_base(s: &[u8], prefix: &[u8]) -> bool {
    let mut si = CeIter::new(s);
    let mut pi = CeIter::new(prefix);
    loop {
        let wp = match pi.next_weight(0) {
            Some(w) => w,
            None => return true,
        };
        match si.next_weight(0) {
            Some(ws) if ws == wp => {}
            _ => return false,
        }
    }
}

// common anostr_find_base (collate.c): clamp from to len, try starts_base at each
// rune boundary advancing one STRICT-decoded rune per failed candidate (a malformed byte
// advances exactly 1); byte index of the match start or NPOS. Empty needle matches at
// min(from, len). Case- and accent-insensitive; combining marks transparent.
pub fn find_base(s: &[u8], needle: &[u8], from: usize) -> usize {
    let len = s.len();
    let mut i = if from < len { from } else { len };
    loop {
        if starts_base(&s[i..], needle) {
            return i;
        }
        if i >= len {
            return NPOS;
        }
        rune_next(s, &mut i); // next candidate start
    }
}

// The rail order (kore.c cmp_demo): collate_natural over both paths with a trailing
// ".ano" stripped (last 4 bytes when len > 4), byte-order tiebreak on the full paths.
pub fn cmp_demo(a: &str, b: &str) -> std::cmp::Ordering {
    let (x, y) = (a.as_bytes(), b.as_bytes());
    let sx = if x.len() > 4 { &x[..x.len() - 4] } else { x };
    let sy = if y.len() > 4 { &y[..y.len() - 4] } else { y };
    let c = collate_natural(sx, sy);
    if c < 0 {
        std::cmp::Ordering::Less
    } else if c > 0 {
        std::cmp::Ordering::Greater
    } else {
        x.cmp(y)
    }
}

// kore.c u8_tail_fix: a byte-capped copy must not end mid-codepoint — back over
// trailing continuation bytes to the last lead; if the lead's declared length overruns the
// end, truncate at the lead. In-place on the owned buffer.
pub fn u8_tail_fix(s: &mut Vec<u8>) {
    let n = s.len();
    let mut k = n;
    while k > 0 && (s[k - 1] & 0xC0) == 0x80 {
        k -= 1;
    }
    if k == 0 {
        return;
    }
    let h = s[k - 1];
    let need: usize = if h < 0xC0 {
        1
    } else if h < 0xE0 {
        2
    } else if h < 0xF0 {
        3
    } else {
        4
    };
    if need > n - k + 1 {
        s.truncate(k - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collate_pins() {
        // gojuon: あ < い
        assert!(collate("あ".as_bytes(), "い".as_bytes()) < 0);
        // accents are level two
        assert!(collate(b"resume", "résumé".as_bytes()) < 0);
        // case is level three, lower before upper
        assert!(collate(b"apple", b"Apple") < 0);
        // base letters are level one
        assert!(collate("Äpfel".as_bytes(), b"Zebra") < 0);
        // the prefix rule: a stem sorts before its own conjugate
        assert!(
            collate(
                b"01-canonical-masked-update",
                b"01-canonical-masked-update-nihongo"
            ) < 0
        );
    }

    #[test]
    fn natural_pins() {
        // digit runs as numbers: 2- before 10-
        assert!(collate_natural(b"2-x", b"10-x") < 0);
        // leading zeros strip; equal value continues to the tail
        assert_eq!(collate_natural(b"01", b"1"), 0);
        assert!(collate_natural(b"01a", b"1b") < 0);
        // longer sorts later on exhaustion
        assert!(collate_natural(b"a", b"ab") < 0);
    }

    #[test]
    fn find_base_pin() {
        assert_eq!(find_base(b"def Kin = moore", b"kin", 0), 4);
        assert_eq!(find_base(b"abc", b"", 1), 1);
        assert_eq!(find_base(b"abc", b"zz", 0), NPOS);
        // accent-transparent
        assert!(starts_base("Ålesund".as_bytes(), b"alesund"));
    }

    #[test]
    fn demo_order_pin() {
        use std::cmp::Ordering;
        assert_eq!(
            cmp_demo(
                "demos/01-canonical-masked-update.ano",
                "demos/01-canonical-masked-update-nihongo.ano"
            ),
            Ordering::Less
        );
        assert_eq!(cmp_demo("demos/2-b.ano", "demos/10-a.ano"), Ordering::Less);
    }

    #[test]
    fn decoder_pins() {
        // lenient: bare continuation is one-byte U+FFFD, no range checks
        let mut i = 0;
        assert_eq!(u8next(b"\x80a", &mut i), RUNE_REPLACEMENT);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(u8next("あ".as_bytes(), &mut i), 0x3042);
        assert_eq!(i, 3);
        let mut i = 3;
        assert_eq!(u8next(b"abc", &mut i), 0);
        assert_eq!(i, 3);
        // strict: overlong C0 80 rejects, advances 1
        let mut i = 0;
        assert_eq!(rune_next(b"\xC0\x80", &mut i), RUNE_REPLACEMENT);
        assert_eq!(i, 1);
        // prev: back over the 3-byte kana
        let s = "aあ".as_bytes();
        let mut i = s.len();
        assert_eq!(rune_prev(s, &mut i), 0x3042);
        assert_eq!(i, 1);
        assert_eq!(rune_prev(s, &mut i), 'a' as u32);
        assert_eq!(i, 0);
        assert_eq!(rune_prev(s, &mut i), RUNE_REPLACEMENT);
    }

    #[test]
    fn width_and_tail() {
        assert_eq!(cw(0x3042), 2);
        assert_eq!(cw(0x3040), 1);
        assert_eq!(cw(0x303F), 1);
        assert_eq!(swidth("aあb".as_bytes()), 4);
        let mut v = "あ".as_bytes()[..2].to_vec();
        u8_tail_fix(&mut v);
        assert!(v.is_empty());
        let mut v = "aあ".as_bytes().to_vec();
        u8_tail_fix(&mut v);
        assert_eq!(v, "aあ".as_bytes());
    }
}
