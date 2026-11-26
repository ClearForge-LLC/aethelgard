"""AET-WO-4400j (1): two SOS `dumpheap -stat` outputs diffed by type: the growth in bytes and count, the top first, and
the share of the total growth the top types carry. Usage: heapdiff.py STAT_A STAT_B [TOP]"""
import re, sys
row = re.compile(r"^\s*([0-9a-f]{8,16})\s+([\d,]+)\s+([\d,]+)\s+(.+?)\s*$")
def read(p):
    out = {}
    for l in open(p):
        m = row.match(l)
        if m:
            out[m.group(4)] = (int(m.group(2).replace(",", "")), int(m.group(3).replace(",", "")), m.group(1))
    return out
a, b = read(sys.argv[1]), read(sys.argv[2])
top = int(sys.argv[3]) if len(sys.argv) > 3 else 25
ta, tb = sum(v[1] for v in a.values()), sum(v[1] for v in b.values())
d = sorted(((b.get(k, (0, 0, ""))[1] - a.get(k, (0, 0, ""))[1], b.get(k, (0, 0, ""))[0] - a.get(k, (0, 0, ""))[0], k,
            b.get(k, a.get(k))[2]) for k in set(a) | set(b)), reverse=True)
grow = tb - ta
print(f"total {ta:,} -> {tb:,} B ({grow:+,}); types {len(a)} -> {len(b)}")
acc = 0
for db, dc, k, mt in d[:top]:
    acc += db
    print(f"{db:>+15,} B {dc:>+12,} objs  {100 * acc / grow if grow else 0:5.1f}% cum  {mt}  {k}")
