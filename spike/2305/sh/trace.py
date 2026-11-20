"""Scratch (AET-WO-2305): print a trace.cbor's records of the kinds named, each [kind, ...]."""
import sys
def dec(b, i):
    ib = b[i]; mt, ai = ib >> 5, ib & 31; i += 1
    if ai < 24: n = ai
    else:
        k = {24:1,25:2,26:4,27:8}[ai]; n = int.from_bytes(b[i:i+k],'big'); i += k
    if mt == 0: return n, i
    if mt == 1: return -1-n, i
    if mt == 2: return b[i:i+n].hex(), i+n
    if mt == 3: return b[i:i+n].decode(), i+n
    if mt == 4:
        out = []
        for _ in range(n): v, i = dec(b, i); out.append(v)
        return out, i
    raise ValueError(mt)
def records(path):
    """The records after the header, each [kind, at_ns, ...]."""
    b = open(path, 'rb').read(); _, i = dec(b, 0); out = []
    while i < len(b):
        r, i = dec(b, i); out.append(r)
    return out
if __name__ == '__main__':
    kinds = {int(k) for k in sys.argv[2:]}
    for r in records(sys.argv[1]):
        if not kinds or r[0] in kinds: print(r)
