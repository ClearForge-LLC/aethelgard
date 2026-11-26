"""AET-WO-4412 (from 4400j): one CLI run's row from its directory (cli.sh). Usage: row.py DIR RC T0 T1 WINDOW NAME BOARD [input]"""
import json, sys
o, rc, t0, t1, w, name, board = sys.argv[1:8]
inp = len(sys.argv) > 8 and sys.argv[8] == "input"
try:
    v = json.load(open(o + "/verdict.json"))
except Exception:
    v = {}
b = v.get("boot", {}) if isinstance(v, dict) else {}
loads = [float(l.split()[1]) for l in open(o + "/load.txt") if len(l.split()) > 1]
row = {
    "name": name, "board": board, "window": w, "input": inp, "rc": int(rc),
    "wall_s": round(float(t1) - float(t0), 2),
    "load_min": min(loads, default=None), "load_max": max(loads, default=None),
    "verdict": v.get("verdict"), "kinds": sorted({f.get("kind") for f in v.get("findings", []) if isinstance(f, dict)}),
    "error": v.get("error", {}).get("kind") if isinstance(v.get("error"), dict) else None,
    "peak_rss_kib": None, "log_kept_bytes": b.get("log_kept_bytes"), "log_lines_dropped": b.get("log_lines_dropped"),
}
def find(d, key, acc):
    if isinstance(d, dict):
        for k, x in d.items():
            if k == key: acc.append(x)
            find(x, key, acc)
    elif isinstance(d, list):
        for x in d: find(x, key, acc)
    return acc
pk = find(v, "peak_rss_kib", [])
row["peak_rss_kib"] = pk[0] if pk else None
if row["peak_rss_kib"]:
    row["permille_of_cap"] = round(row["peak_rss_kib"] * 1000 / 1572864)
w_ = b.get("input", {}).get("witness") if isinstance(b.get("input"), dict) else None
if w_:
    row["armed"] = w_.get("armed") is not None
    row["heartbeats"] = w_.get("heartbeats")
print(json.dumps(row))
