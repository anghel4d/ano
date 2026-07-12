// num.rs — C-locale numeric parsing and printf-faithful formatting: the shared choke points
// every number crosses (registry wnum/dnum, main save_num and --tokens %g, emit %.17g).
// Owned by the registry porter; emit and main call through these frozen signatures.

// Inputs: a word. Output: (value, bytes consumed); consumed == 0 means no conversion.
// Invariants: C-locale strtod exactly — optional isspace* skip, optional sign, then decimal
// (digits, optional '.', optional [eE] exponent), or hex (0[xX] hex digits, optional '.',
// optional [pP] exponent), or case-insensitive inf/infinity/nan/nan(chars); longest valid
// prefix wins; correctly rounded like glibc. Non-finite results ARE returned (callers gate).
pub fn strtod(s: &str) -> (f64, usize) {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    let sign_start = i;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    // inf / infinity, case-insensitive
    if ci_match(b, i, b"inf") {
        let end = if ci_match(b, i + 3, b"inity") { i + 8 } else { i + 3 };
        return (if neg { f64::NEG_INFINITY } else { f64::INFINITY }, end);
    }
    // nan / nan(n-char-seq)
    if ci_match(b, i, b"nan") {
        let mut end = i + 3;
        if end < b.len() && b[end] == b'(' {
            let mut k = end + 1;
            while k < b.len() && (b[k].is_ascii_alphanumeric() || b[k] == b'_') {
                k += 1;
            }
            if k < b.len() && b[k] == b')' {
                end = k + 1;
            }
        }
        return (if neg { -f64::NAN } else { f64::NAN }, end);
    }
    // hex: 0[xX] with at least one hex digit; else the '0' falls through as decimal
    if i + 1 < b.len() && b[i] == b'0' && (b[i + 1] | 32) == b'x' {
        if let Some((v, end)) = hex_float(b, i + 2, neg) {
            return (v, end);
        }
    }
    // decimal: digits [. digits] [eE [+-] digits], at least one digit before the exponent
    let mut j = i;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    let int_digits = j - i;
    let mut frac_digits = 0;
    if j < b.len() && b[j] == b'.' {
        let k = j + 1;
        let mut f = k;
        while f < b.len() && b[f].is_ascii_digit() {
            f += 1;
        }
        frac_digits = f - k;
        if int_digits + frac_digits > 0 {
            j = f;
        }
    }
    if int_digits + frac_digits == 0 {
        return (0.0, 0);
    }
    let mut end = j;
    if j < b.len() && (b[j] | 32) == b'e' {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        let d = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k > d {
            end = k;
        }
    }
    // f64::from_str is correctly rounded on this grammar, matching glibc strtod
    let v: f64 = s[sign_start..end].parse().unwrap_or(0.0);
    (v, end)
}

// Inputs: bytes, positions. Output: true when b[i..] begins with pat, ASCII case-folded.
fn ci_match(b: &[u8], i: usize, pat: &[u8]) -> bool {
    b.len() >= i + pat.len() && b[i..i + pat.len()].iter().zip(pat).all(|(x, y)| x | 32 == *y)
}

// Inputs: bytes, index just past "0x", sign. Output: Some(value, end) when at least one hex
// digit is present; None sends the caller back to the decimal path (glibc parses bare "0x"
// as the decimal "0"). Invariants: mantissa accumulates 61 significant bits then goes sticky;
// [pP] exponent optional (strtod, unlike C literals); round to nearest, ties to even, exactly.
fn hex_float(b: &[u8], mut i: usize, neg: bool) -> Option<(f64, usize)> {
    let mut m: u64 = 0;
    let mut sticky = false;
    let mut e2: i64 = 0;
    let mut any = false;
    while i < b.len() {
        let Some(d) = hexval(b[i]) else { break };
        any = true;
        if m < 1 << 60 {
            m = m * 16 + d;
        } else {
            e2 += 4;
            sticky |= d != 0;
        }
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        let mut f = i + 1;
        while f < b.len() {
            let Some(d) = hexval(b[f]) else { break };
            any = true;
            if m < 1 << 60 {
                m = m * 16 + d;
                e2 -= 4;
            } else {
                sticky |= d != 0;
            }
            f += 1;
        }
        // a lone '.' after the digits still belongs to the subject sequence
        if any {
            i = f;
        }
    }
    if !any {
        return None;
    }
    if i < b.len() && (b[i] | 32) == b'p' {
        let mut k = i + 1;
        let mut esign: i64 = 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            if b[k] == b'-' {
                esign = -1;
            }
            k += 1;
        }
        let d = k;
        let mut ev: i64 = 0;
        while k < b.len() && b[k].is_ascii_digit() {
            ev = (ev * 10 + (b[k] - b'0') as i64).min(1 << 40);
            k += 1;
        }
        if k > d {
            e2 += esign * ev;
            i = k;
        }
    }
    Some((hex_round(m, sticky, e2, neg), i))
}

