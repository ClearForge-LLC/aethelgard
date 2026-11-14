#!/usr/bin/env python3
"""AET-WO-2204 spike: one Renode boot in the CLI's shape, read and classified.

The shape is `renode.rs` BootPlan::commands as `simulate` sends it:
`renode --disable-gui --console --plain -e "<one line>"`, stdout through a pipe,
DOTNET_EnableDiagnostics=0 and DOTNET_EnableWriteXorExecute=0, no logFile. A
fresh HOME per run whose Renode config points the history at /dev/null.

Usage: drive.py <case.json> <out.jsonl> [runs]

case.json: {"name", "platform", "elf", "records": n, "for_ns", "pre": [cmds before RunFor],
            "tail": "cli" | "onelevel", "extra": [[label, cmd], ...] read after the CLI's reads}
"""
import hashlib, json, os, re, subprocess, sys, tempfile, time

BEGIN, END = "AETHELGARD-BOOT-BEGIN", "AETHELGARD-BOOT-END"
VALUE = re.compile(r"0x[0-9A-Fa-f]{1,16}|True|False|-?\d+")
COLD = [0x580244E0, 0x580244E8, 0x580244F0]
BOOT = 0x20000000
# The CLI's register reads for the reference topology (`ref-cmdline.txt`).
REGS = [0x58020000, 0x58020020, 0x58020400, 0x58020420, 0x580244E0, 0x580244E8, 0x580244F0]


def run_for(ns):
    return f'emulation RunFor "{ns // 1_000_000_000}.{ns % 1_000_000_000:09d}"'


def commands(c):
    n = c.get("records", 1)
    recs = [BOOT + 4 + 4 * i for i in range(2 * n)]
    cmds = []
    if c.get("logfile"):
        # A file backend at its default level, and the console at errors only,
        # set before anything runs: no level changes after the run starts.
        cmds += ["logFile @LOGFILE", "logLevel 3 console"]
    cmds += ["emulation SetSeed 2100", "mach create", f"machine LoadPlatformDescription @{c['platform']}",
            f"sysbus LoadELF @{c['elf']}"]
    cmds += [f"sysbus WriteDoubleWord {a:#010x} 0x0" for a in COLD]
    cmds += [f"sysbus WriteDoubleWord {BOOT:#010x} 0x0"]
    cmds += [f"sysbus WriteDoubleWord {a:#010x} 0xffffffff" for a in recs]
    cmds += ["cpu VectorTableOffset 0x08000000"]
    cmds += c.get("pre", [])
    cmds += [run_for(c.get("for_ns", 100_000_000))]
    if c.get("tail", "cli") == "cli":
        cmds += ["logLevel 3"]
    # An in-stream marker: logged through the same queue as the firmware's lines,
    # so in the file everything before it is the run's, everything after the reads'.
    if c.get("marker"):
        cmds += [f'log "{c["marker"]}"']
    reads = [("boot", f"sysbus ReadDoubleWord {BOOT:#010x}"), ("instr", "cpu ExecutedInstructions"),
             ("mips", "cpu PerformanceInMips"), ("pc", "cpu PC")]
    reads += [(f"reg{a:08x}", f"sysbus ReadDoubleWord {a:#010x}") for a in REGS]
    reads += [(f"rec{i}", f"sysbus ReadDoubleWord {a:#010x}") for i, a in enumerate(recs)]
    reads += [("cr2", "sysbus ReadDoubleWord 0x40005404"), ("txdr", "sysbus ReadDoubleWord 0x40005428")]
    reads += [tuple(x) for x in c.get("extra", [])]
    cmds += [f'echo "{BEGIN}"'] + [r[1] for r in reads] + [f'echo "{END}"', "peripherals", "quit"]
    return cmds, reads


