#!/usr/bin/env python3
"""AET-WO-4200 spike: S2, S4 and S5's boots, inside the pinned image (under img.sh).

Builds each firmware variant (fw.py) once, then boots each (platform, firmware) case with
drive.py in the CLI's composition, appending one JSON line per run to spike/4200/out/s2.jsonl.

Usage: s2.py [case ...]   (default: every case)
"""
import json, os, subprocess, sys

HERE = "/repo/spike/4200"
OUT = f"{HERE}/out/s2.jsonl"
DIRECT = ["sysbus.spi1.u4"]
MUX = ["sysbus.spi1.spi1mux", "sysbus.spi1.spi1mux.u4", "sysbus.spi1.spi1mux.imu"]
MUX_ABSENT = ["sysbus.spi1.spi1mux", "sysbus.spi1.spi1mux.imu"]

# name: (platform, firmware, levels, runs)
CASES = {
    "direct-jedec": ("direct", "jedec", DIRECT, 1),
    "direct-cs-jedec": ("direct-cs", "jedec", DIRECT, 1),
    "none-jedec": ("none", "jedec", [], 1),
    "mux-jedec": ("mux", "jedec", MUX, 3),
    "mux-two": ("mux", "two", MUX, 1),
    "mux-absent-jedec": ("mux-absent", "jedec", MUX_ABSENT, 3),
    "mux-cs-never": ("mux", "cs-never", MUX, 1),
    "direct-cs-never": ("direct", "cs-never", DIRECT, 1),
    "mux-forge-claim": ("mux", "forge-claim", MUX, 1),
    "mux-forge-regs": ("mux", "forge-regs", MUX, 1),
    "mux-forge-csusp": ("mux", "forge-csusp", MUX, 1),
    "direct-forge-csusp": ("direct", "forge-csusp", DIRECT, 1),
    "mux-wrong-cmd": ("mux", "wrong-cmd", MUX, 1),
    "mux-mode3": ("mux", "mode3", MUX, 1),
}


def main():
    names = sys.argv[1:] or list(CASES)
    elfs = {}
    for fw in sorted({CASES[n][1] for n in names}):
        p = subprocess.run([sys.executable, f"{HERE}/fw.py", fw], capture_output=True, text=True)
        if p.returncode != 0:
            sys.exit(p.stderr or p.stdout)
        _, elf, sha = p.stdout.split()
        elfs[fw] = elf
        with open(f"{HERE}/out/fw.txt", "a") as f:
            f.write(f"{fw} {sha}\n")
    os.makedirs("/cache/cases", exist_ok=True)
    for n in names:
        plat, fw, levels, runs = CASES[n]
        case = {"name": n, "platform": f"{HERE}/platforms/{plat}.repl", "elf": elfs[fw], "records": 3,
                "for_ns": 100_000_000, "levels": levels, "grep": r"spi1|u4|imu|gpioPortA"}
        path = f"/cache/cases/{n}.json"
        json.dump(case, open(path, "w"))
        subprocess.run([sys.executable, f"{HERE}/drive.py", path, OUT, str(runs)], check=True)
        print("done", n, flush=True)


if __name__ == "__main__":
    main()
