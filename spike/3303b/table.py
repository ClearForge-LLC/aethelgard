#!/usr/bin/env python3
"""AET-WO-3303b: the before/after table from the runs' rows.tsv, beside PREDICTIONS.md's figures.

Usage: python3 -I spike/3303b/table.py ~/.cache/aet-3303b/runs > table.md
Rows: board step exit wall_ms peak_bytes max_bytes (sh/step.sh). Prints Markdown: one row per
(board, harness), before and after side by side, with the prediction for each part; then the
schedule's totals (sum of the parts' wall times, the largest part's peak) and the verify rows.
"""
import sys
from pathlib import Path

RUNS = Path(sys.argv[1])
BOARDS = ["sentinel-01", "sentinel-01-bench", "logger-i2c-only", "logger-spi-only-cap"]
# PREDICTIONS.md §2: (seconds, GB) for each part.
PRED = {
    ("sentinel-01", "every_valid_sequence_i2c1"): (15, 1.0),
    ("sentinel-01", "every_valid_sequence_spi1"): (45, 2.0),
    ("sentinel-01-bench", "every_valid_sequence_i2c1"): (15, 1.0),
    ("sentinel-01-bench", "every_valid_sequence_i2c2"): (15, 1.0),
    ("sentinel-01-bench", "every_valid_sequence_spi1"): (45, 2.0),
    ("logger-i2c-only", "every_valid_sequence_i2c1"): (15, 1.0),
    ("logger-i2c-only", "every_valid_sequence_i2c2"): (15, 1.0),
    ("logger-i2c-only", "every_valid_sequence_gpio"): (4, 0.3),
    ("logger-spi-only-cap", "every_valid_sequence_spi1"): (45, 2.0),
    ("logger-spi-only-cap", "every_valid_sequence_spi2"): (45, 2.0),
}
# Ranges (s lo, s hi, GB lo, GB hi), PREDICTIONS.md §2.
RANGE = {"i2c": (8, 45, 0.5, 2.0), "spi": (20, 120, 1.0, 3.5), "gpio": (2, 15, 0.1, 1.0)}


def rows(phase, board):
    p = RUNS / phase / board / "rows.tsv"
    out = {}
    if p.exists():
        for line in p.read_text().splitlines():
            f = line.split("\t")
            if len(f) >= 5:
                out[f[1]] = (f[2], f[3], f[4])  # last row of a step wins
    return out


def s(ms):
    return f"{int(ms) / 1000:.1f} s" if ms.isdigit() else ms


def gb(b):
    return f"{int(b) / 1e9:.2f} GB" if b.isdigit() else b


def cell(r):
    return "—" if r is None else f"{r[0]} · {s(r[1])} · {gb(r[2])}"


def verdict(board, step, r):
    key = (board, step.removeprefix("prove:"))
    if key not in PRED or r is None or not r[1].isdigit():
        return ""
    unit = next(k for k in RANGE if key[1].endswith(k) or k in key[1].rsplit("_", 1)[-1])
    lo_s, hi_s, lo_g, hi_g = RANGE[unit]
    t, g = int(r[1]) / 1000, int(r[2]) / 1e9
    ok_t = lo_s <= t <= hi_s
    ok_g = lo_g <= g <= hi_g
    return ("time held" if ok_t else "time MISS") + ", " + ("peak held" if ok_g else "peak MISS")


print("| Board | Harness | Before (exit · wall · peak) | After (exit · wall · peak) | Predicted | Held? |")
print("|---|---|---|---|---|---|")
totals = {}
for b in BOARDS:
    before, after = rows("before", b), rows("after", b)
    steps = [k for k in before if k.startswith("prove:")]
    steps += [k for k in after if k.startswith("prove:") and k not in steps]
    for st in steps:
        p = PRED.get((b, st.removeprefix("prove:")))
        pc = f"0 · {p[0]} s · {p[1]} GB" if p else ""
        print(f"| {b} | `{st.removeprefix('prove:')}` | {cell(before.get(st))} | {cell(after.get(st))} | {pc} | {verdict(b, st, after.get(st))} |")
    for ph, rs in (("before", before), ("after", after)):
        sched = [v for k, v in rs.items() if k.startswith("prove:every_valid_sequence") and "(absent)" not in k]
        if sched and all(v[1].isdigit() for v in sched):
            totals[(b, ph)] = (sum(int(v[1]) for v in sched) / 1000, max(int(v[2]) for v in sched) / 1e9, len(sched))
    for ph, rs in (("before", before), ("after", after)):
        if "verify" in rs:
            totals[(b, ph, "verify")] = rs["verify"]

print()
print("| Board | Schedule before (s · peak) | Schedule after: sum of parts (s · largest peak) | verify before (exit · wall · peak) | verify after |")
print("|---|---|---|---|---|")
for b in BOARDS:
    tb, ta = totals.get((b, "before")), totals.get((b, "after"))
    fmt = lambda t: "—" if t is None else f"{t[0]:.1f} s · {t[1]:.2f} GB" + (f" ({t[2]} parts)" if t[2] > 1 else "")
    vb, va = totals.get((b, "before", "verify")), totals.get((b, "after", "verify"))
    print(f"| {b} | {fmt(tb)} | {fmt(ta)} | {cell(vb)} | {cell(va)} |")
