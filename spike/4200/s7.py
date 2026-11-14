#!/usr/bin/env python3
"""AET-WO-4200 build, measured first (ruling: "Across a reset"): the mux and the flash across a
droop-shaped reset, composed as `renode.rs` `droop_schedule`/`assert_reset` compose it:
RunFor to the assert; `machine Reset`; the cold RCC writes; VTOR; `cpu IsHalted true`; the boot
word and records re-armed; RunFor to the release; `cpu IsHalted false`; echo RELEASE-0; the 10 ns
nudge; RunFor to the recovery bound (100 us); the probe's reads; RunFor to the end.

Assert and release times are droop-bor3's order of magnitude (start 1 ms, hold 200 us). The
log lines between RELEASE-0 and PROBE-BEGIN-0 are the recovered boot's.

Usage: s7.py   (inside the image; appends to spike/4200/out/s7.jsonl)
"""
import json, subprocess, sys

HERE = "/repo/spike/4200"
BOOT = 0x20000000
COLD = [0x580244E0, 0x580244E8, 0x580244F0]
RECS = [BOOT + 4 + 4 * i for i in range(6)]


def run_for(ns):
    return f'emulation RunFor "{ns // 1_000_000_000}.{ns % 1_000_000_000:09d}"'


def schedule(assert_ns, release_ns, end_ns):
    c = [run_for(assert_ns), "machine Reset"]
    c += [f"sysbus WriteDoubleWord {a:#010x} 0x0" for a in COLD]
    c += ["cpu VectorTableOffset 0x08000000", "cpu IsHalted true", f"sysbus WriteDoubleWord {BOOT:#010x} 0x0"]
    c += [f"sysbus WriteDoubleWord {a:#010x} 0xffffffff" for a in RECS]
    c += [run_for(release_ns - assert_ns), "cpu IsHalted false", 'echo "AETHELGARD-RELEASE-0"', run_for(10),
          run_for(100_000 - 10), 'echo "AETHELGARD-PROBE-BEGIN-0"', f"sysbus ReadDoubleWord {BOOT:#010x}"]
    c += [f"sysbus ReadDoubleWord {a:#010x}" for a in RECS[:4]]
    c += ['echo "AETHELGARD-PROBE-END-0"', run_for(end_ns - release_ns - 100_000)]
    return c


def main():
    p = subprocess.run([sys.executable, f"{HERE}/fw.py", "jedec"], capture_output=True, text=True, check=True)
    elf = p.stdout.split()[1]
    for name, a, r in [("reset-mux", 1_010_000, 1_230_000), ("reset-mux-early", 20_000, 240_000)]:
        case = {"name": name, "platform": f"{HERE}/platforms/mux.repl", "elf": elf, "records": 3,
                "levels": ["sysbus.spi1.spi1mux", "sysbus.spi1.spi1mux.u4", "sysbus.spi1.spi1mux.imu"],
                "schedule": schedule(a, r, 100_000_000), "grep": r"spi1|u4|imu"}
        path = f"/cache/cases/{name}.json"
        json.dump(case, open(path, "w"))
        subprocess.run([sys.executable, f"{HERE}/drive.py", path, f"{HERE}/out/s7.jsonl", "2"], check=True)
        print("done", name, flush=True)


if __name__ == "__main__":
    main()
