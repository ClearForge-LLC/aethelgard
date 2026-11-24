"""AET-WO-4400h (3): what arming reaches, run against run, from queue.sh's outputs (writes.tsv: view2-writes.awk rows).
Usage: python3 -I delta.py BASE_DIR CONTROL_DIR ARMED_DIR OUT.tsv
  BASE    no injection;  CONTROL  the same frames' bytes without the arm (params or pad);  ARMED  the arm.
Per peripheral register (firmware writes only, the `fw` rows: a CPU PC; monitor-issued accesses carry none):
writes and distinct values in each run, and the class:
  armed-only        written in ARMED, never in CONTROL nor BASE     (what arming newly reaches)
  injection         written in CONTROL and ARMED, never in BASE     (the injection's own traffic, e.g. DMA re-arm)
  more-when-armed   in all three, ARMED's count above 1.5x CONTROL's (rate change)
  values-when-armed written in all three at about the same rate, but ARMED writes more distinct values than CONTROL
                    (e.g. a timer's compare registers carrying the motor outputs)
Rows of other classes are not printed. Also prints the peripherals with any armed-only register."""
import collections, sys
def load(d):
    n = collections.Counter(); vals = collections.defaultdict(set)
    for line in open(f"{d}/writes.tsv"):
        f = line.rstrip("\n").split("\t")
        if f[0] != "W" or f[3] != "fw":
            continue
        k = (f[4], f[6], f[9] if len(f) > 9 else "")
        n[k] += 1; vals[k].add(f[7])
    return n, vals
(bn, bv), (cn, cv), (an, av) = (load(d) for d in sys.argv[1:4])
rows, newp = [], set()
for k in sorted(set(an) | set(cn)):
    a, c, b = an.get(k, 0), cn.get(k, 0), bn.get(k, 0)
    cls = None
    if a and not c and not b: cls = "armed-only"; newp.add(k[0])
    elif a and c and not b: cls = "injection"
    elif a and c and b and a > 1.5 * c: cls = "more-when-armed"
    elif a and c and len(av.get(k, ())) > len(cv.get(k, ())): cls = "values-when-armed"
    if cls: rows.append((cls, k[0], k[1], k[2], b, c, a, len(av.get(k, ())), sorted(av.get(k, ()))[:4]))
with open(sys.argv[4], "w") as o:
    o.write("class\tperipheral\toffset\tregister\tbase_writes\tcontrol_writes\tarmed_writes\tarmed_distinct_values\tfirst_values\n")
    for r in rows: o.write("\t".join(str(x) for x in r) + "\n")
print("armed-only registers:", sum(1 for r in rows if r[0] == "armed-only"), "in", sorted(newp))
print("injection registers:", sum(1 for r in rows if r[0] == "injection"), "; more-when-armed:", sum(1 for r in rows if r[0] == "more-when-armed"),
      "; values-when-armed:", sum(1 for r in rows if r[0] == "values-when-armed"))
