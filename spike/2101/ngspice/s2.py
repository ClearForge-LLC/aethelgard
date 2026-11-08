"""S2 for routes a and b. Prints one line per case: route R byte ack: .meas rise, vector rise, theory, peak, max in-window."""
import subprocess, sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
import deck, common
import s1_b
CASES = [(2200, 0x7F, 1), (47000, 0x7F, 1), (2200, 0x55, 0), (47000, 0x55, 0)]
M = deck.MEAS
def report(route, r, byte, ack, meas_txt, dat):
    t, v = common.parse_wrdata(dat)
    m = re.search(r"trise\s*=\s*([-+0-9.eE]+)", meas_txt)
    meas = float(m.group(1)) if m else None
    vr = common.rise(t, v)
    th = common.theory(r)
    peak = max(v)
    # per release window peak (bits that are 1 after a 0): max within each 2.5 us window
    w = []
    for k in range(9):
        vs = [x for tt, x in zip(t, v) if k * deck.BIT <= tt <= (k + 1) * deck.BIT]
        w.append(max(vs))
    print("route=%s R=%g byte=%02x ack=%d meas_rise_ns=%s vector_rise_ns=%s theory_ns=%.2f peak_v=%.4f reached_70pct=%s window_peaks=%s" % (
        route, r, byte, ack, ("%.4f" % (meas * 1e9)) if meas else "none", ("%.4f" % (vr * 1e9)) if vr else "none", th * 1e9, peak, peak >= 2.31,
        ",".join("%.3f" % x for x in w)))
    if meas:
        print("   meas vs theory: %+.3f%% ; vs 300 ns ceiling: %+.1f ns (%s)" % ((meas - th) / th * 100, meas * 1e9 - 300, "exceeds" if meas * 1e9 > 300 else "below"))
os.makedirs("/tmp/s2", exist_ok=True)
o = "/tmp/s2/o.dat"
for r, byte, ack in CASES:
    d = "/tmp/s2/d.cir"
    lines = deck.circuit_lines(r, byte, ack) + [".control", "set numdgt=15", "run", M, "wrdata %s v(line)" % o, "quit", ".endc", ".end"]
    open(d, "w").write("\n".join(lines) + "\n")
    if os.path.exists(o): os.remove(o)
    p = subprocess.run(["ngspice", "-b", d], capture_output=True, text=True)
    report("a", r, byte, ack, p.stdout + p.stderr, open(o).read())
open("/tmp/s2/clean.cir", "w").write("\n".join(deck.circuit_lines(2200, 0, 0)) + "\n.end\n")
ng = s1_b.Ng()
ng.send("set numdgt=15"); ng.send("source /tmp/s2/clean.cir"); ng.sync()
for r, byte, ack in CASES:
    ng.send("alter r1 = %g" % r)
    ng.send("alter @vctl[pwl] = [ %s ]" % deck.pwl_str(byte, ack))
    if os.path.exists(o): os.remove(o)
    ng.send("run"); ng.send(M); ng.send("wrdata %s v(line)" % o)
    txt = ng.sync()
    report("b", r, byte, ack, txt, open(o).read())
ng.close()
