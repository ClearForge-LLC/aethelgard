"""S4 summary: one row per (route, backend, quantum, R) from the matrix logs and each run's files.

Usage: python3 -I s4-summary.py <dir with *.seq/*.costs.csv> <log>...
Per row: quanta, loop wall time of runs 1 and 2, per-quantum mean, the run-1 split of a quantum
(Renode step incl. 12 reads / ngspice advance / fault command), run 1 == run 2 on the state-sequence
sha256, and the firmware's final counters (attempted, OK, wrong, timed out, last status).
"""
import collections, csv, os, re, sys

d = sys.argv[1]
runs = collections.OrderedDict()
for log in sys.argv[2:]:
    for line in open(log):
        m = re.match(r"^(\S+)-r(\d) (.*)$", line.strip())
        if not m or "quanta=" not in line:
            continue
        tag, run, rest = m.group(1), m.group(2), m.group(3)
        kv = dict(re.findall(r"(\w+)=(\S+)", rest))
        runs.setdefault(tag, {})[run] = kv

print("route backend quantum R | quanta | loop_s r1 r2 | ms/quantum | split ms: renode spice fault | r1==r2 seq_sha256 | firmware at the span's end: attempted ok wrong timedout last_status (1 TXIS, 3 RXNE)")
for tag, rs in runs.items():
    route, be, q, r = tag.split("-", 3)
    r1 = rs.get("1", {})
    r2 = rs.get("2", {})
    split = "-"
    p = os.path.join(d, tag + "-r1.costs.csv")
    if os.path.exists(p):
        rows = list(csv.DictReader(open(p)))
        n = len(rows)
        split = "%.2f %.2f %.2f" % tuple(sum(int(x[k]) for x in rows) / n / 1000 for k in ("renode_us", "spice_us", "fault_us"))
    fw = "-"
    p = os.path.join(d, tag + "-r1.seq")
    if os.path.exists(p):
        last = open(p).read().splitlines()[-1]
        vals = re.search(r"renode=(\S+)", last).group(1).split(",")
        if len(vals) >= 6:
            fw = " ".join(str(int(v, 16)) for v in vals[1:6])
    same = "n/a (one run)"
    if r1.get("seq_sha256") and r2.get("seq_sha256"):
        same = ("yes " if r1["seq_sha256"] == r2["seq_sha256"] else "NO ") + r1["seq_sha256"][:16]
    elif r1.get("seq_sha256"):
        same = "one run " + r1["seq_sha256"][:16]
    print("%s %s %sns %s | %s | %s %s | %s | %s | %s | %s" % (
        route, be, q[1:], r, r1.get("quanta", "?"), r1.get("loop_s", "?"), r2.get("loop_s", "-"),
        r1.get("per_quantum_ms", "?"), split, same, fw))
