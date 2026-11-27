"""AET-WO-4412 prototype: compare two replay consoles for equivalence.
Streams compared, each as (stamp, ...) tuples in log order:
  usart1/usart2/usart3, timer1, dma1, dma2: every logged access (r/w, offset, value, the CPU's PC); register names
      are dropped (the models name their registers differently);
  LED edges (gpioPortA.pa8obs, gpioPortE.pe11obs/pe13obs/pe14obs): (led, state);
  the end-of-window reads: the printed lines between the last AETHELGARD-BOOT-BEGIN and -END.
For each stream: counts, whether the sequences without stamps are identical, whether they are identical with
stamps, the first difference, and the stamp differences where the sequences agree. Warnings are tallied by text.
Usage: compare.py A.console.txt.gz B.console.txt.gz"""
import gzip, re, sys, collections

ACC = re.compile(rb'^\[(\d\d:\d\d:\d\d\.\d{9})\] \[\w+\] (\w+): \[cpu: (0x[0-9A-F]+)\] (Write|Read)(\w+) (?:to|from) (0x[0-9A-F]+) \([^)]*\), (?:value|returned) (0x[0-9A-F]+)')
LED = re.compile(rb'^\[(\d\d:\d\d:\d\d\.\d{9})\] \[NOISY\] (gpioPort\w\.\w+obs): LED state changed to (\w+)')
WARN = re.compile(rb'^\[\d\d:\d\d:\d\d\.\d{9}\] \[(WARNING|ERROR)\] ([\w.]+): (.*)$')
DIGITS = re.compile(rb'0x[0-9A-Fa-f]+|\d+')
KEEP = {b'usart1', b'usart2', b'usart3', b'timer1', b'dma1', b'dma2'}

def ns(stamp):
    h, m, s = stamp.split(b':')
    a, b = s.split(b'.')
    return ((int(h) * 60 + int(m)) * 60 + int(a)) * 10**9 + int(b)

def load(path):
    streams = collections.defaultdict(list)
    warns = collections.Counter()
    blocks, cur, inblock = [], [], False
    for line in gzip.open(path):
        line = line.rstrip(b'\r\n')
        m = ACC.match(line)
        if m and m.group(2) in KEEP:
            streams[m.group(2).decode()].append((ns(m.group(1)), m.group(3), m.group(4)[0:1] + m.group(5), m.group(6), m.group(7)))
            continue
        m = LED.match(line)
        if m:
            streams['led'].append((ns(m.group(1)), m.group(2), m.group(3)))
            continue
        if line == b'AETHELGARD-BOOT-BEGIN':
            inblock, cur = True, []
            continue
        if line == b'AETHELGARD-BOOT-END':
            inblock = False
            blocks.append(cur)
            continue
        if inblock:
            if not line.startswith(b'['):
                cur.append(line)
            continue
        m = WARN.match(line)
        if m:
            warns[(m.group(1), m.group(2), DIGITS.sub(b'#', m.group(3))[:90])] += 1
    return streams, warns, blocks

def main():
    a, b = sys.argv[1], sys.argv[2]
    sa, wa, ba = load(a)
    sb, wb, bb = load(b)
    print("A =", a); print("B =", b)
    for k in sorted(set(sa) | set(sb)):
        x, y = sa.get(k, []), sb.get(k, [])
        bare_x, bare_y = [t[1:] for t in x], [t[1:] for t in y]
        same_bare = bare_x == bare_y
        same_all = x == y
        line = "%-7s A=%d B=%d same_without_stamps=%s same_with_stamps=%s" % (k, len(x), len(y), same_bare, same_all)
        n = min(len(x), len(y))
        first = next((i for i in range(n) if bare_x[i] != bare_y[i]), None)
        if first is None and len(x) != len(y):
            first = n
        if first is not None:
            line += "\n    first difference at #%d: A=%s B=%s" % (first, x[first] if first < len(x) else None, y[first] if first < len(y) else None)
            ndiff = sum(1 for i in range(n) if bare_x[i] != bare_y[i])
            line += "\n    differing positions (of the common length %d): %d" % (n, ndiff)
        upto = first if first is not None else n
        dts = [y[i][0] - x[i][0] for i in range(upto) if x[i][0] != y[i][0]]
        if dts:
            line += "\n    stamps differ at %d of the %d agreeing items; B-A ns: min %d max %d; first at #%d (A %d)" % (
                len(dts), upto, min(dts), max(dts), next(i for i in range(upto) if x[i][0] != y[i][0]), x[next(i for i in range(upto) if x[i][0] != y[i][0])][0])
        print(line)
    print("end-of-window read blocks: A=%d B=%d" % (len(ba), len(bb)))
    if ba and bb:
        la, lb = ba[-1], bb[-1]
        diffs = [(i, la[i], lb[i]) for i in range(min(len(la), len(lb))) if la[i] != lb[i]]
        print("  last block: %d lines vs %d; differing lines: %d" % (len(la), len(lb), len(diffs)))
        for d in diffs[:20]:
            print("   ", d)
        for j, (xa, xb) in enumerate(zip(ba, bb)):
            dd = sum(1 for u, v in zip(xa, xb) if u != v)
            if dd:
                print("  block %d differs in %d lines" % (j, dd))
    print("warnings/errors, A count, B count (only where they differ):")
    for k in sorted(set(wa) | set(wb), key=lambda k: -(wa[k] + wb[k])):
        if wa[k] != wb[k]:
            print("   %7d %7d  %s %s: %s" % (wa[k], wb[k], k[0].decode(), k[1].decode(), k[2].decode(errors='replace')))

main()