// Inputs: an ASCII byte. Output: its hex digit value, or None.
fn hexval(c: u8) -> Option<u64> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u64),
        b'a'..=b'f' => Some((c - b'a' + 10) as u64),
        b'A'..=b'F' => Some((c - b'A' + 10) as u64),
        _ => None,
    }
}

// Inputs: mantissa m with sticky bits strictly below, binary exponent (value = m * 2^e2), sign.
// Output: the nearest f64, ties to even; overflow to inf, underflow through the subnormals to 0.
fn hex_round(mut m: u64, mut sticky: bool, mut e2: i64, neg: bool) -> f64 {
    let signb: u64 = if neg { 1 << 63 } else { 0 };
    if m == 0 {
        return f64::from_bits(signb);
    }
    let lz = m.leading_zeros() as i64;
    m <<= lz;
    e2 -= lz;
    // value = m * 2^e2 with bit 63 set; unbiased exponent of the leading bit
    let mut e = e2 + 63;
    if e > 1023 {
        return f64::from_bits(signb | (0x7ff << 52));
    }
    let drop = if e >= -1022 { 11 } else { 11 + (-1022 - e) };
    let (mut m53, round) = if drop >= 65 {
        sticky |= m != 0;
        (0u64, false)
    } else if drop == 64 {
        sticky |= m << 1 != 0;
        (0u64, m >> 63 == 1)
    } else {
        sticky |= m & ((1u64 << (drop - 1)) - 1) != 0;
        (m >> drop, (m >> (drop - 1)) & 1 == 1)
    };
    if round && (sticky || m53 & 1 == 1) {
        m53 += 1;
    }
    if m53 == 1 << 53 {
        m53 >>= 1;
        e += 1;
        if e > 1023 {
            return f64::from_bits(signb | (0x7ff << 52));
        }
    }
    if m53 < 1 << 52 {
        // subnormal (or zero); min-normal carry lands here too as bits 1<<52 exactly
        f64::from_bits(signb | m53)
    } else {
        f64::from_bits(signb | (((e + 1023) as u64) << 52) | (m53 & ((1 << 52) - 1)))
    }
}

// Inputs: a word. Output: Some(value) iff strtod consumes the WHOLE word and the result is
// finite; None on empty, trailing junk, or non-finite. The one number gate: registry.c's
// wnum and main.c's save_num are both exactly this predicate.
pub fn wnum(s: &str) -> Option<f64> {
    let (v, n) = strtod(s);
    (n == s.len() && n != 0 && v.is_finite()).then_some(v)
}

// Inputs: precision p >= 1, a double. Output: printf "%.*g" bytes, glibc-exact: p significant
// digits, %e form when the decimal exponent is < -4 or >= p, trailing zeros stripped (no '#'),
// exponent at least two digits with sign (1e+16, 9e-05), -0 keeps its sign.
// Default-precision C "%g" is fmt_g(6, x) — print_toks and the unique-repeat diagnostic.
pub fn fmt_g(prec: i32, x: f64) -> String {
    // printf: negative precision means none (6); zero means 1
    let p = if prec < 0 { 6 } else if prec == 0 { 1 } else { prec as usize };
    if x.is_nan() {
        return if x.is_sign_negative() { "-nan".into() } else { "nan".into() };
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf".into() } else { "inf".into() };
    }
    // decide the form off the %e-rounded exponent (std fixed-precision floats are exact)
    let e = format!("{:.*e}", p - 1, x);
    let epos = e.rfind('e').unwrap();
    let exp: i32 = e[epos + 1..].parse().unwrap();
    if exp < -4 || exp >= p as i32 {
        let mant = strip_zeros(&e[..epos]);
        format!("{}e{}{:02}", mant, if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        let f = format!("{:.*}", (p as i32 - 1 - exp) as usize, x);
        strip_zeros(&f).to_string()
    }
}

// Inputs: a fixed-point rendering. Output: it, minus trailing fractional zeros and a bare '.'.
fn strip_zeros(s: &str) -> &str {
    if !s.contains('.') {
        return s;
    }
    s.trim_end_matches('0').trim_end_matches('.')
}

// Inputs: a double. Output: Some(i) iff -9e15 <= x <= 9e15 and x == trunc(x) — the shared
// %lld fast-path guard of dnum and emit's numLit (range check BEFORE the cast; the C cast is
// UB out of range). Note: dnum additionally excludes -0.0 (its own check); numLit does not.
pub fn int_fast(x: f64) -> Option<i64> {
    ((-9e15..=9e15).contains(&x) && x == x.trunc()).then(|| x as i64)
}

// Inputs: a finite double. Output: text strtod parses back bit-exact (dump -> load -> dump
// fixpoints). Integer fast path (int_fast and not -0.0): plain "%lld" digits. Else the first
// p in 1..=17 where fmt_g(p, x) round-trips through strtod == x. -0.0 prints "-0".
pub fn dnum(x: f64) -> String {
    if let Some(i) = int_fast(x) {
        if !(x == 0.0 && x.is_sign_negative()) {
            return i.to_string();
        }
    }
    let mut s = String::new();
    for p in 1..=17 {
        s = fmt_g(p, x);
        if strtod(&s).0 == x {
            break;
        }
    }
    s
}
