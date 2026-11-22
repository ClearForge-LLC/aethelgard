#!/usr/bin/env python3
"""AET-WO-1211e §2: judge two runs of spike/1211a/measure/run.sh, main's binary on main's tree
(BASE) and the branch's on the branch's (AFTER, specification 24). The acceptance: every exit code
equal; `validate` and `solve` output equal once each 64-hex sha256 is masked (the specification's
sha moved with its version line, and each run prints its own directory); every emitted file byte-identical; every generated file
identical but for lines naming a sha256. Usage: python3 -I compare.py BASE AFTER > result.json"""
import json, os, re, sys

base, after = sys.argv[1], sys.argv[2]
HEX = re.compile(rb"[0-9a-f]{64}")
mask = lambda b: HEX.sub(b"H" * 64, b)
out = {"files": 0, "exit_moved": [], "output_moved": [], "emit_moved": [], "generate_moved": [],
       "generate_sha_lines": 0, "generated_files_with_a_sha_line_moved": 0}

def tree(d):
    r = {}
    for root, _, files in os.walk(d):
        for f in files:
            p = os.path.join(root, f)
            r[os.path.relpath(p, d)] = open(p, "rb").read()
    return r

specs = [l.strip() for l in open(os.path.join(base, "specs.txt"))]
assert specs == [l.strip() for l in open(os.path.join(after, "specs.txt"))], "different spec lists"
for s in specs:
    k = s.replace("/", "__")
    b, a = os.path.join(base, k), os.path.join(after, k)
    out["files"] += 1
    for cmd in ("validate", "solve", "emit", "generate"):
        if open(f"{b}/{cmd}.exit").read() != open(f"{a}/{cmd}.exit").read():
            out["exit_moved"].append(f"{s} {cmd}")
    for cmd in ("validate", "solve", "emit", "generate"):
        # Each run prints its own output directory's path.
        ob = open(f"{b}/{cmd}.out", "rb").read().replace(os.path.abspath(base).encode(), b"RUN")
        oa = open(f"{a}/{cmd}.out", "rb").read().replace(os.path.abspath(after).encode(), b"RUN")
        if mask(ob) != mask(oa):
            out["output_moved"].append(f"{s} {cmd}")
    if tree(f"{b}/emit") != tree(f"{a}/emit"):
        out["emit_moved"].append(s)
    tb, ta = tree(f"{b}/generate"), tree(f"{a}/generate")
    if tb.keys() != ta.keys():
        out["generate_moved"].append(f"{s}: file sets differ")
        continue
    for f in tb:
        if tb[f] == ta[f]:
            continue
        lb, la = tb[f].split(b"\n"), ta[f].split(b"\n")
        diff = [(x, y) for x, y in zip(lb, la) if x != y]
        if len(lb) != len(la) or any(not (HEX.search(x) and mask(x) == mask(y)) for x, y in diff):
            out["generate_moved"].append(f"{s}: {f}")
        else:
            out["generate_sha_lines"] += len(diff)
            out["generated_files_with_a_sha_line_moved"] += 1
json.dump(out, sys.stdout, indent=1)
