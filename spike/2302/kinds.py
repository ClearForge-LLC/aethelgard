"""AET-WO-2302 spike: bytes per record kind in a trace.cbor, a sequence of top-level CBOR items in
RUL-06's profile (ints, bytes, text, arrays). Usage: python3 -I kinds.py <trace.cbor>"""
import sys, collections
b = open(sys.argv[1], "rb").read()
def item(i):
    ib = b[i]; mt, ai = ib >> 5, ib & 31; i += 1
    if ai < 24: n = ai
    else:
        k = {24: 1, 25: 2, 26: 4, 27: 8}[ai]; n = int.from_bytes(b[i:i + k], "big"); i += k
    if mt in (0, 1): return n, i
    if mt in (2, 3): return b[i:i + n], i + n
    if mt == 4:
        out = []
        for _ in range(n):
            v, i = item(i); out.append(v)
        return out, i
    raise ValueError(mt)
i = 0; sizes = collections.Counter(); counts = collections.Counter(); first = None
while i < len(b):
    v, j = item(i)
    if first is None:
        first = (v[:3] if isinstance(v, list) else v, j - i)
    else:
        # an event: [index, kind, ...] or [kind, ...]; take the second int if the first is an index
        kd = (v[0], "t0" if v[1] == 0 else "end") if isinstance(v, list) and len(v) > 1 else "?"
        sizes[kd] += j - i; counts[kd] += 1
    i = j
print("header", first[1], "B")
for kd in sorted(sizes, key=str):
    print(f"kind {kd}: {counts[kd]} x, {sizes[kd]} B")
print("events", sum(counts.values()), "records,", sum(sizes.values()), "B; total", len(b), "B")
