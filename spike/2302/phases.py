"""AET-WO-2302 spike: each phase's virtual time from a boot's raw log (watchpoint hooks, hooks.py).

An instruction is 10 ns (100 MIPS); every time printed here is executed instructions x 10 ns,
exact to the instruction. Usage: python3 -I phases.py <run.log> <run.verdict.json>
"""
import json, re, sys

NS = 10
HOOK = re.compile(r"\] \[WARNING\] sysbus: (AET\w+) ([0-9a-f]+) ([0-9a-f]+) (\d+)$")

def main():
    log, verdict = sys.argv[1], json.load(open(sys.argv[2]))
    ev = []
    for line in open(log, errors="replace"):
        m = HOOK.search(line.rstrip("\n"))
        if m:
            ev.append((m.group(1), int(m.group(2), 16), int(m.group(3), 16), int(m.group(4)) * NS))
    boot = verdict.get("boot", {})
    steps = [s for s in boot.get("i2c", []) + boot.get("spi", [])]
    i2c = boot.get("i2c", []); spi = boot.get("spi", [])
    data = lambda s: s.get("step", "").startswith("data")
    order = [s for s in i2c if not data(s)] + [s for s in spi if not data(s)] + \
            [s for s in i2c if data(s)] + [s for s in spi if data(s)]
    out = {}
    bw = [t for tag, a, v, t in ev if tag == "AETW" and a == 0x20000000 and v == 0x2100B007]
    out["boot_word_ns"] = bw[0] if bw else None
    opt = [t for tag, a, v, t in ev if tag == "AETW" and a == 0x20000004]
    out["optsr_ns"] = opt[0] if opt else None
    recs = [(a, v, t) for tag, a, v, t in ev if tag == "AETW" and a >= 0x20000008]
    i, rows = 0, []
    for s in order:
        n = 1 + max(1, -(-int(s.get("width", 1)) // 4))
        chunk = recs[i:i + n]; i += n
        rows.append((s.get("reference"), s.get("step"), chunk[-1][2] if chunk else None))
    out["steps"] = rows
    out["unassigned_record_writes"] = len(recs) - i
    tims = {}
    for tag, a, v, t in ev:
        if tag.startswith("AETT_") and (v & 1) and tag[5:] not in tims:
            tims[tag[5:]] = t
    out["start_outputs_ns"] = tims
    ser = {}
    b = out["boot_word_ns"] or 10**18
    for tag, a, v, t in ev:
        if tag.startswith("AETX_") or tag.startswith("AETR_"):
            p = tag[5:]; d = ser.setdefault(p, {"tx_before_boot": [], "tx_after_boot": [], "rx": []})
            if tag.startswith("AETR_"):
                d["rx"].append(t)
            else:
                d["tx_before_boot" if t < b else "tx_after_boot"].append(t)
    out["serial"] = {p: {"pattern_first_last_ns": (d["tx_before_boot"][:1] + d["tx_before_boot"][-1:]),
                         "pattern_bytes": len(d["tx_before_boot"]),
                         "echo_tx_first_last_ns": (d["tx_after_boot"][:1] + d["tx_after_boot"][-1:]),
                         "echo_bytes": len(d["tx_after_boot"]),
                         "rx_reads": len(d["rx"]), "rx_first_last_ns": d["rx"][:1] + d["rx"][-1:]}
                     for p, d in ser.items()}
    out["verdict"] = verdict.get("verdict"); out["findings"] = sorted({f["kind"] for f in verdict.get("findings", [])})
    print(json.dumps(out, indent=1))

main()
