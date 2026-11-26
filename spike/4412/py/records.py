"""AET-WO-4412 (1): a foreign run's archive (trace.cbor) counted by record kind, with no Aethelgard code (the reader is
4400i's py/archive.py's, the deterministic CBOR profile of crates/aethelgard-chaos/README.md). Usage: records.py RUN_DIR"""
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
import collections
run = sys.argv[1]
b = open(f"{run}/trace.cbor", "rb").read()
header, i = item(b, 0)
assert header[0] == "aethelgard-trace", header[:2]
k = collections.Counter()
while i < len(b):
    ev, i = item(b, i); k[ev[0]] += 1
print(f"trace {header[1]}, {len(b)} B, {sum(k.values())} records, by kind {dict(sorted(k.items()))}")
