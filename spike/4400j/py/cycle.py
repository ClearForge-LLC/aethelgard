"""AET-WO-4400j (1): the CFG2 / slave-mode ERROR / device-line cycle on each SPI controller, from a kept log: the distinct
CFG2 (value, CR1) tuples, and the distinct sequences of events between two CFG2 writes with MASTER set (the unit a
summary would count). Event letters: M = CFG2 MASTER 1, S = CFG2 MASTER 0 (with SPE in CR1 as s0/s1), E = the slave-mode
ERROR, D = a device line on the controller (its mux or a device behind it). Usage: cycle.py LOG [FROM_S]"""
import collections, gzip, re, sys
path = sys.argv[1]; t_from = float(sys.argv[2]) if len(sys.argv) > 2 else 0.0
op = gzip.open if path.endswith(".gz") else open
ts = re.compile(r"^\[(\d+):(\d+):(\d+\.\d+)\] \[(\w+)\] ([^ :]+): (.*)$")
tuples = collections.Counter(); seqs = collections.Counter(); cur = collections.defaultdict(list)
device_kinds = collections.Counter(); first_seen = {}
for raw in op(path, "rt", errors="replace"):
    m = ts.match(raw.rstrip("\n"))
    if not m: continue
    t = int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))
    if t < t_from: continue
    lvl, src, text = m.group(4), m.group(5), m.group(6)
    ctl = src.split(".")[0]
    if not ctl.startswith("spi"): continue
    if text.startswith("AETHELGARD-CFG2 "):
        v, cr1 = (int(x, 16) for x in text.split()[1:3])
        tuples[(ctl, hex(v), hex(cr1))] += 1
        if v >> 22 & 1:
            if cur[ctl]: seqs[(ctl, "".join(cur[ctl]))] += 1
            cur[ctl] = ["M"]
        else:
            cur[ctl].append("S%d" % (cr1 & 1))
    elif lvl == "ERROR" and "slave mode" in text:
        cur[ctl].append("E")
    elif "." in src:
        if not cur[ctl] or cur[ctl][-1] != "D": cur[ctl].append("D")
        device_kinds[(src, lvl, re.sub(r"0x[0-9A-Fa-f]+|\b\d+\b", "#", text)[:60])] += 1
print("CFG2 tuples (controller, value, CR1):")
for k, n in tuples.most_common(): print("  %8d %s" % (n, k))
print("sequences from one MASTER-1 CFG2 to the next:")
for k, n in seqs.most_common(12): print("  %8d %s %s" % (n, k[0], k[1][:120]))
print("distinct sequences:", len(seqs))
print("device line templates on SPI controllers:", len(device_kinds))
for k, n in device_kinds.most_common(10): print("  %8d %s" % (n, k))
