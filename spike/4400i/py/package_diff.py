"""AET-WO-4400i F16 (ruled 2026-11-24T21:38Z): after the write dispatch, show that only campaign state hashes moved.
Usage: python3 -I package_diff.py OLD_PACKAGE_DIR NEW_PACKAGE_DIR [OLD_VERDICT.json NEW_VERDICT.json]
- Every file but MANIFEST.json must be byte-identical; in MANIFEST.json only `verdict_sha256` may differ.
- With the two verify verdict lines (the dispatch's and main's), every differing field must be a campaign run's
  `state_sequence_hash` (and the fab block's own MANIFEST hash, which follows from it).
Prints each difference; exits 1 on any other."""
import hashlib, json, os, sys

def files(d):
    out = {}
    for root, _, names in os.walk(d):
        for n in names:
            p = os.path.join(root, n)
            out[os.path.relpath(p, d)] = hashlib.sha256(open(p, "rb").read()).hexdigest()
    return out

def flat(o, p=""):
    if isinstance(o, dict):
        for k, v in o.items(): yield from flat(v, f"{p}/{k}")
    elif isinstance(o, list):
        for i, v in enumerate(o): yield from flat(v, f"{p}/{i}")
    else: yield p, o

old, new = sys.argv[1], sys.argv[2]
bad = 0
a, b = files(old), files(new)
for f in sorted(set(a) | set(b)):
    if a.get(f) != b.get(f):
        print(f"file differs: {f}")
        if f != "MANIFEST.json": bad = 1
if "MANIFEST.json" in a and "MANIFEST.json" in b:
    ma = dict(flat(json.load(open(os.path.join(old, "MANIFEST.json")))))
    mb = dict(flat(json.load(open(os.path.join(new, "MANIFEST.json")))))
    for k in sorted(set(ma) | set(mb)):
        if ma.get(k) != mb.get(k):
            print(f"MANIFEST {k}: {ma.get(k)} -> {mb.get(k)}")
            if k != "/verdict_sha256": bad = 1
if len(sys.argv) > 4:
    va = dict(flat(json.load(open(sys.argv[3])))); vb = dict(flat(json.load(open(sys.argv[4]))))
    for k in sorted(set(va) | set(vb)):
        if va.get(k) != vb.get(k):
            ok = k.startswith("/campaign/") and k.endswith("/state_sequence_hash") or k == "/fab/files/MANIFEST.json"
            print(f"verdict {k}: {va.get(k)} -> {vb.get(k)}{'' if ok else '   <- NOT a state hash'}")
            if not ok: bad = 1
print("only state hashes moved" if not bad else "OTHER DIFFERENCES")
sys.exit(bad)
