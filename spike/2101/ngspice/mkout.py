"""Builds spike/2101/out/s1-*.txt summaries from the raw per-transient TSVs in /scr (ng-scratch)."""
import sys, os, re, hashlib
sys.path.insert(0, os.path.dirname(__file__))
import common
S = "/scr"; O = "/repo/spike/2101/out"
def load(p):
    startup = None; rows = []
    for ln in open(p):
        if ln.startswith("#"): startup = int(ln.split("\t")[1]); continue
        f = ln.rstrip("\n").split("\t"); rows.append(f)
    return startup, rows
def agg(rows): return hashlib.sha256("".join(r[2] + "\n" for r in rows).encode()).hexdigest()
CMD = {"a": "python3 s1_a.py 1000 /scr/aN.tsv  (x2; ngspice -b per transient, wrdata numdgt=15 text hashed)",
       "b": "python3 s1_b.py 1000 /scr/bN.tsv  (x2; ngspice -p -t dumb under `script` pty, alter @vctl[pwl]=[..]; run; wrdata)",
       "c-clamp": "s1c s1 clamp 1000 /cache/ng-scratch/cclampN.tsv  (x2; libngspice, delta clamp in GetSyncData; raw f64 time+line hashed)",
       "c-bkpt": "s1c s1 bkpt 1000 /cache/ng-scratch/cbkptN.tsv  (x2; libngspice, ngSpice_SetBkpt at all 16 edges before each run)"}
files = {"a": ("a1", "a2"), "b": ("b1", "b2"), "c-clamp": ("cclamp1", "cclamp2"), "c-bkpt": ("cbkpt1", "cbkpt2")}
summ = []
res = {}
for route, (f1, f2) in files.items():
    s1, r1 = load(f"{S}/{f1}.tsv"); s2, r2 = load(f"{S}/{f2}.tsv")
    same = [x[2] for x in r1] == [x[2] for x in r2]
    uniq = len(set(x[2] for x in r1))
    res[route] = (agg(r1), [x[2] for x in r1])
    out = [f"# cmd: IMG=$(cat container/IMAGE); podman run ... $IMG {CMD[route]}", f"# routes file: {route}"]
    for tag, st, rows in (("run1", s1, r1), ("run2", s2, r2)):
        out.append(f"{tag} startup_ms={st / 1e6:.3f}" if st is not None else f"{tag} startup n/a")
        out.append(common.fmt_stats(tag, [int(x[1]) for x in rows]))
        out.append(f"{tag} aggregate_sha256={agg(rows)}")
    out.append(f"run1==run2 per-transient hashes identical: {same}   distinct hashes in run1: {uniq}/1000")
    if route.startswith("c"):
        for tag, rows in (("run1", r1),):
            vs = [int(x[4]) for x in rows]; sy = [int(x[5]) for x in rows]; rd = [int(x[6]) for x in rows]; tr = [int(x[7]) for x in rows]; cl = [int(x[8]) for x in rows]; pts = [int(x[3]) for x in rows]
            out.append(f"GetVSRCData calls/transient min/median/max: {min(vs)}/{sorted(vs)[500]}/{max(vs)}  (mean {sum(vs)/1000:.1f})")
            out.append(f"GetSyncData calls/transient min/median/max: {min(sy)}/{sorted(sy)[500]}/{max(sy)}  (mean {sum(sy)/1000:.1f})")
            out.append(f"accepted points/transient min/median/max: {min(pts)}/{sorted(pts)[500]}/{max(pts)}")
            out.append(f"GetSyncData calls with ngspice redo flag!=0 (rejected step redone by ngspice itself) per transient min/median/max: {min(rd)}/{sorted(rd)[500]}/{max(rd)}; transients with any: {sum(1 for x in rd if x)}")
            out.append(f"GetSyncData calls where time went backwards: total {sum(tr)}")
            out.append(f"delta clamps applied by host per transient: min/max {min(cl)}/{max(cl)}")
    out.append("first 5 / last 3 per-transient (i, ns, sha256):")
    for x in r1[:5] + r1[-3:]: out.append(f"  {x[0]}\t{x[1]}\t{x[2]}")
    open(f"{O}/s1-route-{route}.txt", "w").write("\n".join(out) + "\n")
    st = common.stats([int(x[1]) for x in r1])
    summ.append((route, s1, st, same, agg(r1)))
# cross-route
ab = res["a"][1] == res["b"][1]
lines = ["# cmd: python3 mkout.py (reads /scr/*.tsv; see s1-route-*.txt for each route's own command)", "S1 timing, run 1 (ms per transient), 1000 transients, R=2.2k, byte=i%256, ack 0, .tran 1n 22.5u 0 5n", "route\tstartup_ms\tmin\tmedian\tp95\tmax\ttotal\trun1==run2"]
for route, s1, st, same, a in summ:
    lines.append("%s\t%s\t%.3f\t%.3f\t%.3f\t%.3f\t%.1f\t%s" % (route, ("%.3f" % (s1 / 1e6)) if s1 is not None else "(n/a: per-process, in every transient)", st['min'] / 1e6, st['median'] / 1e6, st['p95'] / 1e6, st['max'] / 1e6, st['total'] / 1e6, same))
lines.append(f"route a per-transient wrdata hashes == route b per-transient wrdata hashes (all 1000): {ab}")
lines.append(f"c-clamp vs c-bkpt per-transient hashes equal count: {sum(1 for x, y in zip(res['c-clamp'][1], res['c-bkpt'][1]) if x == y)}/1000 (differ: clamp and bkpt choose different step points)")
lines.append("c vs a/b: raw-f64 hashes vs text hashes are not comparable; agreement is on rise time, see s2-rise.txt")
open(f"{O}/s1-summary.txt", "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
