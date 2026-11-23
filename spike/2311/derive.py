"""AET-WO-2311 §1.1: the four-port specification with sentinel-02-own's
mcu.foreign_clock added (ArduPilot's declared clock, the same MCU and the
same unmodified ArduPilot), so --foreign admits it. Nothing else changes.
python3 -I derive.py FOUR_PORT SENTINEL_02 OUT"""
import json, sys
four, s02, out = sys.argv[1:]
d = json.load(open(four))
assert "foreign_clock" not in d["mcu"]
d["mcu"]["foreign_clock"] = json.load(open(s02))["mcu"]["foreign_clock"]
json.dump(d, open(out, "w"), indent=2)
