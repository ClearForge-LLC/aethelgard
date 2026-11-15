#!/usr/bin/env python3
"""AET-WO-4220 spike S1: can the pinned Renode (1.17.0) load our C#? Inside the pinned tools image,
under ~/bin/capped-run, --network none, TMPDIR unset.

Each boot is the CLI's composition as 4210's drive.py copies it from crates/aethelgard-cli/src/renode.rs
(the complete config file with synchronous logging, DOTNET_EnableDiagnostics=0,
DOTNET_EnableWriteXorExecute=0, a fresh HOME per run), with `include @<model>.cs` placed before
`machine LoadPlatformDescription` (the .repl cannot include C#: FEEDBACK, P1 grounds). The platform is
4210's emitted spi-imu platform with u2/u5 swapped for the probe; the firmware is 4210's imu-script ELF,
unchanged. While Renode runs, a poller lists /tmp/renode-*/ to see where the compiled DLL goes (P1.7).

Run (repository mounted at /repo, ~/.cache/aet-4220 at /cache):
    python3 /repo/spike/4220/s1/s1.py [case ...]
One JSON line per run into /repo/spike/4220/out/s1.jsonl; raw stdout into /cache/raw/.
"""
import glob, hashlib, json, os, re, subprocess, sys, tempfile, threading, time

HERE = "/repo/spike/4220/s1"
OUT = "/repo/spike/4220/out/s1.jsonl"
BASE = "/cache/boards/spi-imu.repl"
ELF = "/cache/fw/imu-script.elf"
ELF_SHA = "4896a5318f7d19b78b71a82121e7c914b9471837c2f47c749d27feeac21f6c70"
BEGIN, END = "AETHELGARD-BOOT-BEGIN", "AETHELGARD-BOOT-END"
VALUE = re.compile(r"0x[0-9A-Fa-f]{1,16}|True|False|-?\d+")
HOST_TS = re.compile(r"^\[\d\d:\d\d:\d\d\.\d{4}\] ")
COLD = [0x580244E0, 0x580244E8, 0x580244F0]
BOOT = 0x20000000
RECORDS = 5  # imu-script: 3 identity reads + the configuration write + the data read
CONFIG = """[general]
use-synchronous-logging = true
terminal = Termsharp
compiler-cache-enabled = False
serialization-mode = Generated
always-log-machine-name = False
collapse-repeated-log-entries = True
log-timestamps-type = Host
log-history-limit = 1000
history-path = {home}/.config/renode/history
store-table-bits = 41
[monitor]
consume-exceptions-from-command = True
break-script-on-exception = True
number-format = Hexadecimal
[plugins]
enabled-plugins =
[translation]
min-tb-size = 33554432
max-tb-size = 536870912
"""
LEVELS = ["sysbus.spi1.spi1mux", "sysbus.spi1.spi1mux.u5", "sysbus.i2c1.u2"]
U2 = "u2: I2C.BME280 @ i2c1 0x76"
U5 = "u5: Sensors.ICM20948 @ spi1mux 1"


def platform(name, u2, u5):
    text = open(BASE).read()
    assert text.count(U2) == 1 and text.count(U5) == 1
    text = text.replace(U2, u2).replace(U5, u5)
    os.makedirs("/cache/platforms", exist_ok=True)
    path = f"/cache/platforms/{name}.repl"
    open(path, "w").write(text)
    return path


def alt_model():
    """The probe with different answers (0xA6, 0x5B), for the two-includes case."""
    src = open(f"{HERE}/AetProbe.cs").read()
    assert src.count("(byte)0xA5") == 1 and src.count("(byte)0x5A") == 1
    path = "/cache/models/AetProbeAlt.cs"
    os.makedirs("/cache/models", exist_ok=True)
    open(path, "w").write(src.replace("(byte)0xA5", "(byte)0xA6").replace("(byte)0x5A", "(byte)0x5B"))
    return path


PROBE_U2 = "u2: Sensors.AetProbeI2C @ i2c1 0x76"
PROBE_U5 = "u5: Sensors.AetProbeSpi @ spi1mux 1"


def cases():
    probe = f"{HERE}/AetProbe.cs"
    return {
        # P1.2's baseline: the emitted platform, no include.
        "base": dict(platform=platform("base", U2, U5), include=[], runs=3),
        # P1.1, P1.2, P1.3, P1.7, P1.9
        "probe": dict(platform=platform("probe", PROBE_U2, PROBE_U5), include=[[probe]], runs=3),
        # P1.4: a namespace of our own, written in full, then short.
        "ownns-full": dict(platform=platform("ownns-full", PROBE_U2, "u5: Aethelgard.Models.AetProbeOwnNs @ spi1mux 1"),
                           include=[[probe]], runs=1),
        "ownns-short": dict(platform=platform("ownns-short", PROBE_U2, "u5: AetProbeOwnNs @ spi1mux 1"),
                            include=[[probe]], runs=1),
        # P1.5, P1.6: a file that does not compile, then the probe platform.
        "fault": dict(platform=platform("probe", PROBE_U2, PROBE_U5), include=[[f"{HERE}/AetProbeFault.cs"]], runs=1),
        # P1.8 (and beyond it): a changed file alone; then two files defining the same types, in turn.
        "changed": dict(platform=platform("probe", PROBE_U2, PROBE_U5), include=[[alt_model()]], runs=1),
        "two-includes": dict(platform=platform("probe", PROBE_U2, PROBE_U5), include=[[probe], [alt_model()]], runs=1),
        "two-includes-rev": dict(platform=platform("probe", PROBE_U2, PROBE_U5), include=[[alt_model()], [probe]], runs=1),
    }


