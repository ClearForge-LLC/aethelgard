"""AET-WO-4400i (2), D6 R2: read a foreign run's archive (trace.cbor, the deterministic CBOR profile of
aethelgard-chaos, crates/aethelgard-chaos/README.md) with no Aethelgard code: the header's artifacts, and the input
file joined from its kind-38 parts, written to OUT, its sha256 checked against the header's `input`.
Usage: python3 -I archive.py RUN_DIR OUT_FILE"""
import hashlib, sys
def item(b, i):
    ib = b[i]; mt, ai = ib >> 5, ib & 31; i += 1
    if ai < 24: n = ai
    elif ai in (24, 25, 26, 27):
        k = 1 << (ai - 24); n = int.from_bytes(b[i:i + k], "big"); i += k
    else: raise SystemExit(f"archive: unsupported CBOR head {ib:#x}")
    if mt == 0: return n, i
    if mt == 1: return -1 - n, i
    if mt == 2: return bytes(b[i:i + n]), i + n
    if mt == 3: return b[i:i + n].decode(), i + n
    if mt == 4:
        out = []
        for _ in range(n):
            v, i = item(b, i); out.append(v)
        return out, i
    raise SystemExit(f"archive: unsupported CBOR major type {mt}")
run, out = sys.argv[1], sys.argv[2]
b = open(f"{run}/trace.cbor", "rb").read()
header, i = item(b, 0)
assert header[0] == "aethelgard-trace" and header[1] == 20, header[:2]
artifacts = {n: h.hex() for n, h in header[4]}
parts = []
while i < len(b):
    ev, i = item(b, i)
    if ev[0] == 38:
        label, peripheral, spacing, part, nparts, data = ev[2:]
        assert part == len(parts), (part, len(parts))
        parts.append(data)
f = b"".join(parts)
sha = hashlib.sha256(f).hexdigest()
assert sha == artifacts["input"], (sha, artifacts["input"])
open(out, "wb").write(f)
print(f"archive: input {sha} ({len(f)} bytes, {len(parts)} parts) written; artifacts {sorted(artifacts)}")
