"""AET-WO-4400h (2)-(4): decode ArduPilot's USART1 TX (replay.sh's tx.bin) as MAVLink 2, in order.
Run with the ArduPilot image's venv. Usage: python -I decode.py tx.bin OUT.tsv
Each row: index, message type, its fields (JSON). STATUSTEXT, COMMAND_ACK, PARAM_VALUE, HEARTBEAT,
SERVO_OUTPUT_RAW and SYS_STATUS are kept whole; the rest are counted. Prints a summary."""
import collections, json, sys
from pymavlink.dialects.v20 import ardupilotmega as mav
data = open(sys.argv[1], "rb").read()
m = mav.MAVLink(None); m.robust_parsing = True
msgs = []
for i in range(0, len(data), 4096):
    out = m.parse_buffer(data[i:i + 4096]) or []
    msgs.extend(out)
keep = {"STATUSTEXT", "COMMAND_ACK", "PARAM_VALUE", "HEARTBEAT", "SERVO_OUTPUT_RAW", "SYS_STATUS"}
count = collections.Counter(x.get_type() for x in msgs)
with open(sys.argv[2], "w") as f:
    f.write("index\ttype\tfields\n")
    for n, x in enumerate(msgs):
        t = x.get_type()
        if t in keep:
            d = x.to_dict(); d.pop("mavpackettype", None)
            f.write(f"{n}\t{t}\t{json.dumps(d, sort_keys=True)}\n")
print(json.dumps({"bytes": len(data), "messages": len(msgs), "bad": count.get("BAD_DATA", 0), "by_type": dict(count)}, sort_keys=True))
