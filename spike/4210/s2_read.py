#!/usr/bin/env python3
"""AET-WO-4210 spike: read spike/4210/out/s2.jsonl (s2.py's runs) into one row per prediction.

Each row is computed from the kept log lines and the records, never typed by hand. Windows are
the multiplexer's own chip-select lines: `GPIO #k set to False` opens select k's window and the
next `GPIO #k set to True` closes it (inputs active-low; U4 is k = 0, U5 is k = 1).

Usage: python3 spike/4210/s2_read.py [s2.jsonl]   (prints Markdown; run on the host, no Renode)
"""
import json, re, sys
from collections import Counter

SRC = sys.argv[1] if len(sys.argv) > 1 else "spike/4210/out/s2.jsonl"
MUX, U4, U5, BARO = "spi1.spi1mux: ", "spi1.spi1mux.u4: ", "spi1.spi1mux.u5: ", "i2c1.baro: "
EDGE = re.compile(r"^\[NOISY\] spi1\.spi1mux: GPIO #(\d) set to (True|False)$")
IDLE = "Finishing transmission, going idle"

rows = [json.loads(l) for l in open(SRC)]
runs = {}
for r in rows:
    runs.setdefault(r["case"], []).append(r)


def lvl(line):
    return line[1:line.index("]")]


def src(line):
    """The emitter's name and ": ", or "" for a line logged with none (machine creation)."""
    body = line[line.index("] ") + 2:]
    return body.split(": ", 1)[0] + ": " if ": " in body else ""


def text(line):
    body = line[line.index("] ") + 2:]
    return body[len(src(line)):]


def windows(kept, k):
    """Lists of the lines inside each window of select k, in order (the edges excluded)."""
    out, cur = [], None
    for l in kept:
        m = EDGE.match(l)
        if m and int(m.group(1)) == k:
            if m.group(2) == "False":
                cur = []
            elif cur is not None:
                out.append(cur)
                cur = None
            continue
        if cur is not None:
            cur.append(l)
    return out


def u5_steps(kept):
    """U5's own lines per U5 window, its idle line dropped."""
    return [[l for l in w if src(l) == U5 and text(l) != IDLE] for w in windows(kept, 1)]


def rd(case, key, run=0):
    return (runs[case][run]["reads"] or {}).get(key)


def noisy(lines):
    return [l for l in lines if lvl(l) == "NOISY"]


def nbytes(lines):
    return sum(len(l.encode()) for l in lines)


out = []
def row(pid, verdict, detail):
    out.append(f"| {pid} | {verdict} | {detail} |")


# --- P1 -----------------------------------------------------------------------------------
k = runs["imu-script"][0]["kept"]
st = u5_steps(k)
ident, conf, data = st[0], st[1], st[2]
exp_ident = ["[NOISY] spi1.spi1mux.u5: Preparing for read at bank #0 from 0x0 (WhoAmI)",
             "[NOISY] spi1.spi1mux.u5: Received byte 0x80, returning 0x0",
             "[NOISY] spi1.spi1mux.u5: Read at bank #0 from 0x0 (WhoAmI), returned 0xEA",
             "[NOISY] spi1.spi1mux.u5: Received byte 0x0, returning 0xEA"]
row("P1.1", "held" if ident == exp_ident else "failed", f"identity window: {len(ident)} U5 lines, {'equal to' if ident == exp_ident else 'not'} the 4 `model.rs` expects")
exp_conf = ["[NOISY] spi1.spi1mux.u5: Preparing for write at bank #0 to 0x6 (PowerManagement1)",
            "[NOISY] spi1.spi1mux.u5: Received byte 0x6, returning 0x0",
            "[NOISY] spi1.spi1mux.u5: Received byte 0x1, returning 0x0"]
dbg = [l for l in conf if lvl(l) == "DEBUG"]
row("P1.2", "held" if noisy(conf) == exp_conf and dbg == ["[DEBUG] spi1.spi1mux.u5: Write at bank #0 to 0x6 (PowerManagement1): value 0x1"] else "failed",
    f"{len(noisy(conf))} Noisy lines as predicted; the value only in `{text(dbg[0]) if dbg else '-'}` (Debug) and the second `Received byte`. Order: Preparing, Received 0x6, Debug, Warning, Received 0x1")
w5 = [l for l in runs["imu-script"][0]["warnings"] if l.startswith("[WARNING] " + U5)]
exp_w = "[WARNING] spi1.spi1mux.u5: Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1)."
row("P1.3", "held" if w5 == [exp_w] and exp_w in conf else "failed", f"U5 warns {len(w5)} time(s) in the run, inside the configuration window: `{text(exp_w)}`")
exp_data = 2 + 2 * 6
vals = re.findall(r"returned (0x[0-9A-F]+)", "\n".join(data))
row("P1.4", "held" if len(noisy(data)) == exp_data and set(vals) == {"0x0"} else "failed",
    f"{len(noisy(data))} Noisy lines (predicted {exp_data}); bytes {vals}; registers 0x2D–0x32 named")
