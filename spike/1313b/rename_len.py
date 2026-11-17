#!/usr/bin/env python3
"""rename_len.py <spec> <len> <out>: rename_perm.py's renaming (order, net prefixes), then every
device, connector and test-point reference prefixed by <len> characters, so the parts grow by their
reference text by a chosen amount (AET-WO-1313b: a size sample, not one fixed prefix)."""
import json
import pathlib
import subprocess
import sys

spec, n, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
here = pathlib.Path(__file__).parent
subprocess.run([sys.executable, "-I", str(here / "rename_perm.py"), spec, str(n), out], check=True)
d = json.load(open(out))
pre = ("Q" + "x" * n)[:n] if n else ""
ref = lambda r: f"{pre}_{r}" if n else r
for k in ("i2c_devices", "spi_devices"):
    for x in d.get(k, []):
        x["reference"] = ref(x["reference"])
for c in d.get("connectors", []):
    c["reference"] = ref(c["reference"])
for t in d.get("test_points", []):
    t["reference"] = ref(t["reference"])
json.dump(d, open(out, "w"), indent=1)
