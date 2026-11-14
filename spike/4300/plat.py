#!/usr/bin/env python3
"""AET-WO-4300 spike: the platform variants, each the committed reference description
(`boards/sentinel-01/sentinel-01.repl`) plus what the variant adds.

- `none`: the reference unchanged.
- `obs`: a `Miscellaneous.LED` (`pa8led`) observing PA8, TIM1_CH1 at AF1. The stock
  `stm32h7.repl` already routes `gpioPortA: 8 -> timer1@00`; the line below restates that
  route and adds the LED with the `|` form, so neither replaces the other.

Usage: plat.py   (writes next to itself)
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
REF = os.path.join(HERE, "..", "..", "boards", "sentinel-01", "sentinel-01.repl")

OBS = """
// AET-WO-4300 spike: an observer on PA8 (TIM1_CH1, AF1).
pa8led: Miscellaneous.LED @ gpioPortA 8

gpioPortA:
    8 -> timer1@00 | pa8led@0
"""

VARIANTS = {"none": "", "obs": OBS}


def main():
    ref = open(REF).read()
    os.makedirs(os.path.join(HERE, "platforms"), exist_ok=True)
    for name, extra in VARIANTS.items():
        with open(os.path.join(HERE, "platforms", f"{name}.repl"), "w") as f:
            f.write(ref + extra)


if __name__ == "__main__":
    main()
