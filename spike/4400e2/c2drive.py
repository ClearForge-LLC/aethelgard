"""AET-WO-4400e2: 4400c2's driver at 5a1bccaf with only its ELF and argv paths moved under /cache/c2/ (4400c2's ELF c8a2c9fa..., its captured CLI argv). The 4400c2 driver, AET-WO-4400c's spike driver with two additions (inside the tools image, python3 -I).

Added for 4400c2: PLATFORM (env) names the platform file to load in place of the emitted one;
INJECT_AT_S and INJECT_HEX (env) write those bytes into USART1's receiver, one `WriteChar` each,
at the first step boundary at or after that virtual time; INJECT_PACE_NS spaces them by that time.


Boots CI's unmodified ArduPilot ELF on the platform `aethelgard-twin` emits for
sentinel-02-own, with the CLI's own Renode config (crates/aethelgard-cli/src/
renode.rs `config`: synchronous logging, virtual timestamps) and the CLI's own
monitor script for this platform, captured from /proc while the unmodified CLI
booted this board's generated Rust firmware (spike/4400c/sh/gen.sh). Only the
ELF is swapped. Lines left out of the captured script, each a deviation:
  - the boot-word watchpoint hook and its serial injection (the judge's
    machinery; ArduPilot writes no boot word);
  - the run itself (`RunFor 0.1` and the judge's reads): replaced by steps,
    each followed by this driver's reads.
Overlays (each a deviation, named in FEEDBACK) come from STUBS, a file of
monitor commands sent after the setup and before the first step.

RENODE (env, AET-WO-4400e2 deviation C) names another renode executable; default the image's.

usage: drive.py OUT SECONDS STEP_MS [STUBS]
"""
import json
import os
import subprocess
import sys
import threading
import queue
import time

OUT, SECONDS, STEP_MS = sys.argv[1], float(sys.argv[2]), int(sys.argv[3])
STUBS = sys.argv[4] if len(sys.argv) > 4 else None
ELF = "/cache/c2/arducopter.elf"
ARGV = "/cache/c2/renode-argv.txt"
TOOL = os.path.join(OUT, "tool")
os.makedirs(os.path.join(TOOL, "models"), exist_ok=True)
os.makedirs(os.path.join(TOOL, "capture"), exist_ok=True)
HOME = os.path.join(OUT, "home")
os.makedirs(HOME, exist_ok=True)

