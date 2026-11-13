#!/usr/bin/env python3
"""Summarise drive.py's runs: per case, the distinct hashes over 10 runs, and the
firmware's view at each named block of run 0, with the recovery time."""
import json, sys

BOOT_MAGIC, OK = "0x2100B007", "0x22020000"
d, cases = sys.argv[1], sys.argv[2:]
for name in cases:
    runs = [json.loads(l) for l in open(f"{d}/{name}.jsonl")]
    hashes = sorted({r["hash"] for r in runs})
    walls = [r["wall_s"] for r in runs]
    r0 = runs[0]
    print(f"== {name}: runs {len(runs)}, distinct hashes {len(hashes)} ({hashes[0][:12]}…), "
          f"exit {sorted({r['exit'] for r in runs})}, wall {min(walls):.2f}–{max(walls):.2f} s")
    for b in r0["blocks"]:
        if b["label"].startswith("after+") and b["label"] not in ("after+10us", "after+20us", "after+30us"):
            continue
        keep = {k: v for k, v in b.items() if k not in ("label", "t_ns")}
        print(f"  {b['label']:>12} t={b['t_ns'] / 1e6:.3f} ms {keep}")
    rec = next((b for b in r0["blocks"] if b["label"].startswith("after+")
                and b.get("boot") == BOOT_MAGIC and b.get("status") == OK), None)
    print(f"  recovery: {'boot word and OK read first seen at ' + rec['label'] if rec else 'never within the 1 ms window'}")
    print(f"  log warnings: {r0['log_warnings']}")
