"""AET-WO-4412 (1) (4): a `reset` macro for a captured foreign run (patched path, outside the CLI): Renode 1.17.0 runs a
machine's `reset` macro on SYSRESETREQ and otherwise logs "No action for reset" (4400h §2.3). The macro is the CLI's
own boot lines, from `sysbus LoadELF` to `cpu VectorTableOffset` (as 4400h's), wrapped in probes of the state a
silicon system reset keeps or loses (RM0433 Rev 9 §8.4.2, §8.4.4; backup domain §6.4.4, §8.4.6):
  - flash bank 2, AP_FlashStorage's pages 14/15 (0x081C0000, 0x081E0000; 4400h §2.3): kept on silicon;
  - RTC_BKP0R..2R (RTC 0x58004000 + 0x50): kept (backup domain);
  - RCC_RSR (0x580244D0): SFTRSTF on silicon after SYSRESETREQ;
  - SRAM: a word of AXI SRAM (0x24000000) and DTCM (0x20000000): kept on silicon (content not reset).
Each probe is a monitor `sysbus ReadDoubleWord`, printed after an `echo AET-RESET-<when>` marker.
MODE=boot (default): the boot lines; MODE=probe: the probes only (does Renode's reset alone reload anything?);
MODE=vtor: the boot lines without `sysbus LoadELF` (flash survives Renode's reset), then RSR=<hex> written to RCC_RSR
(RM0433 Rev 9 Table 56 row 4, p. 333: SFTRSTF, PINRSTF, CPURSTF set by a system reset from the CPU), read back.
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
mode = os.environ.get("MODE", "boot")
lines = {"boot": boot, "probe": [], "vtor": [c for c in boot if not c.startswith("sysbus LoadELF")]}[mode]
if mode == "vtor" and os.environ.get("RSR"):
    lines = lines + ["sysbus WriteDoubleWord 0x580244D0 %s" % os.environ["RSR"]]
if os.environ.get("RSRHOOK") == "1":
    # D5 (ruled 11:06Z): after the reset, RCC_RSR reads Table 56 row 4 (p. 333): SFTRSTF|PINRSTF|CPURSTF
    # (0x01420000), and a write with RMVF (bit 16) clears it; installed here, so never before a reset.
    log = "__import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, __import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, '%s %%x' %% %s)"
    lines = lines + [
        "python \"__import__('sys').aet_rsr = 0x01420000\"",
        "sysbus SetHookAfterPeripheralRead sysbus.rcc \"if offset == 0xD0: value = __import__('sys').aet_rsr; " + (log % ("AET-RSR-READ", "value")).replace('"', '\\"') + "\"",
        "sysbus SetHookBeforePeripheralWrite sysbus.rcc \"if offset == 0xD0 and value & 0x10000: __import__('sys').aet_rsr = 0; " + (log % ("AET-RSR-RMVF", "value")).replace('"', '\\"') + "\"",
    ]
    if os.environ.get("RSRSELFTEST") == "1":   # the shared state crosses the two hooks: RMVF, read (0), then re-arm
        lines = lines + ["echo \"AET-RSR-SELFTEST\"", "sysbus WriteDoubleWord 0x580244D0 0x10000",
                         "sysbus ReadDoubleWord 0x580244D0", "python \"__import__('sys').aet_rsr = 0x01420000\"",
                         "sysbus ReadDoubleWord 0x580244D0"]
body = probe("AT-MACRO") + lines + probe("AFTER-BOOT-LINES")
with open(out, "w") as f:
    f.write('macro reset\n"""\n' + "\n".join(body) + '\n"""\n')
print(out, len(boot), "boot lines")