def commands(c):
    recs = [BOOT + 4 + 4 * i for i in range(2 * RECORDS)]
    cmds = ["emulation SetSeed 2100", "mach create"]
    for files in c["include"]:
        cmds.append("echo \"AET4220-INCLUDE-BEGIN\"")
        cmds.append("include " + " ".join(f"@{f}" for f in files))
        cmds.append("echo \"AET4220-INCLUDE-END\"")
    cmds += [f"machine LoadPlatformDescription @{c['platform']}", f"sysbus LoadELF @{ELF}"]
    cmds += [f"sysbus WriteDoubleWord {a:#010x} 0x0" for a in COLD]
    cmds += [f"sysbus WriteDoubleWord {BOOT:#010x} 0x0"]
    cmds += [f"sysbus WriteDoubleWord {a:#010x} 0xffffffff" for a in recs]
    cmds += ["cpu VectorTableOffset 0x08000000"]
    cmds += [f"logLevel -1 {s}" for s in LEVELS]
    cmds += ['emulation RunFor "0.100000000"']
    reads = [("boot", f"sysbus ReadDoubleWord {BOOT:#010x}"), ("pc", "cpu PC")]
    reads += [(f"rec{i}", f"sysbus ReadDoubleWord {a:#010x}") for i, a in enumerate(recs)]
    cmds += [f'echo "{BEGIN}"'] + [r[1] for r in reads] + [f'echo "{END}"', "peripherals", "quit"]
    return cmds, reads


def body(line):
    m = HOST_TS.match(line)
    return line[m.end():] if m else None


def poll(stop, seen):
    while not stop.is_set():
        for d in glob.glob("/tmp/renode-*"):
            for root, _, files in os.walk(d):
                for f in files:
                    p = os.path.join(root, f)
                    try:
                        seen.setdefault(p, os.path.getsize(p))
                    except OSError:
                        pass
        time.sleep(0.02)


def run(name, c, r):
    cmds, reads = commands(c)
    line = "; ".join(cmds)
    os.makedirs("/cache/homes", exist_ok=True)
    home = tempfile.mkdtemp(prefix="aet4220-home-", dir="/cache/homes")
    os.makedirs(f"{home}/.config/renode")
    cfg = f"{home}/renode.config"
    open(cfg, "w").write(CONFIG.format(home=home))
    before_home = {os.path.join(a, f) for a, _, fs in os.walk(home) for f in fs}
    env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0",
           "DOTNET_EnableWriteXorExecute": "0"}
    seen, stop = {}, threading.Event()
    th = threading.Thread(target=poll, args=(stop, seen), daemon=True)
    th.start()
    t0 = time.monotonic()
    p = subprocess.run(["renode", "--disable-gui", "--console", "--plain", "--config", cfg, "-e", line],
                       stdin=subprocess.DEVNULL, capture_output=True, env=env, timeout=300)
    wall = time.monotonic() - t0
    stop.set(); th.join()
    out = p.stdout.decode("utf-8", "replace")
    lines = [l.strip("\r") for l in out.split("\n")]
    b = [i for i, l in enumerate(lines) if l == BEGIN]
    e = [i for i, l in enumerate(lines) if l == END]
    vals = []
    if len(b) == 1 and len(e) == 1:
        vals = [l for l in lines[b[0] + 1:e[0]] if VALUE.fullmatch(l)]
    run_part = lines[:b[0]] if b else lines
    logs = [body(l) for l in run_part if body(l) is not None]
    # Monitor text between the include markers (compiler output goes there, untimestamped).
    inc = []
    for i, l in enumerate(lines):
        if l == "AET4220-INCLUDE-BEGIN":
            j = next((k for k in range(i + 1, len(lines)) if lines[k] == "AET4220-INCLUDE-END"), len(lines))
            inc.append(lines[i + 1:j][:60])
    after_home = {os.path.join(a, f) for a, _, fs in os.walk(home) for f in fs}
    row = {"case": name, "run": r, "exit": p.returncode, "wall_s": round(wall, 3),
           "include": c["include"], "include_output": inc,
           "reads": dict(zip([x[0] for x in reads], vals)) if len(vals) == len(reads) else None,
           "n_values": len(vals),
           "errors": [l for l in logs if l.startswith("[ERROR]")][:40],
           "monitor_errors": [l for l in run_part if "error" in l.lower() and body(l) is None][:40],
           "probe_lines": [l for l in logs if "AetProbe" in l][:200],
           "warnings": [l for l in logs if l.startswith("[WARNING]")][:40],
           "tmp_files_seen": seen,
           "tmp_left_after_exit": sorted(glob.glob("/tmp/renode-*")),
           "home_new_files": sorted(after_home - before_home)}
    os.makedirs("/cache/raw", exist_ok=True)
    open(f"/cache/raw/{name}-{r}.stdout", "w").write(out[-400_000:])
    open(f"/cache/raw/{name}-{r}.cmds", "w").write(line + "\n")
    return row


def main():
    assert hashlib.sha256(open(ELF, "rb").read()).hexdigest() == ELF_SHA, "firmware is not 4210's imu-script"
    print("TMPDIR", os.environ.get("TMPDIR", "unset"), flush=True)
    all_cases = cases()
    names = sys.argv[1:] or list(all_cases)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "a") as f:
        for n in names:
            c = all_cases[n]
            for r in range(c["runs"]):
                row = run(n, c, r)
                f.write(json.dumps(row) + "\n"); f.flush()
                print(n, r, row["exit"], row["wall_s"], row["reads"] and {k: row["reads"][k] for k in ("rec1", "rec5")},
                      flush=True)


if __name__ == "__main__":
    main()
