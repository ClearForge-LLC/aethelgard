"""AET-WO-1216b §2: the walker's paths (walker_dump.rs) against 1216's walk 1
(spike/1216/out/spec), board by board, path for path.

A walk-1 path is a row whose `can_lead` starts with "yes". The key is (net,
class, rails, worst mV or "not stated"); walk 1 calls the foreign clock's
net HSE_IN, the walker OSC_IN (PH0). Walk 1's "no:" rows (a crossing on
mcu.supply) are not paths: the walker does not list a row whose every rail
is mcu.supply, and the count of such rows is printed, not compared.
The pin column is not compared: walk 1 reads solve's pins, the walker the
specification's.

usage: python3 -I compare_walk.py <walk1 dir> <walker dir> <universe tsv> -> report on stdout; exit 1 on a difference
"""
import csv
import os
import sys
from collections import Counter


def walk1(path):
    rows = list(csv.DictReader(open(path), delimiter="\t"))
    yes = Counter((r["net"].replace("HSE_IN", "OSC_IN"), r["class"], r["rail"], r["worst_mv"])
                  for r in rows if r["can_lead"].startswith("yes"))
    return yes, sum(1 for r in rows if r["can_lead"].startswith("no"))


def walker(path):
    rows, verdict, leads = Counter(), "?", Counter()
    for line in open(path):
        f = line.rstrip("\n").split("\t")
        if f[0] == "net":
            continue
        if f[0].startswith("#"):
            verdict = f[0] + " " + f[1]
            continue
        key = (f[0], f[1], f[2], f[4])
        rows[key] += 1
        leads[f[3].split(":")[0]] += 1
    return rows, verdict, leads


def main(a, b, universe):
    bad = 0
    total = 0
    for name, _ in (l.rstrip("\n").split("\t") for l in open(universe) if l.strip()):
        w1, no_rows = walk1(os.path.join(a, name + ".tsv"))
        w2, verdict, leads = walker(os.path.join(b, name + ".tsv"))
        total += sum(w1.values())
        only1, only2 = w1 - w2, w2 - w1
        status = "same" if not only1 and not only2 else "DIFFERS"
        bad |= status != "same"
        print(f"{name}\t{status}\tpaths {sum(w1.values())}/{sum(w2.values())}\twalker leads {dict(leads)}"
              f"\twalk1 non-paths {no_rows}\t{verdict[:160]}")
        for k, n in only1.items():
            print(f"  walk1 only x{n}: {k}")
        for k, n in only2.items():
            print(f"  walker only x{n}: {k}")
    print(f"total walk-1 paths {total}; {'all boards same' if not bad else 'DIFFERENCES'}")
    return bad


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:4]))
