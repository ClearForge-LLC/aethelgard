"""AET-WO-4412 (1) (4): a `reset` macro for a captured foreign run (patched path, outside the CLI): Renode 1.17.0 runs a
machine's `reset` macro on SYSRESETREQ and otherwise logs "No action for reset" (4400h §2.3). The macro is the CLI's
own boot lines, from `sysbus LoadELF` to `cpu VectorTableOffset` (as 4400h's), wrapped in probes of the state a
silicon system reset keeps or loses (RM0433 Rev 9 §8.4.2, §8.4.4; backup domain §6.4.4, §8.4.6):
  - flash bank 2, AP_FlashStorage's pages 14/15 (0x081C0000, 0x081E0000; 4400h §2.3): kept on silicon;
  - RTC_BKP0R..2R (RTC 0x58004000 + 0x50): kept (backup domain);
  - RCC_RSR (0x580244D0): SFTRSTF on silicon after SYSRESETREQ;
  - SRAM: a word of AXI SRAM (0x24000000) and DTCM (0x20000000): kept on silicon (content not reset).
Each probe is a monitor `sysbus ReadDoubleWord`, printed after an `echo AET-RESET-<when>` marker.
MODE=boot (default): the boot lines; MODE=probe: the probes only (does Renode's reset alone reload anything?).
Usage: resetmacro.py E_ORIG OUT.resc"""
import os, sys
src, out = sys.argv[1:3]
cmds = [c.strip() for c in open(src).read().split("; ")]
a = next(i for i, c in enumerate(cmds) if c.startswith("sysbus LoadELF"))
b = next(i for i, c in enumerate(cmds) if c.startswith("cpu VectorTableOffset"))
boot = cmds[a:b + 1]
PROBE = [0x081C0000, 0x081C0004, 0x081E0000, 0x58004050, 0x58004054, 0x58004058, 0x580244D0, 0x24000000, 0x20000000,
         0x08000000, 0x08000004]
def probe(when):
    return ['echo "AET-RESET-%s"' % when] + ["sysbus ReadDoubleWord 0x%08X" % x for x in PROBE]
body = probe("AT-MACRO") + (boot if os.environ.get("MODE", "boot") == "boot" else []) + probe("AFTER-BOOT-LINES")
with open(out, "w") as f:
    f.write('macro reset\n"""\n' + "\n".join(body) + '\n"""\n')
print(out, len(boot), "boot lines")
