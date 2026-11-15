#!/usr/bin/env python3
"""AET-WO-4210 spike: build one probe firmware inside the pinned image (under img.sh).

The base is the firmware crate `aet-4210-gen` emitted for `spi-imu.json` (U2 BME280 on I2C1,
U4 W25Q64JV and U5 ICM-20948 on SPI1 behind the multiplexer), copied to /cache/fw/<variant>
with the variant's text edits, and built offline and locked into its own target directory, as
`twin_tier2.rs` `mutant_of()` does. Every variant keeps the three identity reads (records 0-2)
and adds its script steps after them, through the board crate's own `transfer` (a write-only
step passes an empty read buffer). Prints the variant, the ELF's path and its sha256.

NOT RUN: the container that would run it was denied by the permission classifier (FEEDBACK).

Usage: fw.py <variant>
"""
import hashlib, os, shutil, subprocess, sys

BOARD = "/cache/boards/spi-imu"  # aet-4210-gen's output, ~/.cache/aet-4210/boards/spi-imu
FILES = ["Cargo.toml", "Cargo.lock", ".cargo/config.toml", "link.x", "src/main.rs"]
AFTER = "    record(2, status, value);\n"
GPIOA_BSRR = 0x5802_0018


def spi(rec, dev, write, read=0):
    """One step on SPI1: `write`, then `read` bytes clocked in; the record holds the status
    and the first four bytes read (or the bytes written, for a write-only step)."""
    w = ", ".join(f"{b:#04x}" for b in write)
    if read == 0:
        word = int.from_bytes(bytes(write[:4]), "big")
        return (f"    let ok = spi1.transfer(&mut r, Spi1Device::{dev}, &[{w}], &mut []).is_ok();\n"
                f"    record({rec}, if ok {{ STATUS_OK }} else {{ STATUS_RXP }}, {word:#x});\n")
    return (f"    let mut rx{rec} = [0u8; {read}];\n"
            f"    let ok = spi1.transfer(&mut r, Spi1Device::{dev}, &[{w}], &mut rx{rec}).is_ok();\n"
            f"    record({rec}, if ok {{ STATUS_OK }} else {{ STATUS_RXP }}, "
            f"rx{rec}.iter().take(4).fold(0u32, |v, &b| (v << 8) | u32::from(b)));\n")


def store(rec, addr, val, claim):
    """A plain store with no transfer, then a record claiming `claim`: a forgery."""
    return (f"    unsafe {{ core::ptr::write_volatile({addr:#x} as *mut u32, {val:#x}); }}\n"
            f"    record({rec}, STATUS_OK, {claim:#x});\n")


# ICM-20948, bank 0 at reset (ICM20948.cs L214: bank 0 unless REG_BANK_SEL moved it).
WHO = [0x80]                      # WHO_AM_I (0x00) with the read bit
PWR1 = [0x06, 0x01]               # PWR_MGMT_1 = CLKSEL 1 (auto), SLEEP 0: the configuration write
ACCEL = [0x2D | 0x80]             # ACCEL_XOUT_H with the read bit: the data read, 6 bytes
INT_EN1 = [0x11, 0x01]            # INT_ENABLE_1 RAW_DATA_0_RDY_EN
INT_ST1 = [0x1A, 0x01]            # INT_STATUS_1 RAW_DATA_0_RDY_INT, written by firmware (a store over SPI)

def i2c(rec, addr, write, read=0):
    """One step on I2C1 through the board crate's `transfer`: `write`, then `read` bytes."""
    w = ", ".join(f"{b:#04x}" for b in write)
    word = int.from_bytes(bytes(write[:4]), "big")
    buf = f"rx{rec}" if read else "[]"
    pre = f"    let mut rx{rec} = [0u8; {read}];\n" if read else ""
    val = (f"rx{rec}.iter().take(4).fold(0u32, |v, &b| (v << 8) | u32::from(b))" if read else f"{word:#x}")
    return (pre + f"    let ok = i2c1.transfer(&mut r, {addr:#04x}, &[{w}], &mut {buf}).is_ok();\n"
            f"    record({rec}, if ok {{ STATUS_OK }} else {{ STATUS_STOP }}, {val});\n")


