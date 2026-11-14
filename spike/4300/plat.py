#!/usr/bin/env python3
"""AET-WO-4200 spike: the platform variants, each the committed reference description
(`boards/sentinel-01/sentinel-01.repl`, which already declares `spi1` with `spi4`'s model) plus
what the variant attaches to `spi1`. Written to spike/4200/platforms/<name>.repl.

- `none`: the reference unchanged (nothing behind spi1).
- `direct`: U4 as `SPI.GenericSpiFlash` straight on spi1 (W25Q64JV's JEDEC ID EF 40 17; 8 MiB).
- `direct-cs`: `direct` plus PA0 wired to the flash's GPIO input 0 (its chip-select deassert).
- `mux`: `SPI.SPIMultiplexer` on spi1, inputs 0 and 1 active-low, PA0 -> input 0, PA1 -> input 1;
  U4 at mux address 0 and an `ICM20948` (`imu`) at address 1.
- `mux-absent`: `mux` without U4 (the mux, its wiring and the IMU kept).

Usage: plat.py   (run from anywhere; writes next to itself)
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
REF = os.path.join(HERE, "..", "..", "boards", "sentinel-01", "sentinel-01.repl")

MEM = """
// AET-WO-4200 spike: U4's 8 MiB array (W25Q64JV, 64 Mbit), not on sysbus.
u4_memory: Memory.MappedMemory
    size: 0x800000
"""


def flash(at):
    return f"""
// AET-WO-4200 spike: U4 as Renode's GenericSpiFlash with the W25Q64JV's JEDEC ID.
u4: SPI.GenericSpiFlash @ {at}
    underlyingMemory: u4_memory
    manufacturerId: 0xEF
    memoryType: 0x40
    capacityCode: 0x17
"""


MUX = """
// AET-WO-4200 spike: a chip-select multiplexer on spi1; inputs 0 and 1 are active-low.
spi1mux: SPI.SPIMultiplexer @ spi1
    init:
        SetActiveLow 0
        SetActiveLow 1

gpioPortA:
    0 -> spi1mux@0
    1 -> spi1mux@1
"""

IMU = """
// AET-WO-4200 spike: an ICM-20948 (Renode's only SPI IMU besides LSM6DSO) at mux address 1.
imu: Sensors.ICM20948 @ spi1mux 1
"""

VARIANTS = {
    "none": "",
    "direct": MEM + flash("spi1"),
    "direct-cs": MEM + flash("spi1") + "\ngpioPortA:\n    0 -> u4@0\n",
    "mux": MUX + MEM + flash("spi1mux 0") + IMU,
    "mux-absent": MUX + IMU,
}


def main():
    ref = open(REF).read()
    os.makedirs(os.path.join(HERE, "platforms"), exist_ok=True)
    for name, extra in VARIANTS.items():
        with open(os.path.join(HERE, "platforms", f"{name}.repl"), "w") as f:
            f.write(ref + extra)


if __name__ == "__main__":
    main()
