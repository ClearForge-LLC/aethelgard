"""AET-WO-4400i (1), from AET-WO-4400h's frames.py: a timed MAVLink 2 plan -> Renode monitor commands that inject it
into USART1. Run with the ArduPilot image's venv (pymavlink, unmodified). Usage: python -I frames.py PLAN.json OUT_DIR
PLAN.json (plans.py): {"window_ns", "pad", "spacing_ns", "steps": [[t_ns, kind, {fields}], ...]} in time order.
  pad true:     each frame padded with 0x00 to a whole multiple of 64 B, ArduPilot's RX bounce buffer
                (AP_HAL_ChibiOS/UARTDriver.h:25) (4400h's stand-in for IDLE); pad false: the frame's bytes only;
  spacing_ns 0: every byte at the step's instant; otherwise `spacing_ns` apart from it, then one more spacing: the
                line idle for a character time after the last stop bit (RM0433 Rev 9: an idle character is a whole
                frame of 1s), where the IDLE hooks raise IDLE (`AET-IDLE` line; replay.sh IDLE=1).
Writes OUT_DIR/inject.resc and OUT_DIR/injected.tsv (t_ns, message, frame length, padded length, frame hex)."""
import json, struct, sys
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
    if kind == "ftp_list":
        path = f["path"].encode()
        # seq, session, opcode 3 (ListDirectory), size, req_opcode, burst_complete, padding, offset; then the path
        payload = struct.pack("<HBBBBBBI", f["seq"], 0, 3, len(path), 0, 0, 0, 0) + path
        return m.file_transfer_protocol_encode(0, 1, 1, list(payload + bytes(251 - len(payload))))
    raise SystemExit(f"frames: unknown message {kind}")


lines, rows, now = [], [], 0
spacing = plan["spacing_ns"]


def run_to(t):
    global now
    if t < now:
        raise SystemExit(f"frames: a step at {t} ns overlaps the last frame, which ends at {now} ns")
    if t > now:
        lines.append(f'emulation RunFor "{(t - now) / 1e9:.9f}"')
        now = t


for t, kind, f in plan["steps"]:
    if t > plan["window_ns"]:
        raise SystemExit(f"frames: a step at {t} ns is past the window, {plan['window_ns']} ns")
    run_to(max(t, now))
    if kind == "monitor":   # a stimulus, not a USART byte
        lines.append(f["cmd"])
        rows.append(f"{now}\tmonitor {f['cmd']}\t0\t0\t-")
        continue
    msg = encode(kind, f)
    frame = msg.pack(m)
    name = msg.get_type() + (" " + json.dumps(f, sort_keys=True) if f else "")
    sent = frame + bytes(-len(frame) % BUF) if plan["pad"] else frame
    if spacing == 0:
        lines.extend(f"sysbus.usart1 WriteChar 0x{b:02X}" for b in sent)
    else:
        for b in sent:
            lines.append(f"sysbus.usart1 WriteChar 0x{b:02X}")
            run_to(now + spacing)   # the character's own time on the line
        run_to(now + spacing)       # then one idle character
        lines.append(f'echo "AET-IDLE {now}"')
        # No ';' inside the quotes: a monitor line is split there.
        lines.append("python \"(lambda D: D.GetData('aet_hooks') and D.GetData('aet_idleie') and (D.SetData('aet_idle', 1) or D.SetData('aet_idles', (D.GetData('aet_idles') or 0) + 1) or self.Machine['sysbus.usart1'].IRQ.Set(True)))(__import__('System').AppDomain.CurrentDomain)\"")
    rows.append(f"{t}\t{name}\t{len(frame)}\t{len(sent)}\t{frame.hex()}")
run_to(plan["window_ns"])
open(f"{out}/inject.resc", "w").write("\n".join(lines) + "\n")
open(f"{out}/injected.tsv", "w").write("t_ns\tmessage\tframe_bytes\tsent_bytes\tframe_hex\n" + "\n".join(rows) + "\n")
print(f"frames: {sum(1 for r in rows if not r.split(chr(9))[1].startswith('monitor'))} frames, window {plan['window_ns']} ns, pad {plan['pad']}, spacing {spacing} ns")
