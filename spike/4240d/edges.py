"""AET-WO-4240d: count rising edges in an FX2 capture, or in a synthetic one.

  python3 -I edges.py count FILE [--bit N]
      FILE is `sigrok-cli ... -O binary -o FILE`: libsigrok 0.5.2's binary output writes the
      logic payload as is (src/output/binary.c), one byte per sample for fx2lafw's 8-bit mode
      (src/hardware/fx2lafw/protocol.c: unitsize 1 unless sample_wide), channel Dn at bit n.
      Prints the samples, the rising edges on bit N (default 0), and the shortest high and
      low runs in samples. A run of 1 sample means the capture cannot be trusted to count.

  python3 -I edges.py frames FILE --bit N --gate G [--rate 24000000]
      The rising edges on bit N inside each window where bit G is low (an SPI frame: SCK on
      N, CS on G), one line per frame: its rising edges, and the SCK frequency from its first
      to its last rising edge, (edges - 1) * RATE / (last - first) Hz.

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
    out, inside, prev, edges, first, last = [], False, None, 0, None, None
    for i, byte in enumerate(data):
        low = not (byte >> gate) & 1
        level = (byte >> bit) & 1
        if low and not inside:
            inside, edges, first, last = True, 0, None, None
        elif not low and inside:
            inside = False
            out.append((edges, first, last))
        if inside and prev == 0 and level == 1:
            edges += 1
            first = i if first is None else first
            last = i
        prev = level
    for n, (e, a, b) in enumerate(out):
        hz = (e - 1) * rate / (b - a) if e > 1 and b > a else None
        print(f"frame {n}: rising_edges {e} sck_hz {hz:.0f}" if hz else f"frame {n}: rising_edges {e}")
    print(f"frames {len(out)} total_rising_edges {sum(e for e, _, _ in out)}")


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
