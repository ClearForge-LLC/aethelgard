"""AET-WO-4400i (1): ArduPilot's USART1 TX, each byte with the virtual time its TDR write was made (replay.sh's
txtimes.txt, ns), decoded as MAVLink 2 in order; each message stamped with its last byte's time.
Run with the ArduPilot image's venv. Usage: python -I timeline.py RUNDIR
Writes RUNDIR/mav.tsv (t_ns, type, sysid/compid, fields of STATUSTEXT, COMMAND_ACK, PARAM_VALUE, PARAM_ERROR, HEARTBEAT,
FILE_TRANSFER_PROTOCOL) and prints a JSON summary: bytes against tx.bin, counts, the first armed heartbeat."""
import collections, json, sys
from pymavlink.dialects.v20 import ardupilotmega as mav
d = sys.argv[1]
rows = [l.split() for l in open(f"{d}/txtimes.txt") if l.strip()]
data = bytes(int(v, 16) & 0xFF for _, v in rows)
times = [int(t) for t, _ in rows]
txbin = open(f"{d}/tx.bin", "rb").read()
m = mav.MAVLink(None); m.robust_parsing = True
keep = {"STATUSTEXT", "COMMAND_ACK", "PARAM_VALUE", "PARAM_ERROR", "HEARTBEAT", "FILE_TRANSFER_PROTOCOL"}
count = collections.Counter(); first_armed = None; out = []
for i, b in enumerate(data):
    msg = m.parse_char(bytes([b]))
    if msg is None:
        continue
    t = msg.get_type(); count[t] += 1
    if t not in keep:
        continue
    f = msg.to_dict(); f.pop("mavpackettype", None)
    if t == "FILE_TRANSFER_PROTOCOL":
        p = bytes(f["payload"]); f = {"seq": p[0] | p[1] << 8, "opcode": p[3], "req_opcode": p[5], "data0": p[12]}
    if t == "HEARTBEAT" and f["base_mode"] & 128 and first_armed is None:
        first_armed = times[i]
    out.append(f"{times[i]}\t{t}\t{msg.get_srcSystem()}/{msg.get_srcComponent()}\t{json.dumps(f, sort_keys=True)}")
open(f"{d}/mav.tsv", "w").write("t_ns\ttype\tsrc\tfields\n" + "\n".join(out) + "\n")
print(json.dumps({"tdr_bytes": len(data), "tx_bin_bytes": len(txbin), "same_bytes": data == txbin, "by_type": dict(count),
                  "first_heartbeat_ns": next((int(r.split("\t")[0]) for r in out if "\tHEARTBEAT\t" in r), None),
                  "first_armed_heartbeat_ns": first_armed}, sort_keys=True))
