#!/usr/bin/env python3
"""AET-WO-4200 spike: build one SPI probe firmware inside the pinned image.

A copy of the committed reference firmware crate (`boards/sentinel-01/firmware`), with the
named variant's text edits, built offline and locked into its own target directory under
/cache/fw, as `twin_tier2.rs` `mutant()` does (2204's fw.py). Prints the ELF's path and sha256.

Every variant keeps the reference's I2C read (record 0) and adds a second record (record 1)
after it, so the boot block is the boot word plus two records. The SPI code is raw volatile
register access to SPI1 (0x4001_3000) and GPIOA (0x5802_0000): the board crate has no SPI
transfer yet, and the spike does not change it.

Usage: fw.py <variant>
"""
import hashlib, os, shutil, subprocess, sys

REF = "/repo/boards/sentinel-01"
FILES = ["Cargo.toml", "Cargo.lock", ".cargo/config.toml", "link.x", "src/main.rs"]
W = "core::ptr::write_volatile"
R = "core::ptr::read_volatile"
SPI1 = 0x4001_3000
GPIOA_BSRR = 0x5802_0018
CS_LOW, CS_HIGH = 1 << 16, 1

ARRAY = ("static mut AETHELGARD_BOOT: [u32; 3] = [0; 3];", "static mut AETHELGARD_BOOT: [u32; 7] = [0; 7];")
AT = "    // SAFETY: a word only this line writes, in RAM the linker script placed.\n"


def w(addr, val):
    return f"{W}({addr:#x} as *mut u32, {val:#x});"


def spi(off):
    return SPI1 + off


# A helper the variants call: one full-duplex transfer of `tx` through SPI1, TSIZE = len(tx),
# 8-bit frames, master. Each RXDR read waits on SR.RXP (bit 0) for at most 20_000 polls.
# Returns the bytes clocked in, or None when RXP never rose.
HELPER = """
/// AET-WO-4200 spike: one SPI1 transfer, raw registers. `cs` is the GPIOA pin mask asserted
/// (driven low) around it; 0 asserts none.
fn spike_spi(tx: &[u8], rx: &mut [u8], cs: u32, cfg2: u32) -> bool {
    unsafe {
        let r = |a: u32| core::ptr::read_volatile(a as *const u32);
        let w = |a: u32, v: u32| core::ptr::write_volatile(a as *mut u32, v);
        w(0x40013000, 0); // CR1: SPE off, so CFG1/CFG2 accept writes
        w(0x40013008, 0x0000_0007); // CFG1: DSIZE = 7 (8-bit frames)
        w(0x4001300c, cfg2); // CFG2: MASTER and whatever the variant adds
        w(0x40013004, tx.len() as u32); // CR2: TSIZE
        if cs != 0 { w(0x58020018, cs << 16); } // the chip select pin(s) low: asserted
        w(0x40013000, 1); // SPE
        w(0x40013000, 1 | (1 << 9)); // CSTART
        let mut ok = true;
        for (i, &b) in tx.iter().enumerate() {
            core::ptr::write_volatile(0x40013020 as *mut u8, b);
            let mut n = 0u32;
            while r(0x40013014) & 1 == 0 && n < 20_000 { n += 1; }
            if n == 20_000 { ok = false; }
            rx[i] = core::ptr::read_volatile(0x40013030 as *const u8);
        }
        w(0x40013018, 1 << 3); // IFCR: EOTC
        w(0x40013000, 0); // SPE off
        if cs != 0 { w(0x58020018, cs); } // high again
        ok
    }
}
"""

MASTER = 1 << 22


def jedec(cmd=0x9F, cs=1, cfg2=MASTER):
    """The JEDEC ID read: the command, then three dummy bytes; the value is the three bytes
    clocked in after the command, first on the wire as the most significant."""
    return ("    let mut spi_rx = [0u8; 4];\n"
            f"    let spi_ok = spike_spi(&[{cmd:#04x}, 0, 0, 0], &mut spi_rx, {cs:#x}, {cfg2:#x});\n"
            "    let spi_value = (u32::from(spi_rx[1]) << 16) | (u32::from(spi_rx[2]) << 8) | u32::from(spi_rx[3]);\n"
            "    record(1, if spi_ok { STATUS_OK } else { STATUS_RXNE }, spi_value);\n")


