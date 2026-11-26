"""AET-WO-4400j (1): per source and template, how many distinct exact line texts (timestamp dropped, every value kept)
a log holds from FROM_S, and their bytes: the cost of summarising each source by its exact text (first and last kept,
counted) rather than by a template with values elided. Usage: distinct.py LOG [FROM_S]"""
import collections, gzip, re, sys
path = sys.argv[1]; t_from = float(sys.argv[2]) if len(sys.argv) > 2 else 0.0
ts = re.compile(r"^\[(\d+):(\d+):(\d+\.\d+)\] (\[(\w+)\] ([^ :]+): (.*))$")
num = re.compile(r"0x[0-9A-Fa-f]+|\b\d+\b")
texts = collections.defaultdict(set); lines = collections.Counter(); size = collections.Counter()
for raw in gzip.open(path, "rt", errors="replace"):
    m = ts.match(raw.rstrip("\n"))
    if not m or "Unhandled write" in raw: continue
    t = int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))
    if t < t_from: continue
    tpl = m.group(5) + " " + m.group(6) + ": " + num.sub("#", m.group(7))[:70]
    texts[tpl].add(m.group(4)); lines[tpl] += 1; size[tpl] += len(raw)
print(f"{'lines':>8} {'distinct':>8} {'bytes':>11} {'distinct B':>10}  template")
for tpl, n in lines.most_common(16):
    print(f"{n:>8} {len(texts[tpl]):>8} {size[tpl]:>11,} {sum(len(x) + 22 for x in texts[tpl]):>10,}  {tpl}")
