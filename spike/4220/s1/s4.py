#!/usr/bin/env python3
"""AET-WO-4220 spike S4 (P4.1-P4.3): a model driven from Renode's monitor, with no firmware and no RunFor.

Same composition as s1.py (its config and environment), the probe platform, then the model's own
methods called by name between markers. Run under spike/4220/run.sh:
    python3 -I /repo/spike/4220/s1/s4.py
One JSON line into /repo/spike/4220/out/s4.jsonl; raw stdout into /cache/raw/s4.stdout.
"""
import json, os, subprocess, sys, tempfile

sys.path.insert(0, "/repo/spike/4220/s1")
import s1  # noqa: E402  (the same config, platform and environment)

CALLS = [
    "sysbus.spi1.spi1mux.u5 Transmit 0x80",
    "sysbus.spi1.spi1mux.u5 Transmit 0x00",
    "sysbus.spi1.spi1mux.u5 FinishTransmission",
    "sysbus.i2c1.u2 Write [0xD0]",
    "sysbus.i2c1.u2 Read 1",
    "sysbus.i2c1.u2 FinishTransmission",
]


def main():
    plat = s1.platform("probe", s1.PROBE_U2, s1.PROBE_U5)
    cmds = ["emulation SetSeed 2100", "mach create", f"include @{s1.HERE}/AetProbe.cs",
            f"machine LoadPlatformDescription @{plat}"]
    cmds += [f"logLevel -1 {s}" for s in s1.LEVELS]
    for c in CALLS:
        cmds += [f'echo "CALL {c}"', c]
    cmds += ['echo "CALLS-END"', "quit"]
    home = tempfile.mkdtemp(prefix="aet4220-home-", dir="/cache/homes")
    os.makedirs(f"{home}/.config/renode")
    cfg = f"{home}/renode.config"
    open(cfg, "w").write(s1.CONFIG.format(home=home))
    env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0",
           "DOTNET_EnableWriteXorExecute": "0"}
    p = subprocess.run(["renode", "--disable-gui", "--console", "--plain", "--config", cfg, "-e", "; ".join(cmds)],
                       stdin=subprocess.DEVNULL, capture_output=True, env=env, timeout=300)
    out = p.stdout.decode("utf-8", "replace")
    open("/cache/raw/s4.stdout", "w").write(out)
    lines = [l.strip("\r") for l in out.split("\n")]
    start = next(i for i, l in enumerate(lines) if l.startswith("CALL "))
    end = lines.index("CALLS-END")
    row = {"case": "s4-monitor", "exit": p.returncode, "between": lines[start:end]}
    with open("/repo/spike/4220/out/s4.jsonl", "a") as f:
        f.write(json.dumps(row) + "\n")
    print("\n".join(row["between"]))


if __name__ == "__main__":
    main()
