"""AET-WO-1216: reconcile walk 1 (out/spec) with walk 2 (out/drawn), per drawn board.

Every drawn row that leaves the MCU's domain (a resistor or IC to another rail,
a connector, a test point, a hop through a series part) must match a spec row
on the same net; every spec row on a drawn board must match a drawn row,
except classes that are not drawn parts (esc, foreign_clock).
usage: python3 -I compare.py out/spec out/drawn -> out/compare.tsv on stdout
"""
import csv
import os
import sys

DRAWN_TO_SPEC = {"resistor": {"battery_divider", "i2c_pull", "pin_device_pull", "crossing_cs_pull"},
                 "connector": {"connector"}, "test_point": {"test_point"},
                 "ic": {"spi_device_output", "pin_device_output", "pin_device_current_output",
                        "crossing_a_side"}}
NOT_DRAWN = {"esc", "foreign_clock"}

def main(spec_dir, drawn_dir):
    print("\t".join(["board", "side", "net", "class", "detail", "matched"]))
    for f in sorted(os.listdir(drawn_dir)):
        n = f[:-4]
        spec = list(csv.DictReader(open(os.path.join(spec_dir, f)), delimiter="\t"))
        drawn = [r.rstrip("\n").split("\t") for r in open(os.path.join(drawn_dir, f))]
        snet = lambda net: net.lstrip("/")
        for r in drawn:
            pad, pin, struct, net, ref, cls, ppad, val, rails, hop, flag = r
            if flag == "no":
                continue
            want = DRAWN_TO_SPEC.get(cls, set())
            ok = any(s["net"] == snet(net) and s["class"] in want for s in spec)
            print("\t".join([n, "drawn", snet(net), cls, f"{ref}.{ppad} {val} rails={rails} hop={hop} flag={flag}",
                             "yes" if ok else "NO"]))
        for s in spec:
            if s["class"] in NOT_DRAWN or not s["can_lead"].startswith("yes"):
                continue
            ok = any(snet(r[3]) == s["net"] and s["class"] in DRAWN_TO_SPEC.get(r[5], set()) for r in drawn)
            print("\t".join([n, "spec", s["net"], s["class"], s["source"][:70], "yes" if ok else "NO"]))

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