# The CLI's renode.config, verbatim but for the history path (renode.rs `config`).
CONFIG = f"""[general]
use-synchronous-logging = true
terminal = Termsharp
compiler-cache-enabled = False
serialization-mode = Generated
always-log-machine-name = False
collapse-repeated-log-entries = True
log-timestamps-type = Virtual
log-history-limit = 1000
history-path = {HOME}/history
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
open(os.path.join(OUT, "renode.config"), "w").write(CONFIG)

argv = open(ARGV).read().split("\n")
script = [a for a in argv if "LoadPlatformDescription" in a][0]
cmds = script.split("; ")
prefix = cmds[2].split("@")[1].split("/models/")[0]
setup = []
for c in cmds:
    if c.startswith("sysbus AddWatchpointHook") or c.startswith("emulation RunFor"):
        break
    setup.append(c)
mapped = []
for c in setup:
    c = c.replace(prefix + "/firmware.elf", ELF)
    c = c.replace(prefix + "/platform-booted.repl", os.environ.get("PLATFORM", "/cache/s02own/gen/board/sentinel-02-own.repl"))
    c = c.replace(prefix + "/models/", "/repo/models/renode/")
    c = c.replace(prefix, TOOL)
    mapped.append(c)
stubs = []
if STUBS:
    stubs = [l.strip() for l in open(STUBS) if l.strip() and not l.startswith("#")]
open(os.path.join(OUT, "setup.cmds"), "w").write("\n".join(mapped + ["# stubs"] + stubs) + "\n")

p = subprocess.Popen(
    [os.environ.get("RENODE", "renode"), "--config", os.path.join(OUT, "renode.config"), "--disable-gui", "--console", "--plain"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
    env={"HOME": HOME, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0"},
    text=True, bufsize=1)
q = queue.Queue()
log = open(os.path.join(OUT, "console.log"), "w")


def reader():
    for line in p.stdout:
        line = line.rstrip("\n").strip("\r")
        log.write(line + "\n")
        q.put(line)
    q.put(None)


threading.Thread(target=reader, daemon=True).start()
serial = [0]


def ex(commands, wait=600):
    serial[0] += 1
    s = f"AET4400C-{serial[0]}"
    p.stdin.write("\n".join(commands) + f"\necho \"{s}\"\n")
    p.stdin.flush()
    vals = []
    t0 = time.time()
    while True:
        try:
            l = q.get(timeout=max(1, wait - (time.time() - t0)))
        except queue.Empty:
            raise SystemExit(f"no sentinel {s} in {wait}s")
        if l is None:
            raise SystemExit("renode exited")
        if l == s:
            return vals
        if l.startswith("0x") and all(ch in "0123456789abcdefABCDEF" for ch in l[2:]) and len(l) > 2:
            vals.append(int(l, 16))
        elif l in ("True", "False"):
            vals.append(l)


def rss_kib():
    tot = 0
    def walk(pid):
        nonlocal tot
        try:
            for l in open(f"/proc/{pid}/status"):
                if l.startswith("VmRSS:"):
                    tot += int(l.split()[1])
            for k in open(f"/proc/{pid}/task/{pid}/children").read().split():
                walk(int(k))
        except OSError:
            pass
    walk(p.pid)
    return tot


# Diagnostic reads after every step (addresses from RM0433 / stm32h753xx.h).
READS = [
    ("PC", "cpu PC"),
    ("INSN", "cpu ExecutedInstructions"),
    ("ICSR", "sysbus ReadDoubleWord 0xE000ED04"),
    ("EXTI_C1IMR1", "sysbus ReadDoubleWord 0x58000080"),
    ("USART1_CR1", "sysbus ReadDoubleWord 0x40011000"),
    ("USART1_CR3", "sysbus ReadDoubleWord 0x40011008"),
    ("USART1_BRR", "sysbus ReadDoubleWord 0x4001100C"),
    ("USART1_ISR", "sysbus ReadDoubleWord 0x4001101C"),
    ("DMA1_LISR", "sysbus ReadDoubleWord 0x40020000"),
    ("DMA1_HISR", "sysbus ReadDoubleWord 0x40020004"),
    ("DMA1_S4CR", "sysbus ReadDoubleWord 0x40020070"),
    ("DMA1_S4NDTR", "sysbus ReadDoubleWord 0x40020074"),
    ("DMAMUX1_C4CR", "sysbus ReadDoubleWord 0x40020810"),
    ("TIM5_CR1", "sysbus ReadDoubleWord 0x40000C00"),
    ("TIM5_DIER", "sysbus ReadDoubleWord 0x40000C0C"),
    ("TIM5_SR", "sysbus ReadDoubleWord 0x40000C10"),
    ("TIM5_CNT", "sysbus ReadDoubleWord 0x40000C24"),
    ("TIM5_PSC", "sysbus ReadDoubleWord 0x40000C28"),
    ("TIM5_CCR1", "sysbus ReadDoubleWord 0x40000C34"),
    ("NVIC_ISER0", "sysbus ReadDoubleWord 0xE000E100"),
    ("NVIC_ISER1", "sysbus ReadDoubleWord 0xE000E104"),
    ("NVIC_ISPR0", "sysbus ReadDoubleWord 0xE000E200"),
    ("NVIC_ISPR1", "sysbus ReadDoubleWord 0xE000E204"),
    ("I2C1_CR1", "sysbus ReadDoubleWord 0x40005400"),
    ("I2C1_ISR", "sysbus ReadDoubleWord 0x40005418"),
    ("RCC_CR", "sysbus ReadDoubleWord 0x58024400"),
    ("RCC_CFGR", "sysbus ReadDoubleWord 0x58024410"),
]

peak = [0]  # AET-WO-4400e2: the tree's peak VmRSS, sampled every 100 ms (drive.py's sampler)


def sampler():
    while p.poll() is None:
        peak[0] = max(peak[0], rss_kib())
        time.sleep(0.1)


threading.Thread(target=sampler, daemon=True).start()
t_start = time.time()
for c in mapped:
    ex([c])
after_load = dict(zip([r[0] for r in READS], ex([r[1] for r in READS])))
for c in stubs:
    ex([c])
steps = []
cap = os.path.join(TOOL, "capture", "usart1.bin")
t_ns = 0
end_ns = int(SECONDS * 1e9)
inject_at = int(float(os.environ["INJECT_AT_S"]) * 1e9) if os.environ.get("INJECT_AT_S") else None
inject = bytes.fromhex(os.environ.get("INJECT_HEX", ""))
injected_at = None
while t_ns < end_ns:
    if inject_at is not None and injected_at is None and t_ns >= inject_at:
        pace = int(os.environ.get("INJECT_PACE_NS", "0"))
        if pace:
            # One byte per character time at the declared baud: WriteChar, then RunFor one byte.
            for b_ in inject:
                ex([f"sysbus.usart1 WriteChar 0x{b_:02X}", f'emulation RunFor "0.{pace:09d}"'])
            t_ns += pace * len(inject)
        else:
            ex([f"sysbus.usart1 WriteChar 0x{b:02X}" for b in inject])
        injected_at = t_ns
    n = min(STEP_MS * 1_000_000, end_ns - t_ns)
    w0 = time.time()
    ex([f'emulation RunFor "{n // 1_000_000_000}.{n % 1_000_000_000:09d}"'])
    wall = time.time() - w0
    t_ns += n
    # AET-WO-4400e2: STUBS2 (env) sent once, after the first step that reaches STUBS2_AT_S (observation hooks
    # for the last instants only).
    if os.environ.get("STUBS2") and not getattr(sys, "_stubs2_done", False) and t_ns >= int(float(os.environ.get("STUBS2_AT_S", "0")) * 1e9):
        sys._stubs2_done = True
        for c2 in [l.strip() for l in open(os.environ["STUBS2"]) if l.strip() and not l.startswith("#")]:
            ex([c2])
    vals = ex([r[1] for r in READS])
    row = dict(zip([r[0] for r in READS], vals))
    row["t_ns"] = t_ns
    row["wall_s"] = round(wall, 3)
    row["rss_kib"] = rss_kib()
    row["console_bytes"] = os.path.getsize(cap) if os.path.exists(cap) else 0
    steps.append(row)
    print(json.dumps({k: (hex(v) if isinstance(v, int) and k not in ("t_ns", "INSN", "rss_kib", "console_bytes") else v) for k, v in row.items()}), flush=True)
# JUDGE_READS: "name address" lines, read once after the last step (the judge's own register list).
jr = os.environ.get("JUDGE_READS")
if jr:
    names, cmds_ = [], []
    for l in open(jr):
        if l.strip():
            nm, ad = l.split()
            names.append(nm)
            cmds_.append(f"sysbus ReadDoubleWord {ad}")
    vals = ex(cmds_ + ["cpu ExecutedInstructions", "cpu PC", "cpu PerformanceInMips"])
    json.dump(dict(zip(names + ["INSN", "PC", "MIPS"], [hex(v) if isinstance(v, int) else v for v in vals])),
              open(os.path.join(OUT, "judge-reads.json"), "w"), indent=1)
p.stdin.write("quit\n")
p.stdin.flush()
try:
    p.wait(timeout=30)
except subprocess.TimeoutExpired:
    p.kill()
json.dump({"after_load": after_load, "injected_at_ns": injected_at, "injected_hex": inject.hex(), "steps": steps, "wall_total_s": round(time.time() - t_start, 3), "peak_rss_kib": peak[0]},
          open(os.path.join(OUT, "steps.json"), "w"), indent=1)
