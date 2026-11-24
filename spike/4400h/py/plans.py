"""AET-WO-4400h (2)-(4): write the injection plans (frames.py input), so that the runs differ only where named.
  arm.json       heartbeat, stream requests, prearm checks, an arm attempt with defaults, the four parameters
                 and the accel-calibration accept, prearm again, ARM, GUIDED, TAKEOFF, GUIDED_NOGPS + thrust,
                 a read-back
  params.json    arm.json with every frame from the second ARM on replaced by a 64-byte zero frame (the
                 unarmed control: the same parameters, the same byte count)
  pad.json       every frame of arm.json a zero frame (the injection's own traffic)
  none.json      no frame at all (the baseline); like every plan, it carries the barometer stimulus
  dshot.json     arm.json's parameters + MOT_PWM_TYPE 6, a reboot over MAVLink, read-backs after it
                 (persistence across a reset), then the accept, ARM and the same commands
Usage: python3 -I plans.py OUTDIR"""
import json, sys
S = 1_000_000_000
def ms(x): return int(x * 1_000_000)
END = 16 * S
head = [
    (ms(8000), "heartbeat", {}),
    (ms(8050), "command_long", {"command": 511, "params": [1, 1_000_000]}),      # SET_MESSAGE_INTERVAL SYS_STATUS 1 s
    (ms(8100), "command_long", {"command": 511, "params": [36, 200_000]}),       # SERVO_OUTPUT_RAW 200 ms
    (ms(8500), "command_long", {"command": 401}),                                # RUN_PREARM_CHECKS
    (ms(9000), "command_long", {"command": 400, "params": [1, 0]}),              # ARM, defaults (expected refused)
]
params = [
    (ms(9450), "param_set", {"id": "COMPASS_USE", "value": 0}),   # the board has no compass (arm1: "Compass 1 not healthy")
    (ms(9500), "param_set", {"id": "FRAME_CLASS", "value": 1}),
    (ms(9550), "param_set", {"id": "FS_THR_ENABLE", "value": 0}),
    (ms(9600), "param_set", {"id": "DISARM_DELAY", "value": 0}),
    (ms(9650), "param_set", {"id": "INS_ACCOFFS_X", "value": 0.001}),
    (ms(9700), "command_long", {"command": 241, "params": [0, 0, 0, 0, 76, 0, 0]}),  # PREFLIGHT_CALIBRATION accept
    (ms(10500), "command_long", {"command": 401}),
    (ms(10900), "heartbeat", {}),
]
armed = [
    (ms(11000), "command_long", {"command": 400, "params": [1, 0]}),             # ARM, no force
    (ms(12000), "command_long", {"command": 176, "params": [1, 4]}),             # DO_SET_MODE GUIDED
    (ms(12500), "command_long", {"command": 22, "params": [0, 0, 0, 0, 0, 0, 2]}),  # NAV_TAKEOFF 2 m
    (ms(13000), "command_long", {"command": 176, "params": [1, 20]}),            # DO_SET_MODE GUIDED_NOGPS
    (ms(13500), "set_attitude_target", {"thrust": 0.6}),
    (ms(14000), "set_attitude_target", {"thrust": 0.6}),
    (ms(14500), "set_attitude_target", {"thrust": 0.6}),
]
tail = [(ms(15000), "param_request_read", {"id": "FRAME_CLASS"})]
def pads(steps): return [(t, "pad", {}) for t, _, _ in steps]
def shift(steps, by): return [(t + by, k, f) for t, k, f in steps]
# The barometer stimulus, in every plan alike (arm1: "Baro: not healthy"). ArduPilot calls a barometer unhealthy
# whose reading has not changed for its data-change timeout (AP_Baro_Backend.cpp:36-41); the BMP581 model holds the
# Pressure it is given (BMP581.cs:55-56), 101325 Pa. A real barometer's reading moves: here by 0.5 Pa every 0.5 s,
# below its noise. Named in every record: this is a stimulus, not a model change.
def baro(end): return [(t, "monitor", {"cmd": f"sysbus.i2c1.u7 Pressure {101325 + 0.5 * (k % 2)}"})
                       for k, t in enumerate(range(ms(6000), end, ms(500)))]
# Gravity, from t = 0, in every plan alike (arm2, arm3: "Arm: Leaning"). The CLI sets the ICM-42688-P's witness
# stimulus from the parts table (crates/aethelgard-ir/data/parts.json:528: acceleration_y 1 g, x 0; z is the
# model's default 0, ICM42688P.cs:57-59): 1 g along the board's y, a board standing on its edge. A board at rest
# with the sensor's z axis up (U5's declared orientation, x right, z up) reads +1 g on z and 0 on x and y. Named in
# every record: a stimulus that overrides the witness's, not a model change.
GRAVITY = [(0, "monitor", {"cmd": f"sysbus.spi2.spi2mux.u5 Acceleration{a} {v}"}) for a, v in (("X", 0), ("Y", 0), ("Z", 1))]
def write(name, steps, end=END):
    steps = sorted(GRAVITY + steps + baro(end), key=lambda s: s[0])
    json.dump({"window_ns": end, "steps": [list(s) for s in steps]}, open(f"{sys.argv[1]}/{name}.json", "w"), indent=1)
write("arm", head + params + armed + tail)
write("params", head + params + pads(armed) + tail)
write("pad", pads(head + params + armed + tail))
write("none", [])
# The logged runs' plans (LogAllPeripheralsAccess): every frame 2 s earlier and the window 11.5 s, since a 16 s logged
# run passed the 4 GiB cap the ruling sets (s02-arm-log: killed after 14.87 s). The same frames, the same order; the
# window ends at 13 s, at the shifted read-back (s02-none-s first ran to 11.5 s and was run again).
SH = -2 * S
write("arm-s", shift(head + params + armed + tail, SH), end=ms(13000))
write("params-s", shift(head + params + pads(armed) + tail, SH), end=ms(13000))
write("pad-s", shift(pads(head + params + armed + tail), SH), end=ms(13000))
write("none-s", [], end=ms(13000))
reboot = [
    (ms(9690), "param_set", {"id": "MOT_PWM_TYPE", "value": 6}),
    (ms(9800), "command_long", {"command": 246, "params": [1]}),                 # PREFLIGHT_REBOOT_SHUTDOWN: reboot
]
after = 22 * S
read_back = [(ms(18000) + i * ms(50), "param_request_read", {"id": p}) for i, p in
             enumerate(["FRAME_CLASS", "FS_THR_ENABLE", "DISARM_DELAY", "INS_ACCOFFS_X", "MOT_PWM_TYPE", "INS_ACC_ID"])]
again = [(ms(17900), "heartbeat", {}),
         (ms(17950), "command_long", {"command": 511, "params": [36, 200_000]})] + read_back + [
         (ms(18350), "command_long", {"command": 241, "params": [0, 0, 0, 0, 76, 0, 0]}),  # accept again: INS_ACC_ID unsaved
         (ms(18500), "command_long", {"command": 401})] + shift(armed, ms(8000))
write("dshot", head + params[:5] + reboot + again, end=after + 2 * S)
# The DShot and persistence run, short enough for the 4 GiB cap: the parameters and MOT_PWM_TYPE 6 (DShot600), a
# reboot over MAVLink at 7.8 s, then every parameter read back at 16 s. No arm: DShot runs disarmed.
write("dshot-s", shift(head + params[:5] + reboot, SH) + [(t - ms(2000), k, f) for t, k, f in again[:2] + read_back], end=ms(17000))
