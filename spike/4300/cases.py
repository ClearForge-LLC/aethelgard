#!/usr/bin/env python3
"""AET-WO-4300 spike: the run cases, written to spike/4300/cases/<name>.json for drive.py.

Rows that set `timestamps` to Virtual are a labelled deviation from the CLI's composition
(which logs host time only); everything else in the composition is unchanged.

Usage: cases.py   (writes next to itself)
"""
import json, os

HERE = os.path.dirname(os.path.abspath(__file__))
ELF = "/cache/fw/{}/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware"
PLAT = "/repo/spike/4300/platforms/{}.repl"
T = 0x40010000
U = 0x40004400


def rd(label, a):
    return [label, f"sysbus ReadDoubleWord {a:#010x}"]


TIM_READS = [rd("tim1_cr1", T), rd("tim1_ccmr1", T + 0x18), rd("tim1_ccer", T + 0x20), rd("tim1_psc", T + 0x28),
             rd("tim1_arr", T + 0x2C), rd("tim1_ccr1", T + 0x34), rd("tim1_bdtr", T + 0x44), rd("tim1_dmar", T + 0x4C),
             rd("gpioa_moder", 0x58020000), rd("gpioa_afrh", 0x58020024), rd("rcc_apb2enr", 0x580244F0),
             ["led", "sysbus.gpioPortA.pa8led State"]]
UART_READS = [rd("u2_cr1", U), rd("u2_cr2", U + 4), rd("u2_brr", U + 0x0C), rd("u2_isr", U + 0x1C),
              rd("rcc_apb1lenr", 0x580244E8)]
INJECT = [f"sysbus.usart2 WriteChar {b:#04x}" for b in (0x41, 0x00, 0xFE, 0x0A)]


def timer(name, fw, ts="Virtual", for_ns=100_000_000):
    return {"name": name, "platform": PLAT.format("obs"), "elf": ELF.format(fw), "records": 3, "for_ns": for_ns,
            "timestamps": ts, "levels": ["sysbus.gpioPortA.pa8led", "sysbus.timer1"],
            "grep": r"pa8led|timer1", "keep_all": True, "extra": TIM_READS}


def uart(name, fw):
    return {"name": name, "platform": PLAT.format("none"), "elf": ELF.format(fw), "records": 3, "for_ns": 200_000_000,
            "timestamps": "Virtual", "levels": ["sysbus.usart2"], "grep": r"usart2",
            "pre": [f"sysbus.usart2 CreateFileBackend @/cache/raw/{name}.bin true"],
            "mid_at_ns": 20_000_000, "mid": INJECT, "extra": UART_READS}


def cccsr(name):
    return {"name": name, "platform": PLAT.format("none"), "elf": ELF.format(name), "records": 3,
            "for_ns": 100_000_000, "levels": ["sysbus.syscfg"], "grep": r"syscfg",
            "extra": [rd("cccsr", 0x58000420), rd("rcc_apb4enr", 0x580244F4)]}


CASES = [
    timer("pwm50-host", "pwm50", ts="Host"),
    timer("pwm50", "pwm50"),
    timer("pwm400", "pwm400"),
    timer("nomoe", "nomoe"),
    timer("cc1p", "cc1p"),
    timer("nogate", "nogate"),
    timer("claim", "claim"),
    timer("cen-off", "cen-off"),
    timer("bitbang-af", "bitbang-af"),
    timer("pwm600k", "pwm600k", for_ns=20_000_000),
    uart("uart", "uart"),
    uart("uart-sbus", "uart-sbus"),
    uart("uart-nogate", "uart-nogate"),
    cccsr("cccsr-bounded"),
    cccsr("cccsr-wait"),
]


def main():
    os.makedirs(os.path.join(HERE, "cases"), exist_ok=True)
    for c in CASES:
        with open(os.path.join(HERE, "cases", f"{c['name']}.json"), "w") as f:
            json.dump(c, f, indent=1)


if __name__ == "__main__":
    main()
