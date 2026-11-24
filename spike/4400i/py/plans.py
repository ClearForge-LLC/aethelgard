"""AET-WO-4400i (1): the injection plans (frames.py input), written so that runs differ only where named.

Every plan carries the rest stimuli this work order proposes (P4), set before the first instruction:
  - the ICM-42688-P at rest by U5's declared orientation (x right, z up): DS-000347 rev 1.9 §4.7, "placed on a flat
    surface, it will measure 0g on the X- and Y-axes and +1g on the Z-axis"; a chip axis declared up reads +1 g,
    down -1 g, otherwise 0 (rest() below, from the orientation alone);
  - the BMP581 at the parts table's 101325 Pa (parts.json:184), moved +-0.08 Pa (BST-BMP581-DS004-13 Table 9, p. 19,
    the x128 row's typical RMS noise) at that row's typical rate, 12 Hz, so every 83.333 ms.
Plans (window_ns, frames padded or not, spaced or not, IDLE hooks or not):
  early      the window (P5): the parameters and the accelerometer accept every 250 ms from 0.25 s to 6 s, prearm
             checks every 500 ms from 1 s, ARM (no force) every 250 ms from 3 s to 11.5 s; to 12 s; padded
  noreq      early to 4 s, run with no request line (P1.2)
  idle-*     the IDLE runs (P3): eleven distinct requests from 3.0 s, 200 ms apart, each answered by name, one of
             them a frame over 64 B, the last a short frame followed by silence; to 6 s
               idle-pad    padded to a multiple of 64 B, bytes at one instant (4400h's stand-in)
               idle-raw    unpadded, bytes one character time apart (115200 Bd, 8N1: 86.806 us, rounded up to the
                           100 us quantum), no IDLE
               idle-hook   idle-raw with the IDLE hooks (replay.sh IDLE=1)
  reject     a parameter ArduPilot does not have, and one it refuses, then read back (acceptance's PARAM_SET); padded
Usage: python3 -I plans.py OUTDIR"""
import json, sys
S = 1_000_000_000
def ms(x): return int(round(x * 1_000_000))
QUANTUM = 100_000
CHAR = 100_000   # 10 bits at 115200 Bd = 86,806 ns, rounded up to a whole quantum (serial.rs spacing_ns's rule)

ORIENT = {"x": "right", "z": "up"}   # U5, sentinel-03-foreign.json (4400h derive.py; 4300k's sentinel-03.json)
UNIT = {"forward": (1, 0, 0), "aft": (-1, 0, 0), "right": (0, 1, 0), "left": (0, -1, 0), "down": (0, 0, 1), "up": (0, 0, -1)}
def cross(a, b): return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])
def rest(o):
    """Each chip axis's reading at rest, in g: the board frame is x forward, y right, z down (orientation.rs), the
    board lies F.Cu up, so 'up' is -z; an axis reads +1 g pointing up (DS-000347 §4.7). Y = Z x X."""
    x, z = UNIT[o["x"]], UNIT[o["z"]]
    y = cross(z, x)
    up = (0, 0, -1)
    dot = lambda a: sum(p * q for p, q in zip(a, up))
    return {"X": dot(x), "Y": dot(y), "Z": dot(z)}
R = rest(ORIENT)
assert R == {"X": 0, "Y": 0, "Z": 1}, R
GRAVITY = [(0, "monitor", {"cmd": f"sysbus.spi2.spi2mux.u5 Acceleration{a} {v}"}) for a, v in sorted(R.items())]
BARO_PERIOD = 83_333_333   # 1/12 Hz
def baro(end):
    return [(t, "monitor", {"cmd": f"sysbus.i2c1.u7 Pressure {101325 + (0.08 if k % 2 else -0.08):.2f}"})
            for k, t in enumerate(range(0, end, BARO_PERIOD))]

PARAMS = [("COMPASS_USE", 0), ("FRAME_CLASS", 1), ("FS_THR_ENABLE", 0), ("INS_ACCOFFS_X", 0.001)]
ACCEPT = (241, [0, 0, 0, 0, 76, 0, 0])   # MAV_CMD_PREFLIGHT_CALIBRATION, param5 76: accept the accelerometers as they are
ARM = (400, [1, 0])                     # MAV_CMD_COMPONENT_ARM_DISARM, arm, param2 0: no force
PREARM = (401, [])                      # MAV_CMD_RUN_PREARM_CHECKS

def write(name, steps, end, pad, spacing=0):
    steps = sorted(GRAVITY + steps + baro(end), key=lambda s: s[0])
    json.dump({"window_ns": end, "pad": pad, "spacing_ns": spacing, "steps": [list(s) for s in steps]},
              open(f"{sys.argv[1]}/{name}.json", "w"), indent=1)

early = []
for k, t in enumerate(range(ms(250), ms(6000) + 1, ms(250))):
    early += [(t + i * ms(10), "param_set", {"id": p, "value": v}) for i, (p, v) in enumerate(PARAMS)]
    early.append((t + ms(40), "command_long", {"command": ACCEPT[0], "params": ACCEPT[1]}))
# No two padded frames at one instant: 128 B written at once is read only up to the first 64 (idle-pad, 4.0 s).
early += [(t + ms(60), "command_long", {"command": PREARM[0]}) for t in range(ms(1000), ms(11500) + 1, ms(500))]
early += [(t + ms(100), "command_long", {"command": ARM[0], "params": ARM[1]}) for t in range(ms(3000), ms(11500) + 1, ms(250))]
early += [(t + ms(200), "heartbeat", {}) for t in range(ms(500), ms(11500) + 1, ms(1000))]
write("early", early, ms(12000), pad=True)
write("noreq", [s for s in early if s[0] < ms(4000)], ms(4000), pad=True)

# Eleven requests, each answered by a reply that names it: PARAM_REQUEST_READ of a distinct parameter (PARAM_VALUE
# carries its id), and one FILE_TRANSFER_PROTOCOL ListDirectory of a long path that does not exist (over 64 B; a NAK
# carries its sequence number). The last is a short frame followed by silence.
names = ["FRAME_CLASS", "FS_THR_ENABLE", "COMPASS_USE", "INS_ACCOFFS_X", "DISARM_DELAY", "ARMING_CHECK",
         "SERIAL0_BAUD", "LOG_BACKEND_TYPE", "INS_ACC_ID", "SYSID_THISMAV"]
idle = [(ms(3000) + i * ms(200), "param_request_read", {"id": n}) for i, n in enumerate(names[:5])]
idle.append((ms(4000), "ftp_list", {"seq": 7, "path": "@ROMFS/" + "aethelgard-4400i-a-path-that-does-not-exist-" * 1 + "x" * 20}))
idle += [(ms(4200) + i * ms(200), "param_request_read", {"id": n}) for i, n in enumerate(names[5:])]
write("idle-pad", idle, ms(6000), pad=True)
write("idle-raw", idle, ms(6000), pad=False, spacing=CHAR)
write("idle-hook", idle, ms(6000), pad=False, spacing=CHAR)

reject = [(ms(3000), "param_set", {"id": "AET_NOT_A_PARAM", "value": 1}),
          (ms(3200), "param_set", {"id": "FRAME_CLASS", "value": 99}),
          (ms(3400), "param_set", {"id": "STAT_RUNTIME", "value": 5}),
          (ms(3600), "param_request_read", {"id": "FRAME_CLASS"}),
          (ms(3800), "param_request_read", {"id": "STAT_RUNTIME"})]
write("reject", reject, ms(5000), pad=True)
