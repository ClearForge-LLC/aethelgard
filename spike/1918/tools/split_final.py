#!/usr/bin/env python3
"""AET-WO-1918 candidate (S), on a scratch copy only: split one schedule part's final check in K.

Reads <crate>/src/proofs.rs, finds `fn check_final_<part>` and the harness
`fn every_valid_sequence_<part>`, and writes them back as K checks and K harnesses:
  - check_final_<part>_<i> holds the i-th contiguous slice of the original's top-level
    statements (each `kani::assert(...)`, with its comments);
  - every_valid_sequence_<part>_<i> is the original harness, the same schedule, unwind and
    covers, calling check_final_<part>_<i>.
The union of the slices is the original's statements, in order. The original functions are removed.
Stdlib only; run with python3 -I.

usage: split_final.py <crate> <part> <K>
"""
import re
import sys

crate, part, k = sys.argv[1], sys.argv[2], int(sys.argv[3])
path = f"{crate}/src/proofs.rs"
lines = open(path).read().split("\n")


def span(start_pat):
    """Lines [a, b] of a top-level fn whose signature matches start_pat, b its closing `}`."""
    a = next(i for i, l in enumerate(lines) if re.match(start_pat, l))
    b = next(i for i in range(a, len(lines)) if lines[i] == "}")
    return a, b


ca, cb = span(rf"fn check_final_{part}\(")
# Top-level statements: each starts at 4 spaces with `kani::assert(` (comments attach to the next).
body = lines[ca + 1 : cb]
stmts, cur = [], []
for l in body:
    cur.append(l)
    if l.startswith("    ") and not l.startswith("     ") and l.rstrip().endswith(";"):
        stmts.append(cur)
        cur = []
assert not "".join(cur).strip(), f"trailing lines in check_final_{part}: {cur}"
n = len(stmts)
cuts = [round(i * n / k) for i in range(k + 1)]
sig = lines[ca]
checks = []
for i in range(k):
    checks += [sig.replace(f"check_final_{part}(", f"check_final_{part}_{i}(")]
    for s in stmts[cuts[i] : cuts[i + 1]]:
        checks += s
    checks += ["}", ""]

ha, hb = span(rf"fn every_valid_sequence_{part}\(\)")
# Include the attributes and doc comment above the harness.
top = ha
while top > 0 and (lines[top - 1].startswith("#[") or lines[top - 1].startswith("///")):
    top -= 1
harness = lines[top : hb + 1]
calls = [i for i, l in enumerate(harness) if f"check_final_{part}(" in l]
assert len(calls) == 1, f"the harness calls check_final_{part} {len(calls)} times"
harnesses = []
for i in range(k):
    h = [l.replace(f"fn every_valid_sequence_{part}()", f"fn every_valid_sequence_{part}_{i}()") for l in harness]
    h[calls[0]] = h[calls[0]].replace(f"check_final_{part}(", f"check_final_{part}_{i}(")
    harnesses += h + [""]

# Replace the later span first so the earlier indices stay valid.
spans = sorted([(ca, cb, checks), (top, hb, harnesses)], reverse=True)
for a, b, new in spans:
    lines[a : b + 1] = new
open(path, "w").write("\n".join(lines))
print(f"split check_final_{part}: {n} statements into {k} parts of {[cuts[i + 1] - cuts[i] for i in range(k)]}")
