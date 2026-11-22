# M1's judge: the value lines of two Renode logs, in order. Renode's own log records
# ("[hh:mm:ss.ffff] [LEVEL] ...\n") are cut out of the byte stream first, because Renode can write
# one in the middle of a value line (1211's log does, at T3); the value lines are what remains, less
# the banner and the monitor echo. The log records are compared as a multiset with timestamps and the
# PRNG seed removed: a difference there is reported, not counted as a value difference.
import re, sys
REC = re.compile(r"\[\d\d:\d\d:\d\d\.\d+\] (\[[A-Z]+\] [^\n]*)\n")
def split(path):
    text = open(path, encoding="utf-8", errors="replace").read()
    logs = sorted(re.sub(r"seed: \d+", "seed: N", m) for m in REC.findall(text))
    vals = [l for l in REC.sub("", text).splitlines()
            if l.strip() and not l.startswith(("(monitor)", "Renode, version"))]
    return vals, logs
a, b = split(sys.argv[1]), split(sys.argv[2])
print(f"value lines: {len(a[0])} vs {len(b[0])}; equal: {a[0] == b[0]}")
for i, (x, y) in enumerate(zip(a[0], b[0])):
    if x != y:
        print(f"  value line {i}: {x!r} != {y!r}")
print(f"log records (multiset, timestamps and seed removed): {len(a[1])} vs {len(b[1])}; equal: {a[1] == b[1]}")
for x in sorted(set(a[1]) ^ set(b[1])):
    print(f"  only in {'first' if x in a[1] else 'second'}: {x}")
sys.exit(0 if a[0] == b[0] else 1)
