#!/usr/bin/env python3
"""AET-WO-1610 §1.1, D6: what serde says, and where, for (a) an enum left `{}`,
(b) the same enum given `{"": null}`, (c) a unit enum's string replaced by `{}`.

Usage: python3 -I spike/1610/f1_mechanism.py --bin target/debug/aethelgard
"""

import argparse
import json
import subprocess


def validate(binary, text):
    r = subprocess.run([binary, "validate"], input=text.encode(), capture_output=True)
    return r.returncode, json.loads(r.stdout)["error"]


def at(text, message):
    """The line and column serde names, and the text there."""
    words = message.split()
    line, col = int(words[-3]), int(words[-1])
    row = text.split("\n")[line - 1]
    return f"line {line} col {col}: {row!r} -> char {row[col - 1:col]!r}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    a = ap.parse_args()
    base = json.load(open("examples/sentinel-02-bench-hse8.json"))
    cases = []
    d = json.loads(json.dumps(base))
    d["mcu"]["hse"] = {}
    cases.append(("(a) mcu.hse = {}", d))
    d = json.loads(json.dumps(base))
    d["mcu"]["hse"] = {"": None}
    cases.append(("(b) mcu.hse = {\"\": null}", d))
    d = json.loads(json.dumps(base))
    d["mcu"]["core_supply"]["mode"] = {}
    cases.append(("(c) mcu.core_supply.mode (a unit enum) = {}", d))
    d = json.loads(json.dumps(base))
    d["spi_devices"][0]["orientation"]["x"] = {}
    cases.append(("(c) spi_devices[0].orientation.x (a unit enum) = {}", d))
    for name, doc in cases:
        text = json.dumps(doc, indent=2)
        code, e = validate(a.bin, text)
        print(f"## {name}: exit {code} {e['kind']}: {e['message']}")
        if " line " in e["message"]:
            print("   ", at(text, e["message"]))


if __name__ == "__main__":
    main()
