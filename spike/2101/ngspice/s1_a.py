"""Route a: one `ngspice -b` process per transient. Usage: s1_a.py N OUT_TSV  (R=2.2k, byte=i%256, ack 0)."""
import subprocess, sys, time, os
sys.path.insert(0, os.path.dirname(__file__))
import deck, common
N = int(sys.argv[1]); out = sys.argv[2]
work = "/tmp/ra"; os.makedirs(work, exist_ok=True)
rows = []
for i in range(N):
    d = os.path.join(work, "d.cir"); o = os.path.join(work, "o.dat")
    if os.path.exists(o): os.remove(o)
    lines = deck.circuit_lines(2200, i % 256, 0) + [".control", "set numdgt=15", "run", "wrdata %s v(line)" % o, "quit", ".endc", ".end"]
    open(d, "w").write("\n".join(lines) + "\n")
    t0 = time.perf_counter_ns()
    r = subprocess.run(["ngspice", "-b", d], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    ns = time.perf_counter_ns() - t0
    assert r.returncode == 0, r.returncode
    b = open(o, "rb").read()
    rows.append((i, ns, common.sha(b), len(b)))
with open(out, "w") as f:
    for r in rows: f.write("%d\t%d\t%s\t%d\n" % r)
