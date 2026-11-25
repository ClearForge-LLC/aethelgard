"""AET-WO-4400j (1), F4: the CLI's own end-of-window reads, taken every STEP_MS in a replay (replay.sh STEP_MS), each
register judged by the verdict's own rule (`equal`: observed & mask == expected; `at_least`: observed & expected ==
expected). Prints each register that fails at any step: its failing intervals, its share of the stepped time, and the
period of its failures; then, at each LENGTH, which registers fail.
Usage: endreads.py VERDICT READS CONSOLE.gz [LENGTH_MS ...]"""
import gzip, json, re, sys
verdict, reads_p, console = sys.argv[1:4]
lengths = [int(x) for x in sys.argv[4:]]
US = 1000   # AET-READ stamps are in us (replays before this change stamped ms: pass OLD=1)
import os
if os.environ.get("OLD") == "1": US = 1
regs = {int(r["address"], 16): r for r in json.load(open(verdict))["boot"]["registers"]}
cmds = open(reads_p).read().split("; ")
value = re.compile(r"^(0x[0-9A-Fa-f]+|\d+(\.\d+)?)$")
steps, cur, vals = [], None, []
for raw in gzip.open(console, "rt", errors="replace"):
    line = raw.strip()
    m = re.match(r"^AET-READ t=(\d+)$", line)
    if m:
        if cur is not None: steps.append((cur, vals))
        cur, vals = int(m.group(1)) // US, []
    elif cur is not None and value.match(line) and len(vals) < len(cmds):
        vals.append(line)
if cur is not None: steps.append((cur, vals))
bad_len = [t for t, v in steps if len(v) != len(cmds)]
print(f"{len(steps)} steps from {steps[0][0]} to {steps[-1][0]} ms; {len(cmds)} reads each; incomplete steps: {bad_len[:5]}")
def ok(r, v):
    o, m, e = int(v, 16), int(r["mask"], 16), int(r["expected"], 16)
    return (o & m) == e if r["rule"] == "equal" else (o & e) == e
fails = {}
for t, v in steps:
    for c, x in zip(cmds, v):
        m = re.match(r"sysbus ReadDoubleWord (0x[0-9a-f]+)$", c)
        if not m or int(m.group(1), 16) not in regs: continue
        r = regs[int(m.group(1), 16)]
        if not ok(r, x):
            fails.setdefault(r["name"], []).append((t, x))
span = steps[-1][0] - steps[0][0] + (steps[1][0] - steps[0][0] if len(steps) > 1 else 0)
step = steps[1][0] - steps[0][0] if len(steps) > 1 else 1
for name, fs in fails.items():
    iv, start, prev = [], fs[0][0], fs[0][0]
    for t, _ in fs[1:]:
        if t != prev + step: iv.append((start, prev)); start = t
        prev = t
    iv.append((start, prev))
    starts = [a for a, _ in iv]
    gaps = sorted({b - a for a, b in zip(starts, starts[1:])})
    print(f"{name}: fails at {len(fs)} of {len(steps)} steps ({100 * len(fs) / len(steps):.1f} %), values {sorted({x for _, x in fs})}")
    print(f"   intervals (ms, inclusive): {iv[:12]}{' …' if len(iv) > 12 else ''}  ({len(iv)} intervals; starts' spacings {gaps[:6]})")
for L in lengths:
    s = [v for t, v in steps if t == L]
    at = [n for n, fs in fails.items() if any(t == L for t, _ in fs)]
    print(f"at {L} ms: {'no step' if not s else ('fails: ' + ', '.join(at) if at else 'clean')}")
