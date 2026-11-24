"""AET-WO-2314 §1.1 (2): what a measure.sh run shows after its fault hook: the hook lines, the log
lines stamped after the first, by source and op (USART access lines counted), whether the begin and end
markers and the reads between them are present, and the reads. python3 -I after.py OUT.txt"""
import re, sys
from collections import Counter
lines = [l.rstrip("\r\n") for l in open(sys.argv[1], encoding="utf-8", errors="replace") if not l.startswith("(monitor)")]
stamp = re.compile(r"^\[(\d\d):(\d\d):(\d\d)\.(\d{9})\] ")
def ns(l):
    m = stamp.match(l)
    return None if not m else ((int(m[1]) * 60 + int(m[2])) * 60 + int(m[3])) * 10**9 + int(m[4])
faults = [l for l in lines if "AETHELGARD-FAULT" in l]
first = ns(faults[0]) if faults else None
after = Counter()
for l in lines:
    t = ns(l)
    if first is not None and t is not None and t > first:
        m = re.match(r"^\[[^\]]*\] \[(\w+)\] ([\w.]+): (?:\[cpu: [^\]]*\] )?(\w+)", l)
        after[m.groups() if m else ("?", "?", "?")] += 1
begin = [i for i, l in enumerate(lines) if l == "AETHELGARD-BOOT-BEGIN"]
end = [i for i, l in enumerate(lines) if l == "AETHELGARD-BOOT-END"]
print(f"ANSWER 2314: faults={len(faults)} first_ns={first} begin={len(begin)} end={len(end)} last_stamp_ns={max((t for t in map(ns, lines) if t is not None), default=None)}")
for f in faults[:4]:
    print("  ", f[:300])
for k, v in after.most_common(8):
    print("   after:", k, v)
if begin and end:
    print("   reads:", " | ".join(lines[begin[0] + 1:end[0]])[:1500])
