"""AET-WO-2314 §1.1 (2): the CLI's own --foreign monitor script (captured by sh/capture.sh), with the
ELF swapped and, per MODE, execution hooks on the four fault handlers the ELF's vector table names
(entries 3-6 at FLASH_BANK1_BASE, Thumb bit cleared), plus reads after the begin marker.

  python3 -I measure.py CAPDIR ELF OUTSCRIPT MODE [nolog]
MODE:
  none     the captured script as is (with the ELF swapped)
  probe    each hook logs its vector, the CPU's attribute names that matter, and stops nothing
  log      each hook logs `AETHELGARD-FAULT <vector> pc=<handler> icsr=<ICSR> lr=<LR> sp=<SP> psp=<PSP>
           msp=<MSP> stacked=<stacked PC>`; stops nothing
  halt     as log, then sets the CPU's IsHalted
  pause    as log, then asks the emulation to pause (cpu.Pause())
  mpause   as log, then the machine's Pause via the emulation manager
After the begin marker: VTOR, the four live vector entries, CFSR, HFSR, MMFAR, BFAR.
With `nolog`, the CLI's `sysbus LogPeripheralAccess` lines are dropped."""
import struct
import sys

cap, elf, out, mode = sys.argv[1:5]
nolog = sys.argv[5:] == ["nolog"]
VT = 0x08000000
NAMES = {3: "HardFault", 4: "MemManage", 5: "BusFault", 6: "UsageFault"}


def vector_table(path):
    d = open(path, "rb").read()
    phoff, = struct.unpack_from("<I", d, 0x1C)
    phentsize, phnum = struct.unpack_from("<HH", d, 0x2A)
    for i in range(phnum):
        t, off, va, pa, fs, _, _, _ = struct.unpack_from("<8I", d, phoff + i * phentsize)
        if t == 1 and pa <= VT and VT + 28 <= pa + fs:
            return {n: struct.unpack_from("<I", d, off + VT - pa + 4 * n)[0] & ~1 for n in NAMES}
    sys.exit("no loaded segment holds the vector table")


LOG = ("__import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, "
       "__import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, {msg})")
STOP = {
    "probe": "", "log": "", "halt": "; self.IsHalted = True",
    "pause": "; self.Pause()",
    "mpause": "; __import__('Antmicro.Renode.Core').Renode.Core.EmulationManager.Instance.CurrentEmulation.PauseAll()",
}


def hook(n, addr):
    if mode == "probe":
        msg = ("'AETHELGARD-PROBE {v} %s' % ' '.join(sorted(x for x in dir(self) if any(k in x.upper() "
               "for k in ('SP', 'PSR', 'HALT', 'PAUSE', 'BUS', 'LR', 'VEC', 'REG'))))").format(v=NAMES[n])
        return f'cpu AddHook {addr:#x} "{LOG.format(msg=msg)}"'
    regs = ("r = lambda x: int(x.RawValue) if hasattr(x, 'RawValue') else int(x); "
            "lr = r(self.LR); sp = r(self.SP); osp = r(self.OtherSP); fr = osp if lr & 4 else sp; "
            "st = self.Bus.ReadDoubleWord(fr + 24); ic = self.Bus.ReadDoubleWord(0xE000ED04); "
            "vt = r(self.VectorTableOffset); ")
    msg = ("'AETHELGARD-FAULT {v} pc=%x icsr=%x lr=%x sp=%x osp=%x stacked=%x vtor=%x n=%d' % "
           "(pc, ic, lr, sp, osp, st, vt, self.ExecutedInstructions)").format(v=NAMES[n])
    return f'cpu AddHook {addr:#x} "{regs}{LOG.format(msg=msg)}{STOP[mode]}"'


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
    if c.startswith("emulation RunFor") and mode != "none":
        res += [hook(n, a) for n, a in vector_table(elf).items()]
    res.append(c)
    if c == 'echo "AETHELGARD-BOOT-BEGIN"':
        for name, a in [("VTOR", 0xE000ED08)] + [(f"VEC{n}", VT + 4 * n) for n in NAMES] + [
                ("CFSR", 0xE000ED28), ("HFSR", 0xE000ED2C), ("MMFAR", 0xE000ED34), ("BFAR", 0xE000ED38)]:
            res += [f'echo "{name}"', f"sysbus ReadDoubleWord {a:#x}"]
open(out, "w").write("; ".join(res))
