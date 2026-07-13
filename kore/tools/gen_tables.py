#!/usr/bin/env python3
# Inputs: common/ano_unicode_tables.h and common/ano_collate_tables.h (generated C, UCD/DUCET 17.0.0).
# Output: kore/src/tables.rs — the same const arrays as Rust, zero hand transcription.
# Invariants: parsed counts match the C declarations; emitted arrays parse back to the same
# values.

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
SRC = [ROOT / "common" / "ano_unicode_tables.h", ROOT / "common" / "ano_collate_tables.h"]
OUT = ROOT / "kore" / "src" / "tables.rs"

CTYPE = {"uint32_t": "u32", "uint16_t": "u16", "ano_uc_record_t": None}


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def parse_int(tok):
    tok = tok.strip().rstrip("uU")
    return int(tok, 0)


def parse_header(path):
    text = strip_comments(path.read_text())
    arrays = {}
    for m in re.finditer(r"static const (\w+) (\w+)\[(\d+)\] = \{(.*?)\};", text, flags=re.S):
        ctype, name, declared, body = m.group(1), m.group(2), int(m.group(3)), m.group(4)
        if ctype == "ano_uc_record_t":
            rows = [tuple(parse_int(t) for t in g.split(","))
                    for g in re.findall(r"\{([^}]*)\}", body)]
            assert all(len(r) == 3 for r in rows), f"{name}: malformed record"
        else:
            rows = [parse_int(t) for t in body.split(",") if t.strip()]
        assert len(rows) == declared, f"{name}: parsed {len(rows)} rows, declared {declared}"
        arrays[name] = (ctype, declared, rows)
    return arrays


def emit(arrays):
    out = []
    out.append("// GENERATED FILE -- do not edit. Produced by kore/tools/gen_tables.py from")
    out.append("// common/ano_unicode_tables.h and common/ano_collate_tables.h (UCD/DUCET 17.0.0).")
    out.append("// Two-stage case/class lookup: ANO_UC_STAGE2[ANO_UC_STAGE1[cp >> 8] as usize * 256 + (cp & 0xFF) as usize]")
    out.append("// indexes ANO_UC_RECORDS; record 0 is the identity record.")
    out.append("// A collation element packs primary(16).secondary(11).tertiary(5) in one u32.")
    out.append("// ANO_CE_STAGE2[ANO_CE_STAGE1[cp >> 8] as usize * 256 + (cp & 0xFF) as usize] indexes")
    out.append("// ANO_CE_SPANS (offset << 4 | len into ANO_CE_POOL); span 0 = unlisted, UCA implicit weights.")
    out.append("// Decompositions: bsearch ANO_DECOMP_CP; ANO_DECOMP_SPAN is offset << 3 | len into ANO_DECOMP_POOL.")
    out.append("")
    out.append("pub const ANO_UC_TABLE_MAX: u32 = 0x10000;")
    out.append("pub const ANO_UC_LETTER: u8 = 1;")
    out.append("pub const ANO_UC_DIGIT: u8 = 2;")
    out.append("pub const ANO_UC_WHITESPACE: u8 = 4;")
    out.append("pub const ANO_UC_MARK: u8 = 8;")
    out.append("pub const ANO_UC_PUNCT: u8 = 16;")
    out.append("")
    out.append("#[derive(Clone, Copy)]")
    out.append("pub struct UcRecord {")
    out.append("    pub upper_delta: i32,")
    out.append("    pub lower_delta: i32,")
    out.append("    pub flags: u8,")
    out.append("}")
    for name, (ctype, declared, rows) in arrays.items():
        out.append("")
        rname = name.upper()
        if ctype == "ano_uc_record_t":
            out.append(f"pub static {rname}: [UcRecord; {declared}] = [")
            for r in rows:
                out.append(f"    UcRecord {{ upper_delta: {r[0]}, lower_delta: {r[1]}, flags: {r[2]} }},")
            out.append("];")
        else:
            rt = CTYPE[ctype]
            out.append(f"pub static {rname}: [{rt}; {declared}] = [")
            width = 8 if rt == "u32" else 12
            for i in range(0, len(rows), width):
                chunk = rows[i:i + width]
                if rt == "u32":
                    out.append("    " + " ".join(f"0x{v:08X}," for v in chunk))
                else:
                    out.append("    " + " ".join(f"{v}," for v in chunk))
            out.append("];")
    out.append("")
    return "\n".join(out)


def verify_roundtrip(rust_text, arrays):
    for name, (ctype, declared, rows) in arrays.items():
        rname = name.upper()
        m = re.search(rf"pub static {rname}: \[[^\]]*\] = \[(.*?)\n\];", rust_text, flags=re.S)
        assert m, f"{rname}: not found in emitted Rust"
        body = m.group(1)
        if ctype == "ano_uc_record_t":
            back = [(int(a), int(b), int(c)) for a, b, c in re.findall(
                r"UcRecord \{ upper_delta: (-?\d+), lower_delta: (-?\d+), flags: (\d+) \}", body)]
        else:
            back = [parse_int(t) for t in body.replace("\n", " ").split(",") if t.strip()]
        assert len(back) == declared, f"{rname}: emitted {len(back)} rows, declared {declared}"
        assert back == rows, f"{rname}: emitted values differ from the C parse"
        print(f"ok {name}[{declared}] -> {rname}")


def main():
    arrays = {}
    for src in SRC:
        arrays.update(parse_header(src))
    rust = emit(arrays)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(rust)
    verify_roundtrip(rust, arrays)
    print(f"wrote {OUT} ({len(rust.splitlines())} lines, {len(arrays)} arrays)")


if __name__ == "__main__":
    sys.exit(main())
