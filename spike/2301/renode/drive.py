#!/usr/bin/env python3
"""AET-WO-2301 spike, P1 and P3: drive Renode 1.17.0's monitor on a piped stdin
(AET-WO-2101 H1) through one boot of the generated firmware, apply one means
between quanta, and read the firmware's view between whole-line sentinels.

Usage: drive.py <case> <runs> <out.jsonl>

The prefix is the CLI's own boot (`renode.rs` BootPlan::commands, captured from
`aethelgard campaign` on pullup-2k2: out/p1-cli-cmdline.txt), with the reads
moved into blocks. Each run gets a fresh HOME whose Renode config points the
monitor history at /dev/null (`ShellProvider.cs` L33, `history-path`).
"""
import hashlib, json, os, re, subprocess, sys, tempfile, time

VALUE = re.compile(r"0x[0-9A-Fa-f]+|True|False")

FW = "/cache/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
REPL = "/repo/boards/sentinel-01/sentinel-01.repl"
BOOT_WORD, STATUS_AT, VALUE_AT = 0x20000000, 0x20000004, 0x20000008
COLD = [0x580244E0, 0x580244E8, 0x580244F0]
VTOR = 0x08000000
UNSET = 0xFFFFFFFF


def prefix(log):
    # The log goes to a file and the console shows errors only: Renode's log
    # thread otherwise splits echoed commands on stdout (measured: fragments
    # such as `-2"` and a bare `0x20000000` landed between the sentinels).
    c = [f"logFile @{log}", "logLevel 3 console", "emulation SetSeed 2100", "mach create",
         f"machine LoadPlatformDescription @{REPL}", f"sysbus LoadELF @{FW}"]
    c += [f"sysbus WriteDoubleWord {a:#010x} 0x0" for a in COLD]
    c += [f"sysbus WriteDoubleWord {BOOT_WORD:#010x} 0x0"]
    c += [f"sysbus WriteDoubleWord {a:#010x} {UNSET:#010x}" for a in (STATUS_AT, VALUE_AT)]
    c += [f"cpu VectorTableOffset {VTOR:#010x}"]
    return c


def rearm():
    """The bridge's action, not the firmware's: what the CLI writes before the first boot."""
    return [f"sysbus WriteDoubleWord {BOOT_WORD:#010x} 0x0"] + [
        f"sysbus WriteDoubleWord {a:#010x} {UNSET:#010x}" for a in (STATUS_AT, VALUE_AT)]


def cold_and_vtor():
    return [f"sysbus WriteDoubleWord {a:#010x} 0x0" for a in COLD] + [
        f"cpu VectorTableOffset {VTOR:#010x}"]


def run_for(ns):
    return f'emulation RunFor "{ns // 1_000_000_000}.{ns % 1_000_000_000:09d}"'


READS = [
    ("boot", f"sysbus ReadDoubleWord {BOOT_WORD:#010x}"),
    ("status", f"sysbus ReadDoubleWord {STATUS_AT:#010x}"),
    ("value", f"sysbus ReadDoubleWord {VALUE_AT:#010x}"),
    ("instr", "cpu ExecutedInstructions"),
    ("pc", "cpu PC"),
    ("halted", "cpu IsHalted"),
    ("vtor", "cpu VectorTableOffset"),
    ("rsr", "sysbus ReadDoubleWord 0x580244d0"),
    ("ispr0", "sysbus ReadDoubleWord 0xe000e200"),
    ("syst_csr", "sysbus ReadDoubleWord 0xe000e010"),
    ("pwr_csr1", "sysbus ReadDoubleWord 0x58024804"),
    ("cr2", "sysbus ReadDoubleWord 0x40005404"),
]


class Script:
    def __init__(self):
        self.cmds, self.blocks, self.t = [], [], 0

    def add(self, cmds):
        self.cmds += cmds

    def run(self, ns):
        self.cmds.append(run_for(ns))
        self.t += ns

    def read(self, label, keys=None):
        n = len(self.blocks)
        keys = keys or [k for k, _ in READS]
        self.blocks.append((label, self.t, keys))
        self.cmds.append(f'echo "AET2301-BEGIN-{n}"')
        self.cmds += [cmd for k, cmd in READS if k in keys]
        self.cmds.append(f'echo "AET2301-END-{n}"')


MS, US = 1_000_000, 1_000
STEP = 10 * US  # recovery is located at 10 µs resolution


