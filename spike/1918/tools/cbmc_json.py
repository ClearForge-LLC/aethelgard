#!/usr/bin/env python3
"""AET-WO-1918 P4: measure one CBMC output file without loading it whole.

Streams the file in 64 MiB chunks and counts: bytes; `"stepType"` (one per trace step); `"trace"`
keys (one per property that carries a trace); each `"status"` value; and each property's
class, from its `"property"` name with digits stripped (e.g. `.assertion.N`, `.cover.N`).
A count can be split across a chunk boundary, so counts are exact to within the number of chunks.
Stdlib only; run with python3 -I.

usage: cbmc_json.py <cbmc output>
"""
import collections
import re
import sys

CHUNK = 64 << 20
path = sys.argv[1]
size = 0
steps = traces = 0
status = collections.Counter()
classes = collections.Counter()
prop = re.compile(rb'"property": "([^"]*)"')
stat = re.compile(rb'"status": "([A-Z_]+)"')
tail = b""
with open(path, "rb") as f:
    while True:
        chunk = f.read(CHUNK)
        if not chunk:
            break
        size += len(chunk)
        buf = tail + chunk
        cut = buf.rfind(b"\n")
        body, tail = (buf, b"") if cut < 0 else (buf[: cut + 1], buf[cut + 1 :])
        steps += body.count(b'"stepType"')
        traces += body.count(b'"trace"')
        status.update(m.decode() for m in stat.findall(body))
        for m in prop.findall(body):
            name = re.sub(rb"[0-9]+", b"N", m.split(b".", 1)[-1] if b"." in m else m)
            classes[name.decode(errors="replace")] += 1
print(f"bytes={size} gib={size / 2**30:.3f} trace_keys={traces} trace_steps={steps}")
print("status:", dict(status))
for k, v in classes.most_common(12):
    print(f"  property class {k}: {v}")
