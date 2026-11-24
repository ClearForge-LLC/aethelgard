"""AET-WO-2314 §1.1 (spike/1609/plant.py, the newline literal checked only for `panic`): a copy of ArduPilot's ELF that fails after its last witness.

The last witness of unmodified ArduPilot on sentinel-02-own is U5's (ICM-42688-P) first FIFO
data read, `data (reads[0])`: in `AP_InertialSensor_Invensensev3::read_fifo()`, the device
transfer of the FIFO bytes (after the FIFO_COUNTH `block_read`, which is not that witness: a plant
there, measured first, leaves `data (reads[0])` unwitnessed). SITE `data` writes the plant straight
after that transfer returns, before `accumulate_samples`, so it follows the witness by causality,
not by a guessed time; SITE `count` writes it after the FIFO_COUNTH read (before the witness). The site and every target are found by symbol in the ELF's own symtab, and the
bytes replaced are checked first, so a different build is refused rather than patched blind.

  python3 -I plant.py SRC DST VARIANT [SITE: data (default) or count]
VARIANT:
  panic  `AP_HAL::panic("\\n")`: ArduPilot's own panic (system.cpp:327), which loops printing
         and sleeping in the calling thread (here the SPI bus thread);
  udf    `udf #0`: an undefined instruction, a UsageFault escalated to HardFault, taken by
         ArduPilot's HardFault_Handler (CrashCatcher here, AP_CRASHDUMP_ENABLED);
  halt   `cpsid i; b .`: interrupts off and a self-loop in thread mode (chSysHalt's shape);
  reset  SCB->AIRCR = 0x05FA0004 (SYSRESETREQ), then `b .`: a software reset, as the
         watchdog's or `hal.scheduler->reboot()`'s would be.
"""
import struct
import sys

src, dst, variant = sys.argv[1:4]
where = sys.argv[4] if len(sys.argv) > 4 else "data"
data = bytearray(open(src, "rb").read())
assert data[:4] == b"\x7fELF" and data[4] == 1 and data[5] == 1, "not a 32-bit little-endian ELF"
shoff, = struct.unpack_from("<I", data, 0x20)
shentsize, shnum = struct.unpack_from("<HH", data, 0x2E)
secs = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
symtab, = [s for s in secs if s[1] == 2]
strtab = secs[symtab[6]]


def cstr(off):
    return bytes(data[off:data.index(0, off)])


def symbol(name):
    hits = set()
    for i in range(symtab[5] // 16):
        st_name, value, size = struct.unpack_from("<III", data, symtab[4] + i * 16)
        if cstr(strtab[4] + st_name) == name:
            hits.add((value & ~1, size))
    (hit,) = hits
    return hit


def file_off(a):
    (s,) = [s for s in secs if s[1] == 1 and s[3] <= a < s[3] + s[5]]
    return a - s[3] + s[4]


def movw_movt(op, rd, imm16):
    i, imm4, imm3, imm8 = (imm16 >> 11) & 1, imm16 >> 12, (imm16 >> 8) & 7, imm16 & 0xFF
    return struct.pack("<HH", op | (i << 10) | imm4, (imm3 << 12) | (rd << 8) | imm8)


def mov32(rd, value):
    return movw_movt(0xF240, rd, value & 0xFFFF) + movw_movt(0xF2C0, rd, value >> 16)


def bl(at, target):
    off = target - (at + 4)
    assert -(1 << 24) <= off < (1 << 24) and off % 2 == 0
    s = (off >> 24) & 1
    i1, i2 = (off >> 23) & 1, (off >> 22) & 1
    j1, j2 = (~(i1 ^ s)) & 1, (~(i2 ^ s)) & 1
    imm10, imm11 = (off >> 12) & 0x3FF, (off >> 1) & 0x7FF
    return struct.pack("<HH", 0xF000 | (s << 10) | imm10, 0xD000 | (j1 << 13) | (j2 << 11) | imm11)


read_fifo, rf_size = symbol(b"_ZN30AP_InertialSensor_Invensensev39read_fifoEv")
block_read, _ = symbol(b"_ZN30AP_InertialSensor_Invensensev310block_readEhPhm")
panic, _ = symbol(b"_ZN6AP_HAL5panicEPKcz")
accumulate, _ = symbol(b"_ZN30AP_InertialSensor_Invensensev318accumulate_samplesEPK8FIFODatah")


def call_to(target):
    (a,) = [a for a in range(read_fifo, read_fifo + rf_size, 2)
            if bytes(data[file_off(a):file_off(a) + 4]) == bl(a, target)]
    return a


if where == "count":
    # After the call to block_read that reads FIFO_COUNTH: cmp r0,#0; beq; ldrh.w r3,[sp,#2];
    # cmp r3,#0; beq.
    call = call_to(block_read)
    site = call + 4
    expect = "0028 3bd0 bdf8 0230 002b 37d0"
else:
    # After the FIFO data transfer (`blx r3`, the device's transfer): cbz r0; mov r2,sl;
    # mov r1,r8; mov r0,r4; then the bl to accumulate_samples.
    site = call_to(accumulate) - 8
    call = site - 2
    expect = "88b1 5246 4146 2046"
    assert bytes(data[file_off(call):file_off(call) + 2]) == bytes.fromhex("9847"), "no blx r3 before the site"
expect = bytes.fromhex(expect.replace(" ", ""))
assert bytes(data[file_off(site):file_off(site) + len(expect)]) == expect, "the site is not the pinned build's"
# The "\n" that panic() itself prints after each message (its literal pool).
if variant == "panic":
    newline = 0x0810C5C3
    assert bytes(data[file_off(newline):file_off(newline) + 2]) == b"\n\x00", "the newline literal moved"
    patch = mov32(0, newline) + bl(site + 8, panic)
elif variant == "udf":
    patch = struct.pack("<H", 0xDE00)
elif variant == "halt":
    patch = struct.pack("<HH", 0xB672, 0xE7FE)
elif variant == "reset":
    patch = mov32(0, 0xE000ED0C) + mov32(1, 0x05FA0004) + struct.pack("<HH", 0x6001, 0xE7FE)
else:
    sys.exit(f"unknown variant {variant}")
at = file_off(site)
data[at:at + len(patch)] = patch
open(dst, "wb").write(data)
print(f"ANSWER 2314: plant {variant} at {site:#010x} ({len(patch)} bytes) after the {where} "
      f"call at {call:#010x}; panic {panic:#010x}")
