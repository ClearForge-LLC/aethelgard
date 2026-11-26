"""AET-WO-4400j (1), from 4400h's: what fills a log. Reads a log (gz or plain; Renode's virtual timestamps) and prints,
per kind (level, source, the message with numbers elided), bytes and lines in each virtual second, the top first; then
per source (level + peripheral) the same. With --tsv, one row per (source kind, second) for the tables.
Usage: python3 -I flood.py LOG [TOP] [--tsv OUT] [--raw]   --raw: also count `Unhandled write` lines (a replay's console)"""
import collections, gzip, re, sys
args = [a for a in sys.argv[1:] if not a.startswith("--")]
tsv = sys.argv[sys.argv.index("--tsv") + 1] if "--tsv" in sys.argv else None
if tsv: args.remove(tsv)
raw = "--raw" in sys.argv
line_re = re.compile(rb"^\[(\d+):(\d+):(\d+(?:\.\d+)?)\] \[(\w+)\] ([^ :]+): ?(.*)$")
num = re.compile(rb"0x[0-9A-Fa-f]+|\b\d+\b")
op = gzip.open if args[0].endswith(".gz") else open
by = collections.defaultdict(collections.Counter); size = collections.Counter(); lines = collections.Counter()
src_by = collections.defaultdict(collections.Counter); total = collections.Counter(); other = other_b = 0
for r in op(args[0], "rb"):
    r = r.rstrip(b"\r\n")
    m = line_re.match(r)
    if not m:
        other += 1; other_b += len(r) + 1; continue
    if not raw and b"Unhandled write" in r:
        continue
    t = int(int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3)))
    kind = (m.group(4) + b" " + m.group(5) + b": " + num.sub(b"#", m.group(6))[:90]).decode("utf-8", "replace")
    src = (m.group(4) + b" " + m.group(5)).decode()
    n = len(r) + 1
    by[kind][t] += n; size[kind] += n; lines[kind] += 1; src_by[src][t] += n; total[t] += n
top = int(args[1]) if len(args) > 1 else 15
print("bytes per virtual second:", dict(sorted(total.items())), "| untimed lines:", other, other_b, "B")
for kind, b in size.most_common(top):
    print(f"{b:>11} B {lines[kind]:>8} lines  {kind}\n            per s: {dict(sorted(by[kind].items()))}")
print("-- by source")
for src, c in sorted(src_by.items(), key=lambda kv: -sum(kv[1].values()))[:top]:
    print(f"{sum(c.values()):>11} B  {src}  per s: {dict(sorted(c.items()))}")
if tsv:
    with open(tsv, "w") as f:
        f.write("kind\tsecond\tbytes\n")
        for kind in size:
            for t, b in sorted(by[kind].items()):
                f.write(f"{kind}\t{t}\t{b}\n")
