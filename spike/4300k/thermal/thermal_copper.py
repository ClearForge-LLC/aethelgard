# thermal_copper.py PCB REF... : copper of the pad's net electrically joined to a ref's EP pad, per layer.
# Pure python (no shapely). Shapes: pad rect(rotated), segment capsule, via circle. Union area by row bitmasks.
import sys, re, math
txt = open(sys.argv[1]).read()
def parse(s):
    i = 0; n = len(s); stack = [[]]
    tok = re.compile(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()"]+')
    for m in tok.finditer(s):
        t = m.group(0)
        if t == '(': stack.append([])
        elif t == ')':
            x = stack.pop(); stack[-1].append(x)
        else:
            stack[-1].append(t[1:-1] if t[0] == '"' else t)
    return stack[0][0]
root = parse(txt)
def kids(x, name): return [c for c in x[1:] if isinstance(c, list) and c and c[0] == name]
def kid(x, name):
    k = kids(x, name); return k[0] if k else None
CU = ["F.Cu", "In1.Cu", "In2.Cu", "B.Cu"]
items = []  # dict(kind, layers, net, shape)
fps = {}
for fp in kids(root, 'footprint'):
    at = kid(fp, 'at'); fx, fy = float(at[1]), float(at[2]); rot = float(at[3]) if len(at) > 3 else 0
    layer = kid(fp, 'layer')[1]
    ref = [p[2] for p in kids(fp, 'property') if p[1] == 'Reference'][0]
    fps[ref] = (fx, fy, rot)
    for pad in kids(fp, 'pad'):
        layers = kid(pad, 'layers')[1:]
        net = kid(pad, 'net'); net = net[1] if net else None
        if net is None: continue
        pa = kid(pad, 'at'); px, py = float(pa[1]), float(pa[2]); prot = float(pa[3]) if len(pa) > 3 else 0
        a = math.radians(-rot)  # KiCad: y down, rotation CCW on screen
        gx = fx + px*math.cos(a) - py*math.sin(a); gy = fy + px*math.sin(a) + py*math.cos(a)
        sz = kid(pad, 'size'); w, h = float(sz[1]), float(sz[2])
        shape_t = pad[3]  # smd/thru_hole/np_thru_hole
        ls = []
        for l in layers:
            if l == '*.Cu': ls = CU[:]
            elif l in CU: ls.append(l)
        if not ls: continue
        tot = rot + prot
        items.append(dict(kind='pad', ref=ref, num=pad[1], layers=ls, net=net, c=(gx, gy), size=(w, h), rot=tot, ptype=pad[3], pshape=pad[4]))
for s in kids(root, 'segment'):
    st, en, wd, ly, nt = kid(s,'start'), kid(s,'end'), kid(s,'width'), kid(s,'layer'), kid(s,'net')
    items.append(dict(kind='seg', layers=[ly[1]], net=nt[1], a=(float(st[1]),float(st[2])), b=(float(en[1]),float(en[2])), w=float(wd[1])))
for v in kids(root, 'via'):
    at, sz, ly, nt = kid(v,'at'), kid(v,'size'), kid(v,'layers'), kid(v,'net')
    items.append(dict(kind='via', layers=CU[:], net=nt[1], c=(float(at[1]),float(at[2])), d=float(sz[1]), drill=float(kid(v,'drill')[1])))
zones = []
for z in kids(root, 'zone'):
    if kid(z, 'keepout'): continue
    pts = [(float(p[1]), float(p[2])) for p in kid(kid(z,'polygon'),'pts')[1:] if p[0]=='xy']
    zones.append(dict(layer=kid(z,'layer')[1], net=kid(z,'net')[1], pts=pts, filled=bool(kid(z,'filled_polygon'))))
# geometry
def corners(it):
    w, h = it['size']; a = math.radians(-it['rot']); cx, cy = it['c']
    return [(cx + x*math.cos(a) - y*math.sin(a), cy + x*math.sin(a) + y*math.cos(a)) for x, y in ((-w/2,-h/2),(w/2,-h/2),(w/2,h/2),(-w/2,h/2))]
def pt_seg(p, a, b):
    ax, ay = a; bx, by = b; px, py = p
    dx, dy = bx-ax, by-ay; L = dx*dx+dy*dy
    t = 0 if L == 0 else max(0, min(1, ((px-ax)*dx+(py-ay)*dy)/L))
    return math.hypot(px-ax-t*dx, py-ay-t*dy)
def in_poly(p, poly):
    x, y = p; c = False
    for i in range(len(poly)):
        (x1,y1),(x2,y2) = poly[i], poly[(i+1)%len(poly)]
        if (y1 > y) != (y2 > y) and x < (x2-x1)*(y-y1)/(y2-y1)+x1: c = not c
    return c
def seg_seg(a,b,c,d):
    def cr(o,p,q): return (p[0]-o[0])*(q[1]-o[1])-(p[1]-o[1])*(q[0]-o[0])
    return cr(a,b,c)*cr(a,b,d) <= 0 and cr(c,d,a)*cr(c,d,b) <= 0
def dist_pt_poly(p, poly):
    if in_poly(p, poly): return 0
    return min(pt_seg(p, poly[i], poly[(i+1)%len(poly)]) for i in range(len(poly)))
def poly_seg_dist(poly, a, b):
    if in_poly(a, poly) or in_poly(b, poly): return 0
    for i in range(len(poly)):
        if seg_seg(poly[i], poly[(i+1)%len(poly)], a, b): return 0
    return min(min(pt_seg(q, a, b) for q in poly), min(pt_seg(a, poly[i], poly[(i+1)%len(poly)]) for i in range(len(poly))), min(pt_seg(b, poly[i], poly[(i+1)%len(poly)]) for i in range(len(poly))))
def polys_touch(p, q):
    if any(in_poly(x, q) for x in p) or any(in_poly(x, p) for x in q): return True
    for i in range(len(p)):
        for j in range(len(q)):
            if seg_seg(p[i], p[(i+1)%len(p)], q[j], q[(j+1)%len(q)]): return True
    return False
def touch(A, B):
    ka, kb = A['kind'], B['kind']
    def core(X):  # (type, data, radius)
        if X['kind'] == 'via': return ('pt', X['c'], X['d']/2)
        if X['kind'] == 'seg': return ('seg', (X['a'], X['b']), X['w']/2)
        if X['pshape'] in ('circle','oval') and abs(X['size'][0]-X['size'][1]) < 1e-9: return ('pt', X['c'], X['size'][0]/2)
        return ('poly', corners(X), 0)
    ta, da, ra = core(A); tb, db, rb = core(B)
    r = ra + rb
    if ta == 'pt' and tb == 'pt': return math.dist(da, db) <= r
    if ta == 'pt' and tb == 'seg': return pt_seg(da, *db) <= r
    if tb == 'pt' and ta == 'seg': return pt_seg(db, *da) <= r
    if ta == 'seg' and tb == 'seg':
        return seg_seg(*da, *db) or min(pt_seg(da[0],*db),pt_seg(da[1],*db),pt_seg(db[0],*da),pt_seg(db[1],*da)) <= r
    if ta == 'poly' and tb == 'poly': return polys_touch(da, db)
    if ta == 'poly': da, db, ta, tb = db, da, tb, ta
    # now tb == 'poly'
    if ta == 'pt': return dist_pt_poly(da, db) <= r
    return poly_seg_dist(db, *da) <= r
def zone_touch(Z, X):
    poly = Z['pts']
    if X['kind'] == 'pad':
        if X['pshape'] in ('circle','oval'): return dist_pt_poly(X['c'], poly) <= X['size'][0]/2
        return any(in_poly(c, poly) for c in corners(X))
    if X['kind'] == 'via': return dist_pt_poly(X['c'], poly) <= X['d']/2
    return in_poly(X['a'], poly) or in_poly(X['b'], poly)
def zone_area(Z):
    p = Z['pts']; return abs(sum(p[i][0]*p[(i+1)%len(p)][1]-p[(i+1)%len(p)][0]*p[i][1] for i in range(len(p))))/2
# row-bitmask union area
RES = 0.02
import os
NOZONE = bool(os.environ.get('NOZONE'))
def union_area(shapes, bbox):
    x0, y0, x1, y1 = bbox; x0 -= 0.1; y0 -= 0.1
    W = int((x1-x0)/RES)+8; H = int((y1+0.1-y0)/RES)+8
    rows = [0]*H
    def fillspan(r, xa, xb):
        if r < 0 or r >= H: return
        a = max(0, int(math.floor((xa-x0)/RES))); b = min(W-1, int(math.floor((xb-x0)/RES)))
        if b >= a: rows[r] |= ((1 << (b-a+1)) - 1) << a
    for typ, d, rad in shapes:
        if typ == 'poly':
            ys = [p[1] for p in d]; 
            for r in range(int((min(ys)-y0)/RES), int((max(ys)-y0)/RES)+2):
                y = y0+(r+0.5)*RES; xs = []
                for i in range(len(d)):
                    (xa,ya),(xb,yb) = d[i], d[(i+1)%len(d)]
                    if (ya > y) != (yb > y): xs.append(xa+(y-ya)*(xb-xa)/(yb-ya))
                xs.sort()
                for k in range(0, len(xs)-1, 2): fillspan(r, xs[k], xs[k+1])
        else:
            pa, pb = (d, d) if typ == 'pt' else d
            ymin = min(pa[1], pb[1])-rad; ymax = max(pa[1], pb[1])+rad
            for r in range(int((ymin-y0)/RES), int((ymax-y0)/RES)+2):
                y = y0+(r+0.5)*RES
                # x-interval of capsule at row y
                lo, hi = None, None
                dy = pb[1]-pa[1]; dx = pb[0]-pa[0]
                # sample endpoints circles + rectangle body via sweeping: exact for capsule by union of circle spans and body span
                spans = []
                for c in (pa, pb):
                    t = rad*rad-(y-c[1])**2
                    if t >= 0: s = math.sqrt(t); spans.append((c[0]-s, c[0]+s))
                L = math.hypot(dx, dy)
                if L > 0:
                    nx, ny = -dy/L*rad, dx/L*rad
                    body = [(pa[0]+nx,pa[1]+ny),(pb[0]+nx,pb[1]+ny),(pb[0]-nx,pb[1]-ny),(pa[0]-nx,pa[1]-ny)]
                    xs = []
                    for i in range(4):
                        (xa,ya),(xb,yb) = body[i], body[(i+1)%4]
                        if (ya > y) != (yb > y): xs.append(xa+(y-ya)*(xb-xa)/(yb-ya))
                    xs.sort()
                    for k in range(0, len(xs)-1, 2): spans.append((xs[k], xs[k+1]))
                for a, b in spans: fillspan(r, a, b)
    return sum(bin(r).count('1') for r in rows)*RES*RES
def shape_of(X):
    if X['kind'] == 'via': return ('pt', X['c'], X['d']/2)
    if X['kind'] == 'seg': return ('seg', (X['a'], X['b']), X['w']/2)
    if X['pshape'] in ('circle','oval') and abs(X['size'][0]-X['size'][1]) < 1e-9: return ('pt', X['c'], X['size'][0]/2)
    return ('poly', corners(X), 0)
def bbox_of(shs):
    xs=[];ys=[]
    for t,d,r in shs:
        pts = d if t=='poly' else ([d] if t=='pt' else list(d))
        for p in pts: xs.append(p[0]-r); ys.append(p[1]-r); xs.append(p[0]+r); ys.append(p[1]+r)
    return min(xs),min(ys),max(xs),max(ys)

for ref in sys.argv[2:]:
    pad9 = [i for i in items if i['kind']=='pad' and i['ref']==ref and i['num']=='9'][0]
    net = pad9['net']
    print(f"== {ref} EP pad 9: net {net}, centre {pad9['c']}, size {pad9['size']}, rot {pad9['rot']}, layers {pad9['layers']}, pad area {pad9['size'][0]*pad9['size'][1]:.3f} mm2")
    same = [i for i in items if i['net']==net]
    # per-layer elements and components (items only; zones handled apart)
    start_layer = pad9['layers'][0]
    # connectivity graph over (item index, layer); vias bridge layers; items link on a layer if touching
    nodes = [(k, l) for k, it in enumerate(same) for l in it['layers']]
    idx = {n: i for i, n in enumerate(nodes)}
    par = list(range(len(nodes)))
    def find(a):
        while par[a] != a: par[a] = par[par[a]]; a = par[a]
        return a
    def uni(a, b): par[find(a)] = find(b)
    for k, it in enumerate(same):      # multi-layer item (via, TH pad) joins its layers
        for l in it['layers'][1:]: uni(idx[(k, it['layers'][0])], idx[(k, l)])
    for l in CU:
        on = [k for k, it in enumerate(same) if l in it['layers']]
        for a in range(len(on)):
            for b in range(a+1, len(on)):
                if find(idx[(on[a], l)]) != find(idx[(on[b], l)]) and touch(same[on[a]], same[on[b]]): uni(idx[(on[a], l)], idx[(on[b], l)])
        for z in zones:
            if z['layer'] == l and z['net'] == net:
                zk = ('Z', l); 
                for k in on:
                    if zone_touch(z, same[k]): 
                        # mark zone joined: union with a sentinel node per zone
                        pass
    # zone joins: a zone is joined to every same-net item that lies in its outline on its layer (KiCad refill connects them)
    zj = {}
    for z in zones:
        if z['net'] != net or NOZONE: continue
        mem = [k for k, it in enumerate(same) if z['layer'] in it['layers'] and zone_touch(z, same[k])]
        zj[z['layer']] = mem
        for a in mem[1:]: uni(idx[(mem[0], z['layer'])], idx[(a, z['layer'])])
    root_pad = find(idx[(next(k for k, it in enumerate(same) if it is pad9), start_layer)])
    comp = [(k, l) for (k, l) in nodes if find(idx[(k, l)]) == root_pad]
    print(f"   same-net items: {len(same)}; items in the pad's joined set: {len(set(k for k,l in comp))}")
    vias = [same[k] for k, l in comp if same[k]['kind']=='via' and l=='F.Cu']
    inpad = [v for v in vias if abs(v['c'][0]-pad9['c'][0]) <= 1e-6+0 or True]
    def within(v):
        a = math.radians(pad9['rot']); dx, dy = v['c'][0]-pad9['c'][0], v['c'][1]-pad9['c'][1]
        lx = dx*math.cos(a)-dy*math.sin(a); ly = dx*math.sin(a)+dy*math.cos(a)
        return abs(lx) <= pad9['size'][0]/2 and abs(ly) <= pad9['size'][1]/2
    thv = [v for v in vias if within(v)]
    print(f"   vias with centre inside pad 9: {len(thv)}  (size/drill {sorted(set((v['d'],v['drill']) for v in thv))})")
    print(f"   {'layer':7} {'pad+tracks+vias (cm2)':>24} {'of which tracks':>16} {'zone outline cm2':>17} zone filled?")
    for l in CU:
        part = [same[k] for k, ll in comp if ll == l]
        # local copper = everything joined EXCEPT that reached only via zone? report both with and without zone-linked pieces
        shs = [shape_of(p) for p in part]
        ar = union_area(shs, bbox_of(shs)) if shs else 0
        tr = [p for p in part if p['kind']=='seg']
        ar_tr = union_area([shape_of(p) for p in tr], bbox_of([shape_of(p) for p in tr])) if tr else 0
        zl = [z for z in zones if z['layer']==l and z['net']==net]
        za = sum(zone_area(z) for z in zl)
        print(f"   {l:7} {ar/100:24.3f} {ar_tr/100:16.3f} {za/100:17.2f} {'; '.join('FILLED' if z['filled'] else 'UNFILLED' for z in zl) or '-'}   (items {len(part)}; zone-joined members {len(zj.get(l, []))})")
