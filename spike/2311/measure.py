"""AET-WO-2311 §1.1: the CLI's own --foreign monitor script (captured from Renode's argv by
sh/capture.sh), with paths rewritten to the capture and three additions:
  - on each declared port's USART, a write hook logging every write to offsets 0x4..0x7 (CR2 and
    any byte or half-word inside it): `AETHELGARD-CR2 <offset> <value> <CR1 as it stands>`;
  - optional EXTRA monitor commands (adversarial probes): those before a line `--after` run after the
    hooks, before the run; those after it run after the run, before the begin marker;
  - after the begin marker, each serial pin's GPIO PUPDR and each USART's CR2, read back.
python3 -I measure.py CAPDIR ELF OUTSCRIPT [EXTRA_FILE]"""
import sys
cap, elf, out = sys.argv[1:4]
extra = open(sys.argv[4]).read().splitlines() if len(sys.argv) > 4 else []
before, after = (extra[:extra.index("--after")], extra[extra.index("--after") + 1:]) if "--after" in extra else (extra, [])
cmds = open(f"{cap}/script.txt").read().splitlines()
usarts = {"usart1": 0x40011000, "usart2": 0x40004400, "usart3": 0x40004800, "uart4": 0x40004C00}
gpio = {"A": 0x58020000, "B": 0x58020400}
PUPDR = 0x0C
log = "__import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, __import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, 'AETHELGARD-CR2 %x %x %x' % (offset, value, self.ReadDoubleWord(0)))"
hooks = [f'sysbus SetHookBeforePeripheralWrite sysbus.{u} "(offset >= 4 and offset < 8) and {log}"' for u in usarts]
res = []
for c in cmds:
    if c.startswith("machine LoadPlatformDescription"):
        c = f"machine LoadPlatformDescription @{cap}/platform-booted.repl"
    elif c.startswith("sysbus LoadELF"):
        c = f"sysbus LoadELF @{elf}"
    if c.startswith("emulation RunFor"):
        res += hooks + before
    if c == 'echo "AETHELGARD-BOOT-BEGIN"':
        res += after
    res.append(c)
    if c == 'echo "AETHELGARD-BOOT-BEGIN"':
        for p, base in gpio.items():
            res += [f'echo "PUPDR GPIO{p}"', f"sysbus ReadDoubleWord {base + PUPDR:#x}"]
        for u, base in usarts.items():
            res += [f'echo "CR2 {u}"', f"sysbus ReadDoubleWord {base + 4:#x}"]
open(out, "w").write("; ".join(res))
