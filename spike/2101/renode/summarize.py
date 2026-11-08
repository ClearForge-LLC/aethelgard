#!/usr/bin/env python3 -I
"""Read raw Renode output on stdin; decode each BEGIN <label> .. END block (12 result
words, ExecutedInstructions, PC); print a decoded line per block, then
`HASH <sha256>` over all blocks' values (the whole result block + ExecutedInstructions + PC)
and `LOG` lines that mention the I2C path (i2c1/sensor/bme/dummy/slave/underflow)."""
import sys, re, hashlib
ST = {0: "OK", 1: "TIMEOUT_TXIS", 2: "TIMEOUT_TC", 3: "TIMEOUT_RXNE", 4: "TIMEOUT_STOPF"}
lines = [l.rstrip("\r\n") for l in sys.stdin]
blocks, cur, label = [], None, None
logs = []
for l in lines:
    m = re.match(r"^BEGIN ?(.*)$", l)
    if m:
        cur, label = [], m.group(1); continue
    if l.startswith("END") and cur is not None:
        blocks.append((label, cur)); cur = None; continue
    if cur is not None:
        if re.match(r"^0x[0-9A-Fa-f]+$", l.strip()):
            cur.append(int(l.strip(), 16))
        continue
    if re.search(r"i2c1|sensor|bme|dummy|Unknown slave|underflow|unknown slave|Error|ERROR|exception|fault", l, re.I) \
       and "non existing peripheral" not in l and not l.startswith("(") and "Setting initial" not in l:
        logs.append(l)
h = hashlib.sha256()
for label, v in blocks:
    if len(v) == 12:
        w, ins, pc = v[:10], v[10], v[11]
        h.update(("|".join(map(str, v)) + "\n").encode())
        print(f"REPEAT {label or '-'}: magic={w[0]:#x} attempted={w[1]} ok={w[2]} wrong={w[3]} timed_out={w[4]} last_status={w[5]}({ST.get(w[5],'?')}) "
              f"last_byte={w[6]:#04x} ISR_end={w[7]:#x} resets={w[8]} alive={w[9]} instr={ins} pc={pc:#x}")
        continue
    if len(v) != 14:
        print(f"BLOCK {label!r}: BAD ({len(v)} values) {v}"); continue
    w, ins, pc = v[:12], v[12], v[13]
    h.update(("|".join(map(str, v)) + "\n").encode())
    print(f"BLOCK {label or '-'}: magic={w[0]:#x} status={w[1]}({ST.get(w[1],'?')}) byte0={w[2]:#04x} byte1={w[3]:#04x} "
          f"ISR_end={w[4]:#x} spins[txis,tc,rxne,stopf]={w[5:9]} nbytes={w[9]} seq={w[10]} done={w[11]} instr={ins} pc={pc:#x}")
print("NBLOCKS", len(blocks))
print("HASH", h.hexdigest())
for l in logs: print("LOG", l)
