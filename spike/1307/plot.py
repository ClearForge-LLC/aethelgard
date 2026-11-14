#!/usr/bin/env python3
"""AET-WO-1307 S1 figure: render a router dump (AET_DUMP JSON from the probe) to a PNG, stdlib only.

Usage: plot.py <dump.json> <out.png> <ref> <pad_lo> <pad_hi> [margin_mm]
Crops to the named part's pads pad_lo..pad_hi plus a margin. Colours:
  pads: GND dark green, VDD_3V3 orange, other nets light grey, no net dark grey;
  F.Cu tracks red, B.Cu tracks blue; vias: GND green, VDD_3V3 yellow, others magenta;
  pads pad_lo..pad_hi of the part outlined in black.
"""
import json, struct, sys, zlib

dump, out, ref, lo, hi = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
margin = float(sys.argv[6]) * 1e6 if len(sys.argv) > 6 else 3e6
d = json.load(open(dump))
PX = 40 / 1e6  # 40 px per mm

sel = [p for p in d["pads"] if p[0].split()[0] == ref and p[0].split()[1].isdigit() and lo <= int(p[0].split()[1]) <= hi]
x0 = min(p[2] for p in sel) - margin; y0 = min(p[3] for p in sel) - margin
x1 = max(p[4] for p in sel) + margin; y1 = max(p[5] for p in sel) + margin
W, H = int((x1 - x0) * PX) + 1, int((y1 - y0) * PX) + 1
img = [[(255, 255, 255)] * W for _ in range(H)]


def px(x, y):
    return int((x - x0) * PX), int((y - y0) * PX)


def rect(ax, ay, bx, by, c):
    (i0, j0), (i1, j1) = px(min(ax, bx), min(ay, by)), px(max(ax, bx), max(ay, by))
    for j in range(max(j0, 0), min(j1 + 1, H)):
        row = img[j]
        for i in range(max(i0, 0), min(i1 + 1, W)):
            row[i] = c


def disc(cx, cy, r, c):
    (ci, cj), rp = px(cx, cy), r * PX
    for j in range(int(cj - rp) - 1, int(cj + rp) + 2):
        for i in range(int(ci - rp) - 1, int(ci + rp) + 2):
            if 0 <= i < W and 0 <= j < H and (i - ci) ** 2 + (j - cj) ** 2 <= rp * rp:
                img[j][i] = c


def net_colour(n, plane, other):
    return plane.get(n, other)


for name, net, ax, ay, bx, by in d["pads"]:
    c = net_colour(net, {"GND": (0, 110, 0), "VDD_3V3": (240, 140, 0), "": (90, 90, 90)}, (190, 190, 190))
    rect(ax, ay, bx, by, c)
    r, n = name.split()[0], name.split()[1]
    if r == ref and n.isdigit() and lo <= int(n) <= hi:
        for (px0, py0, px1, py1) in [(ax, ay, bx, ay), (ax, by, bx, by), (ax, ay, ax, by), (bx, ay, bx, by)]:
            rect(px0, py0, px1, py1, (0, 0, 0))
w = d["w"] / 2
for net, layer, sx, sy, ex, ey in d["segs"]:
    rect(min(sx, ex) - w, min(sy, ey) - w, max(sx, ex) + w, max(sy, ey) + w, (220, 30, 30) if layer == 0 else (40, 60, 230))
for net, vx, vy in d["vias"]:
    disc(vx, vy, 300000, net_colour(net, {"GND": (0, 200, 0), "VDD_3V3": (250, 220, 0)}, (200, 0, 200)))
    disc(vx, vy, 150000, (255, 255, 255))

raw = b"".join(b"\x00" + bytes(v for p in row for v in p) for row in img)


def chunk(t, b):
    return struct.pack(">I", len(b)) + t + b + struct.pack(">I", zlib.crc32(t + b) & 0xFFFFFFFF)


open(out, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
                      + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
print(f"{out}: {W}x{H} px, window ({x0/1e6:.2f},{y0/1e6:.2f})-({x1/1e6:.2f},{y1/1e6:.2f}) mm")
