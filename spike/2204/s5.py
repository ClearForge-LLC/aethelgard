#!/usr/bin/env python3
"""AET-WO-2204 spike S5: device references named for Renode's reserved names.

1. The name set: every top-level entry the pinned platform chain declares, read
   from the image's own copies (/opt/renode/platforms/cpus: stm32h753 -> stm32h743
   -> stm32h7), with their sha256.
2. For each colliding reference: the reference specification with U2 renamed,
   `generate` (exit, and the device's line in the emitted .repl), then `simulate
   --platform <emitted>` (load only), and for one of them the full boot.

Usage: s5.py <tag>
"""
import hashlib, json, os, re, subprocess, sys

CPUS = "/opt/renode/platforms/cpus"
CHAIN = ["stm32h753.repl", "stm32h743.repl", "stm32h7.repl"]
SPEC = "/repo/examples/sentinel-01-minimal.json"
ELF = "/cache/fw/ref/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
NAMES = ["nvic", "cpu", "rcc", "exti", "timer2", "usart1", "dwt", "crypto", "sysbus", "machine",
         "using", "init", "as", "none", "true", "NVIC", "gpioPortA", "i2c1", "u2"]
AET = "/cache/target/debug/aethelgard"


def main():
    tag = sys.argv[1]
    base = f"/cache/raw/{tag}"
    os.makedirs(base, exist_ok=True)
    rows = []
    names = {}
    for f in CHAIN:
        text = open(f"{CPUS}/{f}").read()
        typed = re.findall(r"^([A-Za-z][A-Za-z0-9_]*):\s*[A-Z][A-Za-z0-9_.]*\s*@", text, re.M)
        untyped = re.findall(r"^([A-Za-z][A-Za-z0-9_]*):\s*$", text, re.M)
        names[f] = {"sha256": hashlib.sha256(text.encode()).hexdigest(), "typed": typed,
                    "untyped": sorted(set(untyped) - set(typed)),
                    "using": re.findall(r'^using\s+"([^"]+)"', text, re.M)}
    rows.append({"name_set": names, "typed_total": len({n for v in names.values() for n in v["typed"]})})
    for ref in NAMES:
        d = json.load(open(SPEC))
        d["i2c_devices"][0]["reference"] = ref
        spec = f"{base}/{ref}.json"
        json.dump(d, open(spec, "w"), indent=2)
        out = f"{base}/{ref}-gen"
        subprocess.run(["rm", "-rf", out])
        g = subprocess.run([AET, "generate", "--spec", spec, "--out", out], capture_output=True, text=True)
        row = {"reference": ref, "generate_exit": g.returncode, "generate_tail": g.stdout[-300:] if g.returncode else None}
        repl = f"{out}/board/sentinel-01.repl"
        if os.path.exists(repl):
            row["repl_line"] = [l for l in open(repl).read().splitlines() if "I2C.BME280" in l]
            s = subprocess.run([AET, "simulate", "--platform", repl], capture_output=True, text=True)
            v = json.loads(s.stdout) if s.stdout.strip().startswith("{") else {}
            row["simulate_exit"] = s.returncode
            row["verdict"] = v.get("verdict")
            row["findings"] = [(x.get("kind"), x.get("message", "")[:240]) for x in v.get("findings", [])]
            if not v:
                row["simulate_raw"] = s.stdout[-400:]
            if ref in ("nvic", "using"):
                b = subprocess.run([AET, "simulate", "--platform", repl, "--firmware", ELF, "--spec", spec, "--for", "10ms"],
                                   capture_output=True, text=True)
                bv = json.loads(b.stdout) if b.stdout.strip().startswith("{") else {}
                row["boot_exit"] = b.returncode
                row["boot_verdict"] = bv.get("verdict")
                row["boot_findings"] = [(x.get("kind"), x.get("message", "")[:240]) for x in bv.get("findings", [])]
        rows.append(row)
    with open(f"/repo/spike/2204/out/{tag}.jsonl", "w") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")


if __name__ == "__main__":
    main()