def recovery(s, label, span=1 * MS):
    for i in range(span // STEP):
        s.run(STEP)
        s.read(f"{label}+{(i + 1) * STEP // US}us", ["boot", "status", "value", "instr", "pc"])


def case(name, log):
    s = Script()
    s.add(prefix(log))
    if name == "systick-before":
        s.add(["sysbus WriteDoubleWord 0xe000e014 999", "sysbus WriteDoubleWord 0xe000e018 0",
               "sysbus WriteDoubleWord 0xe000e010 7"])
    s.run(10 * MS)
    s.read("t10ms")
    s.add(rearm())
    s.read("rearmed")
    hold = None
    if name == "control":
        pass
    elif name == "machine-reset":
        s.add(["machine Reset"])
    elif name == "machine-reset-vtor":
        s.add(["machine Reset"] + cold_and_vtor())
    elif name == "cpu-reset":
        s.add(["cpu Reset"])
    elif name == "held-halted":
        s.add(["machine Reset"] + cold_and_vtor() + ["cpu IsHalted true"])
        hold = ["cpu IsHalted false"]
    elif name == "held-halted-nudge":
        # As held-halted, then one 10 ns RunFor right after the release: does
        # only the first segment after a release execute nothing?
        s.add(["machine Reset"] + cold_and_vtor() + ["cpu IsHalted true"])
        hold = ["cpu IsHalted false", run_for(10)]
    elif name == "held-cpuwait":
        s.add(["cpu OnGPIO 4096 true", "machine Reset"] + cold_and_vtor())
        hold = ["cpu OnGPIO 4096 false"]
    elif name == "pvd-exti16":
        s.add(["sysbus.exti OnGPIO 16 true"])
    elif name == "systick-after":
        s.add(["sysbus WriteDoubleWord 0xe000e014 999", "sysbus WriteDoubleWord 0xe000e018 0",
               "sysbus WriteDoubleWord 0xe000e010 7"])
    elif name == "nvic-pend1":
        s.add(["sysbus WriteDoubleWord 0xe000e200 0x2"])
    elif name == "systick-before":
        pass
    else:
        raise SystemExit(f"unknown case {name}")
    s.read("applied")
    if hold:
        s.run(1 * MS)
        s.read("held1ms")
        s.add(hold)
        s.read("released")
    recovery(s, "after")
    s.run(20 * MS - s.t)
    s.read("t20ms")
    s.add(["quit"])
    return s


def parse(out, blocks):
    lines = [l.strip("\r") for l in out.splitlines()]
    got = []
    for n, (label, t, keys) in enumerate(blocks):
        try:
            b = lines.index(f"AET2301-BEGIN-{n}")
            e = lines.index(f"AET2301-END-{n}", b)
        except ValueError:
            got.append({"label": label, "t_ns": t, "missing": True})
            continue
        # In stdin mode the monitor echoes each command after its prompt,
        # `(machine-0) …`, and log lines start with `[`; a value is the rest.
        # A log line can land inside an echoed command and split it, so a
        # value is a whole line that is a value, nothing else (`renode.rs` R3).
        vals = [l.strip() for l in lines[b + 1:e] if VALUE.fullmatch(l.strip())]
        got.append({"label": label, "t_ns": t, **dict(zip(keys, vals)),
                    **({"extra": vals[len(keys):]} if len(vals) != len(keys) else {})})
    return got


def main():
    name, runs, dest = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    with open(dest, "w") as f:
        for r in range(runs):
            home = tempfile.mkdtemp(prefix="aet2301-home-")
            s = case(name, f"{home}/renode.log")
            os.makedirs(f"{home}/.config/renode")
            with open(f"{home}/.config/renode/config", "w") as c:
                c.write("[general]\nhistory-path = /dev/null\n")
            env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin",
                   "DOTNET_EnableDiagnostics": "0"}
            t0 = time.monotonic()
            p = subprocess.run(["renode", "--disable-gui", "--console", "--plain"],
                               input="\n".join(s.cmds) + "\n", capture_output=True,
                               text=True, env=env, timeout=600)
            wall = time.monotonic() - t0
            blocks = parse(p.stdout, s.blocks)
            digest = hashlib.sha256(json.dumps(blocks, sort_keys=True).encode()).hexdigest()
            with open(f"{home}/renode.log", errors="replace") as lf:
                logged = lf.read()
            warn = [l for l in logged.splitlines() if "WARNING" in l or "ERROR" in l]
            stray = [l for l in p.stdout.splitlines() if "WARNING" in l]
            f.write(json.dumps({"case": name, "run": r, "exit": p.returncode, "wall_s": round(wall, 3),
                                "hash": digest, "blocks": blocks, "log_lines": len(logged.splitlines()),
                                "warnings_on_stdout": len(stray),
                                "log_warnings": sorted(set(w.split("] ", 1)[-1][:160] for w in warn))[:20],
                                "stderr_tail": p.stderr[-400:]}) + "\n")
            f.flush()
            if r == 0:
                with open(dest + ".cmds", "w") as c:
                    c.write("\n".join(s.cmds) + "\n")
                with open(dest + ".stdout", "w") as c:
                    c.write(p.stdout[-200_000:])


if __name__ == "__main__":
    main()