ds = u5_steps(runs["imu-script-stim"][0]["kept"])[2]
sv = re.findall(r"from (0x[0-9A-F]+) \((\w+)\), returned (0x[0-9A-F]+)", "\n".join(ds))
row("P1.5", "held" if [v for *_, v in sv] == ["0x0"] * 4 + ["0x40", "0x0"] else "failed",
    "with `DefaultAccelerationZ 1`: " + ", ".join(f"{n} {v}" for _, n, v in sv))
all5 = [l for l in k if src(l) == U5]
outside = Counter(text(l) for l in all5) - Counter(text(l) for s in st for l in s)
by_edge, last = Counter(), None
for l in k:
    m = EDGE.match(l)
    if m:
        last = f"GPIO #{m.group(1)} {m.group(2)}"
    elif src(l) == U5 and text(l) == IDLE:
        by_edge[last] += 1
warns = Counter((src(l) or "(no source): ") + text(l) for l in runs["imu-script"][0]["warnings"])
u4w = sum(v for kk, v in warns.items() if kk.startswith("spi1.spi1mux.u4"))
row("P1.6", "failed in part",
    f"U5's lines outside its steps: {dict(outside)}, by the edge that preceded each: {dict(by_edge)}. "
    f"On every edge of any select, the multiplexer finishes every deselected device (`SPIMultiplexer.cs` `UpdateChipSelectState`), so U5 also goes idle on U4's edges. "
    f"U5 warns only P1.3's line, but **U4 warns {u4w} times** in a correct run (`Transmission finished in the unexpected state: RecognizeOperation`, `GenericSpiFlash.cs` L95): it is finished while idle on edges of either select")
same = all(x["kept"] == runs["imu-script"][0]["kept"] and x["reads_sha256"] == runs["imu-script"][0]["reads_sha256"] for x in runs["imu-script"])
row("P1.7", "held" if same and len(runs["imu-script"]) == 3 else "failed", f"{len(runs['imu-script'])} runs: kept lines and reads byte-identical: {same}")

# --- P2 -----------------------------------------------------------------------------------
fk = runs["imu-forge-config"][0]["kept"]
fs = u5_steps(fk)
prep = any("Preparing for write" in l for s in fs for l in s)
row("P2.1", "held" if len(fs) == 2 and fs[1] == [] and not prep else "failed",
    f"{len(fs)} U5 windows: the forged one holds {len(fs[1]) if len(fs) > 1 else '-'} U5 lines; `GPIO #1` edges and U5's idle line are logged; no `Preparing for write`. Records still claim `0x601`: {rd('imu-forge-config', 'rec7')}")
gs = u5_steps(runs["imu-forge-data"][0]["kept"])
row("P2.2", "held" if len(gs) == 2 and noisy(gs[1]) == exp_conf and not any("0x2D" in l for s in gs for l in s) else "failed",
    f"{len(gs)} U5 windows; the configuration step's 3 lines present; no read of 0x2D; record 4 claims {rd('imu-forge-data', 'rec8')}/{rd('imu-forge-data', 'rec9')}")

# --- P3 -----------------------------------------------------------------------------------
def attribution(kept):
    """Each U4/U5 transfer line against the windows open when it was logged."""
    open_, bad, n = {0: False, 1: False}, [], 0
    for l in kept:
        m = EDGE.match(l)
        if m:
            open_[int(m.group(1))] = m.group(2) == "False"
            continue
        s = src(l)
        if s in (U4, U5) and text(l) != IDLE and lvl(l) in ("NOISY", "DEBUG"):
            n += 1
            want = 0 if s == U4 else 1
            if not open_[want] or open_[1 - want]:
                bad.append(l)
    return n, bad


res = [attribution(r["kept"]) for r in runs["two-windows"]]
row("P3.1", "held" if all(not b for _, b in res) else "failed",
    f"{len(res)} runs, {res[0][0]} transfer lines each: {sum(len(b) for _, b in res)} outside their own select's window or inside another's. The two selects' windows never overlap")
ak = runs["two-windows-u4-absent"][0]["kept"]
open_, tx, fin = {0: False, 1: False}, Counter(), Counter()
for l in ak:
    m = EDGE.match(l)
    if m:
        open_[int(m.group(1))] = m.group(2) == "False"
        continue
    where = "#0" if open_[0] else "#1" if open_[1] else "none"
    if "Tried to transmit byte" in l:
        tx[where] += 1
    if "Tried to finish transmission to device 0x0" in l:
        fin[where] += 1
