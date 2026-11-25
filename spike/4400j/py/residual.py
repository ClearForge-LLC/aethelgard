"""AET-WO-4400j (1): what a log would keep under a summary scheme, per virtual second and in all.
Scheme `exact`: each listed source's lines are summarised by their exact text (timestamp dropped, every value kept):
the first of each text kept, later ones counted (first and last stamps), as Unhandled write is today, capped at
1,024 keys (a further text is kept as any line). Scheme `exact+fifo`: the same, and U5's FIFO reads (`bank 0 read 0x30`)
by template, values elided, after its data step is witnessed (AFTER_S). Both keep every other line verbatim.
This models a simpler scheme than D2 as worded (no cycle unit, no wait for multi-line steps): a bound on what D2 keeps.
Usage: residual.py LOG AFTER_S"""
import collections, gzip, re, sys
path, after = sys.argv[1], float(sys.argv[2])
ts = re.compile(r"^\[(\d+):(\d+):(\d+\.\d+)\] (\[(\w+)\] ([^ :]+): (.*))$")
SOURCES = ("spi1", "spi2", "spi1.spi1mux", "spi2.spi2mux", "spi1.spi1mux.u4", "spi2.spi2mux.u5", "adc3")
def listed(level, src, text):
    if "Unhandled write" in text: return False
    if src in ("spi1", "spi2"): return text.startswith("AETHELGARD-CFG2 ") or "slave mode" in text
    return src in SOURCES and level in ("NOISY", "WARNING")
for scheme in ("none", "exact", "exact+fifo"):
    keys, kept, per = set(), 0, collections.Counter()
    why = collections.Counter()   # what the kept bytes are, from 6 s: (overflow past the key cap | first of a key | unlisted)
    for raw in gzip.open(path, "rt", errors="replace"):
        m = ts.match(raw.rstrip("\n"))
        if not m:
            kept += len(raw); continue
        if "Unhandled write" in raw: continue
        t = int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))
        level, src, text = m.group(5), m.group(6), m.group(7)
        key = None
        if scheme != "none" and listed(level, src, text):
            key = m.group(4)
            if scheme == "exact+fifo" and src == "spi2.spi2mux.u5" and t >= after and text.startswith("bank 0 read 0x30 -> "):
                key = "U5 FIFO read"
            if key in keys: continue
            if len(keys) < 1024:
                keys.add(key)
                if t >= 6: why["first of a key: " + src] += len(raw)
            elif t >= 6: why["past the 1,024-key cap: " + src] += len(raw)
        elif t >= 6: why["not listed: " + level + " " + src] += len(raw)
        kept += len(raw); per[int(t)] += len(raw)
    if scheme != "none":
        print("   from 6 s:", ", ".join(f"{k} {v:,} B" for k, v in why.most_common(6)))
    print(f"{scheme:>10}: kept {kept:,} B ({100 * kept / 8388608:.0f} % of 8 MiB); keys {len(keys)}; per s {dict(sorted(per.items()))}")
