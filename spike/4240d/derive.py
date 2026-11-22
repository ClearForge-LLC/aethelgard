"""Derive examples/sentinel-02-bench-hse8.json from sentinel-02-bench.json (AET-WO-4240d)."""
import json, sys
src, dst = sys.argv[1], sys.argv[2]
raw = open(src, encoding="utf-8").read()
s = json.loads(raw)
assert json.dumps(s, indent=2, ensure_ascii=False) + "\n" == raw, "round trip differs"
s["project_name"] = "sentinel-02-bench-hse8"
mcu = {}
for k, v in s["mcu"].items():
    if k == "hse_hz":
        assert v == 0
        mcu["hse"] = {"external": {
            "hz_num": 8000000,
            "hz_den": 1,
            "ppm": 80000,
            "source": "ST-LINK-V3E MCO = STM32F723 HSI/2 (RN0093 Rev 40 p.13); DS11853 Rev 8 Table 42 p.130 ACCHSI -8/+4.5 % at TA -40..105 C, so 80000 per side; B-1 confirms 8.000 MHz",
            "startup_max_us": 20000,
            "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)",
        }}
    else:
        mcu[k] = v
s["mcu"] = mcu
board = {}
for k, v in s["board"].items():
    board[k] = v
    if k == "forward":
        pass
board["drawn"] = False
s["board"] = board
for f in ("source",):
    t = mcu["hse"]["external"][f]
    assert 1 <= len(t) <= 200 and t.isascii() and t.isprintable(), (f, len(t))
open(dst, "w", encoding="utf-8").write(json.dumps(s, indent=2, ensure_ascii=False) + "\n")
