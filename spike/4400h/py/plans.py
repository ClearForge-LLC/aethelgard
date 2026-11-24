"""AET-WO-4400h (2)-(4): write the injection plans (frames.py input), so that the runs differ only where named.
  arm.json       heartbeat, stream requests, prearm checks, an arm attempt with defaults, the four parameters
                 and the accel-calibration accept, prearm again, ARM, GUIDED, TAKEOFF, GUIDED_NOGPS + thrust,
                 a read-back
  params.json    arm.json with every frame from the second ARM on replaced by a 64-byte zero frame (the
                 unarmed control: the same parameters, the same byte count)
  pad.json       every frame of arm.json a zero frame (the injection's own traffic)
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
def write(name, steps, end=END):
    json.dump({"window_ns": end, "steps": [list(s) for s in steps]}, open(f"{sys.argv[1]}/{name}.json", "w"), indent=1)
write("arm", head + params + armed + tail)
write("params", head + params + pads(armed) + tail)
write("pad", pads(head + params + armed + tail))
reboot = [
    (ms(9690), "param_set", {"id": "MOT_PWM_TYPE", "value": 6}),
    (ms(9800), "command_long", {"command": 246, "params": [1]}),                 # PREFLIGHT_REBOOT_SHUTDOWN: reboot
]
after = 22 * S
def shift(steps, by): return [(t + by, k, f) for t, k, f in steps]
read_back = [(ms(18000) + i * ms(50), "param_request_read", {"id": p}) for i, p in
             enumerate(["FRAME_CLASS", "FS_THR_ENABLE", "DISARM_DELAY", "INS_ACCOFFS_X", "MOT_PWM_TYPE", "INS_ACC_ID"])]
again = [(ms(17900), "heartbeat", {}),
         (ms(17950), "command_long", {"command": 511, "params": [36, 200_000]})] + read_back + [
         (ms(18350), "command_long", {"command": 241, "params": [0, 0, 0, 0, 76, 0, 0]}),  # accept again: INS_ACC_ID unsaved
         (ms(18500), "command_long", {"command": 401})] + shift(armed, ms(8000))
write("dshot", head + params[:4] + reboot + again, end=after + 2 * S)
