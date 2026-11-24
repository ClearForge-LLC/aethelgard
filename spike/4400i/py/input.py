"""AET-WO-4400i (2): a declared input file (`simulate --foreign --input`), written by pymavlink 2.4.50 in the pinned
ArduPilot image, an encoder that is not the judge's. Usage: python -I input.py NAME OUT.json [PORT]
Each frame filled with zero bytes to a whole multiple of 64 (ArduPilot's RX bounce buffer; ruling D3: declared fill).
  arm5     within 5 s (ruling D1): heartbeats, the four parameters and the accelerometer accept, prearm checks, ARMs
  arm12    the same to 12 s, ARM every 250 ms from 3.15 s (the spike's armed replay)
  force    arm5 with one ARM carrying the force code 21196 (refused: InputForcesArm)
  junk     arm5 with one frame that is text, not MAVLink (refused: InputNotMavlink)
  reject   arm5 plus a PARAM_SET of a parameter ArduPilot does not have (ruling D9: PARAM_ERROR)"""
import json, sys
from pymavlink.dialects.v20 import ardupilotmega as mav
name, out = sys.argv[1], sys.argv[2]
port = sys.argv[3] if len(sys.argv) > 3 else "CONSOLE"
m = mav.MAVLink(None, srcSystem=255, srcComponent=190)
def ms(x): return int(round(x * 1_000_000))
PARAMS = [("COMPASS_USE", 0), ("FRAME_CLASS", 1), ("FS_THR_ENABLE", 0), ("INS_ACCOFFS_X", 0.001)]
def plan(end, arm_from, arm_every, params_until):
    s = []
    for t in range(ms(500), end, ms(1000)):
        s.append((t, m.heartbeat_encode(mav.MAV_TYPE_GCS, mav.MAV_AUTOPILOT_INVALID, 0, 0, 0)))
    for k, t in enumerate(range(ms(750), params_until, ms(500))):
        for i, (p, v) in enumerate(PARAMS):
            s.append((t + i * ms(10), m.param_set_encode(1, 1, p.encode(), float(v), mav.MAV_PARAM_TYPE_REAL32)))
        s.append((t + ms(50), m.command_long_encode(1, 1, 241, 0, 0, 0, 0, 0, 76, 0, 0)))
    for t in range(ms(1000), end, ms(1000)):
        s.append((t + ms(100), m.command_long_encode(1, 1, 401, 0, 0, 0, 0, 0, 0, 0, 0)))
    for t in range(arm_from, end - ms(100), arm_every):
        s.append((t, m.command_long_encode(1, 1, 400, 0, 1, 0, 0, 0, 0, 0, 0)))
    return sorted(s, key=lambda x: x[0])
def frame(t, msg):
    b = msg.pack(m)
    return {"at_ns": t, "mavlink": b.hex(), "fill": -len(b) % 64}
if name == "arm12":
    # The parameters again after `ArduPilot Ready` (5.2 s): sent only before it (arm12-s03-early-params), they
    # are not in force at the arm ("Motors: Check frame class and type"), as in the spike's early plan.
    steps = plan(ms(12000), ms(3150), ms(250), ms(6300))
else:
    steps = plan(ms(5000), ms(3150), ms(250), ms(3000))
frames = [frame(t, msg) for t, msg in steps]
if name == "force":
    frames.append(frame(ms(4900), m.command_long_encode(1, 1, 400, 0, 1, 21196, 0, 0, 0, 0, 0)))
if name == "junk":
    frames.append({"at_ns": ms(4900), "mavlink": b"ARM NOW".hex(), "fill": 0})
if name == "reject":
    frames.append(frame(ms(4300), m.param_set_encode(1, 1, b"AET_NOT_A_PARAM", 1.0, mav.MAV_PARAM_TYPE_REAL32)))
frames.sort(key=lambda f: f["at_ns"])
json.dump({"input_version": 1, "port": port, "frames": frames}, open(out, "w"), indent=1)
print(f"input: {name}, {len(frames)} frames, port {port}")
