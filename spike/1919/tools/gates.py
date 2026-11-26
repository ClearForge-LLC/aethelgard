#!/usr/bin/env python3
"""AET-WO-1919 (1): one verify's time, gate by gate and harness by harness, from ci/probe.py's lines
(as built at 4254a2a0: every line carries start=). Reads a probe log, or a CI job log holding the
judge step's PROBE lines (the timestamp column is dropped).

Prints:
  - verify's wall time, and the time before its first gate (generation, in-process) and after its
    last (the package derived and compared);
  - each gate (an `aethelgard` subcommand verify ran), in the order it started, with its seconds;
  - each prove gate: its harness, seconds, kani-driver's lifetime, inside it kani-compiler's and
    cbmc's seconds and the rest of kani-driver's own (its parse), the CLI's own time outside
    kani-driver (generating the board from --spec, copying, judging), and kani-driver's peak;
  - the sums per gate kind.
usage: gates.py LOG [--tsv]"""
import re
import sys
from collections import defaultdict

text = open(sys.argv[1], errors="replace").read()
lines = []
for raw in text.splitlines():
    i = raw.find("PROBE ")
    if i >= 0:
        lines.append(raw[i:])


def num(field, line):
    m = re.search(rf"\b{field}=([0-9.]+)", line)
    return float(m.group(1)) if m else None


gates, peaks, cbmc, compiles = [], {}, {}, defaultdict(float)
for ln in lines:
    s, st = num("seconds", ln), num("start", ln)
    if ln.startswith("PROBE gate "):
        argv = ln.split(" argv=", 1)[1]
        gates.append((st, s, argv))
    elif ln.startswith("PROBE peak "):
        h = re.search(r"harness=(\S+)", ln).group(1)
        kib = int(num("vmhwm_kib", ln))
        if h.endswith("/cbmc"):
            cbmc[h[:-5]] = cbmc.get(h[:-5], 0) + s
        elif h != "?":
            peaks[h] = (kib, s, st)
    elif ln.startswith("PROBE compile "):
        h = re.search(r"harness=(\S+)/compile", ln).group(1)
        compiles[h] += s

gates.sort()
verify = [g for g in gates if g[2].startswith("verify ")]
inner = [g for g in gates if not g[2].startswith("verify ")]
tsv = "--tsv" in sys.argv


def kind(argv):
    w = argv.split()
    if w[0] == "check":
        return "check " + w[1]
    if w[0] == "simulate" and "--firmware" in w:
        return "simulate --firmware"
    return w[0]


if verify:
    v0, vs, _ = verify[0]
    first = inner[0][0] if inner else v0 + vs
    last_end = max(st + s for st, s, _ in inner) if inner else v0
    print(f"verify\t{vs:.0f} s\t({vs / 60:.1f} min)")
    print(f"  before the first gate (in-process)\t{first - v0:.0f} s")
    print(f"  after the last gate (in-process)\t{v0 + vs - last_end:.0f} s")
sums = defaultdict(lambda: [0, 0.0])
print("\nstart_s\tseconds\tgate")
for st, s, argv in inner:
    k = kind(argv)
    sums[k][0] += 1
    sums[k][1] += s
    h = re.search(r"--harness (\S+)", argv)
    label = f"prove {h.group(1)}" if h else k
    print(f"{st:.0f}\t{s:.1f}\t{label}")
print("\nprove gates, by seconds:\nharness\tgate_s\tdriver_s\tcompile_s\tcbmc_s\tparse_s\tcli_s\tpeak_GiB")
rows = []
for st, s, argv in inner:
    h = re.search(r"--harness (\S+)", argv)
    if not h:
        continue
    h = h.group(1)
    c, b = compiles.get(h, 0.0), cbmc.get(h, 0.0)
    kib, d, _ = peaks.get(h, (0, 0.0, 0))
    rows.append((s, h, d, c, b, d - c - b, s - d, kib / 1048576))
for s, h, d, c, b, r, cli, pk in sorted(rows, reverse=True):
    print(f"{h.split('::')[-1]}\t{s:.1f}\t{d:.1f}\t{c:.1f}\t{b:.1f}\t{r:.1f}\t{cli:.1f}\t{pk:.3f}")
if rows:
    tot = [sum(x[i] for x in rows) for i in (0, 2, 3, 4, 5, 6)]
    print(f"ALL {len(rows)}\t" + "\t".join(f"{t:.0f}" for t in tot))
print("\nby kind:\nkind\tcount\tseconds\tminutes")
for k, (n, s) in sorted(sums.items(), key=lambda x: -x[1][1]):
    print(f"{k}\t{n}\t{s:.0f}\t{s / 60:.1f}")
