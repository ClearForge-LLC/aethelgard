#!/usr/bin/env python3
"""AET-WO-2204 spike S3, and one more S4 case.

S3: the reference's final PC, ICSR and xPSR (the `CPSR` alias) at several run
lengths, and `systick-before` (2301's: SysTick armed from the monitor before the
run, CSR = 7, reload 999), which parks in `fault`.

S4: the reference run for 1 us, so the judge's own reads of GPIOA/GPIOB, whose
clocks are still off, log a locked-access warning inside the read block; in the
three compositions, ten runs each.

Then the file composition with a `log` marker after the run, ten runs each on
the 1 us reference and on `flood-locked`.

Usage: s3.py <tag>
"""
import json, os, subprocess, sys

REPL = "/repo/boards/sentinel-01/sentinel-01.repl"
ELF = "/cache/fw/ref/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
CORE = [["icsr", "sysbus ReadDoubleWord 0xe000ed04"], ["cpsr", "cpu CPSR"]]
SYSTICK = ["sysbus WriteDoubleWord 0xe000e014 999", "sysbus WriteDoubleWord 0xe000e018 0",
           "sysbus WriteDoubleWord 0xe000e010 7"]
SUM = {"locked": r"\[WARNING\] [^\r\n]*Tried to read a locked peripheral[^\r\n(]*"}


def run(raw, case, runs):
    cf = f"{raw}/{case['name']}.case.json"
    json.dump(case, open(cf, "w"))
    subprocess.run(["python3", "/repo/spike/2204/drive.py", cf, f"{raw}/drive.jsonl", str(runs)], check=True)


def main():
    tag = sys.argv[1]
    raw = f"/cache/raw/{tag}"
    os.makedirs(raw, exist_ok=True)
    for ns in (10_000_000, 10_000_010, 10_000_020, 10_000_030, 20_000_000, 100_000_000):
        run(raw, {"name": f"ref-{ns}", "platform": REPL, "elf": ELF, "for_ns": ns, "tail": "cli", "extra": CORE}, 1)
    for ns in (10_000_000, 100_000_000):
        run(raw, {"name": f"systick-before-{ns}", "platform": REPL, "elf": ELF, "for_ns": ns, "pre": SYSTICK,
                  "tail": "cli", "extra": CORE}, 1)
    for mode in ("cli", "onelevel", "file"):
        run(raw, {"name": f"ref-1us-{mode}", "platform": REPL, "elf": ELF, "for_ns": 1000,
                  "tail": "cli" if mode == "cli" else "onelevel", "logfile": mode == "file", "sum": SUM}, 10)
    # The file composition with an in-stream marker after the run: the judge's
    # own locked reads must fall after it, a firmware's late locked read before.
    run(raw, {"name": "ref-1us-file-marker", "platform": REPL, "elf": ELF, "for_ns": 1000, "tail": "onelevel",
              "logfile": True, "marker": "AETHELGARD-RUN-END", "sum": SUM}, 10)
    run(raw, {"name": "flood-locked-file-marker", "platform": REPL,
              "elf": "/cache/fw/flood-locked/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware",
              "for_ns": 10_000_000, "tail": "onelevel", "logfile": True, "marker": "AETHELGARD-RUN-END", "sum": SUM}, 10)
    with open(f"/repo/spike/2204/out/{tag}.jsonl", "w") as out:
        for l in open(f"{raw}/drive.jsonl"):
            out.write(l)


if __name__ == "__main__":
    main()
