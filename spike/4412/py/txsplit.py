"""AET-WO-4412 (1): USART1's transmitted bytes from a replay's console (the CLI's own `LogPeripheralAccess usart1`),
split at a virtual instant (the reset), each half written as bytes for py/decode.py. Usage: txsplit.py CONSOLE.gz T_S OUTDIR"""
import gzip, re, sys
src, t, out = sys.argv[1], float(sys.argv[2]), sys.argv[3]
pat = re.compile(rb'^\[(\d\d):(\d\d):(\d\d\.\d+)\] \[INFO\] usart1: \[cpu: [^\]]*\] WriteByte to 0x28 \(TransmitData\), value 0x([0-9A-F]{1,2})')
parts = {"before": bytearray(), "after": bytearray()}
for line in gzip.open(src):
    m = pat.match(line)
    if not m: continue
    s = int(m[1]) * 3600 + int(m[2]) * 60 + float(m[3])
    parts["before" if s < t else "after"].append(int(m[4], 16))
for k, v in parts.items():
    open(f"{out}/tx-{k}.bin", "wb").write(v); print(k, len(v))
