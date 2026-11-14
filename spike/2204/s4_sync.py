#!/usr/bin/env python3
"""AET-WO-2204 spike S4, a fifth composition: Renode's `use-synchronous-logging`
(config `[general]`, `Logger.cs` L822) with one console level and no level change.
Ten runs each on the fixtures that broke the others: `ref` (reads byte-exact?),
`flood-max` (markers and values whole?), `flood-locked` (the late locked line?),
and ref at 1 us (the judge's own locked reads: whole lines, after BEGIN?).
Then S1/S2's witnesses on this composition, three runs each.

Usage: s4_sync.py <tag> [runs]
"""
import json, os, subprocess, sys

REPL = "/repo/boards/sentinel-01/sentinel-01.repl"
SUM = {"underflow": r"\[WARNING\] i2c1: Receive buffer underflow!",
       "locked": r"\[WARNING\] [^\r\n]*Tried to read a locked peripheral[^\r\n(]*",
       "nackcf": r"\[WARNING\] i2c1: Unhandled write to offset 0x1C\. Unhandled bits: \[4\] when writing value 0x10\. Tags: NACKCF \(0x1\)\."}


def elf(v):
    return f"/cache/fw/{v}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"


def main():
    tag = sys.argv[1]
    runs = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    raw = f"/cache/raw/{tag}"
    os.makedirs(raw, exist_ok=True)
    cases = [("ref", 10_000_000), ("flood-max", 10_000_000), ("flood-locked", 10_000_000), ("ref", 1000)]
    for v, ns in cases:
        case = {"name": f"{v}-{ns}-sync", "platform": REPL, "elf": elf(v), "for_ns": ns, "tail": "onelevel",
                "sync": True, "sum": SUM, "grep": r"locked peripheral"}
        cf = f"{raw}/{case['name']}.case.json"
        json.dump(case, open(cf, "w"))
        subprocess.run(["python3", "/repo/spike/2204/drive.py", cf, f"{raw}/drive.jsonl", str(runs)], check=True)
    # S1/S2's witness on this composition: the device lines and the access log on
    # the console, read before BEGIN; the judge's own access-logged reads after it.
    count = {"u2_pair": r"\] i2c1\.u2: Write D0[\s\S]{0,4000}?\] i2c1\.u2: Read 60",
             "u5_pair": r"\] i2c1\.u5: Write D0[\s\S]{0,4000}?\] i2c1\.u5: Read 60",
             "access_cpu": r"\[cpu: 0x[0-9A-Fa-f]+\][^\n]*(from|to) 0x"}
    for v in ("two-ref", "two-forged-first", "two-forged-last", "ref", "forge-all"):
        two = v.startswith("two-")
        pre = ["logLevel -1 sysbus.i2c1.u2", "sysbus LogPeripheralAccess sysbus.i2c1 true"]
        pre += ["logLevel -1 sysbus.i2c1.u5"] if two else []
        case = {"name": f"{v}-witness-sync", "platform": "/cache/gen2/board/sentinel-01.repl" if two else REPL,
                "elf": elf(v), "records": 2 if two else 1, "for_ns": 10_000_000, "tail": "onelevel", "sync": True,
                "pre": pre, "count": count}
        cf = f"{raw}/{case['name']}.case.json"
        json.dump(case, open(cf, "w"))
        subprocess.run(["python3", "/repo/spike/2204/drive.py", cf, f"{raw}/drive.jsonl", "3"], check=True)
    # Where the judge's own locked lines fall at 1 us: before or after BEGIN.
    out = open(f"{raw}/drive.jsonl.ref-1000-sync.stdout").read().split("\n")
    b = [i for i, l in enumerate(out) if l.strip("\r") == "AETHELGARD-BOOT-BEGIN"]
    locked = [i for i, l in enumerate(out) if "locked peripheral" in l]
    with open(f"/repo/spike/2204/out/{tag}.jsonl", "w") as f:
        for l in open(f"{raw}/drive.jsonl"):
            f.write(l)
        f.write(json.dumps({"ref-1us-sync-run0": {"begin_line": b, "locked_lines": locked}}) + "\n")


if __name__ == "__main__":
    main()
