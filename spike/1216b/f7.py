"""AET-WO-1216b F7: walk 2's rows on a drawn pin device, classified by the pin
row's kind rather than the part's rails (1216 FEEDBACK F7).

A walk-2 row whose peer is a pin device (an `ic` row) touches every rail the
part does, so drawn.py flags it off-domain by rail. This reads the device
pin's kind from parts.json `pin_parts` by the pad number and keeps it as a
path only when the kind can drive the pin: an input (`logic_input`,
`enable_input`) or an open drain (which only sinks) is not a source.

usage: python3 -I f7.py <drawn.tsv> <spec.json> <parts.json> -> TSV on stdout
columns: net pad device_pin kind verdict
"""
import json
import sys

NOT_A_SOURCE = {"logic_input", "enable_input", "open_drain_output"}


def main(drawn, spec_path, parts_path):
    spec = json.load(open(spec_path))
    rows = {p["part"]: p for p in json.load(open(parts_path))["pin_parts"]}
    devices = {d["reference"]: d for d in spec.get("pin_devices", [])}
    print("\t".join(["net", "pad", "device_pin", "kind", "verdict"]))
    for line in open(drawn):
        f = line.rstrip("\n").split("\t")
        net, ref, cls, pad, flag = f[3].lstrip("/"), f[4], f[5], f[6], f[10]
        if cls != "ic" or ref not in devices or flag == "no":
            continue
        pins = rows[devices[ref]["part"]]["pins"]
        name, kind = next(((n, p["kind"]) for n, p in pins.items() if str(p["number"]) == pad),
                          ("?", "unheld"))
        verdict = "not a source" if kind in NOT_A_SOURCE else "path"
        print("\t".join([net, f"{ref}.{pad}", name, kind, verdict]))


if __name__ == "__main__":
    main(*sys.argv[1:4])
