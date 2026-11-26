"""AET-WO-4400j (1): the CLI-path table from every run's row.json (sh/cli.sh) whose name starts with one of PREFIXES:
per (board, input, window), n, the verdicts, wall s, peak KiB and its share of 1,572,864 KiB, the kept log against
8 MiB, the arm, and the load. Also writes OUT.tsv with every row. Usage: table.py CLIDIR OUT PREFIX..."""
import collections, glob, json, os, statistics, sys
d, out, prefixes = sys.argv[1], sys.argv[2], tuple(sys.argv[3:])
rows = []
for p in sorted(glob.glob(os.path.join(d, "*", "row.json"))):
    r = json.load(open(p))
    if r["name"].startswith(prefixes) and r.get("verdict"):
        rows.append(r)
with open(out + ".tsv", "w") as f:
    keys = ["name", "board", "input", "window", "verdict", "kinds", "wall_s", "peak_rss_kib", "permille_of_cap",
            "log_kept_bytes", "armed", "heartbeats", "load_min", "load_max"]
    f.write("\t".join(keys) + "\n")
    for r in rows:
        f.write("\t".join(str(r.get(k)) for k in keys) + "\n")
g = collections.defaultdict(list)
for r in rows:
    w = r["window"]
    ms = int(w[:-2]) / 1000 if w.endswith("us") else int(w[:-2])
    g[(r["board"], r["input"], ms)].append(r)
print("| board | input | window | n | verdicts | wall s | peak KiB (‰ of cap) | kept log B (% of 8 MiB) | armed | load |")
print("|---|---|---|---|---|---|---|---|---|---|")
for (b, i, w), rs in sorted(g.items()):
    v = collections.Counter(r["verdict"] + ("" if r["verdict"] == "pass" else ": " + ",".join(
        k for k in r["kinds"] if k not in ("device_unmodelled", "locked_peripheral_access", "serial_pull"))) for r in rs)
    pk = [r["peak_rss_kib"] for r in rs]; lg = [r["log_kept_bytes"] for r in rs]; wl = [r["wall_s"] for r in rs]
    rng = lambda xs, f="{:,}": f.format(min(xs)) if min(xs) == max(xs) else (f + "–" + f).format(min(xs), max(xs))
    print(f"| {b} | {'yes' if i else 'no'} | {('%.4f' % (w / 1000)).rstrip('0').rstrip('.')} s | {len(rs)} | {', '.join(f'{k} ×{n}' for k, n in v.items())} "
          f"| {rng(wl, '{:.1f}')} | {rng(pk)} ({round(min(pk) * 1000 / 1572864)}–{round(max(pk) * 1000 / 1572864)}) "
          f"| {rng(lg)} ({100 * min(lg) / 8388608:.0f}–{100 * max(lg) / 8388608:.0f} %) "
          f"| {','.join(str(r.get('armed', '-')) for r in rs)} | {min(r['load_min'] for r in rs):.0f}–{max(r['load_max'] for r in rs):.0f} |")
