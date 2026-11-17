"""AET-WO-4400c2: one line per candidate run: the dataflash line, MAVLink msgids after the
injection, the COMMAND_ACK for command 512, AUTOPILOT_VERSION, the barometer probe time, and
whether the barometer traffic matches across runs. usage: python3 -I summarize.py RUNS_DIR NAME..."""
import collections, hashlib, json, os, re, sys

def frames(b):
    out, off = [], 0
    while True:
        j = b.find(b"\xfd", off)
        if j < 0 or j + 12 > len(b):
            return out
        ln = b[j + 1]
        msgid = int.from_bytes(b[j + 7:j + 10], "little")
        out.append((j, msgid, b[j + 10:j + 10 + ln]))
        off = j + 12 + ln

base = sys.argv[1]
for name in sys.argv[2:]:
    d = os.path.join(base, name)
    cap = open(os.path.join(d, "tool/capture/usart1.bin"), "rb").read()
    tx = [l for l in open(os.path.join(d, "console.log")) if "usart1:" in l and "WriteByte to 0x28" in l]
    st = json.load(open(os.path.join(d, "steps.json")))
    inj = st["injected_at_ns"]
    # bytes written after the injection instant
    def t(l):
        h, m, s = l[1:l.index("]")].split(":"); return int((int(h) * 3600 + int(m) * 60 + float(s)) * 1e9)
    first_after = next((i for i, l in enumerate(tx) if t(l) >= inj), len(tx))
    after = frames(cap[first_after:])
    ids = collections.Counter(m for _, m, _ in after)
    acks = [p for _, m, p in after if m == 77]
    ack512 = [p for p in acks if len(p) >= 2 and int.from_bytes(p[0:2], "little") == 512]
    flash = re.findall(rb"(Unknown SPI Flash 0x[0-9A-F]+|SPI Flash 0x[0-9A-F]+ found)", cap)
    baro = [l for l in open(os.path.join(d, "console.log")) if "] [NOISY] i2c1.u" in l]
    first = baro[0][1:baro[0].index("]")] if baro else None
    body = "".join(l[l.index("[NOISY]"):] for l in baro[:12]).encode()
    print(json.dumps({"run": name, "flash": [f.decode() for f in flash],
                      "msgids_after_inject": dict(sorted(ids.items())),
                      "command_ack_512": [{"result": p[2] if len(p) > 2 else 0} for p in ack512],
                      "autopilot_version": ids.get(148, 0),
                      "baro_first": first, "baro_first12_sha": hashlib.sha256(body).hexdigest()[:16],
                      "console_bytes": len(cap)}))
