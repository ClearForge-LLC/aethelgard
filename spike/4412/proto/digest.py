"""AET-WO-4412 round 5: per-run digests of the judged figures, from replay consoles.
For each run: transfer lines (count, first EN ns, last end ns, md5 of the words, md5 of the full lines with stamps,
decoded packet histogram md5); USART1/2/3 bytes md5 and bytes+stamps md5; the last end-read block (PC, instruction
count, md5 of the other lines). Usage: digest.py RUN... (directories under rep/)."""
import gzip, re, sys, hashlib, collections
X = re.compile(rb'^\[[^]]*\] \[NOISY\] dma2: stream 1 req 15 NDTR (\d+) EN (\d+) end (\d+) reqs (\d+)[^:]*:(.*)$')
U = re.compile(rb'^\[(\d\d:\d\d:\d\d\.\d{9})\] \[\w+\] (usart[123]): \[cpu: (0x[0-9A-F]+)\] (Write|Read)(\w+) (?:to|from) (0x[0-9A-F]+) \([^)]*\), (?:value|returned) (0x[0-9A-F]+)')
def h(x): return hashlib.md5(x).hexdigest()[:10]
for r in sys.argv[1:]:
    words = hashlib.md5(); full = hashlib.md5(); n = 0; first = last = None; pk = collections.Counter()
    ub = {k: hashlib.md5() for k in (b'usart1', b'usart2', b'usart3')}; us = {k: hashlib.md5() for k in ub}
    blocks = []; cur = None
    for line in gzip.open('rep/%s/console.txt.gz' % r):
        line = line.rstrip(b'\r\n')
        m = X.match(line)
        if m:
            n += 1; en, end = int(m.group(2)), int(m.group(3))
            first = en if first is None else first; last = end
            words.update(m.group(5)); full.update(m.group(1) + b' ' + m.group(2) + b' ' + m.group(3) + b' ' + m.group(4) + m.group(5))
            w = [int(x, 16) for x in m.group(5).split()]
            if len(w) == 76:
                for c in range(4):
                    v = 0
                    for k in range(16): v = (v << 1) | (w[4 * (1 + k) + c] == 6)
                    pk[(c, v)] += 1
            continue
        m = U.match(line)
        if m:
            ub[m.group(2)].update(m.group(3) + m.group(4) + m.group(6) + m.group(7)); us[m.group(2)].update(line.split(b']', 1)[0] + m.group(3) + m.group(4) + m.group(6) + m.group(7))
            continue
        if line == b'AETHELGARD-BOOT-BEGIN': cur = []; continue
        if line == b'AETHELGARD-BOOT-END': blocks.append(cur); cur = None; continue
        if cur is not None and not line.startswith(b'['): cur.append(line)
    lb = blocks[-1] if blocks else []
    dec = h(repr(sorted(pk.items())).encode())
    print("%-26s xfer n=%d first=%s last=%s words=%s full=%s decode=%s | usart bytes %s/%s/%s stamps %s/%s/%s | end pc=%s instr=%s rest=%s" % (
        r, n, first, last, h(words.digest()), h(full.digest()), dec,
        h(ub[b'usart1'].digest()), h(ub[b'usart2'].digest()), h(ub[b'usart3'].digest()),
        h(us[b'usart1'].digest()), h(us[b'usart2'].digest()), h(us[b'usart3'].digest()),
        lb[2].decode() if len(lb) > 2 else '-', lb[0].decode() if lb else '-', h(b'\n'.join(lb[1:2] + lb[3:]))))
