"""AET-WO-4400h (2): a timed MAVLink 2 plan -> Renode monitor commands that inject it into USART1.

Run with the ArduPilot image's venv (pymavlink, unmodified). Usage:
  python -I frames.py PLAN.json OUT_DIR
PLAN.json: {"window_ns": N, "steps": [[t_ns, "msg", {fields}], ...]} in time order, where "msg" is
heartbeat | param_set | param_request_read | command_long | set_attitude_target | pad | monitor (a frame of 64 zero bytes, the control).
Each frame is padded with 0x00 to exactly 64 bytes, ArduPilot's RX bounce buffer
(AP_HAL_ChibiOS/UARTDriver.h:25), so that DMA1 stream 0's NDTR reaches 0 and its transfer-complete
interrupt hands the bytes to the driver (UARTDriver.cpp:606-642): the twin models no USART IDLE
(STM32F7_USART.cs:154, :231), the other path ArduPilot reads by. Zero bytes are not a MAVLink start
byte (0xFD), so ArduPilot's parser skips them.
Writes OUT_DIR/inject.resc (RunFor steps and `sysbus.usart1 WriteChar` lines, ending at window_ns)
and OUT_DIR/injected.tsv (t_ns, message, seq, the frame's hex): the injected bytes, for the record.
"""
import json, sys
from pymavlink.dialects.v20 import ardupilotmega as mav

plan = json.load(open(sys.argv[1]))
out = sys.argv[2]
m = mav.MAVLink(None, srcSystem=255, srcComponent=190)
BUF = 64


def encode(kind, f):
    if kind == "heartbeat":
        return m.heartbeat_encode(mav.MAV_TYPE_GCS, mav.MAV_AUTOPILOT_INVALID, 0, 0, 0)
    if kind == "param_set":
        return m.param_set_encode(1, 1, f["id"].encode(), float(f["value"]), mav.MAV_PARAM_TYPE_REAL32)
    if kind == "param_request_read":
        return m.param_request_read_encode(1, 1, f["id"].encode(), -1)
    if kind == "command_long":
        p = (list(f.get("params", [])) + [0] * 7)[:7]
        return m.command_long_encode(1, 1, int(f["command"]), 0, *[float(x) for x in p])
    if kind == "set_attitude_target":
        # type_mask 0b111 ignores body rates: a level attitude with the given thrust (GUIDED_NOGPS).
        return m.set_attitude_target_encode(0, 1, 1, 0b00000111, [1.0, 0.0, 0.0, 0.0], 0, 0, 0, float(f["thrust"]))
    raise SystemExit(f"frames: unknown message {kind}")


lines, rows, now = [], [], 0
for t, kind, f in plan["steps"]:
    if t < now:
        raise SystemExit("frames: steps out of order")
    if t > plan["window_ns"]:
        raise SystemExit(f"frames: a step at {t} ns is past the window, {plan['window_ns']} ns")
    if t > now:
        lines.append(f'emulation RunFor "{(t - now) / 1e9:.9f}"')
        now = t
    if kind == "monitor":   # a stimulus, not a USART byte: e.g. the BMP581 model's own Pressure property
        lines.append(f["cmd"])
        rows.append(f"{t}\tmonitor {f['cmd']}\t0\t-")
        continue
    if kind == "pad":
        frame = b""
        name = "pad"
    else:
        msg = encode(kind, f)
        frame = msg.pack(m)
        name = msg.get_type() + (" " + json.dumps(f, sort_keys=True) if f else "")
    if len(frame) > BUF:
        raise SystemExit(f"frames: {name} is {len(frame)} B, over the {BUF} B bounce buffer")
    padded = frame + bytes(BUF - len(frame))
    lines.extend(f"sysbus.usart1 WriteChar 0x{b:02X}" for b in padded)
    rows.append(f"{t}\t{name}\t{len(frame)}\t{frame.hex()}")
if plan["window_ns"] > now:
    lines.append(f'emulation RunFor "{(plan["window_ns"] - now) / 1e9:.9f}"')
open(f"{out}/inject.resc", "w").write("\n".join(lines) + "\n")
open(f"{out}/injected.tsv", "w").write("t_ns\tmessage\tframe_bytes\tframe_hex\n" + "\n".join(rows) + "\n")
print(f"frames: {len(rows)} frames, {len(rows) * BUF} bytes injected, window {plan['window_ns']} ns")
