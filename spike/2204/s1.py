#!/usr/bin/env python3
"""AET-WO-2204 spike S1 and S2: each probe firmware read in the CLI's shape, with
and without the two non-register witnesses, and judged by today's `simulate`.

Raw output under /cache/raw/<tag>; the summary, one JSON line per run, goes to
/repo/spike/2204/out/<tag>.jsonl. Usage: s1.py <tag> <variant>... [--runs N]
"""
import json, os, subprocess, sys

REF_REPL = "/repo/boards/sentinel-01/sentinel-01.repl"
REF_SPEC = "/repo/examples/sentinel-01-minimal.json"
GEN2_REPL = "/cache/gen2/board/sentinel-01.repl"
GEN2_SPEC = "/repo/spike/2204/two-bme.json"
I2C = [["cr1", "sysbus ReadDoubleWord 0x40005400"], ["isr", "sysbus ReadDoubleWord 0x40005418"],
       ["rxne", "sysbus.i2c1 RxNotEmpty"], ["icsr", "sysbus ReadDoubleWord 0xe000ed04"],
       ["cpsr", "cpu CPSR"], ["rxdr", "sysbus ReadDoubleWord 0x40005424"]]
# Each witness counted over the whole stdout. The device lines are Renode's
# Noisy log of the BME280 model; the access lines are LogPeripheralAccess's.
COUNT = {"u2_write_d0": r"i2c1\.u2: Write D0\b", "u2_read_60": r"i2c1\.u2: Read 60\b",
         "u5_write_d0": r"i2c1\.u5: Write D0\b", "u5_read_60": r"i2c1\.u5: Read 60\b",
         "noisy_any": r"\[NOISY\]", "access_any": r"(Read|Write)UInt32 (from|to) 0x",
         "access_rxdr_60": r"from 0x24 \(ReceiveData\), returned 0x60",
         "access_cpu": r"\[cpu: 0x[0-9A-Fa-f]+\][^\n]*(from|to) 0x",
         "unknown_slave": r"Unknown slave at address", "underflow": r"Receive buffer underflow"}
WITNESS = ["logLevel -1 sysbus.i2c1.u2", "sysbus LogPeripheralAccess sysbus.i2c1 true"]


def main():
    args = sys.argv[1:]
    runs = 1
    if "--runs" in args:
        i = args.index("--runs")
        runs = int(args[i + 1])
        del args[i:i + 2]
    tag, variants = args[0], args[1:]
    raw = f"/cache/raw/{tag}"
    os.makedirs(raw, exist_ok=True)
    summary = f"/repo/spike/2204/out/{tag}.jsonl"
    for v in variants:
        two = v.startswith("two-")
        repl, spec = (GEN2_REPL, GEN2_SPEC) if two else (REF_REPL, REF_SPEC)
        elf = f"/cache/fw/{v}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
        witness = WITNESS + (["logLevel -1 sysbus.i2c1.u5"] if two else [])
        # The witness runs log to a file with the console at errors from the
        # start (S4): with the access log on, the judge's own reads log too,
        # and on the console those lines split the read block.
        for mode, pre in (("plain", []), ("witness", witness)):
            case = {"name": f"{v}-{mode}", "platform": repl, "elf": elf, "records": 2 if two else 1,
                    "for_ns": 10_000_000, "pre": pre, "tail": "onelevel", "extra": I2C, "count": COUNT,
                    "logfile": mode == "witness",
                    "grep": r"i2c1|Unknown slave" if mode == "witness" else r"Unknown slave"}
            cf = f"{raw}/{v}-{mode}.case.json"
            json.dump(case, open(cf, "w"))
            subprocess.run(["python3", "/repo/spike/2204/drive.py", cf, f"{raw}/drive.jsonl", str(runs)], check=True)
        p = subprocess.run(["/cache/target/debug/aethelgard", "simulate", "--platform", repl, "--firmware", elf,
                            "--spec", spec, "--for", "10ms"], capture_output=True, text=True)
        verdict = json.loads(p.stdout) if p.stdout.strip().startswith("{") else {"raw": p.stdout[-500:]}
        with open(f"{raw}/simulate.jsonl", "a") as f:
            f.write(json.dumps({"variant": v, "exit": p.returncode, "verdict": verdict}) + "\n")
    with open(summary, "w") as out:
        for l in open(f"{raw}/drive.jsonl"):
            r = json.loads(l)
            r.pop("grep", None) if "witness" not in r["case"] else None
            out.write(json.dumps(r) + "\n")
        for l in open(f"{raw}/simulate.jsonl"):
            s = json.loads(l)
            vd = s["verdict"]
            out.write(json.dumps({"simulate": s["variant"], "exit": s["exit"], "verdict": vd.get("verdict"),
                                  "findings": [(f.get("kind"), f.get("severity")) for f in vd.get("findings", [])],
                                  "witness": vd.get("boot", {}).get("i2c_witness"),
                                  "i2c": vd.get("boot", {}).get("i2c")}) + "\n")


if __name__ == "__main__":
    main()
