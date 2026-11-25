# AET-WO-4411 option 1: read an efficiency figure from its PDF vectors. usage: PYTHONPATH=<pymupdf> python3 vec-eff.py PDF PAGE(1-based) X0 X1 Y0 Y1 AMPS...
# X0..X1: the plot's horizontal span in points (picks one of the page's charts). Axes from the tick labels: '0.001' and the last decade at x,
# the efficiency labels at y (linear). Curves: each legend label's swatch colour (the line drawn just left of the label).
import sys, math, fitz
pdf, page, x0, x1, y0, y1 = sys.argv[1], int(sys.argv[2]) - 1, *map(float, sys.argv[3:7])
amps = [float(a) for a in sys.argv[7:]]
pg = fitz.open(pdf)[page]
words = [w for w in pg.get_text('words') if x0 - 25 <= w[0] <= x1 and y0 <= w[1] <= y1]
num = lambda s: s.replace('.', '', 1).isdigit()
cx = lambda w: (w[0] + w[2]) / 2; cy = lambda w: (w[1] + w[3]) / 2
lab = [w for w in words if num(w[4])]
xs = sorted([w for w in lab if w[4] in ('0.001', '0.01', '0.1', '1', '10')], key=cx)
bottom = max(cy(w) for w in xs); xs = [w for w in xs if abs(cy(w) - bottom) < 2]
xmap = [(math.log10(float(w[4])), cx(w)) for w in xs]
ys = [w for w in lab if w[2] < xs[0][0] + 5 and abs(cy(w) - bottom) > 3 and cy(w) < bottom]
ymap = sorted((float(w[4]), cy(w)) for w in ys)
(la, xa), (lb, xb) = xmap[0], xmap[-1]
(ea, ya), (eb, yb) = ymap[0], ymap[-1]
X = lambda a: xa + (math.log10(a) - la) * (xb - xa) / (lb - la)
E = lambda y: ea + (y - ya) * (eb - ea) / (yb - ya)
print('x decades', [w[4] for w in xs], 'y', ymap[0][0], '..', ymap[-1][0])
leg = [w for w in words if w[4].endswith('V') and w[4][:-1].isdigit()]
segs = []
for d in pg.get_drawings():
    col = d.get('color')
    for it in d['items']:
        if it[0] in ('l', 'c'): a, b = it[1], it[-1]
        else: continue
        if y0 <= min(a.y, b.y) and max(a.y, b.y) <= y1 and x0 - 25 <= min(a.x, b.x) <= x1: segs.append((col, a.x, a.y, b.x, b.y, d.get('width')))
for L in leg:
    sw = [s for s in segs if s[0] and abs((s[2] + s[4]) / 2 - cy(L)) < 2.5 and s[3] <= L[0] + 1 and s[3] > L[0] - 25]
    if not sw: print(L[4], 'no swatch'); continue
    col = sw[0][0]
    curve = [s for s in segs if s[0] == col and min(s[1], s[3]) >= xa - 1 and max(s[1], s[3]) <= xb + 1 and s not in sw]
    for a in amps:
        x = X(a); hit = [s for s in curve if min(s[1], s[3]) <= x <= max(s[1], s[3]) and s[1] != s[3]]
        vals = [E(s[2] + (x - s[1]) * (s[4] - s[2]) / (s[3] - s[1])) for s in hit]
        print(f'{L[4]:>4} colour {tuple(round(c, 2) for c in col)} at {a} A: ' + (', '.join(f'{v:.2f} %' for v in vals if 0 < v <= 100) or 'no segment'))
