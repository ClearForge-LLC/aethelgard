#!/usr/bin/env python3
"""AET-WO-2204 spike: build one probe firmware inside the pinned image.

A copy of a generated firmware crate (the committed reference, or the two-BME280
tree `generate` wrote under /cache/gen2), with the named variant's text edits,
built offline and locked into its own target directory under /cache/fw, as
`twin_tier2.rs` `mutant()` does. Prints the ELF's path and sha256.

Usage: fw.py <variant>
"""
import hashlib, os, shutil, subprocess, sys

REF = "/repo/boards/sentinel-01"
GEN2 = "/cache/gen2/board"
FILES = ["Cargo.toml", "Cargo.lock", ".cargo/config.toml", "link.x", "src/main.rs"]

TRANSFER = "i2c1.transfer(&mut r, 0x76, &[0xd0], &mut buf0)"
W = "core::ptr::write_volatile"
R = "core::ptr::read_volatile"
I2C1 = 0x4000_5400


def w(off, val):
    return f"{W}({I2C1 + off:#x} as *mut u32, {val:#x});"


# S1: every candidate register stored at its reference value, no transfer.
# STOP alone sets STOPF (StopTransfer); ICR then clears it, as the board does.
FORGE_ALL = ("{ unsafe { " + w(0x04, 0x4000) + w(0x04, 0x020104EC) + w(0x28, 0xD0) + w(0x18, 0x1)
             + w(0x24, 0x60) + w(0x1C, 0x30) + " } Ok::<usize, I2cError>(1) }")
WRONG = ("{ let _ = i2c1.transfer(&mut r, 0x77, &[0xd0], &mut buf0); buf0 = [0x60u8]; "
         "Ok::<usize, I2cError>(1) }")
WRONG_FORGE = ("{ let _ = i2c1.transfer(&mut r, 0x77, &[0xd0], &mut buf0); buf0 = [0x60u8]; unsafe { "
               + w(0x04, 0x020104EC) + w(0x28, 0xD0) + " } Ok::<usize, I2cError>(1) }")
# S4: 500 reads of an empty RXDR (one warning each), then one read of GPIOC.MODER,
# whose clock is off (one locked-access warning), just before the boot word.
BURST_AT = "    // SAFETY: a word only this line writes, in RAM the linker script placed.\n"
BURST = ("    for _ in 0..500 { unsafe { " + f"{R}({I2C1 + 0x24:#x} as *const u32);" + " } }\n"
         "    unsafe { " + f"{R}(0x5802_0800 as *const u32);" + " }\n")

# S4: after the boot word, two different warnings alternate to the end of the
# run (an empty RXDR read, then an ICR write of the untagged NACKCF bit), paced by
# a spin so the log stays far under the CLI's 8 MiB budget; no two consecutive
# lines are equal, so Renode cannot fold them.
FLOOD_AT = ("    // No sleep: the judge requires instructions executed to equal MIPS x time.\n"
            "    loop {\n        core::hint::spin_loop();\n    }\n")
def flood(spins):
    return ("    loop {\n        unsafe { " + f"{R}({I2C1 + 0x24:#x} as *const u32); {W}({I2C1 + 0x1C:#x} as *mut u32, 0x10);"
            + " }\n        for _ in 0..%d { core::hint::spin_loop(); }\n    }\n" % spins)

# B10 made direct: the flood of `flood-max`, with one read of GPIOC.MODER (clock
# off, so one locked-access warning) after the pair numbered LOCKED_AT, which
# is set from a measured run to land a few hundred lines before the run ends.
LOCKED_AT = int(os.environ.get("LOCKED_AT", "0"))
FLOOD_LOCKED = ("    let mut n: u32 = 0;\n    loop {\n        unsafe { " + f"{R}({I2C1 + 0x24:#x} as *const u32); {W}({I2C1 + 0x1C:#x} as *mut u32, 0x10);"
                + " }\n        if n == %d { unsafe { " % LOCKED_AT + f"{R}(0x5802_0800 as *const u32);" + " } }\n"
                "        n = n.wrapping_add(1);\n    }\n")

VARIANTS = {
    "flood-locked": (REF, [(FLOOD_AT, FLOOD_LOCKED)]),
    "flood": (REF, [(FLOOD_AT, flood(2000))]),
    # B10's case: unpaced, so lines are still queued when the run ends.
    "flood-fast": (REF, [(FLOOD_AT, flood(20))]),
    # Past the CLI's 8 MiB budget, as 2301's A2 fixture was (254 MB).
    "flood-max": (REF, [(FLOOD_AT, flood(0))]),
    "ref": (REF, []),
    "forge-regs": (REF, [("let mut buf0 = [0u8; 1];", "let mut buf0 = [0x60u8; 1];"),
                         (TRANSFER, "{ unsafe { " + w(0x04, 0x020104EC) + w(0x28, 0xD0)
                          + " } Ok::<usize, I2cError>(1) }")]),
    "forge-all": (REF, [("let mut buf0 = [0u8; 1];", "let mut buf0 = [0x60u8; 1];"),
                        (TRANSFER, FORGE_ALL)]),
    "wrong-addr": (REF, [(TRANSFER, WRONG)]),
    "wrong-addr-forge": (REF, [(TRANSFER, WRONG_FORGE)]),
    "burst": (REF, [(BURST_AT, BURST + BURST_AT)]),
    "two-ref": (GEN2, []),
    # U2's read replaced by a claim; U5 (the last on the bus) real.
    "two-forged-first": (GEN2, [("let mut buf0 = [0u8; 1];", "let mut buf0 = [0x60u8; 1];"),
                                (TRANSFER, "Ok::<usize, I2cError>(1)")]),
    # U2 real; U5's read replaced by a claim and its controller state hand-written.
    "two-forged-last": (GEN2, [("let mut buf1 = [0u8; 1];", "let mut buf1 = [0x60u8; 1];"),
                               ("i2c1.transfer(&mut r, 0x77, &[0xd0], &mut buf1)",
                                "{ unsafe { " + w(0x04, 0x020104EE) + w(0x28, 0xD0)
                                + " } Ok::<usize, I2cError>(1) }")]),
}


def main():
    name = sys.argv[1]
    base, edits = VARIANTS[name]
    src = f"{base}/firmware"
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

    edit("Cargo.toml", 'path = ".."', f'path = "{base}"')
    t = open(f"{dst}/.cargo/config.toml").read()
    (old,) = [l for l in t.splitlines() if l.startswith("target-dir")]
    edit(".cargo/config.toml", old, f'target-dir = "{dst}/target"')
    for a, b in edits:
        edit("src/main.rs", a, b)
    env = dict(os.environ)
    env.pop("CARGO_TARGET_DIR", None)
    p = subprocess.run(["cargo", "build", "--release", "--offline", "--locked", "-q"], cwd=dst, env=env,
                       stdin=subprocess.DEVNULL, capture_output=True, text=True)
    if p.returncode != 0:
        sys.exit(f"{name}: build failed\n{p.stderr[-3000:]}")
    elf = f"{dst}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
    print(elf, hashlib.sha256(open(elf, "rb").read()).hexdigest())


if __name__ == "__main__":
    main()