CLAIM = "    record(1, STATUS_OK, 0x00ef4017);\n"

VARIANTS = {
    # The reference read, then the JEDEC read through SPI1 with PA0 as chip select.
    "jedec": [(AT, jedec() + AT)],
    # No SPI access at all: the record claims the JEDEC ID.
    "forge-claim": [(AT, CLAIM + AT)],
    # The helper's exact register sequence, chip select included, with no TXDR write and no
    # RXDR read, so it ends in the register state `jedec` ends in; then the claim.
    "forge-regs": [(AT, "    unsafe { " + w(spi(0x00), 0) + w(spi(0x08), 7) + w(spi(0x0C), MASTER) + w(spi(0x04), 4)
                    + w(GPIOA_BSRR, CS_LOW) + w(spi(0x00), 1) + w(spi(0x00), 1 | (1 << 9))
                    + w(spi(0x18), 1 << 3) + w(spi(0x00), 0) + w(GPIOA_BSRR, CS_HIGH) + " }\n" + CLAIM + AT)],
    # As forge-regs, plus CSUSP (CR1 bit 10) while SPE is set: EndTransfer with no transfer, which
    # sets EOT and calls the attached model's FinishTransmission.
    "forge-csusp": [(AT, "    unsafe { " + w(spi(0x00), 0) + w(spi(0x08), 7) + w(spi(0x0C), MASTER) + w(spi(0x04), 4)
                     + w(GPIOA_BSRR, CS_LOW) + w(spi(0x00), 1) + w(spi(0x00), 1 | (1 << 9)) + w(spi(0x00), 1 | (1 << 10))
                     + w(spi(0x18), 1 << 3) + w(spi(0x00), 0) + w(GPIOA_BSRR, CS_HIGH) + " }\n" + CLAIM + AT)],
    # A real transfer of the wrong command (0x90, Manufacturer/Device ID), then the claim.
    "wrong-cmd": [(AT, "    let mut spi_rx = [0u8; 4];\n    let _ = spike_spi(&[0x90, 0, 0, 0], &mut spi_rx, 1, 0x400000);\n"
                   + CLAIM + AT)],
    # The JEDEC read with chip select never asserted (PA0 left high).
    "cs-never": [(AT, jedec(cs=0) + AT)],
    # The JEDEC read with CPHA (CFG2 bit 24) and CPOL (bit 25) set: mode 3.
    "mode3": [(AT, jedec(cfg2=MASTER | (1 << 24) | (1 << 25)) + AT)],
    # Two devices on SPI1: the JEDEC read on PA0, then the ICM-20948's WHO_AM_I (bank 0, register
    # 0x00, read bit 7 set) on PA1, recorded as record 2.
    "two": [(AT, jedec() + "    let mut imu_rx = [0u8; 2];\n"
             "    let imu_ok = spike_spi(&[0x80, 0], &mut imu_rx, 2, 0x400000);\n"
             "    record(2, if imu_ok { STATUS_OK } else { STATUS_RXNE }, u32::from(imu_rx[1]));\n" + AT)],
}


# Pass 2 (FEEDBACK, "Corrections during the runs"): every chip select idles high before any
# SPI code, as the generator's `into_output` does for PA0. Pass 1 left PA1 undriven; Renode's
# `GPIO.Connect` signals the initial low at load, so the mux's active-low input 1 (the IMU)
# was selected throughout, and every byte met two selected devices.
IDLE = "    unsafe { " + w(GPIOA_BSRR, 0b11) + " }\n"


def main():
    name = sys.argv[1]
    edits = [(a, IDLE + b) for a, b in VARIANTS[name]]
    src = f"{REF}/firmware"
    dst = f"/cache/fw/{name}"
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

    edit("Cargo.toml", 'path = ".."', f'path = "{REF}"')
    t = open(f"{dst}/.cargo/config.toml").read()
    (old,) = [l for l in t.splitlines() if l.startswith("target-dir")]
    edit(".cargo/config.toml", old, f'target-dir = "{dst}/target"')
    edit("src/main.rs", *ARRAY)
    edit("src/main.rs", "#[panic_handler]", HELPER + "\n#[panic_handler]")
    for a, b in edits:
        edit("src/main.rs", a, b)
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