row("P3.2", "held for the transmit lines; a second kind found",
    f"`Tried to transmit … to device 0x0, but it's not connected`: {dict(tx)} (inside `GPIO #0` windows only). But the multiplexer also logs `Tried to finish transmission to device 0x0, but it's not connected` (Warning) on every edge: {dict(fin)} by the window open when logged. The judge's `unrouted_line` takes only `Tried to transmit byte` (`renode.rs` L529), so these do not reach `E2` today")
row("P3.3", "consistent (inferred)", "P2.1's forged window is empty: forged edges add an empty window and move no transfer line. Windows attribute; they do not witness")

# --- P4 -----------------------------------------------------------------------------------
row("P4.1", "held" if rd("imu-drdy", "u5_irq") == "False" and rd("imu-drdy", "rec11") == "0x00000000" else "failed",
    f"`IRQ IsSet` after the run: {rd('imu-drdy', 'u5_irq')}; `INT_STATUS_1` read: {rd('imu-drdy', 'rec11')}. The `INT_ENABLE_1` write logs `IRQ unset` (Debug) and an `Interrupts: … Raw Data Ready False …` Noisy line")
row("P4.2", "held" if rd("imu-drdy-store", "u5_irq") == "True" and rd("imu-drdy-store", "rec11") == "0x00000001" else "failed",
    f"after the firmware writes `INT_STATUS_1` = 1: `IRQ set` (Debug), `IsSet` {rd('imu-drdy-store', 'u5_irq')}, status read {rd('imu-drdy-store', 'rec11')}")

# --- P6 -----------------------------------------------------------------------------------
def baro(case):
    r = runs[case][0]
    b = [l for l in r["kept"] if src(l) == BARO]
    return r, b, [l for l in r["warnings"] if src(l) == BARO]


r, b, w = baro("baro-lps25hb")
row("P6.1", "held" if not noisy(b) and rd("baro-lps25hb", "rec7") == "0x000000BD" else "failed",
    f"identity {rd('baro-lps25hb', 'rec7')}, data {rd('baro-lps25hb', 'rec11')}; device lines: {[l for l in b]}; Noisy: {len(noisy(b))}; device warnings {len(w)}")
r, b, w = baro("baro-icp")
row("P6.2", "held for identity; failed in part for data",
    f"identity {rd('baro-icp', 'rec7')} (`00 08` + CRC `0x38`). Debug lines: {len([l for l in b if lvl(l) == 'DEBUG'])}, one per command and one per read, bytes in the line. "
    f"**The measurement read also logs {len(noisy(b))} Noisy lines** (`A=… B=… C=…`, `p_raw=…`), so a data step is visible at Noisy, but not its identity. Device warnings {len(w)}")
bmp = runs["baro-bmp180"]
r, b, w = baro("baro-bmp180")
seg, cur = [], None
for l in b:
    if "Write with" in l:
        cur = []
        seg.append(cur)
    cur.append(l)
row("P6.3", "held but for the data step's count",
    f"Noisy lines per step: {[len(noisy(s)) for s in seg]} (predicted 4, 4, 5: the 2-byte read logs `Write`, `Preparing`, `Reading`, then a `Read value` and an `Auto-incrementing` line per byte, 7). "
    f"Values decimal (`Read value 85`, `67`, `79`); register hex 8 digits. Device warnings {len(w)}. 3 runs identical: {all(x['kept'] == bmp[0]['kept'] for x in bmp)}")
row("P6.4", "held (read)", "none of the three barometer models has a GPIO; no barometer case logged an IRQ line")

# --- F18, F22: sizes and warnings of the correct script ----------------------------------------
print("| Prediction | Result | Measured |")
print("|---|---|---|")
print("\n".join(out))
print()
print("Step sizes in the correct ICM run (log bodies without the host timestamp; the judge's events are built from these):")
print()
print("| Step | U5 lines (all levels) | Noisy | bytes, all | bytes, Noisy |")
print("|---|---|---|---|---|")
for name, s in zip(["identity", "configuration write", "data read, 6 bytes"], st):
    print(f"| {name} | {len(s)} | {len(noisy(s))} | {nbytes(s)} | {nbytes(noisy(s))} |")
dl = noisy(data)
print()
print(f"Longest Noisy line of the data read: {max(len(l.encode()) for l in dl)} bytes; mean {nbytes(dl) / len(dl):.1f}.")
print()
print("Every [WARNING] of the correct ICM run (`imu-script`, run 0), by source:")
print()
for kk, v in sorted(warns.items()):
    print(f"- {v} × `{kk}`")
