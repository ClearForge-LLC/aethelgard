# M1's judge: the value lines of two Renode logs, in order. A value line is any line that is not
# Renode's own log line ("[hh:mm:ss.ffff] [LEVEL] ..."), its banner, or the monitor echo; the Renode
# log lines are compared as a multiset with their timestamps and the PRNG seed removed, so a
# reordering of log interleaving is reported but not counted as a value difference.
import re, sys
def split(path):
    vals, logs = [], []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        m = re.match(r"^\[\d\d:\d\d:\d\d\.\d+\] (.*)$", line)
        if m:
            logs.append(re.sub(r"seed: \d+", "seed: N", m.group(1)))
        elif line.startswith(("(monitor)", "Renode, version")) or not line.strip():
            continue
        else:
            vals.append(line)
    return vals, sorted(logs)
a, b = split(sys.argv[1]), split(sys.argv[2])
print(f"value lines: {len(a[0])} vs {len(b[0])}; equal: {a[0] == b[0]}")
for i, (x, y) in enumerate(zip(a[0], b[0])):
    if x != y:
        print(f"  value line {i}: {x!r} != {y!r}")
print(f"log lines (multiset, timestamps and seed removed): equal: {a[1] == b[1]}")
sys.exit(0 if a[0] == b[0] else 1)