VARIANTS = {
    # P6.3: BMP180 at 0x77 (plat.py `baro-bmp180`): identity, a temperature conversion, the result.
    "baro-bmp180": (6, i2c(3, 0x77, [0xD0], 1) + i2c(4, 0x77, [0xF4, 0x2E]) + i2c(5, 0x77, [0xF6], 2)),
    # P6.1: LPS25HB at 0x5D (ArduPilot's address): WHO_AM_I, CTRL_REG1 power-up, PRESS_OUT (auto-increment bit).
    "baro-lps25hb": (6, i2c(3, 0x5D, [0x0F], 1) + i2c(4, 0x5D, [0x20, 0x90]) + i2c(5, 0x5D, [0xA8], 3)),
    # P6.2: ICP-101xx at 0x63: the ID command and its 3 bytes, a measurement command, a 9-byte read.
    "baro-icp": (6, i2c(3, 0x63, [0xEF, 0xC8], 3) + i2c(4, 0x63, [0x48, 0xA3])
                 + "    let mut rx5 = [0u8; 9];\n    let ok = i2c1.transfer(&mut r, 0x63, &[], &mut rx5).is_ok();\n"
                 "    record(5, if ok { STATUS_OK } else { STATUS_STOP }, rx5.iter().take(4).fold(0u32, |v, &b| (v << 8) | u32::from(b)));\n"),
    # S2: identity (record 2), the configuration write, the data read.
    "imu-script": (5, spi(3, "U5", PWR1) + spi(4, "U5", ACCEL, 6)),
    # S2 forgery: the configuration step claimed, with only the chip select toggled (PA1 low,
    # then high): the multiplexer's GPIO lines, no transfer.
    "imu-forge-config": (5, store(3, GPIOA_BSRR, 1 << 17, 0x0601) + store(4, GPIOA_BSRR, 1 << 1, 0)),
    # S2 forgery: the data step claimed with no transfer at all.
    "imu-forge-data": (5, spi(3, "U5", PWR1) + "    record(4, STATUS_OK, 0);\n"),
    # S2/F16: interleaved steps on two devices of one controller, then U4's identity again.
    "two-windows": (7, spi(3, "U5", PWR1) + spi(4, "U4", [0x9F], 3) + spi(5, "U5", ACCEL, 6)
                    + spi(6, "U4", [0x9F], 3)),
    # S3: the data-ready enable, then a status read after the data read. Does the model raise
    # its IRQ on its own?
    "imu-drdy": (6, spi(3, "U5", INT_EN1) + spi(4, "U5", ACCEL, 6) + spi(5, "U5", [0x1A | 0x80], 1)),
    # S3: as imu-drdy, plus the firmware writing the status bit itself.
    "imu-drdy-store": (6, spi(3, "U5", INT_EN1) + spi(4, "U5", INT_ST1) + spi(5, "U5", [0x1A | 0x80], 1)),
}


def main():
    name = sys.argv[1]
    records, steps = VARIANTS[name]
    src, dst = f"{BOARD}/firmware", f"/cache/fw/{name}"
    shutil.rmtree(dst, ignore_errors=True)
    for rel in FILES:
        os.makedirs(os.path.dirname(f"{dst}/{rel}"), exist_ok=True)
        shutil.copy(f"{src}/{rel}", f"{dst}/{rel}")

    def edit(rel, a, b):
        p = f"{dst}/{rel}"
        t = open(p).read()
        if t.count(a) != 1:
            sys.exit(f"{name}: {rel} holds `{a}` {t.count(a)} times, not once")
        open(p, "w").write(t.replace(a, b))

    edit("Cargo.toml", 'path = ".."', f'path = "{BOARD}"')
    t = open(f"{dst}/.cargo/config.toml").read()
    (old,) = [l for l in t.splitlines() if l.startswith("target-dir")]
    edit(".cargo/config.toml", old, f'target-dir = "{dst}/target"')
    edit("src/main.rs", "static mut AETHELGARD_BOOT: [u32; 7] = [0; 7];",
         f"static mut AETHELGARD_BOOT: [u32; {1 + 2 * records}] = [0; {1 + 2 * records}];")
    edit("src/main.rs", AFTER, AFTER + steps)
    env = dict(os.environ)
    env.pop("CARGO_TARGET_DIR", None)
    p = subprocess.run(["cargo", "build", "--release", "--offline", "--locked", "-q"], cwd=dst, env=env,
                       stdin=subprocess.DEVNULL, capture_output=True, text=True)
    if p.returncode != 0:
        sys.exit(f"{name}: build failed\n{p.stderr[-3000:]}")
    elf = f"{dst}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
    print(name, elf, hashlib.sha256(open(elf, "rb").read()).hexdigest())


if __name__ == "__main__":
    main()
