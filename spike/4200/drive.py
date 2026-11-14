#!/usr/bin/env python3
"""AET-WO-4200 spike: one Renode boot in the CLI's composition, read and classified.

The composition is `renode.rs` as `AET-WO-2204` built it:
- `renode --disable-gui --console --plain --config <scratch>/renode.config -e "<one line>"`,
  stdout through a pipe, the CLI's environment (DOTNET_EnableDiagnostics=0,
  DOTNET_EnableWriteXorExecute=0), a fresh HOME per run;
- the config file states every key `renode::config` states (synchronous logging on, the
  history under the run's home), copied verbatim from `crates/aethelgard-cli/src/renode.rs`;
- no console `logLevel`; a source level `logLevel -1 <source>` for each witness source the
  case names, set before the run (2204 D1);
- `echo AETHELGARD-BOOT-BEGIN`, the reads, `echo AETHELGARD-BOOT-END`, `peripherals`, `quit`.

Usage: drive.py <case.json> <out.jsonl> [runs]

case.json: {"name", "platform" (path in the container), "elf", "records": n, "for_ns",
            "levels": [sources at Noisy], "pre": [cmds before RunFor],
            "extra": [[label, cmd], ...] read after the standard reads,
            "grep": regex of whole log bodies to keep (before BEGIN)}
"""
import hashlib, json, os, re, subprocess, sys, tempfile, time

BEGIN, END = "AETHELGARD-BOOT-BEGIN", "AETHELGARD-BOOT-END"
VALUE = re.compile(r"0x[0-9A-Fa-f]{1,16}|True|False|-?\d+")
COLD = [0x580244E0, 0x580244E8, 0x580244F0]
BOOT = 0x20000000
SPI1 = 0x40013000
# The CLI's register reads for the reference topology, then SPI1's own and GPIOA.ODR.
REGS = [0x58020000, 0x58020020, 0x58020400, 0x58020420, 0x580244E0, 0x580244E8, 0x580244F0]
SPI_REGS = [("spi_cr1", SPI1 + 0x00), ("spi_cr2", SPI1 + 0x04), ("spi_cfg1", SPI1 + 0x08),
            ("spi_cfg2", SPI1 + 0x0C), ("spi_sr", SPI1 + 0x14), ("gpioa_odr", 0x58020014)]
HOST_TS = re.compile(r"^\[\d\d:\d\d:\d\d\.\d{4}\] ")

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


def run_for(ns):
    return f'emulation RunFor "{ns // 1_000_000_000}.{ns % 1_000_000_000:09d}"'


def commands(c):
    n = c.get("records", 2)
    recs = [BOOT + 4 + 4 * i for i in range(2 * n)]
    cmds = ["emulation SetSeed 2100", "mach create", f"machine LoadPlatformDescription @{c['platform']}",
            f"sysbus LoadELF @{c['elf']}"]
    cmds += [f"sysbus WriteDoubleWord {a:#010x} 0x0" for a in COLD]
    cmds += [f"sysbus WriteDoubleWord {BOOT:#010x} 0x0"]
    cmds += [f"sysbus WriteDoubleWord {a:#010x} 0xffffffff" for a in recs]
    cmds += ["cpu VectorTableOffset 0x08000000"]
    cmds += [f"logLevel -1 {s}" for s in c.get("levels", [])]
    cmds += c.get("pre", [])
    cmds += [run_for(c.get("for_ns", 100_000_000))]
    reads = [("boot", f"sysbus ReadDoubleWord {BOOT:#010x}"), ("instr", "cpu ExecutedInstructions"),
             ("pc", "cpu PC"), ("icsr", "sysbus ReadDoubleWord 0xe000ed04")]
    reads += [(f"reg{a:08x}", f"sysbus ReadDoubleWord {a:#010x}") for a in REGS]
    reads += [(f"rec{i}", f"sysbus ReadDoubleWord {a:#010x}") for i, a in enumerate(recs)]
    reads += [(k, f"sysbus ReadDoubleWord {a:#010x}") for k, a in SPI_REGS]
    reads += [tuple(x) for x in c.get("extra", [])]
    cmds += [f'echo "{BEGIN}"'] + [r[1] for r in reads] + [f'echo "{END}"', "peripherals", "quit"]
    return cmds, reads


def body(line):
    m = HOST_TS.match(line)
    return line[m.end():] if m else None


def classify(stdout, reads):
    lines = [l.strip("\r") for l in stdout.split("\n")]
    b = [i for i, l in enumerate(lines) if l == BEGIN]
    e = [i for i, l in enumerate(lines) if l == END]
    res = {"begins": len(b), "ends": len(e)}
    if len(b) != 1 or len(e) != 1 or e[0] < b[0]:
        return res, lines
    between = lines[b[0] + 1:e[0]]
    vals = [l for l in between if VALUE.fullmatch(l)]
    other = [l for l in between if not VALUE.fullmatch(l) and body(l) is None]
    res.update({"values": len(vals), "n_other": len(other), "other": other[:10],
                "reads": dict(zip([r[0] for r in reads], vals)) if len(vals) == len(reads) else None,
                "judge_log_lines": [body(l) for l in between if body(l) is not None][:20]})
    tree = lines[e[0] + 1:]
    res["listing"] = [l.strip() for l in tree if "── " in l][:80]
    return res, lines[:b[0]]


def main():
    case = json.load(open(sys.argv[1]))
    dest, runs = sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 1
    cmds, reads = commands(case)
    line = "; ".join(cmds)
    with open(dest, "a") as f:
        for r in range(runs):
            os.makedirs("/cache/homes", exist_ok=True)
            home = tempfile.mkdtemp(prefix="aet4200-home-", dir="/cache/homes")
            os.makedirs(f"{home}/.config/renode")
            cfg = f"{home}/renode.config"
            with open(cfg, "w") as cf:
                cf.write(CONFIG.format(home=home))
            env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0",
                   "DOTNET_EnableWriteXorExecute": "0"}
            t0 = time.monotonic()
            p = subprocess.run(["renode", "--disable-gui", "--console", "--plain", "--config", cfg, "-e", line],
                               stdin=subprocess.DEVNULL, capture_output=True, env=env, timeout=300)
            wall = time.monotonic() - t0
            out = p.stdout.decode("utf-8", "replace")
            res, run_part = classify(out, reads)
            run_log = [body(l) for l in run_part if body(l) is not None]
            grep = case.get("grep")
            row = {"case": case["name"], "run": r, "exit": p.returncode, "wall_s": round(wall, 3),
                   "stdout_bytes": len(p.stdout), **res,
                   "errors": [l for l in run_log if l.startswith("[ERROR]")][:20],
                   "warnings": [l for l in run_log if l.startswith("[WARNING]")][:20],
                   "kept": [l for l in run_log if grep and re.search(grep, l)][:200],
                   "run_log_lines": len(run_log),
                   "reads_sha256": hashlib.sha256(json.dumps(res.get("reads"), sort_keys=True).encode()).hexdigest()[:16]}
            f.write(json.dumps(row) + "\n")
            f.flush()
            if r == 0:
                # Raw output stays in the cache, not the repository.
                os.makedirs("/cache/raw", exist_ok=True)
                with open(f"/cache/raw/{case['name']}.stdout", "w") as so:
                    so.write(out[-400_000:])
                with open(f"/cache/raw/{case['name']}.cmds", "w") as so:
                    so.write(line + "\n")


if __name__ == "__main__":
    main()
