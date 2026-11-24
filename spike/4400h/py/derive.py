"""AET-WO-4400h (3): Sentinel-03 (main's fixture, schema 29) made runnable as a foreign board, three declarations added
and nothing else changed:
  - mcu.foreign_clock, copied from sentinel-02-own (ArduPilot's declared clock: the same MCU, the same unmodified
    ArduPilot), so --foreign admits it (spike/2311/derive.py's rule);
  - U5's (the ICM-42688-P's) orientation, copied from sentinel-02-own's U5, which `generate hwdef` needs
    (ImuOrientationUndeclared without it). AET-WO-4300k's examples/sentinel-03.json (e82feda9, schema 30, unmerged)
    declares the same {"x": "right", "z": "up"};
  - board.forward, copied from sentinel-02-own ("top"), which an orientation needs (ForwardUndeclared without it);
    4300k's examples/sentinel-03.json declares the same "top".
Usage: python3 -I derive.py SENTINEL_03 SENTINEL_02_OWN OUT"""
import json, sys
s03, s02, out = sys.argv[1:]
d = json.load(open(s03)); src = json.load(open(s02))
assert "foreign_clock" not in d["mcu"]
d["mcu"]["foreign_clock"] = src["mcu"]["foreign_clock"]
imu = [s for s in d["spi_devices"] if s["reference"] == "U5" and s["part"] == "ICM42688P"]
ref = [s for s in src["spi_devices"] if s["reference"] == "U5" and s["part"] == "ICM42688P"]
assert len(imu) == 1 and len(ref) == 1 and not imu[0].get("orientation")
imu[0]["orientation"] = ref[0]["orientation"]
assert "forward" not in d["board"]
d["board"]["forward"] = src["board"]["forward"]
json.dump(d, open(out, "w"), indent=2)
