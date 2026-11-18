"""AET-WO-4400e §1.1: read one run's Renode log (the CLI's own lines, virtual timestamps) into the witness table.

usage: python3 -I analyze.py RUN_DIR OUT_JSON
Reads RUN_DIR/console.log and RUN_DIR/tool/capture/usart1.bin. Every time is the log's own virtual stamp.
Device lines are matched as the whole text after the stamp, as the judge matches them (README, Booting firmware).
"""
import json
import re
import sys

run, out = sys.argv[1], sys.argv[2]
stamp = re.compile(r"^\[(\d\d):(\d\d):(\d\d)\.(\d{9})\] (.*)$")
DEVICES = {
    "u4": "spi1.spi1mux.u4",
    "u5": "spi2.spi2mux.u5",
    "u2": "i2c1.u2",
    "u7": "i2c1.u7",
}
lines = []
for raw in open(f"{run}/console.log", errors="replace"):
    m = stamp.match(raw.rstrip("\n").strip("\r"))
    if m:
        h, mi, s, ns, text = m.groups()
        t = ((int(h) * 60 + int(mi)) * 60 + int(s)) * 1_000_000_000 + int(ns)
        lines.append((t, text))

res = {"lines": len(lines), "devices": {}, "errors": {}, "warnings_by_device": {}}
repeat = [x for x in lines if re.search(r"\(\d+\)$|repeated|times\)", x[1])]
res["collapse_markers"] = len(repeat)
first_model = None
for ref, path in DEVICES.items():
    own = [(t, x) for t, x in lines if x.split(" ", 1)[1].startswith(path + ":") if " " in x]
    noisy = [(t, x) for t, x in own if x.startswith("[NOISY]")]
    res["devices"][ref] = {
        "path": path,
        "lines": len(own),
        "first": noisy[0] if noisy else None,
        "first_20": noisy[:20],
        "by_text": {},
    }
    counts = {}
    for t, x in noisy:
        key = re.sub(r" -> .*", " -> …", x) if "read" in x else x
        counts[key] = counts.get(key, 0) + 1
    res["devices"][ref]["by_text"] = dict(sorted(counts.items(), key=lambda kv: -kv[1])[:25])
    res["warnings_by_device"][ref] = sorted({x for t, x in own if not x.startswith("[NOISY]")})[:20]
    if noisy and (first_model is None or noisy[0][0] < first_model[0]):
        first_model = noisy[0]
res["first_model_line"] = first_model


def first(pattern):
    for t, x in lines:
        if re.search(pattern, x):
            return (t, x)
    return None


res["witness"] = {
    "u4 identity (JEDEC 0x9F)": first(r"spi1\.spi1mux\.u4: .*(0x9F|9F)"),
    "u2 identity": first(r"^\[NOISY\] i2c1\.u2: Write D0$"),
    "u7 identity": first(r"^\[NOISY\] i2c1\.u7: read 0x01 -> 0x50$"),
    "u7 first data (0x1D, 6 B)": first(r"^\[NOISY\] i2c1\.u7: read 0x1D -> (0x.. ){5}0x..$"),
    "u2 first data (0xF7)": first(r"^\[NOISY\] i2c1\.u2: Write F7$"),
    "u5 identity (0x75 -> 0x47)": first(r"^\[NOISY\] spi2\.spi2mux\.u5: bank 0 read 0x75 -> 0x47$"),
    "u5 first data (0x1D)": first(r"^\[NOISY\] spi2\.spi2mux\.u5: bank 0 read 0x1D"),
    "u5 FIFO count (0x2E)": first(r"^\[NOISY\] spi2\.spi2mux\.u5: bank 0 read 0x2E"),
    "u5 FIFO data (0x30)": first(r"^\[NOISY\] spi2\.spi2mux\.u5: bank 0 read 0x30"),
}
errs = {}
for t, x in lines:
    if x.startswith("[ERROR]"):
        k = re.sub(r"\[cpu: 0x[0-9A-F]+\] ", "", x)
        e = errs.setdefault(k, {"count": 0, "first": t})
        e["count"] += 1
res["errors"] = errs
try:
    data = open(f"{run}/tool/capture/usart1.bin", "rb").read()
    res["console_text"] = [s for s in re.findall(rb"[ -~]{4,}", data)][:40]
    res["console_text"] = [s.decode() for s in res["console_text"]]
except OSError:
    res["console_text"] = None
json.dump(res, open(out, "w"), indent=1, default=str)
print(json.dumps({k: res[k] for k in ("lines", "collapse_markers", "first_model_line", "witness")}, indent=1, default=str))
print(json.dumps(res["errors"], indent=1))
