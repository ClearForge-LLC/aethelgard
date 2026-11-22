"""AET-WO-4300h: the solver's pin table, read independently from the open pin
data `aethelgard-svd-hal` embeds (STM32H753ZITx.xml and the GPIO modes file).

It follows the crate's reading (`svd-hal/src/lib.rs` `parse`) without linking
it: I/O pins of the package, canonical name `P[A-K][0-9]{1,2}`, a signal's AF
from the GPIO file's `GPIO_AF<n>_…` values, kept only where the package's own
<Signal> list carries the signal.

usage: python3 -I pintable.py <st-open-pin-data dir> <solve dir>
       -> out/pintable.tsv (pin, position, signal, af); stdout: the check that every
          committed board's solved (pin, signal, af) is in it
"""
import glob
import json
import os
import re
import sys
import xml.etree.ElementTree as ET


def local(tag):
    return tag.split("}", 1)[-1]


def main(data, solve_dir):
    mcu = ET.parse(os.path.join(data, "STM32H753ZITx.xml")).getroot()
    gpio = ET.parse(os.path.join(data, "GPIO-STM32H747_gpio_v1_0_Modes.xml")).getroot()
    gpins = {p.get("Name"): p for p in gpio if local(p.tag) == "GPIO_Pin"}
    rows = []
    for p in mcu:
        if local(p.tag) != "Pin" or p.get("Type") != "I/O":
            continue
        full = p.get("Name")
        m = re.match(r"P[A-K][0-9]{1,2}(?![0-9])", full)
        name = m.group(0)
        sigs = {s.get("Name") for s in p if local(s.tag) == "Signal"}
        for ps in gpins[full]:
            if local(ps.tag) != "PinSignal" or ps.get("Name") not in sigs:
                continue
            for sp in ps:
                if local(sp.tag) == "SpecificParameter" and sp.get("Name") == "GPIO_AF":
                    for v in sp:
                        af = re.match(r"GPIO_AF(\d+)_", v.text or "")
                        rows.append((name, p.get("Position"), ps.get("Name"), int(af.group(1))))
    rows = sorted(set(rows))
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "out/pintable.tsv"), "w") as f:
        f.write("pin\tposition\tsignal\taf\n")
        for r in rows:
            f.write("\t".join(map(str, r)) + "\n")
    have = {(r[0], r[2], r[3]) for r in rows}
    n = bad = 0
    for sj in sorted(glob.glob(os.path.join(solve_dir, "*.json"))):
        for v in json.load(open(sj)).get("assignments", {}).values():
            n += 1
            if (v["pin"], v["signal"], v["alternate_function"]) not in have:
                bad += 1
                print("MISSING", os.path.basename(sj), v)
    print(f"{len(rows)} (pin, signal, af) rows; {n - bad}/{n} solved assignments found in them")


if __name__ == "__main__":
    main(*sys.argv[1:3])
