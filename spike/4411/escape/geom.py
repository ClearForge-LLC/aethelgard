# AET-WO-4411 lever (1): U1's VDD_3V3 and GND pads, the vias of those nets near
# U1, and the plane's feed (L2 pad 2), from an emitted board.
# Usage: geom.py <board.kicad_pcb> [radius_mm]
import math, re, sys

text = open(sys.argv[1]).read()
R = float(sys.argv[2]) if len(sys.argv) > 2 else 2.5


def blocks(s, head):
    out = []
    for m in re.finditer(r"\(" + head + r"[\s]", s):
        i = m.start()
        if out and i < out[-1][1]:
            continue
        d, j = 0, i
        while True:
            c = s[j]
            if c == "(":
                d += 1
            elif c == ")":
                d -= 1
                if d == 0:
                    break
            elif c == '"':
                j = s.index('"', j + 1)
            j += 1
        out.append((s[i:j + 1], j + 1))
    return [b for b, _ in out]


def at(b):
    m = re.search(r"\(at ([-\d.]+) ([-\d.]+)(?: ([-\d.]+))?\)", b)
    return float(m.group(1)), float(m.group(2)), float(m.group(3) or 0)


pads = {}
for fp in blocks(text, "footprint"):
    ref = re.search(r'\(property "Reference" "([^"]+)"', fp).group(1)
    fx, fy, fr = at(fp)
    for p in blocks(fp, "pad"):
        num = re.match(r'\(pad "([^"]*)"', p).group(1)
        px, py, _ = at(p)
        t = math.radians(-fr)
        x = fx + px * math.cos(t) - py * math.sin(t)
        y = fy + px * math.sin(t) + py * math.cos(t)
        net = re.search(r'\(net (?:\d+ )?"([^"]*)"\)', p)
        pads[(ref, num)] = (x, y, net.group(1) if net else "")
vias = []
for v in blocks(text, "via"):
    x, y, _ = at(v)
    net = re.search(r'\(net (?:\d+ )?"([^"]*)"\)', v)
    vias.append((x, y, net.group(1) if net else ""))
feed = pads[("L2", "2")]
print(f"feed L2.2 at {feed[0]:.3f} {feed[1]:.3f}")
u1 = {n: v for (r, n), v in pads.items() if r == "U1" and v[2] in ("VDD_3V3", "GND")}
for n, (x, y, net) in sorted(u1.items(), key=lambda kv: int(kv[0]) if kv[0].isdigit() else 999):
    near = sorted((math.hypot(vx - x, vy - y), vx, vy, vn) for vx, vy, vn in vias if vn in ("GND", "VDD_3V3"))
    near = [f"{vn}@({vx:.3f},{vy:.3f}) d={d:.2f}" for d, vx, vy, vn in near if d <= R]
    print(f"U1.{n:>4} {net:8} ({x:.3f},{y:.3f})  " + "; ".join(near[:4]))
