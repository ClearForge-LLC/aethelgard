"""Edge widths from AethelgardSTM32Timer's TraceEdges lines ("edge CHn v ns"): per channel, the high times and the
rise-to-rise spacing inside frames (gap < 10 us), the pulses per frame, and the level each frame ends at."""
import gzip, re, sys, collections
E = re.compile(rb'\] timer1: edge CH(\d) ([01]) (\d+)')
ev = collections.defaultdict(list)
for line in gzip.open(sys.argv[1]):
    m = E.search(line)
    if m: ev[int(m.group(1))].append((int(m.group(3)), int(m.group(2))))
for c in sorted(ev):
    xs = ev[c]
    high = collections.Counter(); rr = collections.Counter(); per_frame = collections.Counter()
    last_rise = None; rise_at = None; n_in = 0; ends_low = ends_high = 0
    for t, v in xs:
        if v == 1:
            if last_rise is not None and t - last_rise < 10000: rr[t - last_rise] += 1; n_in += 1
            else:
                if last_rise is not None: per_frame[n_in + 1] += 1
                n_in = 0
            last_rise = rise_at = t
        elif rise_at is not None:
            high[t - rise_at] += 1
    per_frame[n_in + 1] += 1
    print("CH%d edges=%d final level=%d" % (c, len(xs), xs[-1][1]))
    print("  high ns:", dict(high.most_common(6)))
    print("  rise-to-rise ns (in frame):", dict(rr.most_common(4)))
    print("  pulses per frame:", dict(per_frame.most_common(4)))
