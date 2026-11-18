import json, sys
from collections import deque
cells_file, result = sys.argv[1], sys.argv[2]
h = 31250
cells = set()
for line in open(cells_file):
    l, x, y, s = line.split()
    if l == 'In2.Cu':
        cells.add((int(x)//h, int(y)//h))
r = json.load(open(result))
viol = {(v['x0']//h, v['y0']//h) for v in r['VDD_3V3-31250']['all']}
seen = {}
comp_id = 0
sizes = []
for c in cells:
    if c in seen: continue
    q = deque([c]); seen[c] = comp_id; n = 0
    while q:
        a = q.popleft(); n += 1
        for d in ((1,0),(-1,0),(0,1),(0,-1)):
            b = (a[0]+d[0], a[1]+d[1])
            if b in cells and b not in seen:
                seen[b] = comp_id; q.append(b)
    sizes.append(n); comp_id += 1
big = max(range(len(sizes)), key=lambda i: sizes[i])
print('components', len(sizes), 'largest', sizes[big], 'others', sorted(s for i,s in enumerate(sizes) if i != big)[-10:])
for v in sorted(viol):
    print('violator', (v[0]*h, v[1]*h), 'component size', sizes[seen[v]], 'main' if seen[v] == big else 'ISLAND')
