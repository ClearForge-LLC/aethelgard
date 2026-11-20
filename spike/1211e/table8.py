#!/usr/bin/env python3
"""Extract each LQFP144 I/O pin's I/O structure from DS12117 Rev 10 Table 8
(pin/ball definition, pp. 64-87), keyed by the vendored ST open pin data's
pin names (crates/aethelgard-svd-hal/data/st-open-pin-data/STM32H753ZITx.xml).

Usage: python3 -I table8.py <ds12117-rev10.txt> <STM32H753ZITx.xml>
Prints TSV `name  position  type  structure  page  lqfp144-in-table`, where
the last column is the LQFP144 position Table 8 states for the row (the
third of its eight package columns), to be checked against the XML's.
Every XML pin of type I/O, Reset, Boot or MonoIO must match exactly one row."""
import re, sys
import xml.etree.ElementTree as ET

lines = [l.strip() for l in open(sys.argv[1], encoding="utf-8")]
start = next(i for i, l in enumerate(lines) if l.startswith("Table 8. Pin/ball definition"))
end = next(i for i, l in enumerate(lines) if l.startswith("Table 9.") and i > start)
TYPES = {"I/O", "I", "S", "ANA"}
STRUCT = re.compile(r"^(FT\w*|TT\w*|B|RST|-)$")
GPIO = re.compile(r"^P[A-K]\d{1,2}(_C)?\b")
POS = re.compile(r"^(\d{1,3}|[A-Z]{1,2}\d{1,2}|-)(\(\d\))?$")
ns = {"x": "http://dummy.com"}
root = ET.parse(sys.argv[2]).getroot()
xml = {}
for p in root.iter():
    if p.tag.endswith("}Pin") or p.tag == "Pin":
        xml[p.get("Name")] = (int(p.get("Position")), p.get("Type"))

# Every row: (name line(s) joined, type, structure, page, the 8 lines before).
# The page the table starts on: the last marker above it.
page = next(int(m.group(1)) for l in reversed(lines[:start]) if (m := re.match(r"^=== page (\d+) ===$", l)))
rows = []
for i in range(start, end):
    m = re.match(r"^=== page (\d+) ===$", lines[i])
    if m:
        page = int(m.group(1))
        continue
    if lines[i] in TYPES and i + 1 < end and STRUCT.match(lines[i + 1]):
        # The name: the line above, or, for a name the layout wraps
        # (`PC14-` / `OSC32_IN` / ...), the lines from the one that starts
        # with a port pin name. A ball name (`E3`) never starts `P[A-K]\d`.
        k = next((k for k in range(i - 1, i - 7, -1) if GPIO.match(lines[k])), i - 1)
        # A structure the layout wraps (`FT_h` / `a` is FT_ha).
        struct = lines[i + 1]
        if re.match(r"^[a-z]{1,2}$", lines[i + 2]):
            struct += lines[i + 2]
        rows.append(("".join(lines[k:i]), lines[i], struct, page, lines[k - 8 : k]))

def base(name):
    # The port pin (`PC14`, `PC2_C`), or the name up to a note or bracket.
    m = re.match(r"^P[A-K]\d{1,2}(_C)?(?![\d_])", name)
    return m.group(0) if m else re.sub(r"[\s(].*$", "", name)

failed = 0
for name, (position, typ) in sorted(xml.items(), key=lambda kv: kv[1][0]):
    if typ == "Power" or typ == "":
        continue
    hits = [r for r in rows if base(r[0]) == base(name)]
    if len(hits) != 1:
        print(f"NO-UNIQUE-ROW {name} ({typ}, {position}): {len(hits)} rows {[(h[0], h[3]) for h in hits]}", file=sys.stderr)
        failed += 1
        continue
    rname, rtype, struct, pg, before = hits[0]
    # The LQFP144 column is the third of eight; the eight end at `before[-1]`.
    lq = re.sub(r"\(\d\)$", "", before[2]) if len(before) == 8 else "?"
    flag = "" if lq == str(position) else "\tPOSITION-MISMATCH"
    print(f"{name}\t{position}\t{rtype}\t{struct}\t{pg}\t{lq}{flag}")
print(f"failed {failed}", file=sys.stderr)
