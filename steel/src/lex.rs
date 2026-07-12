// lex.rs — the tokenizer for both surfaces: lex_ascii (maximal munch) and lex_ja (spaced,
// grammar-first word resolution, postfix re-root, K_TGT deletion). Mirrors src/lex.c.
// Both skins are registry-blind; every diagnostic renders "line %d: %s", byte-exact.
// K_TGT and the JA post column are internal — define private buffer types here; they never
// reach the public Toks.

use crate::{ANO_NAMESZ, Diag, Interner, Symbol, TokKind, Toks};

// Internal token kind: a public TokKind or the に target marker, deleted in JA
// normalization step 2 and never returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BK {
    T(TokKind),
    Tgt,
}

const fn t(k: TokKind) -> BK {
    BK::T(k)
}

// Growing token columns; post marks JA postfix operators, internal only.
// Invariant: all five columns grow together.
struct TokBuf {
    kind: Vec<BK>,
    name: Vec<Symbol>,
    num: Vec<f64>,
    line: Vec<i32>,
    post: Vec<bool>,
}

impl TokBuf {
    fn new() -> TokBuf {
        TokBuf { kind: Vec::new(), name: Vec::new(), num: Vec::new(), line: Vec::new(), post: Vec::new() }
    }

    // Inputs: kind, line. Output: index of the appended token (name EMPTY, num 0, post false).
    fn push(&mut self, k: BK, line: i32) -> usize {
        let ix = self.kind.len();
        self.kind.push(k);
        self.name.push(Symbol::EMPTY);
        self.num.push(0.0);
        self.line.push(line);
        self.post.push(false);
        ix
    }

    fn truncate(&mut self, n: usize) {
        self.kind.truncate(n);
        self.name.truncate(n);
        self.num.truncate(n);
        self.line.truncate(n);
        self.post.truncate(n);
    }
}

// Inputs: line, message. Output: the refusal "line %d: %s".
fn lex_err(line: i32, msg: impl std::fmt::Display) -> Diag {
    Diag::refuse(format!("line {}: {}", line, msg))
}

/* char classes, ASCII only, locale-free */
fn nstart(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_lowercase()
}
fn nchar(c: u8) -> bool {
    nstart(c) || c.is_ascii_digit() || c == b'_'
}
fn dig(c: u8) -> bool {
    c.is_ascii_digit()
}

// Inputs: codepoint >= U+0080. Output: true when it is never an identifier char — the
// whitespace/numeral machinery (U+3000 ideographic space, U+3001 、, U+30FB ・) and the
// retired generator glyph U+2195 ↕, kept out so a stale ↕ errors by codepoint.
fn ublack(cp: u32) -> bool {
    cp == 0x3000 || cp == 0x3001 || cp == 0x30FB || cp == 0x2195
}

// Inputs: s at a UTF-8 char boundary, nonempty. Output: (codepoint, byte length 1-4), None
// malformed. Strict (anoptic utf8_decode): rejects overlongs, encoded surrogates,
// cp > U+10FFFF, and truncation — decoder and validator in one.
fn ucp(s: &[u8]) -> Option<(u32, usize)> {
    let b0 = *s.first()?;
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
        return None; // continuation or F8-FF lead
    };
    if s.len() - 1 < need {
        return None; // truncated at end
    }
    for k in 1..=need {
        if (s[k] & 0xC0) != 0x80 {
            return None;
        }
        r = (r << 6) | (s[k] & 0x3F) as u32;
    }
    if r < min || r > 0x10FFFF || (0xD800..=0xDFFF).contains(&r) {
        return None;
    }
    Some((r, 1 + need))
}

// Inputs: src (validated UTF-8), index i inside an identifier. Output: index one past its
// last char — ASCII [A-Za-z0-9_] plus any non-blacklisted codepoint >= U+0080; maximal munch
// stops at ASCII operator bytes and blacklist.
fn nspan(src: &[u8], mut i: usize) -> usize {
    loop {
        let c = if i < src.len() { src[i] } else { 0 };
        if nchar(c) {
            i += 1;
            continue;
        }
        if c >= 0x80 {
            if let Some((cp, l)) = ucp(&src[i..]) {
                if !ublack(cp) {
                    i += l;
                    continue;
                }
            }
        }
        return i;
    }
}

// Inputs: src (validated UTF-8), index i. Output: byte length of an identifier-start char
// at i — ASCII nstart or a non-blacklisted codepoint >= U+0080 — else 0. The `:` and `^`
// sigils use it so a symbol or alias name may be UTF-8 (:山賊).
fn nstart_span(src: &[u8], i: usize) -> usize {
    let c = if i < src.len() { src[i] } else { 0 };
    if nstart(c) {
        return 1;
    }
    if c >= 0x80 {
        if let Some((cp, l)) = ucp(&src[i..]) {
            if !ublack(cp) {
                return l;
            }
        }
    }
    0
}

// Inputs: a lexed name. Output: its keyword kind, or None when not one.
fn kwkind(nm: &str) -> Option<TokKind> {
    Some(match nm {
        "def" => TokKind::Def,
        "spawn" => TokKind::Spawn,
        "at" => TokKind::AtKw,
        "to" => TokKind::To,
        "via" => TokKind::Via,
        "along" => TokKind::Along,
        "order" => TokKind::Order,
        "by" => TokKind::By,
        "take" => TokKind::Take,
        "desc" => TokKind::Desc,
        "top" => TokKind::Top,
        "grade" => TokKind::Grade,
        "fold" => TokKind::FoldKw,
        "scan" => TokKind::ScanKw,
        "scan2" => TokKind::Scan2,
        "cross" => TokKind::Cross,
        "expand" => TokKind::Expand,
        "til" => TokKind::Iota,
        _ => return None,
    })
}

