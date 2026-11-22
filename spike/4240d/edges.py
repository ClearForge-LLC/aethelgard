"""AET-WO-4240d: count rising edges in an FX2 capture, or in a synthetic one.

  python3 -I edges.py count FILE [--bit N]
      FILE is `sigrok-cli ... -O binary -o FILE`: libsigrok 0.5.2's binary output writes the
      logic payload as is (src/output/binary.c), one byte per sample for fx2lafw's 8-bit mode
      (src/hardware/fx2lafw/protocol.c: unitsize 1 unless sample_wide), channel Dn at bit n.
      Prints the samples, the rising edges on bit N (default 0), and the shortest high and
      low runs in samples. A run of 1 sample means the capture cannot be trusted to count.

  python3 -I edges.py frames FILE --bit N --gate G [--rate 24000000]
      The rising edges on bit N inside each window where bit G is low (an SPI frame: SCK on
      N, CS on G), one line per frame: its rising edges, and the SCK frequency **within bytes**.
      The generated SPI driver writes one byte to TXDR and waits for RXP before the next, so
      the TxFIFO empties at every byte and SCK stalls between bytes (RM0433 Rev 9 p. 2176). A
      frame's edges are therefore taken 8 at a time (one byte in mode 0), and the frequency is
      7 * bytes * RATE / (the sum of each byte's first-to-eighth rising-edge span). The longest
      stall between bytes (the boundary's rise-to-rise interval less the median in-byte
      interval), in samples, is printed for the record. No sck_hz is printed for a frame whose
      edges are not whole bytes, whose bytes' 7 intervals differ by more than 2 samples (a
      spurious or missed edge), or that is still open at the capture's end.

  python3 -I edges.py synth HZ HIGH PERIOD [--rate 24000000] [--samples 24000000] [--phase P]
      Samples an ideal square wave of HZ whose high phase is HIGH/PERIOD of a cycle at RATE,
      for SAMPLES samples, and prints the same three figures. Integer arithmetic throughout.
"""

import sys
from fractions import Fraction


def runs(samples, bit):
    """(rising edges, shortest high run, shortest low run) over an iterable of bytes."""
    edges, prev, run, hi_min, lo_min = 0, None, 0, None, None
    first = True
    for byte in samples:
        level = (byte >> bit) & 1
        if level == prev:
            run += 1
            continue
        if prev is not None:
            # The first run is cut by the capture's start, so it is not a whole phase.
            if not first:
                if prev:
                    hi_min = run if hi_min is None else min(hi_min, run)
                else:
                    lo_min = run if lo_min is None else min(lo_min, run)
            first = False
            if level:
                edges += 1
        prev, run = level, 1
    return edges, hi_min, lo_min


def count(path, bit):
    data = open(path, "rb").read()
    e, hi, lo = runs(data, bit)
    print(f"samples {len(data)} rising_edges {e} min_high_run {hi} min_low_run {lo}")


def frames(path, bit, gate, rate):
    data = open(path, "rb").read()
    out, inside, prev, rises = [], False, None, []
    for i, byte in enumerate(data):
        low = not (byte >> gate) & 1
        level = (byte >> bit) & 1
        if low and not inside:
            inside, rises = True, []
        elif not low and inside:
            inside = False
            out.append((rises, True))
        if inside and prev == 0 and level == 1:
            rises.append(i)
        prev = level
    if inside:
        out.append((rises, False))
    total = 0
    for n, (r, closed) in enumerate(out):
        total += len(r)
        line = f"frame {n}: rising_edges {len(r)}"
        whole = [r[k : k + 8] for k in range(0, len(r) - len(r) % 8, 8)]
        steps = [[b[j + 1] - b[j] for j in range(7)] for b in whole]
        aligned = all(max(s) - min(s) <= 2 for s in steps)
        if not closed:
            line += " UNTERMINATED (CS still low at the capture's end): no sck_hz"
        elif len(r) % 8:
            line += f" MISALIGNED ({len(r) % 8} edges over whole bytes): no sck_hz"
        elif not aligned:
            line += " MISALIGNED (a byte's 7 intervals differ by more than 2 samples): no sck_hz"
        elif whole:
            span = sum(b[-1] - b[0] for b in whole)
            inner = sorted(x for s in steps for x in s)[len(steps) * 7 // 2]
            stall = max((whole[k + 1][0] - whole[k][-1] - inner for k in range(len(whole) - 1)), default=0)
            line += f" bytes {len(whole)} sck_hz {7 * len(whole) * rate / span:.0f} max_stall_samples {stall}"
        print(line)
    print(f"frames {len(out)} total_rising_edges {total}")


def synth(hz, high, period, rate, n, phase):
    duty = Fraction(high, period)
    f = Fraction(hz)
    ph = Fraction(phase)

    def gen():
        for k in range(n):
            x = (f * k / rate + ph) % 1
            yield 1 if x < duty else 0

    e, hi, lo = runs(gen(), 0)
    print(
        f"hz {hz} duty {high}/{period} rate {rate} samples {n}: rising_edges {e} "
        f"min_high_run {hi} min_low_run {lo} (ideal hz*n/rate = {float(f * n / rate):.3f})"
    )


def main(argv):
    if len(argv) >= 2 and argv[0] == "count":
        bit = int(argv[argv.index("--bit") + 1]) if "--bit" in argv else 0
        count(argv[1], bit)
    elif len(argv) >= 6 and argv[0] == "frames":
        rate = int(argv[argv.index("--rate") + 1]) if "--rate" in argv else 24_000_000
        frames(argv[1], int(argv[argv.index("--bit") + 1]), int(argv[argv.index("--gate") + 1]), rate)
    elif len(argv) >= 4 and argv[0] == "synth":
        opt = lambda k, d: Fraction(argv[argv.index(k) + 1]) if k in argv else Fraction(d)
        synth(
            Fraction(argv[1]),
            int(argv[2]),
            int(argv[3]),
            int(opt("--rate", 24_000_000)),
            int(opt("--samples", 24_000_000)),
            opt("--phase", "1/7"),
        )
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
