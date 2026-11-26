"""AET-WO-4400j: arm12.json cut to the frames that end inside a window (each byte 100 us apart, `fill` zero bytes
after its MAVLink bytes), so a shorter window takes the same input's prefix. Usage: cut.py IN WINDOW_MS OUT"""
import json, sys
src, w, out = sys.argv[1], int(sys.argv[2]) * 10**6, sys.argv[3]
d = json.load(open(src))
spacing = 100_000
keep = [f for f in d["frames"] if f["at_ns"] + (len(f["mavlink"]) // 2 + f.get("fill", 0)) * spacing <= w - spacing]
d["frames"] = keep
json.dump(d, open(out, "w"))
print(out, len(keep), "frames, last at", keep[-1]["at_ns"] if keep else None)
