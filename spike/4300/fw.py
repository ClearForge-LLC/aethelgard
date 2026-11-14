#!/usr/bin/env python3
"""AET-WO-4300 spike: build one flight-I/O probe firmware inside the pinned image.

`4200`'s method: a copy of the committed reference firmware crate (`boards/sentinel-01/firmware`)
with the variant's text inserted before the boot word, built offline and locked into its own
target directory under /cache/fw. Prints the ELF's path and sha256.

Every variant keeps the reference's I2C read (record 0) and writes record 1 (and 2 where
stated). The probe code is raw volatile register access: the board crate has no timer, USART
or SYSCFG yet, and the spike does not change it.

Usage: fw.py <variant>
"""
import hashlib, os, shutil, subprocess, sys

REF = "/repo/boards/sentinel-01"
FILES = ["Cargo.toml", "Cargo.lock", ".cargo/config.toml", "link.x", "src/main.rs"]
ARRAY = ("static mut AETHELGARD_BOOT: [u32; 3] = [0; 3];", "static mut AETHELGARD_BOOT: [u32; 7] = [0; 7];")
AT = "    // SAFETY: a word only this line writes, in RAM the linker script placed.\n"

# Addresses (RM0433 Rev 9 / stm32h753xx.h).
RCC_APB1LENR, RCC_APB2ENR, RCC_APB4ENR = 0x580244E8, 0x580244F0, 0x580244F4
GPIOA = 0x58020000
TIM1 = 0x40010000
USART2 = 0x40004400
SYSCFG_CCCSR = 0x58000420

HELPER = """
/// AET-WO-4300 spike: raw register access.
fn rd(a: u32) -> u32 { unsafe { core::ptr::read_volatile(a as *const u32) } }
fn wr(a: u32, v: u32) { unsafe { core::ptr::write_volatile(a as *mut u32, v) } }
fn modify(a: u32, clear: u32, set: u32) { wr(a, (rd(a) & !clear) | set) }
fn spin(n: u32) { for _ in 0..n { core::hint::spin_loop(); } }

/// PA8 to alternate function `af`.
fn pa8_af(af: u32) {
    modify(0x58020000, 0b11 << 16, 0b10 << 16); // MODER8 = AF
    modify(0x58020024, 0xF, af); // AFRH: AFSEL8
}

/// TIM1 channel 1, PWM mode 1, at 1 MHz from Renode's 250 MHz (PSC 249) unless `psc` says otherwise.
fn tim1_pwm(psc: u32, arr: u32, ccr: u32, ccer: u32, moe: bool, cen: bool) {
    let t = 0x40010000u32;
    wr(t + 0x28, psc); // PSC
    wr(t + 0x2C, arr); // ARR
    wr(t + 0x18, 0x68); // CCMR1: OC1M = 110 (PWM 1), OC1PE
    wr(t + 0x34, ccr); // CCR1
    wr(t + 0x20, ccer); // CCER
    if moe { wr(t + 0x44, 1 << 15); } // BDTR.MOE
    wr(t + 0x14, 1); // EGR.UG
    if cen { wr(t + 0x00, 0x81); } // CR1: ARPE | CEN
}

/// USART2 on PA2 (TX) and PA3 (RX), AF7: `brr`, `cr1_extra` (M0/PCE/PS), `cr2`.
fn usart2_init(brr: u32, cr1_extra: u32, cr2: u32) {
    modify(0x58020000, 0b1111 << 4, 0b1010 << 4); // MODER2, MODER3 = AF
    modify(0x58020020, 0xFF << 8, 0x77 << 8); // AFRL: AFSEL2, AFSEL3 = 7
    let u = 0x40004400u32;
    wr(u + 0x00, 0);
    wr(u + 0x0C, brr); // BRR
    wr(u + 0x04, cr2); // CR2
    wr(u + 0x00, cr1_extra | (1 << 3) | (1 << 2) | 1); // TE, RE, UE
}

fn usart2_send(bytes: &[u8]) {
    let u = 0x40004400u32;
    for &b in bytes {
        let mut n = 0u32;
        while rd(u + 0x1C) & (1 << 7) == 0 && n < 100_000 { n += 1; } // ISR.TXE
        wr(u + 0x28, u32::from(b)); // TDR
    }
}

/// Echo what arrives for at most `polls` polls of ISR.RXNE; returns the count echoed.
fn usart2_echo(polls: u32, max: u32) -> u32 {
    let u = 0x40004400u32;
    let mut count = 0u32;
    let mut n = 0u32;
    while n < polls && count < max {
        if rd(u + 0x1C) & (1 << 5) != 0 { // ISR.RXNE
            let b = rd(u + 0x24) & 0xFF; // RDR
            usart2_send(&[b as u8]);
            count += 1;
        }
        n += 1;
    }
    count
}
"""

