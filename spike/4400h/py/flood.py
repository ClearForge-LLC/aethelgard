"""AET-WO-4400h (1): what fills the log. Reads a replay's console.txt.gz (log-timestamps-type Virtual) and prints,
per kind (level, source, the message with numbers elided), the line and byte counts in each virtual second,
the top kinds first. Usage: python3 -I flood.py console.txt.gz [TOP] [kept]
With "kept", Renode's `Unhandled write` warnings are left out: the CLI summarises them by kind (4400e D7 (a)), so
the rest is what its 8 MiB budget keeps."""
import collections, gzip, re, sys
line_re = re.compile(rb"^\[(\d+):(\d+):(\d+(?:\.\d+)?)\] \[(\w+)\] ([^ :]+): ?(.*)$")
num = re.compile(rb"0x[0-9A-Fa-f]+|\d+")
kept = len(sys.argv) > 3 and sys.argv[3] == "kept"
by = collections.defaultdict(lambda: collections.Counter()); size = collections.Counter(); total = collections.Counter()
other = 0
for raw in gzip.open(sys.argv[1], "rb"):
    raw = raw.rstrip(b"\r\n")
    if kept and b"Unhandled write" in raw:
        continue
    m = line_re.match(raw)
    if not m:
        other += 1; continue
    t = int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))
    kind = (m.group(4) + b" " + m.group(5) + b": " + num.sub(b"#", m.group(6))[:90]).decode("utf-8", "replace")
    by[kind][int(t)] += 1; size[kind] += len(raw) + 1; total[int(t)] += len(raw) + 1
top = int(sys.argv[2]) if len(sys.argv) > 2 else 12
print("bytes per virtual second:", dict(sorted(total.items())), "untimed lines:", other)
for kind, b in size.most_common(top):
    c = by[kind]
    print(f"{b:>10} B {sum(c.values()):>8} lines  {kind}\n           per s: {dict(sorted(c.items()))}")
