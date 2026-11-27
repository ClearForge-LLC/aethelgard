"""Decode DShot frames from AethelgardSTM32Timer burst lines and AethelgardSTM32DMA paced-end lines.
A frame is the bursts between two paced ends of dma2's stream. Each burst holds CCR1..CCR4; a bit is 1 when its
CCR is the long pulse. Prints counts, the distinct packets per channel with their decode, and CRC checks."""
import gzip, re, sys, collections
BURST = re.compile(rb'^\[(\d\d):(\d\d):(\d\d)\.(\d{9})\] \[NOISY\] timer1: burst 0x34: (.*)$')
END = re.compile(rb'^\[(\d\d):(\d\d):(\d\d)\.(\d{9})\] \[NOISY\] dma2: stream (\d) paced end \(([^)]*)\): (\d+) items on (\d+) requests, NDTR (\d+), TCIF; EN (\d+) ns, end (\d+) ns')
frames, cur, ends = [], [], collections.Counter()
burst_bytes = end_bytes = 0
def t(m): return int(m.group(1))*3600 + int(m.group(2))*60 + int(m.group(3)) + int(m.group(4))/1e9
for line in gzip.open(sys.argv[1]):
    line = line.rstrip(b'\r\n')
    m = BURST.match(line)
    if m:
        burst_bytes += len(line) + 1
        cur.append((t(m), [int(x, 16) for x in m.group(5).split()]))
        continue
    m = END.match(line)
    if m:
        end_bytes += len(line) + 1
        ends[(m.group(6).decode(), int(m.group(7)), int(m.group(8)), int(m.group(9)))] += 1
        frames.append((t(m), cur, int(m.group(10)), int(m.group(11)))); cur = []
print("frames=%d bursts=%d burst_log_bytes=%d end_log_bytes=%d trailing_bursts=%d" % (len(frames), sum(len(f[1]) for f in frames), burst_bytes, end_bytes, len(cur)))
print("paced ends (why, items, requests, NDTR):", dict(ends))
shapes = collections.Counter(len(f[1]) for f in frames)
print("bursts per frame:", dict(shapes))
pk = [collections.Counter() for _ in range(4)]
bad = 0; first_nonzero = None
widths = collections.Counter()
durs = collections.Counter((f[3] - f[2]) for f in frames)
print("frame duration EN->TCIF ns:", dict(durs.most_common(6)))
for te, bs, _, _ in frames:
    words = [b[1] for b in bs]
    for w in words: widths.update(w)
    if len(words) != 19: continue
    for c in range(4):
        bits = [words[1 + k][c] for k in range(16)]
        if any(b not in (3, 6) for b in bits) or words[0][c] or words[17][c] or words[18][c]:
            bad += 1; continue
        v = 0
        for b in bits: v = (v << 1) | (1 if b == 6 else 0)
        pk[c][v] += 1
        if v and first_nonzero is None: first_nonzero = (te, c, v)
print("CCR values seen:", dict(widths)); print("malformed channel-frames:", bad)
for c in range(4):
    items = sorted(pk[c].items(), key=lambda kv: -kv[1])[:8]
    out = []
    for v, n in items:
        val = v >> 4; crc = v & 0xF; tel = val & 1; thr = val >> 1
        ok = crc == ((val ^ (val >> 4) ^ (val >> 8)) & 0xF)
        out.append("0x%04X thr=%d tel=%d crc=%X %s x%d" % (v, thr, tel, crc, "ok" if ok else "BAD", n))
    print("CH%d:" % (c + 1), "; ".join(out))
print("first non-zero packet (paced end t, ch, packet):", first_nonzero)
