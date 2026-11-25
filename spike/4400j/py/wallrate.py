"""AET-WO-4400j (1): RSS growth per wall second from a replay's rss.txt (wall stamp, VmRSS KiB every 100 ms), in
windows of the run: is the growth paced by wall time (the EOF reader) or by virtual time (the firmware's traffic)?
Usage: wallrate.py RSS.txt [WINDOW_S]"""
import sys
pts = [(float(a), int(b)) for a, b in (l.split() for l in open(sys.argv[1])) if int(b) > 0]
w = float(sys.argv[2]) if len(sys.argv) > 2 else 5.0
t0 = pts[0][0]; out = []
i = 0
while i < len(pts):
    j = i
    while j < len(pts) and pts[j][0] - pts[i][0] < w: j += 1
    if j >= len(pts): break
    out.append((pts[i][0] - t0, pts[j][0] - t0, (pts[j][1] - pts[i][1]) / 1024 / (pts[j][0] - pts[i][0])))
    i = j
for a, b, r in out: print(f"wall {a:5.1f}-{b:5.1f} s: {r:7.1f} MiB per wall second")
print(f"whole run: {(pts[-1][1] - pts[0][1]) / 1024 / (pts[-1][0] - pts[0][0]):.1f} MiB/s over {pts[-1][0] - pts[0][0]:.1f} s")
