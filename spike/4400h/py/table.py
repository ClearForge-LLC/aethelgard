"""AET-WO-4400h (1): the window table from each run's row.json (window.sh), in run order. Usage: table.py WIN_DIR > window.tsv"""
import json, os, sys
d = sys.argv[1]
rows = []
for n in os.listdir(d):
    p = os.path.join(d, n, "row.json")
    if os.path.exists(p):
        r = json.load(open(p)); r["mtime"] = os.path.getmtime(p); rows.append(r)
rows.sort(key=lambda r: r["mtime"])
cols = ["name", "window", "bounds", "usart_log", "cpus", "exit", "ok", "wall_s", "peak_rss_kib", "permille_of_1572864",
        "load1_min", "load1_max", "log_kept_bytes", "log_lines_dropped", "failing_kinds"]
print("\t".join(cols))
for r in rows:
    env = open(os.path.join(d, r["name"], "env.txt")).read()
    v = json.load(open(os.path.join(d, r["name"], "verdict.json")))
    b = v.get("boot", {})
    bounds = "main (30 s, 1,572,864 KiB)"
    if "AET4400H_WALL_S" in env:
        bounds = "wall 600 s, " + ("RSS 6 GiB (measure)" if "AET4400H_RSS_KIB" in env else "RSS 1,572,864 KiB")
    r.update(bounds=bounds, usart_log="off" if "NO_USART_LOG" in env else "on",
             log_kept_bytes=b.get("log_kept_bytes"), log_lines_dropped=b.get("log_lines_dropped"),
             failing_kinds=",".join(r["kinds"]) or "-")
    print("\t".join(str(r.get(c)) for c in cols))