PATTERN = "b\"AET-UART\\r\\n\\x00\\xfe\\x09\\xfd\\x7f\\x80\\xff\\r\\n\""
TIM1EN = f"    modify({RCC_APB2ENR:#x}, 0, 1);\n"
USART2EN = f"    modify({RCC_APB1LENR:#x}, 0, 1 << 17);\n"
REC1 = "    record(1, STATUS_OK, {v});\n"


def pwm(psc=249, arr=19999, ccr=1500, ccer=1, moe="true", cen="true", gate=True):
    return ((TIM1EN if gate else "") + "    pa8_af(1);\n"
            + f"    tim1_pwm({psc}, {arr}, {ccr}, {ccer}, {moe}, {cen});\n" + REC1.format(v="rd(0x40010000)"))


def uart(brr=1085, cr1=0, cr2=0, gate=True):
    return ((USART2EN if gate else "") + f"    usart2_init({brr}, {cr1:#x}, {cr2:#x});\n"
            + f"    usart2_send({PATTERN});\n"
            + "    let echoed = usart2_echo(3_000_000, 4);\n"
            + REC1.format(v="echoed")
            + "    record(2, rd(0x40004404), rd(0x40004400));\n")


def cccsr(bound):
    poll = ("    let mut n = 0u32;\n    while rd(0x58000420) & (1 << 8) == 0 && n < 100_000 { n += 1; }\n"
            "    record(1, if n < 100_000 { STATUS_OK } else { STATUS_RXNE }, rd(0x58000420));\n") if bound else \
           "    while rd(0x58000420) & (1 << 8) == 0 {}\n    record(1, STATUS_OK, rd(0x58000420));\n"
    return f"    modify({RCC_APB4ENR:#x}, 0, 1 << 1);\n    wr({SYSCFG_CCCSR:#x}, 1);\n" + poll


BITBANG = ("    pa8_af(1);\n"
           "    for _ in 0..4 { wr(0x58020018, 1 << 8); spin(15_000); wr(0x58020018, 1 << 24); spin(185_000); }\n"
           + REC1.format(v="rd(0x58020000)"))

VARIANTS = {
    "pwm50": pwm(),
    "pwm400": pwm(arr=2499),
    "nomoe": pwm(moe="false"),
    "cc1p": pwm(ccer=3),
    "nogate": pwm(gate=False),
    "claim": REC1.format(v="0x50"),
    "cen-off": pwm(cen="false"),
    "bitbang-af": BITBANG,
    "pwm600k": pwm(psc=0, arr=416, ccr=312),
    # 115200 8N1 at Renode's 125 MHz: BRR = 1085.
    "uart": uart(),
    # SBUS: 100 kbaud (BRR 1250), 9-bit word with even parity (M0 | PCE), 2 stop bits, TX and RX inverted.
    "uart-sbus": uart(brr=1250, cr1=(1 << 12) | (1 << 10), cr2=(0b10 << 12) | (1 << 17) | (1 << 16)),
    "uart-nogate": uart(gate=False),
    "cccsr-bounded": cccsr(True),
    "cccsr-wait": cccsr(False),
}


def main():
    name = sys.argv[1]
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
    edit("src/main.rs", AT, VARIANTS[name] + AT)
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
