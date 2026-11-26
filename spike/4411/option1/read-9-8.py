# AET-WO-4411: Figure 9-8 (SNVSAN3F p. 27, the held copy sha256 3b0920a4…) read at U11's load with the module. Render: gs -r200 -dFirstPage=27 -dLastPage=27 -sDEVICE=png16m (the tools image). Usage: python3 -I read.py p27.png
# Digitize Murata's DC-bias chart (AET-WO-4300d3 amendment). Pure Python PNG decode.
import zlib, struct, sys
p = sys.argv[1]
b = open(p, 'rb').read(); i = 8; idat = b''; plte = None
while i < len(b):
    n = struct.unpack('>I', b[i:i+4])[0]; t = b[i+4:i+8]; d = b[i+8:i+8+n]; i += 12 + n
    if t == b'IHDR': w, h, bd, ct, _, _, il = struct.unpack('>IIBBBBB', d)
    if t == b'PLTE': plte = [tuple(d[k:k+3]) for k in range(0, len(d), 3)]
    if t == b'IDAT': idat += d
assert bd == 8 and il == 0, (bd, il)
bpp = {2: 3, 6: 4, 3: 1, 0: 1, 4: 2}[ct]
raw = zlib.decompress(idat); stride = w * bpp; rows = []; prev = bytearray(stride); o = 0
for y in range(h):
    f = raw[o]; o += 1; line = bytearray(raw[o:o+stride]); o += stride
    for x in range(stride):
        a = line[x-bpp] if x >= bpp else 0; c = prev[x-bpp] if x >= bpp else 0; up = prev[x]
        if f == 1: line[x] = (line[x] + a) & 255
        elif f == 2: line[x] = (line[x] + up) & 255
        elif f == 3: line[x] = (line[x] + ((a + up) >> 1)) & 255
        elif f == 4:
            pp = a + up - c; pa = abs(pp - a); pb = abs(pp - up); pc = abs(pp - c)
            pr = a if pa <= pb and pa <= pc else (up if pb <= pc else c); line[x] = (line[x] + pr) & 255
    rows.append(bytes(line)); prev = line
def px(x, y):
    r = rows[y]
    if ct == 3: return plte[r[x]]
    if ct == 0: return (r[x],) * 3
    return tuple(r[x*bpp:x*bpp+3])
print('size', w, h, 'colour type', ct)
dark = lambda c: max(c) < 90 and abs(c[0]-c[1]) < 20 and abs(c[1]-c[2]) < 20
# The frame: the longest dark vertical and horizontal runs.
cols = [sum(dark(px(x, y)) for y in range(h)) for x in range(w)]
import math
XL, XR, YT, YB = 987, 1489, 372.5, 713.0   # 0.001 A, 10 A; 100 %, 30 %
def x_of(a): return XL + (XR - XL) * (math.log10(a) + 3) / 4
def eff(y): return 100 - (y - YT) * 70 / (YB - YT)
teal = lambda c: c[2] > c[0] + 35 and c[1] > c[0] + 10 and max(c) < 200
red = lambda c: c[0] > 170 and c[1] < 90 and c[2] < 110
for a in [0.2, 0.3, 0.4, 0.5, 0.6]:
    x = round(x_of(a))
    for name, f in [('36V', teal), ('12V', red)]:
        ys = [y for y in range(int(YT), int(YB)) for dx in (-1, 0, 1) if f(px(x + dx, y))]
        if ys:
            print(f'{a} A x={x} {name}: y {min(ys)}..{max(ys)} -> {eff(max(ys)):.1f}..{eff(min(ys)):.1f} %')
