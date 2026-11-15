#!/usr/bin/env python3
"""AET-WO-4210 spike: S2, S3 and F16's boots, inside the pinned image (under img.sh).

NOT RUN. The first container of this work order (a no-op that only read back its caps) was
denied by the permission classifier, and the kickoff forbids working around a denial
(FEEDBACK, "Where it did not run"). This file is the harness the blocked rows name.

The platforms are aet-4210-gen's `spi-imu` output plus what each case appends (PLATFORMS);
the firmwares are fw.py's; each boot is drive.py's (4200's, the CLI's composition), one JSON
line per run into spike/4210/out/s2.jsonl.

Run (from the repository root, after `aet-4210-gen` has written ~/.cache/aet-4210/boards/spi-imu):
    bash spike/4210/img.sh -- python3 spike/4210/s2.py [case ...]
"""
import json, os, subprocess, sys

HERE = "/repo/spike/4210"
BOARD = "/cache/boards/spi-imu"
OUT = f"{HERE}/out/s2.jsonl"
MUX = ["sysbus.spi1.spi1mux", "sysbus.spi1.spi1mux.u4", "sysbus.spi1.spi1mux.u5", "sysbus.i2c1.u2"]

PLATFORMS = {
    "imu": "",
    "imu-u4-absent": None,  # the emitted platform with U4's two entries removed (below)
    "baro-bmp180": "\nbaro: Sensors.BMP180 @ i2c1 0x77\n",
    "baro-lps25hb": "\nbaro: Sensors.LPS25HB @ i2c1 0x5D\n",
    "baro-icp": "\nbaro: Sensors.ICP_101xx @ i2c1 0x63\n",
}

# name: (platform, firmware, levels, runs, pre-run monitor commands)
STIM = ["sysbus.spi1.spi1mux.u5 DefaultAccelerationZ 1"]
CASES = {
    "imu-script": ("imu", "imu-script", MUX, 3, []),                       # P1.1-P1.4, P1.6, P1.7
    "imu-script-stim": ("imu", "imu-script", MUX, 1, STIM),                # P1.5
    "imu-forge-config": ("imu", "imu-forge-config", MUX, 1, []),           # P2.1
    "imu-forge-data": ("imu", "imu-forge-data", MUX, 1, []),               # P2.2
    "two-windows": ("imu", "two-windows", MUX, 3, []),                     # P3.1
    "two-windows-u4-absent": ("imu-u4-absent", "two-windows", MUX[:1] + MUX[2:], 1, []),  # P3.2
    "imu-drdy": ("imu", "imu-drdy", MUX, 1, []),                           # P4.1
    "imu-drdy-store": ("imu", "imu-drdy-store", MUX, 1, []),               # P4.2
    "baro-bmp180": ("baro-bmp180", "baro-bmp180", MUX + ["sysbus.i2c1.baro"], 3, []),    # P6.3
    "baro-lps25hb": ("baro-lps25hb", "baro-lps25hb", MUX + ["sysbus.i2c1.baro"], 1, []),  # P6.1
    "baro-icp": ("baro-icp", "baro-icp", MUX + ["sysbus.i2c1.baro"], 1, []),             # P6.2
}
# S3 reads the IMU's IRQ line after the run, beside the standard reads.
IRQ = [["u5_irq", "sysbus.spi1.spi1mux.u5 IRQ"]]


def platform(name):
    base = open(f"{BOARD}/sentinel-01.repl").read()
    if PLATFORMS[name] is None:
        cut = base.split("\nu4_memory:")[0] + "\nu5:" + base.split("\nu5:")[1]
        text = cut
    else:
        text = base + PLATFORMS[name]
    path = f"/cache/platforms/{name}.repl"
    os.makedirs("/cache/platforms", exist_ok=True)
    open(path, "w").write(text)
    return path


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
        plat, fw, levels, runs, pre = CASES[n]
        records = {"imu-script": 5, "imu-forge-config": 5, "imu-forge-data": 5, "two-windows": 7,
                   "imu-drdy": 6, "imu-drdy-store": 6}.get(fw, 6)
        case = {"name": n, "platform": platform(plat), "elf": elfs[fw], "records": records,
                "for_ns": 100_000_000, "levels": levels, "pre": pre,
                "extra": IRQ if fw.startswith("imu-drdy") else [],
                "grep": r"spi1|u4|u5|i2c1|baro|gpioPortA"}
        path = f"/cache/cases/{n}.json"
        json.dump(case, open(path, "w"))
        subprocess.run([sys.executable, f"{HERE}/drive.py", path, OUT, str(runs)], check=True)
        print("done", n, flush=True)


if __name__ == "__main__":
    main()