def classify(stdout, reads):
    lines = [l.strip("\r") for l in stdout.split("\n")]  # as `renode.rs` clean_lines
    b = [i for i, l in enumerate(lines) if l == BEGIN]
    e = [i for i, l in enumerate(lines) if l == END]
    res = {"begins": len(b), "ends": len(e)}
    if len(b) != 1 or len(e) != 1 or e[0] < b[0]:
        return res
    between = lines[b[0] + 1:e[0]]
    logs = [l for l in between if l.startswith("[")]
    vals = [l for l in between if VALUE.fullmatch(l)]
    other = [l for l in between if not l.startswith("[") and not VALUE.fullmatch(l)]
    res.update({"log_between": len(logs), "values": len(vals), "other": other[:10], "n_other": len(other),
                "reads": dict(zip([r[0] for r in reads], vals)) if len(vals) == len(reads) else None})
    tree = [l for l in lines[e[0] + 1:] if "── " in l]
    res["listing_entries"] = len(tree)
    res["listing_slot"] = any("Slot:" in l for l in lines[e[0] + 1:])
    return res


def main():
    case = json.load(open(sys.argv[1]))
    dest, runs = sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 1
    cmds, reads = commands(case)
    with open(dest, "a") as f:
        for r in range(runs):
            home = tempfile.mkdtemp(prefix="aet2204-home-", dir="/cache/homes")
            line = "; ".join(cmds).replace("LOGFILE", f"{home}/renode.log")
            os.makedirs(f"{home}/.config/renode")
            with open(f"{home}/.config/renode/config", "w") as cf:
                cf.write("[general]\nhistory-path = /dev/null\n")
            env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0",
                   "DOTNET_EnableWriteXorExecute": "0"}
            t0 = time.monotonic()
            p = subprocess.run(["renode", "--disable-gui", "--console", "--plain", "-e", line],
                               stdin=subprocess.DEVNULL, capture_output=True, env=env, timeout=300)
            wall = time.monotonic() - t0
            out = p.stdout.decode("utf-8", "replace")
            res = classify(out, reads)
            logged = ""
            if case.get("logfile") and os.path.exists(f"{home}/renode.log"):
                logged = open(f"{home}/renode.log", errors="replace").read()
            # Witness patterns and the grep read the log file when there is one.
            logs = logged if case.get("logfile") else out
            pats = case.get("count", {})
            counts = {k: len(re.findall(v, logs)) for k, v in pats.items()}
            # Renode folds a repeated line into one with a ` (N)` count: sum them.
            for k, v in case.get("sum", {}).items():
                counts[k] = sum(int(m) if m else 1 for m in re.findall(v + r"(?: \((\d+)\))?\r?$", out, re.M))
            if case.get("logfile"):
                row_file = {k: sum(int(m) if m else 1 for m in re.findall(v + r"(?: \((\d+)\))?\r?$", logged, re.M))
                            for k, v in case.get("sum", {}).items()}
                row_file["file_bytes"] = len(logged)
                if case.get("marker"):
                    at = [i for i, l in enumerate(logged.splitlines()) if l.endswith(case["marker"])]
                    row_file["markers"] = len(at)
                    if len(at) == 1:
                        before = "\n".join(logged.splitlines()[:at[0]])
                        row_file["before_marker"] = {
                            k: sum(int(m) if m else 1 for m in re.findall(v + r"(?: \((\d+)\))?\r?$", before, re.M))
                            for k, v in case.get("sum", {}).items()}
                counts["file"] = row_file
            row = {"case": case["name"], "run": r, "exit": p.returncode, "wall_s": round(wall, 3),
                   "stdout_bytes": len(p.stdout), "counts": counts, **res,
                   "reads_sha256": hashlib.sha256(json.dumps(res.get("reads"), sort_keys=True).encode()).hexdigest()[:16]}
            if case.get("grep"):
                row["grep"] = [l.split("] ", 1)[-1][:200] for l in logs.splitlines() if re.search(case["grep"], l)][:80]
                row["log_bytes"] = len(logs)
            f.write(json.dumps(row) + "\n")
            f.flush()
            if r == 0:
                with open(f"{dest}.{case['name']}.stdout", "w") as so:
                    so.write(out[-400_000:])
                with open(f"{dest}.{case['name']}.cmds", "w") as so:
                    so.write(line + "\n")


if __name__ == "__main__":
    main()