// Inputs: source (directives already blanked), token buffer, interner. Output: tokens
// appended, T_NL between nonempty lines, no trailing NL and no EOF.
// Invariants: folds/scans fused with no interior whitespace; NAME+'/' and NAME+'\'
// fuse for any non-keyword name (resolution at emit), never before '=';
// ^ begins the alias sigil, @ is always T_AT.
fn lex_ascii(s: &str, b: &mut TokBuf, it: &mut Interner) -> Result<(), Diag> {
    let src = s.as_bytes();
    let n = src.len();
    let at = |i: usize| -> u8 {
        if i < n { src[i] } else { 0 }
    };
    let mut line: i32 = 1;
    let mut i = 0usize;
    while i < n {
        let c = src[i];
        if c == b'\n' {
            if !b.kind.is_empty() && *b.kind.last().unwrap() != t(TokKind::Nl) {
                b.push(t(TokKind::Nl), line);
            }
            line += 1;
            i += 1;
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            i += 1;
            continue;
        }
        if c == b'-' && at(i + 1) == b'-' {
            while i < n && src[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if nstart(c) {
            let mut j = nspan(src, i + 1);
            let nm = it.intern(&s[i..j]);
            let kk = kwkind(&s[i..j]);
            // reducer fold/scan: name/ name\ glued — max/ min/ avg/ or any reducer name,
            // registry-blind (resolution at emit); 'max/= 2' stays SLASHEQ; keywords stay keywords
            if kk.is_none() && at(j) == b'/' && at(j + 1) != b'=' {
                let ix = b.push(t(TokKind::Fold), line);
                b.name[ix] = nm;
                i = j + 1;
                continue;
            }
            if kk.is_none() && at(j) == b'\\' {
                let ix = b.push(t(TokKind::ScanOp), line);
                b.name[ix] = nm;
                i = j + 1;
                continue;
            }
            if let Some(k) = kk {
                b.push(t(k), line);
                i = j;
                continue;
            }
            let ix = b.push(t(TokKind::Name), line);
            b.name[ix] = nm;
            if at(j) == b'\'' {
                b.push(t(TokKind::Tick), line); // postfix tick
                j += 1;
            }
            i = j;
            continue;
        }
        if dig(c) {
            let mut j = i;
            while dig(at(j)) {
                j += 1;
            }
            if at(j) == b'.' && dig(at(j + 1)) {
                j += 1;
                while dig(at(j)) {
                    j += 1;
                }
            }
            if j - i >= 64 {
                return Err(lex_err(line, "number too long"));
            }
            let v: f64 = s[i..j].parse().unwrap_or(0.0);
            if nstart(at(j)) {
                // counter: 3mo
                let mut k = j + 1;
                while nchar(at(k)) {
                    k += 1;
                }
                let ix = b.push(t(TokKind::Counter), line);
                b.num[ix] = v;
                b.name[ix] = it.intern(&s[j..k]);
                i = k;
            } else {
                let ix = b.push(t(TokKind::Num), line);
                b.num[ix] = v;
                i = j;
            }
            continue;
        }
        if c == b'"' {
            let mut j = i + 1;
            while j < n && src[j] != b'"' && src[j] != b'\n' {
                j += 1;
            }
            if at(j) != b'"' {
                return Err(lex_err(line, "unterminated string"));
            }
            let ix = b.push(t(TokKind::Str), line);
            b.name[ix] = it.intern(&s[i + 1..j]);
            i = j + 1;
            continue;
        }
        if c == b':' {
            let st = nstart_span(src, i + 1);
            if st == 0 {
                return Err(lex_err(line, "':' needs a name: symbols are :Name"));
            }
            let j = nspan(src, i + 1 + st);
            let ix = b.push(t(TokKind::Sym), line);
            b.name[ix] = it.intern(&s[i + 1..j]);
            i = j;
            continue;
        }
        if c == b'_' {
            if nchar(at(i + 1)) {
                return Err(lex_err(line, "names cannot start with '_'"));
            }
            b.push(t(TokKind::Wild), line);
            i += 1;
            continue;
        }
        if c >= 0x80 {
            // UTF-8 identifier
            let (cp, l) = match ucp(&src[i..]) {
                Some(x) => x,
                None => return Err(lex_err(line, "malformed UTF-8")),
            };
            if ublack(cp) {
                return Err(lex_err(line, format!("unknown character U+{:04X}", cp)));
            }
            let mut j = nspan(src, i + l);
            let nm = it.intern(&s[i..j]);
            // reducer fold/scan on a UTF-8 name: 脅威/ 脅威\ fuse exactly as ASCII names do
            if at(j) == b'/' && at(j + 1) != b'=' {
                let ix = b.push(t(TokKind::Fold), line);
                b.name[ix] = nm;
                i = j + 1;
                continue;
            }
            if at(j) == b'\\' {
                let ix = b.push(t(TokKind::ScanOp), line);
                b.name[ix] = nm;
                i = j + 1;
                continue;
            }
            let ix = b.push(t(TokKind::Name), line);
            b.name[ix] = nm;
            if at(j) == b'\'' {
                b.push(t(TokKind::Tick), line); // postfix tick
                j += 1;
            }
            i = j;
            continue;
        }
        let d = at(i + 1);
        match c {
            b',' => {
                b.push(t(TokKind::Comma), line);
                i += 1;
            }
            b';' => {
                b.push(t(TokKind::Semi), line);
                i += 1;
            }
            b'(' => {
                b.push(t(TokKind::Lp), line);
                i += 1;
            }
            b')' => {
                b.push(t(TokKind::Rp), line);
                i += 1;
            }
            b'[' => {
                b.push(t(TokKind::Lb), line);
                i += 1;
            }
            b']' => {
                b.push(t(TokKind::Rb), line);
                i += 1;
            }
            b'~' => {
                b.push(t(TokKind::Tilde), line);
                i += 1;
            }
            b'.' => {
                b.push(t(TokKind::Dot), line);
                i += 1;
            }
            b'%' => {
                b.push(t(TokKind::Pct), line);
                i += 1;
            }
            b'=' => {
                if d == b'>' {
                    b.push(t(TokKind::Arrow), line);
                    i += 2;
                } else if d == b'=' {
                    b.push(t(TokKind::EqEq), line);
                    i += 2;
                } else {
                    b.push(t(TokKind::Eq), line);
                    i += 1;
                }
            }
            b'!' => {
                if d == b'=' {
                    b.push(t(TokKind::Ne), line);
                    i += 2;
                } else {
                    b.push(t(TokKind::Bang), line);
                    i += 1;
                }
            }
            b'<' => {
                if d == b'=' {
                    b.push(t(TokKind::Le), line);
                    i += 2;
                } else if d == b'-' {
                    b.push(t(TokKind::LArrow), line);
                    i += 2;
                } else {
                    b.push(t(TokKind::Lt), line);
                    i += 1;
                }
            }
            b'>' => {
                if d == b'=' {
                    b.push(t(TokKind::Ge), line);
                    i += 2;
                } else {
                    b.push(t(TokKind::Gt), line);
                    i += 1;
                }
            }
            b'|' => {
                if d == b'>' {
                    b.push(t(TokKind::PipeGt), line);
                    i += 2;
                } else if d == b'/' {
                    let ix = b.push(t(TokKind::Fold), line);
                    b.name[ix] = it.intern("|");
                    i += 2;
                } else if d == b'\\' {
                    let ix = b.push(t(TokKind::ScanOp), line);
                    b.name[ix] = it.intern("|");
                    i += 2;
                } else {
                    b.push(t(TokKind::Bar), line);
                    i += 1;
                }
            }
            b'&' => {
                if d == b'/' {
                    let ix = b.push(t(TokKind::Fold), line);
                    b.name[ix] = it.intern("&");
                    i += 2;
                } else if d == b'\\' {
                    let ix = b.push(t(TokKind::ScanOp), line);
                    b.name[ix] = it.intern("&");
                    i += 2;
                } else {
                    b.push(t(TokKind::Amp), line);
                    i += 1;
                }
            }
            b'+' => {
                if d == b'=' {
                    b.push(t(TokKind::PlusEq), line);
                    i += 2;
                } else if d == b'/' {
                    let ix = b.push(t(TokKind::Fold), line);
                    b.name[ix] = it.intern("+");
                    i += 2;
                } else if d == b'\\' {
                    let ix = b.push(t(TokKind::ScanOp), line);
                    b.name[ix] = it.intern("+");
                    i += 2;
                } else {
                    b.push(t(TokKind::Plus), line);
                    i += 1;
                }
            }
            b'-' => {
                if d == b'=' {
                    b.push(t(TokKind::MinusEq), line);
                    i += 2;
                } else {
                    b.push(t(TokKind::Minus), line);
                    i += 1;
                }
            }
            b'*' => {
                if d == b'=' {
                    b.push(t(TokKind::StarEq), line);
                    i += 2;
                } else if d == b'/' {
                    let ix = b.push(t(TokKind::Fold), line);
                    b.name[ix] = it.intern("*");
                    i += 2;
                } else if d == b'\\' {
                    let ix = b.push(t(TokKind::ScanOp), line);
                    b.name[ix] = it.intern("*");
                    i += 2;
                } else {
                    b.push(t(TokKind::Star), line);
                    i += 1;
                }
            }
            b'/' => {
                if d == b'=' {
                    b.push(t(TokKind::SlashEq), line);
                    i += 2;
                } else {
                    b.push(t(TokKind::Slash), line);
                    i += 1;
                }
            }
            b'#' => {
                if d == b'/' {
                    let ix = b.push(t(TokKind::Fold), line);
                    b.name[ix] = it.intern("#");
                    i += 2;
                } else {
                    return Err(lex_err(line, "'#' begins only the fold '#/'"));
                }
            }
            b'@' => {
                b.push(t(TokKind::At), line);
                i += 1;
            }
            b'^' => {
                let st = nstart_span(src, i + 1); // ^alias sigil, the deictic pronoun
                if st != 0 {
                    let j = nspan(src, i + 1 + st);
                    let ix = b.push(t(TokKind::Alias), line);
                    b.name[ix] = it.intern(&s[i + 1..j]);
                    i = j;
                } else {
                    return Err(lex_err(line, "'^' begins only the ^alias sigil"));
                }
            }
            b'\'' => return Err(lex_err(line, "stray tick: ' is postfix on a name")),
            b'\\' => return Err(lex_err(line, "stray '\\': scans are +\\ *\\ &\\ |\\ or name\\ glued")),
            _ => return Err(lex_err(line, format!("unknown byte 0x{:02X}", c))),
        }
    }
    Ok(())
}

// Inputs: codepoint. Output: digit value 0-9 — kanji 〇一..九 or fullwidth ０-９ — or -1.
fn jadig(cp: u32) -> i32 {
    if (0xFF10..=0xFF19).contains(&cp) {
        return (cp - 0xFF10) as i32;
    }
    match cp {
        0x3007 => 0,
        0x4E00 => 1,
        0x4E8C => 2,
        0x4E09 => 3,
        0x56DB => 4,
        0x4E94 => 5,
        0x516D => 6,
        0x4E03 => 7,
        0x516B => 8,
        0x4E5D => 9,
        _ => -1,
    }
}

// Inputs: codepoint. Output: magnitude 10/100/1000/10000, or 0.
fn jamag(cp: u32) -> i32 {
    match cp {
        0x5341 => 10,
        0x767E => 100,
        0x5343 => 1000,
        0x4E07 => 10000,
        _ => 0,
    }
}

// Inputs: w, a whole word. Output: Some((value, unit "" | "mo")) when w is a numeral —
// Arabic digits, kanji named magnitudes (六十, 九千九百九十九, bare 千 = 1000), digit-string
// decimal with 〇 and ・ (一・〇五), optional ヶ月 counter suffix (三ヶ月 = 3 "mo") — else None.
fn ja_numeral(w: &str) -> Option<(f64, &'static str)> {
    let bytes = w.as_bytes();
    let mut len = bytes.len();
    if len == 0 || len >= 128 {
        return None;
    }
    let mut unit = "";
    if len > 6 && &bytes[len - 6..len] == b"\xE3\x83\xB6\xE6\x9C\x88" {
        // ヶ月
        len -= 6;
        unit = "mo";
    }
    let buf = &bytes[..len];
    if dig(buf[0]) {
        // Arabic digits
        let mut p = 0usize;
        while p < len && dig(buf[p]) {
            p += 1;
        }
        if p < len && buf[p] == b'.' {
            p += 1;
            if p >= len || !dig(buf[p]) {
                return None;
            }
            while p < len && dig(buf[p]) {
                p += 1;
            }
        }
        if p != len {
            return None;
        }
        let v: f64 = w[..len].parse().ok()?;
        return Some((v, unit));
    }
    // decode and classify: ty 0 = digit, 1 = magnitude, 2 = ・
    let mut dv = [0i32; 32];
    let mut ty = [0u8; 32];
    let mut nn = 0usize;
    let mut p = 0usize;
    while p < len {
        let (cp, l) = ucp(&buf[p..])?;
        if nn >= 32 {
            return None;
        }
        p += l;
        let d = jadig(cp);
        let m = jamag(cp);
        if d >= 0 {
            ty[nn] = 0;
            dv[nn] = d;
        } else if m != 0 {
            ty[nn] = 1;
            dv[nn] = m;
        } else if cp == 0x30FB {
            ty[nn] = 2;
            dv[nn] = 0;
        } else {
            return None;
        }
        nn += 1;
    }
    let mut hasdot = false;
    let mut hasmag = false;
    for k in 0..nn {
        hasdot |= ty[k] == 2;
        hasmag |= ty[k] == 1;
    }
    let mut v = 0f64;
    if hasdot {
        // 一・〇五 = 1.05
        if hasmag || ty[0] == 2 {
            return None;
        }
        let mut k = 0usize;
        while k < nn && ty[k] == 0 {
            v = v * 10.0 + dv[k] as f64;
            k += 1;
        }
        if k >= nn || ty[k] != 2 || k + 1 >= nn {
            return None;
        }
        let mut sc = 0.1f64;
        k += 1;
        while k < nn {
            if ty[k] != 0 {
                return None;
            }
            v += dv[k] as f64 * sc;
            sc /= 10.0;
            k += 1;
        }
    } else if hasmag {
        // 九千九百九十九 = 9999
        let mut sect = 0f64;
        let mut cur = 0f64;
        let mut curset = false;
        for k in 0..nn {
            if ty[k] == 0 {
                cur = dv[k] as f64;
                curset = true;
            } else if dv[k] == 10000 {
                sect += cur;
                v += (if sect > 0.0 { sect } else { 1.0 }) * 10000.0;
                sect = 0.0;
                cur = 0.0;
                curset = false;
            } else {
                sect += (if curset { cur } else { 1.0 }) * dv[k] as f64;
                cur = 0.0;
                curset = false;
            }
        }
        v += sect + cur;
    } else {
        // positional: 三 = 3
        for k in 0..nn {
            v = v * 10.0 + dv[k] as f64;
        }
    }
    Some((v, unit))
}

// particle/verb/keyword table; post marks operators the surface puts after their
// operand (re-rooted before it in normalization); the 4th column is the op payload for
// folds/scans ("" = none, interns to Symbol::EMPTY).
const JATAB: &[(&str, BK, bool, &str)] = &[
    /* structural particles */
    ("と", t(TokKind::Amp), false, ""),
    ("か", t(TokKind::Bar), false, ""),
    ("の", t(TokKind::Dot), false, ""),
    ("で", t(TokKind::At), true, ""),
    ("、", t(TokKind::Comma), false, ""),
    ("が", t(TokKind::Comma), false, ""),
    ("は", t(TokKind::Comma), false, ""),
    ("に", BK::Tgt, false, ""),
    /* comparisons (postfix on the comparand) */
    ("より", t(TokKind::Gt), true, ""),
    ("超", t(TokKind::Gt), true, ""),
    ("未満", t(TokKind::Lt), true, ""),
    ("同", t(TokKind::EqEq), true, ""),
    ("以上", t(TokKind::Ge), true, ""),
    ("以下", t(TokKind::Le), true, ""),
    ("不同", t(TokKind::Ne), true, ""),
    ("ない", t(TokKind::Bang), true, ""),
    /* assignment family (postfix; に marks the target) */
    ("たす", t(TokKind::PlusEq), true, ""),
    ("ひく", t(TokKind::MinusEq), true, ""),
    ("かける", t(TokKind::StarEq), true, ""),
    ("わる", t(TokKind::SlashEq), true, ""),
    ("にする", t(TokKind::Eq), true, ""),
    /* presence writes (postfix on the component), despawn, sequencing, rule/def hinge */
    ("付", t(TokKind::Plus), true, ""),
    ("除", t(TokKind::Minus), true, ""),
    ("消", t(TokKind::Tilde), false, ""),
    ("て", t(TokKind::Semi), false, ""),
    ("なる", t(TokKind::Arrow), false, ""),
    /* folds (prefix, op payload) */
    ("総和", t(TokKind::Fold), false, "+"),
    ("総積", t(TokKind::Fold), false, "*"),
    ("総数", t(TokKind::Fold), false, "#"),
    ("最大", t(TokKind::Fold), false, "max"),
    ("最小", t(TokKind::Fold), false, "min"),
    ("平均", t(TokKind::Fold), false, "avg"),
    ("皆", t(TokKind::Fold), false, "&"),
    ("或", t(TokKind::Fold), false, "|"),
    /* scans (prefix, op payload) */
    ("累和", t(TokKind::ScanOp), false, "+"),
    ("累積", t(TokKind::ScanOp), false, "*"),
    ("累大", t(TokKind::ScanOp), false, "max"),
    ("累皆", t(TokKind::ScanOp), false, "&"),
    ("累或", t(TokKind::ScanOp), false, "|"),
    /* the generator (prefix): ASCII spells it til */
    ("連番", t(TokKind::Iota), false, ""),
    /* system nouns, global: payload is the resolution-level name; a registry entry of
     * that name wins at emit (the !find guards), exactly as it does on the ASCII surface */
    ("前", t(TokKind::Name), false, "prev"),
    ("行", t(TokKind::Name), false, "row"),
    ("番号", t(TokKind::Name), false, "index"),
    ("字", t(TokKind::Name), false, "char"),
    /* keywords */
    ("定義", t(TokKind::Def), false, ""),
    ("生成", t(TokKind::Spawn), false, ""),
    ("於", t(TokKind::AtKw), false, ""),
    ("至", t(TokKind::To), false, ""),
    ("経由", t(TokKind::Via), false, ""),
    ("沿", t(TokKind::Along), false, ""),
    ("整列", t(TokKind::Order), false, ""),
    ("別", t(TokKind::By), false, ""),
    ("取", t(TokKind::Take), false, ""),
    ("降順", t(TokKind::Desc), false, ""),
    ("上位", t(TokKind::Top), false, ""),
    ("格付", t(TokKind::Grade), false, ""),
    ("縮約", t(TokKind::FoldKw), false, ""),
    ("走査", t(TokKind::ScanKw), false, ""),
    ("二重走査", t(TokKind::Scan2), false, ""),
    ("交差", t(TokKind::Cross), false, ""),
    ("展開", t(TokKind::Expand), false, ""),
    /* ASCII structural glyphs, usable directly in JA source */
    ("(", t(TokKind::Lp), false, ""),
    (")", t(TokKind::Rp), false, ""),
    ("[", t(TokKind::Lb), false, ""),
    ("]", t(TokKind::Rb), false, ""),
    (";", t(TokKind::Semi), false, ""),
    ("<-", t(TokKind::LArrow), false, ""),
    ("|>", t(TokKind::PipeGt), false, ""),
    ("'", t(TokKind::Tick), false, ""),
    ("_", t(TokKind::Wild), false, ""),
    ("+", t(TokKind::Plus), false, ""),
    ("-", t(TokKind::Minus), false, ""),
    ("*", t(TokKind::Star), false, ""),
    ("/", t(TokKind::Slash), false, ""),
    ("%", t(TokKind::Pct), false, ""),
    ("=", t(TokKind::Eq), false, ""),
    ("|", t(TokKind::Bar), false, ""),
];

// Inputs: a whole word. Output: true when the closed grammar owns it on either surface —
// kwkind keywords (til included), the fused reducers max/min/avg, every jatab word (ASCII
// glyph rows included: "(", "+", "_", "'", "<-", "|>", ...), every JA numeral (kanji,
// fullwidth, Arabic digit strings, counters). EXACT-BYTE — parse consults this at def heads.
pub fn lex_reserved(w: &str) -> bool {
    if kwkind(w).is_some() {
        return true;
    }
    if w == "max" || w == "min" || w == "avg" {
        return true;
    }
    if JATAB.iter().any(|r| r.0 == w) {
        return true;
    }
    ja_numeral(w).is_some()
}

// Inputs: a whole word. Output: lex_reserved(w), else fold ASCII A-Z to a-z (>= 256 bytes:
// false, longer than any reserved word; at least one byte must actually fold) and re-check.
// Non-ASCII bytes copy exact. Registry loader only; program-level names stay exact-byte.
pub fn lex_reserved_fold(w: &str) -> bool {
    if lex_reserved(w) {
        return true;
    }
    if w.len() >= ANO_NAMESZ {
        return false; // longer than any name slot, so than any reserved word
    }
    let mut folded = false;
    let f: String = w
        .chars()
        .map(|c| {
            if c.is_ascii_uppercase() {
                folded = true;
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
        .collect();
    folded && lex_reserved(&f)
}

// Inputs: a whole word (valid UTF-8). Output: true when it is a legal identifier: nstart or
// a non-blacklisted codepoint >= U+0080 first, nchar or the same after.
fn word_name(w: &str) -> bool {
    let b = w.as_bytes();
    let n = b.len();
    let mut i = 0usize;
    let mut first = true;
    while i < n {
        let c = b[i];
        if c < 0x80 {
            if !(if first { nstart(c) } else { nchar(c) }) {
                return false;
            }
            i += 1;
        } else {
            match ucp(&b[i..]) {
                Some((cp, l)) if !ublack(cp) => i += l,
                _ => return false,
            }
        }
        first = false;
    }
    n > 0
}

// Inputs: token columns, index j of an operand's last token. Output: index of that
// primary's first token — a matched (…)/[…] group (with a leading callee name and a
// postfix tick folded in), else the atom at j. Invariant: never crosses T_NL or 0.
fn grab_primary(b: &TokBuf, j: i32) -> i32 {
    if j < 0 || b.kind[j as usize] == t(TokKind::Nl) {
        return j;
    }
    let kj = b.kind[j as usize];
    if kj == t(TokKind::Rp) || kj == t(TokKind::Rb) {
        let (open, close) = if kj == t(TokKind::Rp) {
            (t(TokKind::Lp), t(TokKind::Rp))
        } else {
            (t(TokKind::Lb), t(TokKind::Rb))
        };
        let mut depth = 0i32;
        let mut o = j;
        while o >= 0 && b.kind[o as usize] != t(TokKind::Nl) {
            if b.kind[o as usize] == close {
                depth += 1;
            } else if b.kind[o as usize] == open {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            o -= 1;
        }
        if o < 0 || b.kind[o as usize] != open {
            return j; // unbalanced: bail
        }
        let mut o = o;
        if o > 0 && (b.kind[(o - 1) as usize] == t(TokKind::Name) || b.kind[(o - 1) as usize] == t(TokKind::Alias)) {
            o -= 1; // callee
        }
        return o;
    }
    if kj == t(TokKind::Tick) && j > 0 && b.kind[(j - 1) as usize] == t(TokKind::Name) {
        return j - 1;
    }
    j // single atom
}

// Inputs: token columns, index k of a postfix operator. Output: index where the operator
// re-roots — the start of the primary ending at k-1, extended left over hop chains
// (a.b.c), a numeric/wildcard shape run (8 8, 4 _), and a leading `to`. K_TGT (に) is
// still present and barriers an assignment target from the callee grab.
fn operand_start(b: &TokBuf, k: i32) -> i32 {
    let numwild = |x: i32| {
        let kk = b.kind[x as usize];
        kk == t(TokKind::Num) || kk == t(TokKind::Wild)
    };
    let mut j = grab_primary(b, k - 1);
    loop {
        if j >= 2 && b.kind[(j - 1) as usize] == t(TokKind::Dot) {
            j = grab_primary(b, j - 2);
            continue;
        }
        if j >= 1 && numwild(j) && numwild(j - 1) {
            j -= 1;
            continue;
        }
        if j >= 1 && b.kind[(j - 1) as usize] == t(TokKind::To) {
            j -= 1;
            break;
        }
        break;
    }
    j
}

// Inputs: source, token buffer, interner. Output: the normalized ASCII-equivalent
// stream, T_NL between nonempty lines, no trailing NL. Per word, in order: ^alias /
// :sym sigils; the closed grammar — particle, keyword, and fold/scan table (with op
// payload); kanji/Arabic numeral; a fused reducer word (name/ name\); then any legal
// identifier, ASCII or UTF-8, as a keyword via kwkind or T_NAME carrying its surface
// spelling (resolution against the registry happens at emit, never here); "strings";
// else error. Then normalization: re-root each postfix operator before its operand
// (span-aware), then delete the fused K_TGT markers. Invariant: K_TGT and post flags
// never survive this function.
fn lex_ja(s: &str, b: &mut TokBuf, it: &mut Interner) -> Result<(), Diag> {
    let src = s.as_bytes();
    let n = src.len();
    let at = |i: usize| -> u8 {
        if i < n { src[i] } else { 0 }
    };
    let mut line: i32 = 1;
    let mut i = 0usize;
    while i < n {
        let c = src[i];
        if c == b'\n' {
            if !b.kind.is_empty() && *b.kind.last().unwrap() != t(TokKind::Nl) {
                b.push(t(TokKind::Nl), line);
            }
            line += 1;
            i += 1;
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            i += 1;
            continue;
        }
        if c >= 0x80 {
            let (cp, l) = match ucp(&src[i..]) {
                Some(x) => x,
                None => return Err(lex_err(line, "malformed UTF-8")),
            };
            if cp == 0x3000 {
                i += l; // ideographic space
                continue;
            }
        }
        if c == b'-' && at(i + 1) == b'-' {
            while i < n && src[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'"' {
            // ASCII string, may hold spaces
            let mut j = i + 1;
            while j < n && src[j] != b'"' && src[j] != b'\n' {
                j += 1;
            }
            if at(j) != b'"' {
                return Err(lex_err(line, "unterminated string"));
            }
            let ix = b.push(t(TokKind::Str), line);
            b.name[ix] = it.intern(&s[i + 1..j]);
            i = j + 1;
            continue;
        }
        // word: run to the next space/newline (ASCII or U+3000)
        let mut j = i;
        while j < n {
            let d = src[j];
            if d == b' ' || d == b'\t' || d == b'\r' || d == b'\n' {
                break;
            }
            if d >= 0x80 {
                let (cp, l) = match ucp(&src[j..]) {
                    Some(x) => x,
                    None => return Err(lex_err(line, "malformed UTF-8")),
                };
                if cp == 0x3000 {
                    break;
                }
                j += l;
                continue;
            }
            j += 1;
        }
        if j - i >= 128 {
            return Err(lex_err(line, "word too long"));
        }
        let w = &s[i..j];
        i = j;
        // sigils: ^alias, :symbol (identifiers, not particles — name may be UTF-8, :山賊)
        if let Some(rest) = w.strip_prefix('^') {
            if word_name(rest) {
                let ix = b.push(t(TokKind::Alias), line);
                b.name[ix] = it.intern(rest);
                continue;
            }
        }
        if let Some(rest) = w.strip_prefix(':') {
            if word_name(rest) {
                let ix = b.push(t(TokKind::Sym), line);
                b.name[ix] = it.intern(rest);
                continue;
            }
        }
        if let Some(&(_, kind, post, nm)) = JATAB.iter().find(|r| r.0 == w) {
            let ix = b.push(kind, line);
            b.post[ix] = post;
            b.name[ix] = it.intern(nm);
            continue;
        }
        if let Some((v, u)) = ja_numeral(w) {
            let ix = b.push(t(if u.is_empty() { TokKind::Num } else { TokKind::Counter }), line);
            b.num[ix] = v;
            if !u.is_empty() {
                b.name[ix] = it.intern(u);
            }
            continue;
        }
        // fused reducer words: name/ name\ — the ASCII fold/scan fusion as one word (脅威/)
        {
            let wb = w.as_bytes();
            let wl = wb.len();
            if wl > 1 && (wb[wl - 1] == b'/' || wb[wl - 1] == b'\\') {
                let stem = &w[..wl - 1];
                if word_name(stem) {
                    let ix = b.push(t(if wb[wl - 1] == b'/' { TokKind::Fold } else { TokKind::ScanOp }), line);
                    b.name[ix] = it.intern(stem);
                    continue;
                }
            }
        }
        // identifier, ASCII or UTF-8: keyword, else a name by its surface spelling
        if word_name(w) {
            let nm = it.intern(w);
            match kwkind(w) {
                Some(k) => {
                    b.push(t(k), line);
                }
                None => {
                    let ix = b.push(t(TokKind::Name), line);
                    b.name[ix] = nm;
                }
            }
            continue;
        }
        return Err(lex_err(line, format!("unknown word '{}'", w)));
    }
    // normalize, step 1: re-root each postfix operator before its operand span. K_TGT is
    // still present so an assignment target (Col に …) is not grabbed as a call callee.
    let mut k = 0usize;
    while k < b.kind.len() {
        if !b.post[k] {
            k += 1;
            continue;
        }
        if k == 0 || b.kind[k - 1] == t(TokKind::Nl) {
            return Err(lex_err(b.line[k], "postfix operator with no operand"));
        }
        let start = operand_start(b, k as i32) as usize;
        let (ok, on, ov, ol) = (b.kind[k], b.name[k], b.num[k], b.line[k]);
        let mut m = k;
        while m > start {
            b.kind[m] = b.kind[m - 1];
            b.name[m] = b.name[m - 1];
            b.num[m] = b.num[m - 1];
            b.line[m] = b.line[m - 1];
            b.post[m] = b.post[m - 1];
            m -= 1;
        }
        b.kind[start] = ok;
        b.name[start] = on;
        b.num[start] = ov;
        b.line[start] = ol;
        b.post[start] = false;
        k += 1;
    }
    // step 2: delete the fused TGT markers (compact every column)
    let mut m = 0usize;
    for k in 0..b.kind.len() {
        if b.kind[k] == BK::Tgt {
            continue;
        }
        b.kind[m] = b.kind[k];
        b.name[m] = b.name[k];
        b.num[m] = b.num[k];
        b.line[m] = b.line[k];
        b.post[m] = b.post[k];
        m += 1;
    }
    b.truncate(m);
    Ok(())
}

// Inputs: raw source bytes (directives already blanked to spaces by main — line numbers
// hold; NUL-truncation already applied), ja flag, interner. Output: token columns ending in
// exactly ONE Eof carrying the last token's line (line 1 when empty); Nl between nonempty
// lines only — never leading, doubled, or trailing.
// Invariants: strict UTF-8 validated HERE, once, over the whole buffer, before either skin
// runs — "malformed UTF-8" at the physical line (1 + newlines before the failure); name
// column is Symbol::EMPTY when absent; glyph-fold payloads ("+","*","&","|","#") intern like
// any spelling; the ASCII/JA fused-reducer divergence (til/) is kept, not repaired.
pub fn lex(src: &[u8], ja: bool, it: &mut Interner) -> Result<Toks, Diag> {
    // C sees a NUL-terminated buffer; keep the bytes before the first NUL.
    let src = match src.iter().position(|&x| x == 0) {
        Some(p) => &src[..p],
        None => src,
    };
    // strict well-formedness once at the boundary; skins then decode unchecked
    {
        let n = src.len();
        let mut pl: i32 = 1;
        let mut i = 0usize;
        while i < n {
            let c = src[i];
            if c < 0x80 {
                if c == b'\n' {
                    pl += 1;
                }
                i += 1;
                continue;
            }
            match ucp(&src[i..]) {
                Some((_, l)) => i += l,
                None => return Err(lex_err(pl, "malformed UTF-8")),
            }
        }
    }
    let s = match std::str::from_utf8(src) {
        Ok(s) => s,
        Err(_) => return Err(lex_err(1, "malformed UTF-8")), // unreachable: ucp == str validity
    };
    let mut b = TokBuf::new();
    if ja {
        lex_ja(s, &mut b, it)?;
    } else {
        lex_ascii(s, &mut b, it)?;
    }
    if b.kind.last() == Some(&t(TokKind::Nl)) {
        let m = b.kind.len() - 1; // NL separates, never terminates
        b.truncate(m);
    }
    let line = b.line.last().copied().unwrap_or(1);
    b.push(t(TokKind::Eof), line);
    let mut toks = Toks::new();
    for k in 0..b.kind.len() {
        let kind = match b.kind[k] {
            BK::T(x) => x,
            BK::Tgt => unreachable!(),
        };
        toks.push(kind, b.name[k], b.num[k], b.line[k]);
    }
    Ok(toks)
}

// Differential tests against the frozen C oracle (src/anoc). Skipped when the oracle
// binary is absent. Replicates main.c's directive blanking and --tokens printing so the
// comparison is byte-exact end to end.
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    fn oracle() -> Option<PathBuf> {
        let p = repo().join("src/anoc");
        p.exists().then_some(p)
    }

    // C printf "%g" (precision 6): %e form when exp < -4 or >= 6, trailing zeros stripped,
    // exponent at least two digits with sign.
    fn fmt_g6(x: f64) -> String {
        if x == 0.0 {
            return if x.is_sign_negative() { "-0".into() } else { "0".into() };
        }
        if !x.is_finite() {
            return format!("{}", x); // tokens never carry non-finite
        }
        let e = format!("{:.5e}", x);
        let epos = e.find('e').unwrap();
        let exp: i32 = e[epos + 1..].parse().unwrap();
        if exp < -4 || exp >= 6 {
            let mant = e[..epos].trim_end_matches('0').trim_end_matches('.');
            format!("{}e{}{:02}", mant, if exp < 0 { "-" } else { "+" }, exp.abs())
        } else {
            let prec = (5 - exp).max(0) as usize;
            let f = format!("{:.*}", prec, x);
            if f.contains('.') {
                f.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                f
            }
        }
    }

    // main.c parse_directives: blank whole "--!" lines to spaces, return the ja flag.
    fn blank_directives(src: &[u8]) -> (Vec<u8>, bool) {
        let end = src.iter().position(|&b| b == 0).unwrap_or(src.len());
        let mut out = src[..end].to_vec();
        let mut ja = false;
        let mut i = 0usize;
        while i < out.len() {
            let eol = out[i..].iter().position(|&b| b == b'\n').map(|p| i + p);
            let len = eol.unwrap_or(out.len()) - i;
            let mut p = i;
            while p < i + len && (out[p] == b' ' || out[p] == b'\t') {
                p += 1;
            }
            if (p - i) + 3 <= len && &out[p..p + 3] == b"--!" {
                let mut txt = out[p + 3..i + len].to_vec();
                while matches!(txt.last(), Some(b'\r') | Some(b' ') | Some(b'\t')) {
                    txt.pop();
                }
                let txt = String::from_utf8_lossy(&txt).into_owned();
                if txt.split([' ', '\t']).find(|w| !w.is_empty()) == Some("ja") {
                    ja = true;
                }
                for b in &mut out[i..i + len] {
                    *b = b' ';
                }
            }
            match eol {
                Some(e) => i = e + 1,
                None => break,
            }
        }
        (out, ja)
    }

    fn my_tokens(src: &[u8]) -> Result<String, Diag> {
        let (blanked, ja) = blank_directives(src);
        let mut it = Interner::new();
        let toks = lex(&blanked, ja, &mut it)?;
        let mut out = String::new();
        for i in 0..toks.len() {
            out.push_str(&format!(
                "{} {} {}\n",
                toks.kind[i].c_name(),
                it.resolve(toks.name[i]),
                fmt_g6(toks.num[i])
            ));
        }
        Ok(out)
    }

    // Compare against the oracle for one file already on disk. Returns true when compared.
    fn diff_file(anoc: &Path, path: &Path) -> bool {
        let src = std::fs::read(path).unwrap();
        let out = Command::new(anoc).arg("--tokens").arg(path).current_dir(repo()).output().unwrap();
        let mine = my_tokens(&src);
        if out.status.code() == Some(0) {
            let want = String::from_utf8_lossy(&out.stdout).into_owned();
            match mine {
                Ok(got) => assert_eq!(got, want, "token stream diverges: {}", path.display()),
                Err(d) => panic!("oracle lexed {} but port refused: {}", path.display(), d.msg),
            }
            return true;
        }
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let pfx = format!("{}: line ", path.display());
        if stderr.starts_with(&pfx) {
            // lex refusal (parse never runs under --tokens)
            match mine {
                Err(d) => {
                    assert_eq!(out.status.code(), Some(2), "{}", path.display());
                    assert_eq!(stderr, format!("{}: {}\n", path.display(), d.msg), "refusal diverges: {}", path.display());
                }
                Ok(_) => panic!("oracle refused {} at lex but port accepted: {}", path.display(), stderr),
            }
            return true;
        }
        false // directive/registry refusal, out of lex scope
    }

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "ano") {
                out.push(p);
            }
        }
    }

    #[test]
    fn differential_demos() {
        let Some(anoc) = oracle() else { return };
        let mut files = Vec::new();
        walk(&repo().join("demos"), &mut files);
        files.sort();
        let mut compared = 0;
        for f in &files {
            if diff_file(&anoc, f) {
                compared += 1;
            }
        }
        assert!(compared > 200, "only {} of {} demos compared", compared, files.len());
    }

    #[test]
    fn differential_adversarial() {
        let Some(anoc) = oracle() else { return };
        let cases: &[&[u8]] = &[
            b"x' y\n",
            b"max/ 2\nmax/= 2\nmin\\ 2\n",
            b"til/ 8\n",
            b"a--b c\n",
            b"3mo 3mo2 0.5mo\n",
            "3\u{6708}\n".as_bytes(),
            b"1. .5 1.5e3\n",
            b"_ x _)\n",
            b"_x\n",
            b":a1 :Name\n",
            b": a\n",
            b":1\n",
            b":\n",
            b"#\n",
            b"# /\n",
            b"#/ gold\n",
            b"^\n",
            b"^a.b ^_\n",
            "^\u{4e16}\u{754c} :\u{5c71}\u{8cca}\n".as_bytes(),
            b"@ a@b\n",
            b"'\n",
            b"\\\n",
            b"name\\ name\\= a/=2\n",
            b"`\n",
            b"$\n",
            b"\x7f\n",
            b"|/ x\n|\\ x\n&/ x\n&\\ x\n+\\ x\n*/ x\n",
            b"a=>b a==b a!=b a<=b a<-b a>=b a|>b\n",
            b"-/ x\n",
            b"-= x\n",
            "\u{5317}&\u{4e21}\u{624b}\n".as_bytes(),
            "\u{8105}\u{5a01}/ 8\n".as_bytes(),
            "\u{8105}\u{5a01}\\ 8\n".as_bytes(),
            "\u{8105}\u{5a01}' 8\n".as_bytes(),
            "\u{2195}5\n".as_bytes(),
            "\u{3001}\n".as_bytes(),
            "\u{30fb}\n".as_bytes(),
            b"\"unterminated\n",
            b"\"\"\n\"a b\"\n",
            b"\"closes at eof",
            b"123456789012345678901234567890123456789012345678901234567890123 x\n",
            b"1234567890123456789012345678901234567890123456789012345678901234 x\n",
            b"9999999999999999999999 0.000001 1000000 123456.5\n",
            b"\n\n\nx\n\n\n",
            b"",
            b"-- only a comment\n",
            b"x",
            b"x'",
            b"max/",
            b"a.b.c' d\n",
            b"\xff\n",
            b"a\nb\xc0\xaf\n",
            b"\xed\xa0\x80\n",
            b"\xe3\x80\n",
            b"a \xf8 b\n",
            // JA skin
            "--! ja\n\u{4e09} \u{4e94}\u{5341} \u{4e09}\u{4e94}\u{5341}\n".as_bytes(),
            "--! ja\n\u{4e00}\u{30fb}\u{3007}\u{4e94}\n".as_bytes(),
            "--! ja\n\u{3007}\u{4e07}\n".as_bytes(),
            "--! ja\n\u{4e5d}\u{5343}\u{4e5d}\u{767e}\u{4e5d}\u{5341}\u{4e5d}\n".as_bytes(),
            "--! ja\n\u{5343}\n".as_bytes(),
            "--! ja\n\u{4e09}\u{30f6}\u{6708}\n".as_bytes(),
            "--! ja\n\u{30f6}\u{6708}\n".as_bytes(),
            "--! ja\n\u{ff10}\u{ff11}\u{ff12}\n".as_bytes(),
            b"--! ja\ntil/ 8\n",
            b"--! ja\ntil 8\n",
            "--! ja\nCol \u{306b} 5 \u{306b}\u{3059}\u{308b}\n".as_bytes(),
            "--! ja\n5 \u{3088}\u{308a}\n".as_bytes(),
            "--! ja\n\u{3088}\u{308a} 5\n".as_bytes(),
            "--! ja\nx\n\u{3067}\n".as_bytes(),
            "--! ja\n\u{3067} x\n".as_bytes(),
            "--! ja\na \u{306e} b \u{306e} c \u{3067} x\n".as_bytes(),
            "--! ja\nf ( x ) \u{3067} y\n".as_bytes(),
            "--! ja\n^w ( x ) \u{3067} y\n".as_bytes(),
            "--! ja\n( x \u{3067} y\n".as_bytes(),
            "--! ja\n8 8 \u{3067} x\n".as_bytes(),
            "--! ja\n\u{81f3} 8 8 \u{3067} x\n".as_bytes(),
            "--! ja\n4 _ \u{3067} x\n".as_bytes(),
            "--! ja\nx' \u{3067} y\n".as_bytes(),
            "--! ja\n\u{3001}x\n".as_bytes(),
            "--! ja\n-x\n".as_bytes(),
            "--! ja\nx--y\n".as_bytes(),
            "--! ja\n-- comment\nx\n".as_bytes(),
            "--! ja\n\"a b\" x\n".as_bytes(),
            "--! ja\n\"unterminated\n".as_bytes(),
            "--! ja\n\u{7dcf}\u{548c} gold \u{7d2f}\u{548c} gold \u{6700}\u{5927} gold\n".as_bytes(),
            "--! ja\n\u{8105}\u{5a01}/ 8 x\\ 8\n".as_bytes(),
            "--! ja\n3/ x\n".as_bytes(),
            "--! ja\n^ x\n".as_bytes(),
            "--! ja\n: x\n".as_bytes(),
            "--! ja\n\u{3000}x\u{3000}y\u{3000}\n".as_bytes(),
            "--! ja\nz \u{306b} ( f x ) \u{306b}\u{3059}\u{308b}\n".as_bytes(),
            "--! ja\na \u{3068} b \u{304b} c \u{3001} d \u{306a}\u{3044}\n".as_bytes(),
            "--! ja\ngold 100 \u{305f}\u{3059}\n".as_bytes(),
            "--! ja\nx ' \u{3067} y\n".as_bytes(),
            "--! ja\nx ) \u{3067} y\n".as_bytes(),
            "--! ja\n[ a \u{3001} b ] \u{3067} y\n".as_bytes(),
            b"3mo",
            b"5",
            b":name",
            b"^name",
            b"name\\",
            b"#/",
            b"a.b'",
        ];
        // 128+ byte JA word and 130 'a's ASCII name (long but legal)
        let mut long_ja = b"--! ja\n".to_vec();
        long_ja.extend(std::iter::repeat_n(b'a', 130));
        long_ja.push(b'\n');
        let mut long_ascii = std::iter::repeat_n(b'a', 130).collect::<Vec<u8>>();
        long_ascii.push(b'\n');
        let mut all: Vec<Vec<u8>> = cases.iter().map(|c| c.to_vec()).collect();
        all.push(long_ja);
        all.push(long_ascii);
        let dir = std::env::temp_dir().join(format!("steel-lex-diff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut compared = 0;
        for (i, c) in all.iter().enumerate() {
            let p = dir.join(format!("case{:03}.ano", i));
            std::fs::write(&p, c).unwrap();
            if diff_file(&anoc, &p) {
                compared += 1;
            }
        }
        std::fs::remove_dir_all(&dir).ok();
        assert!(compared >= all.len() - 2, "only {} of {} cases compared", compared, all.len());
    }

    #[test]
    fn reserved_words() {
        assert!(lex_reserved("til"));
        assert!(lex_reserved("max"));
        assert!(lex_reserved("\u{7dcf}\u{548c}"));
        assert!(lex_reserved("|>"));
        assert!(lex_reserved("<-"));
        assert!(lex_reserved("_"));
        assert!(lex_reserved("'"));
        assert!(lex_reserved("42"));
        assert!(lex_reserved("3.5"));
        assert!(lex_reserved("\u{516d}\u{5341}"));
        assert!(lex_reserved("\u{4e09}\u{30f6}\u{6708}"));
        assert!(!lex_reserved("Til"));
        assert!(!lex_reserved("gold"));
        assert!(!lex_reserved("3x"));
        assert!(!lex_reserved("\u{30f6}\u{6708}"));
        assert!(lex_reserved_fold("Til"));
        assert!(lex_reserved_fold("MAX"));
        assert!(lex_reserved_fold("til"));
        assert!(!lex_reserved_fold("gold"));
        assert!(!lex_reserved_fold("Gold"));
        let long = "A".repeat(256);
        assert!(!lex_reserved_fold(&long));
    }
}
