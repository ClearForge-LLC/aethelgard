#!/usr/bin/env python3
"""AET-WO-2204 spike S4: does every log line the firmware caused reach the judge,
and do the reads stay byte-exact? Three compositions, ten runs each, on the
reference, on `burst` (500 empty RXDR reads, then one locked GPIOC read, just
before the boot word) and on `flood` (two alternating warnings to the run's end):

- cli:      today's static tail, `RunFor; logLevel 3`;
- onelevel: no level change; the console keeps its level through the reads;
- file:     `logFile` first, the console at errors from the start, no change after.

Plus today's `simulate` on `burst` and `flood`, ten runs each. Usage: s4.py <tag> [runs] [variants,] [simulated variants,]
"""
import json, os, subprocess, sys

REPL = "/repo/boards/sentinel-01/sentinel-01.repl"
SPEC = "/repo/examples/sentinel-01-minimal.json"
SUM = {"underflow": r"\[WARNING\] i2c1: Receive buffer underflow!",
       "locked": r"\[WARNING\] [^\r\n]*Tried to read a locked peripheral[^\r\n(]*",
       "nackcf": r"\[WARNING\] i2c1: Unhandled write to offset 0x1C\. Unhandled bits: \[4\] when writing value 0x10\. Tags: NACKCF \(0x1\)\."}


def main():
    tag = sys.argv[1]
    runs = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    raw = f"/cache/raw/{tag}"
    os.makedirs(raw, exist_ok=True)
    for v in sys.argv[3].split(",") if len(sys.argv) > 3 else ("ref", "burst", "flood"):
        elf = f"/cache/fw/{v}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
        for mode in ("cli", "onelevel", "file"):
            case = {"name": f"{v}-{mode}", "platform": REPL, "elf": elf, "records": 1, "for_ns": 10_000_000,
                    "tail": "cli" if mode == "cli" else "onelevel", "logfile": mode == "file", "sum": SUM,
                    "grep": r"locked peripheral"}
            cf = f"{raw}/{v}-{mode}.case.json"
            json.dump(case, open(cf, "w"))
            subprocess.run(["python3", "/repo/spike/2204/drive.py", cf, f"{raw}/drive.jsonl", str(runs)], check=True)
    sims = sys.argv[4].split(",") if len(sys.argv) > 4 else ("burst", "flood")
    for r, v in [(r, v) for v in sims for r in range(runs)]:
        elf = f"/cache/fw/{v}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
        p = subprocess.run(["/cache/target/debug/aethelgard", "simulate", "--platform", REPL, "--firmware", elf,
                            "--spec", SPEC, "--for", "10ms"], capture_output=True, text=True)
        vd = json.loads(p.stdout) if p.stdout.strip().startswith("{") else {}
        with open(f"{raw}/simulate.jsonl", "a") as f:
            f.write(json.dumps({"simulate": v, "run": r, "exit": p.returncode, "verdict": vd.get("verdict"),
                                "findings": [(x.get("kind"), x.get("message", "")[:160]) for x in vd.get("findings", [])],
                                "raw": None if vd else p.stdout[-300:]}) + "\n")
    with open(f"/repo/spike/2204/out/{tag}.jsonl", "w") as out:
        for name in ("drive.jsonl", "simulate.jsonl"):
            for l in open(f"{raw}/{name}"):
                out.write(l)


if __name__ == "__main__":
    main()
