"""AET-WO-1606 P3: every file the baseline and the branch runs wrote, byte for byte.

usage: python3 -I compare.py <runs-main> <runs-br>

Substitutions, each named in the output: the bridge digest (main's and the branch's, both to
`<BRIDGE_DIGEST>`); the run directory's own path; and the values derived from a trace, each checked
to be so: a manifest's sha256 of its trace and its state hash, and the campaign line's
`state_sequence_hash`, which is that state hash. `build.json`'s new `tool.rustc` is
removed before comparing and reported. Every other byte must be equal.
"""
import hashlib
import json
import os
import sys

OLD = b"5bdc40e4cac43a6aceba8c1fb30f22b78f2b86be77d4dd75df2b32c37c92636e"
NEW = b"2e95092a66cd337a764c741b2c88f950056684fd14a128126a23f50bf249cc2f"


def files(root):
    out = []
    for d, _, names in os.walk(root):
        for n in names:
            out.append(os.path.relpath(os.path.join(d, n), root))
    return sorted(out)


def main(a, b):
    fa, fb = files(a), files(b)
    status = 0
    if fa != fb:
        print("FILE SETS DIFFER:", sorted(set(fa) ^ set(fb)))
        status = 1
    counts = {"identical": 0, "digest only": 0}
    states = {}
    # Manifests before campaign lines, so each run's state hash is known when its line is read.
    order = sorted(set(fa) & set(fb), key=lambda r: (r.endswith("campaign.jsonl"), r))
    for rel in order:
        if rel.startswith("bridge/") or rel in ("doctor.json", "doctor.err"):
            continue  # compared by hand: the bridge's own modes and doctor change by design
        x = open(os.path.join(a, rel), "rb").read()
        y = open(os.path.join(b, rel), "rb").read()
        if x == y:
            counts["identical"] += 1
            continue
        # The runs wrote their own path as the image saw it, /cache/<runs-dir>.
        pa, pb = (("/cache/" + os.path.basename(r)).encode() for r in (a, b))
        nx = x.replace(OLD, b"<BRIDGE_DIGEST>").replace(pa, b"<RUN>")
        ny = y.replace(NEW, b"<BRIDGE_DIGEST>").replace(pb, b"<RUN>")
        if rel.endswith("manifest.cbor"):
            # Derived: sha256 of this run's trace, then the replayer's state hash over it. Equal
            # after substituting each side's own values exactly when the traces are.
            d = os.path.dirname(rel)
            tx, ty = (open(os.path.join(r, d, "trace.cbor"), "rb").read() for r in (a, b))
            hx, hy = hashlib.sha256(tx).digest(), hashlib.sha256(ty).digest()
            nx = x.replace(hx, b"<TRACE>").replace(x[-32:], b"<STATE>")
            ny = y.replace(hy, b"<TRACE>").replace(y[-32:], b"<STATE>")
            states[os.path.basename(d)] = (x[-32:].hex(), y[-32:].hex())
            if nx == ny and tx.replace(OLD, b"") == ty.replace(NEW, b""):
                counts["derived"] = counts.get("derived", 0) + 1
                continue
        if rel.endswith("campaign.jsonl"):
            # Each run's state_sequence_hash is its manifest's state hash (checked here).
            for run, (sx, sy) in states.items():
                if run.encode() in nx:
                    assert nx.count(sx.encode()) == 1 and ny.count(sy.encode()) == 1, run
                    nx, ny = nx.replace(sx.encode(), b"<STATE>"), ny.replace(sy.encode(), b"<STATE>")
            if nx == ny:
                counts["derived"] = counts.get("derived", 0) + 1
                print(f"{rel}: equal after each run's state hash, its manifest's, and the path")
                continue
        if rel.endswith("build.json"):
            vx, vy = json.loads(nx), json.loads(ny)
            rustc = vy["tool"].pop("rustc", None)
            print(f"{rel}: branch adds tool.rustc = {json.dumps(rustc)}; main has {vx['tool'].get('rustc')}")
            nx, ny = json.dumps(vx).encode(), json.dumps(vy).encode()
        if nx == ny:
            how = "digest only" if (OLD in x) else "path only"
            counts[how] = counts.get(how, 0) + 1
            if how != "digest only":
                print(f"{rel}: equal after the path substitution")
            elif not rel.endswith("trace.cbor"):
                print(f"{rel}: equal after the digest substitution")
            continue
        print(f"DIFFERS: {rel} ({len(x)} vs {len(y)} bytes)")
        status = 1
    print("summary:", counts, "exit", status)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1].rstrip("/"), sys.argv[2].rstrip("/")))
