"""AET-WO-4400j (1): the single facts RESULTS.md cites, each from its record in ~/.cache/aet-4400j (sh/tables.sh runs all).
  facts.py cr2 LOG             USART2's CR2 STOP field and CR1 as written, from the kept log's access lines: each change
  facts.py pb13 CONSOLE        PB13's output episodes from GPIOB's access log (replay `gpiob`): count, durations
  facts.py vect CONSOLE READS  ICSR VECTACTIVE at each AET-READ step (replays `reads`, `reads-in`): steps in a handler
  facts.py cr1 CONSOLE READS   USART2 CR1 and CR2 as read back at chosen ms (the model's PCE)
  facts.py arm CONSOLE         the first armed heartbeat's virtual time, from USART1's TDR write stamps (replay `stats-in`)
  facts.py quantum CONSOLE     ICSR, PB13 and SPI2EN per 100 us step (replay `quantum`): episodes and the longest
  facts.py ciwall LOG...       CI's foreign job: wall between consecutive ANSWER lines, and the peak each printed"""
import datetime, gzip, re, sys

STAMP = re.compile(r"^\[(\d+):(\d+):(\d+\.\d+)\]")
VALUE = re.compile(r"^(0x[0-9A-Fa-f]+|\d+(\.\d+)?)$")


def stamp(line):
    m = STAMP.match(line)
    return int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3)) if m else None


def lines(path):
    return gzip.open(path, "rt", errors="replace") if path.endswith(".gz") else open(path, errors="replace")


def steps(console, cmds):
    cur, vals, out = None, [], []
    for raw in lines(console):
        line = raw.strip()
        m = re.match(r"^AET-READ t=(\d+)$", line)
        if m:
            if cur is not None:
                out.append((cur, vals))
            cur, vals = int(m.group(1)), []
        elif cur is not None and VALUE.match(line) and len(vals) < len(cmds):
            vals.append(line)
    if cur is not None:
        out.append((cur, vals))
    return [(t, v) for t, v in out if len(v) == len(cmds)]


def runs(seq, pred):
    out, start, prev = [], None, None
    for t, v in seq:
        if pred(v):
            start = t if start is None else start
            prev = t
        elif start is not None:
            out.append((start, prev)); start = None
    if start is not None:
        out.append((start, prev))
    return out


what = sys.argv[1]
if what == "cr2":
    last = None
    for raw in lines(sys.argv[2]):
        if "usart2: " not in raw or "ControlRegister2), value" not in raw:
            continue
        stop = int(raw.split()[-1], 16) >> 12 & 3
        if stop != last:
            print(f"{stamp(raw):.9f} s  CR2 STOP={stop}  ({raw.split()[-1]})")
            last = stop
elif what == "pb13":
    prev, t_out, durs = None, None, []
    for raw in lines(sys.argv[2]):
        if "gpioPortB" not in raw or "to 0x0 (" not in raw or "Write" not in raw:
            continue
        mode = int(raw.split()[-1], 16) >> 26 & 3
        t = stamp(raw)
        if mode != prev:
            if mode == 1:
                t_out = t
            elif prev == 1 and t_out is not None:
                durs.append(t - t_out)
            prev = mode
    durs.sort()
    print(f"PB13 output episodes {len(durs)}; duration min {durs[0]:.9f} s, median {durs[len(durs) // 2]:.9f} s, "
          f"max {durs[-1]:.9f} s; total {sum(durs):.6f} s")
elif what in ("vect", "cr1"):
    cmds = open(sys.argv[3]).read().split("; ")
    st = steps(sys.argv[2], cmds)
    if what == "vect":
        i = cmds.index("sysbus ReadDoubleWord 0xe000ed04")
        exc = [(t, int(v[i], 16) & 0x1FF) for t, v in st if int(v[i], 16) & 0x1FF]
        print(f"in a handler at {len(exc)} of {len(st)} steps; vectors {sorted({e for _, e in exc})}; at {exc}")
    else:
        i1, i2 = cmds.index("sysbus ReadDoubleWord 0x40004400"), cmds.index("sysbus ReadDoubleWord 0x40004404")
        for t, v in st:
            if t in (7000, 8000, 8300, 9000, 9500, 10100, 10300, 11000):
                print(f"{t} ms  USART2 CR1 {v[i1]}  CR2 {v[i2]}")
elif what == "arm":
    ts, vals = [], []
    for raw in lines(sys.argv[2]):
        if "usart1: " in raw and "to 0x28" in raw and "Write" in raw:
            ts.append(stamp(raw)); vals.append(int(raw.split()[-1], 16))
    b = bytes(vals)
    for i in range(len(b) - 16):
        if b[i] == 0xFD and b[i + 5] == 1 and b[i + 6] == 1 and b[i + 7:i + 10] == b"\x00\x00\x00":
            payload = b[i + 10:i + 10 + b[i + 1]]
            if len(payload) >= 7 and payload[6] & 0x80:
                print(f"{len(b)} bytes written to TDR; first armed heartbeat at offset {i}, virtual {ts[i]:.9f} s, "
                      f"base_mode {payload[6]}")
                break
elif what == "quantum":
    cmds = ["sysbus ReadDoubleWord 0xe000ed04", "sysbus ReadDoubleWord 0x58020400", "sysbus ReadDoubleWord 0x580244e8"]
    st = [(t, [int(x, 16) for x in v]) for t, v in steps(sys.argv[2], cmds)]
    for name, pred in [("ICSR VECTACTIVE != 0", lambda v: v[0] & 0x1FF != 0),
                       ("PB13 output (GPIOB MODER)", lambda v: v[1] >> 26 & 3 != 2),
                       ("SPI2EN clear (RCC APB1LENR)", lambda v: not v[2] >> 14 & 1)]:
        r = runs(st, pred)
        n = [(b - a) // 100 + 1 for a, b in r]
        print(f"{name}: {sum(n)} of {len(st)} quantum steps, {len(r)} episodes, longest {max(n, default=0)} quanta, "
              f"{n.count(1)} of one quantum; at 11.9999 s: {any(a <= 11999900 <= b for a, b in r)}")
elif what == "ciwall":
    for path in sys.argv[2:]:
        print(f"== {path.split('/')[-1]}")
        prev = None
        for raw in open(path, errors="replace"):
            if not raw.startswith("foreign\t") or "ANSWER 4400e:" not in raw:
                continue
            m = re.search(r"(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+)Z ANSWER 4400e: (\S+) .*'peak_rss_kib': (\d+)", raw)
            if not m:
                continue
            t = datetime.datetime.fromisoformat(m.group(1)[:26])
            gap = f"{(t - prev).total_seconds():.1f}" if prev else "-"
            prev = t
            print(f"{m.group(2)}: {gap} s since the previous ANSWER, peak {m.group(3)} KiB")
