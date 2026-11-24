"""AET-WO-1609 §1.1: the CLI's own --foreign monitor script (captured by sh/capture.sh), with
the ELF swapped and three additions, so the run that is judged is also the run that is explained:
  - an execution hook on AP_HAL::panic's entry and on HardFault_Handler's (addresses from
    plant.py's symbols, given here), each logging `AETHELGARD-HOOK <name> pc=<pc>` once per entry,
    with Renode's virtual timestamp;
  - after the begin marker, SCB's CFSR (0xE000ED28) and HFSR (0xE000ED2C).
With a fourth argument `nolog`, the CLI's `sysbus LogPeripheralAccess` lines are dropped (to
measure what they cost a run; that script is not the judge's).
python3 -I measure.py CAPDIR ELF OUTSCRIPT [nolog]"""
import sys
cap, elf, out = sys.argv[1:4]
nolog = sys.argv[4:] == ["nolog"]
hooks = {"panic": 0x080C033C, "HardFault_Handler": 0x08000340}
log = ("__import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, "
       "__import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, "
       "'AETHELGARD-HOOK {name} pc=%x' % pc)")
res = []
for c in open(f"{cap}/script.txt").read().splitlines():
    if c.startswith("machine LoadPlatformDescription"):
        c = f"machine LoadPlatformDescription @{cap}/platform-booted.repl"
    elif c.startswith("include @") and "/models/" in c:
        c = f"include @{cap}/models/" + c.split("/models/", 1)[1]
    elif c.startswith("sysbus LoadELF"):
        c = f"sysbus LoadELF @{elf}"
    if nolog and c.startswith("sysbus LogPeripheralAccess"):
        continue
    if c.startswith("emulation RunFor"):
        res += [f'cpu AddHook {a:#x} "{log.format(name=n)}"' for n, a in hooks.items()]
    res.append(c)
    if c == 'echo "AETHELGARD-BOOT-BEGIN"':
        res += ['echo "CFSR"', "sysbus ReadDoubleWord 0xE000ED28", 'echo "HFSR"', "sysbus ReadDoubleWord 0xE000ED2C"]
open(out, "w").write("; ".join(res))
