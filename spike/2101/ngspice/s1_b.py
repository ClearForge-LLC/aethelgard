"""Route b: ONE long-lived ngspice (-p -t dumb) under a pty (`script`), commands on stdin, sentinel via echo.
Usage: s1_b.py N OUT_TSV."""
import subprocess, sys, time, os, select, re
sys.path.insert(0, os.path.dirname(__file__))
import deck, common

class Ng:
    def __init__(self):
        t0 = time.perf_counter_ns()
        self.p = subprocess.Popen(["script", "-qec", "ngspice -p -t dumb", "/dev/null"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.fd = self.p.stdout.fileno(); self.t0 = t0; self.n = 0
    def send(self, t): self.p.stdin.write((t + "\n").encode()); self.p.stdin.flush()
    def sync(self, to=60):
        self.n += 1; tag = "SENT%d" % self.n
        self.send("echo " + tag)
        buf = b""; end = time.time() + to
        pat = re.compile(("^" + tag + "\r?$").encode(), re.M)
        while not pat.search(buf) and time.time() < end:
            r, _, _ = select.select([self.fd], [], [], 0.5)
            if r:
                d = os.read(self.fd, 65536)
                if not d: break
                buf += d
        assert pat.search(buf), "no sentinel"
        return buf.decode(errors="replace")
    def close(self):
        self.send("quit"); self.p.wait(10)

def main(N, out, r=2200):
    work = "/tmp/rb"; os.makedirs(work, exist_ok=True)
    d = os.path.join(work, "d.cir"); o = os.path.join(work, "o.dat")
    open(d, "w").write("\n".join(deck.circuit_lines(r, 0, 0)) + "\n.end\n")
    ng = Ng()
    ng.send("set numdgt=15"); ng.send("source " + d); ng.sync()
    startup = time.perf_counter_ns() - ng.t0
    rows = []
    for i in range(N):
        if os.path.exists(o): os.remove(o)
        t0 = time.perf_counter_ns()
        pts = deck.pwl_points(i % 256, 0)
        ng.send("alter @vctl[pwl] = [ %s ]" % deck.pwl_str(i % 256, 0))
        ng.send("run"); ng.send("wrdata %s v(line)" % o)
        txt = ng.sync()
        ns = time.perf_counter_ns() - t0
        assert "rror" not in txt.replace("ERROR: (external)", ""), txt[-400:]
        b = open(o, "rb").read()
        rows.append((i, ns, common.sha(b), len(b)))
    ng.close()
    with open(out, "w") as f:
        f.write("# startup_ns\t%d\n" % startup)
        for x in rows: f.write("%d\t%d\t%s\t%d\n" % x)

if __name__ == "__main__":
    main(int(sys.argv[1]), sys.argv[2])
