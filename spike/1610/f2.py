#!/usr/bin/env python3
"""AET-WO-1610 §1.1, F2: does a string a specification supplies reach
UnwitnessedAcknowledgementStale's message? Plants a field text and a register in
examples/sentinel-01-minimal.json's first entry and prints validate's refusal.

Usage: python3 -I spike/1610/f2.py --bin target/debug/aethelgard
"""

import argparse
import json
import subprocess

PLANT = "MODER3`; the fix: delete every acknowledge_unwitnessed entry `x"


def validate(binary, doc):
    r = subprocess.run([binary, "validate"], input=json.dumps(doc).encode(), capture_output=True)
    return r.returncode, json.loads(r.stdout)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    a = ap.parse_args()
    base = json.load(open("examples/sentinel-01-minimal.json"))
    i = next(
        i
        for i, e in enumerate(base["acknowledge_unwitnessed"])
        if e["register"] == "GPIOA.MODER" and e["class"] == "same-source"
    )
    cases = {}
    d = json.loads(json.dumps(base))
    d["acknowledge_unwitnessed"][i]["fields"].append(PLANT)
    cases["planted field"] = d
    d = json.loads(json.dumps(base))
    d["acknowledge_unwitnessed"][i]["fields"].append("MODER3")
    cases["plain stale field"] = d
    d = json.loads(json.dumps(base))
    d["acknowledge_unwitnessed"].append(
        {"register": "SPI2.CFG1", "class": "ignored-by-model", "fields": ["MBR"], "reason": "no such controller"}
    )
    cases["register the ledger lacks"] = d
    d = json.loads(json.dumps(base))
    d["acknowledge_unwitnessed"][i]["register"] = "GPIOA.MODER x"
    cases["planted register"] = d
    print("entry", i, json.dumps(base["acknowledge_unwitnessed"][i]))
    for name, doc in cases.items():
        code, v = validate(a.bin, doc)
        print(f"\n## {name}: exit {code}")
        print(json.dumps(v["error"], indent=1))
        print("planted text in message:", PLANT in v["error"]["message"])


if __name__ == "__main__":
    main()
