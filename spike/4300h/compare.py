"""AET-WO-4300h: reconcile walk 1 (out/facing) with walk 2 (out/inward) on every
drawn board, both ways, and the three reads of Table 8's structure.

usage: python3 -I compare.py <spike/4300h dir> <power-parts.json> <table8-text.tsv> <table8-visual.tsv>
       -> out/compare.tsv (one row per (board, end, pin) either walk lists, with its verdict),
          out/table8.tsv (one row per pin either walk lists: the three structures), and a
          count line per verdict on stdout
"""
import json
import os
import sys


def tsv(path):
    lines = [l.rstrip("\n").split("\t") for l in open(path)]
    return [dict(zip(lines[0], l)) for l in lines[1:]]


def main(here, pp_path, text_path, visual_path):
    boards = tsv(os.path.join(here, "out/boards.tsv"))
    compare = [["board", "end", "pin", "walk1", "walk2", "verdict"]]
    pins_seen = set()
    counts = {}
    for b in boards:
        n = b["name"]
        w1 = {}
        for r in tsv(os.path.join(here, "out/facing", n + ".tsv")):
            if r["class"] in ("connector", "test_point", "power_input"):
                w1[(r["end"], r["pin"])] = r
                pins_seen.add(r["pin"])
        if b["emit"] != "0":
            for (end, pin), r in sorted(w1.items()):
                compare.append([n, end, pin, r["class"] + " " + r["structure"], "-", "not drawn"])
                counts["not drawn"] = counts.get("not drawn", 0) + 1
            continue
        w2 = {}
        for r in tsv(os.path.join(here, "out/inward", n + ".tsv")):
            # a VBAT end reaches its pins through the divider: walk 1's `power_input`, keyed by reference
            vbat = r["class"] == "via_rail" and r["end_net"] == "VBAT"
            if r["class"] in ("direct", "series", "ic_bridge") or vbat:
                end = r["end"][:-2] if r["end"].startswith("TP") and r["end"].endswith(".1") else r["end"]
                end = end.split(".")[0] if vbat else end
                w2.setdefault((end, r["pin"]), []).append(r)
                if r["pin"] != "-":
                    pins_seen.add(r["pin"])
        for k in sorted(set(w1) | set(w2)):
            a, z = w1.get(k), w2.get(k, [])
            zc = ",".join(sorted({r["class"] + " " + r["structure"] for r in z})) or "-"
            if a and z and all(r["class"] != "ic_bridge" for r in z):
                v = ("agree" if all(r["class"] == "direct" for r in z) else "agree (through a series part)") \
                    if all(r["structure"] == a["structure"] for r in z) else "structure differs"
            elif z and all(r["class"] == "ic_bridge" for r in z):
                v = "ic_bridge: walk 2 only, read by hand"
            elif a and not z:
                v = "walk 1 only"
            else:
                v = "walk 2 only"
            compare.append([n, k[0], k[1], (a["class"] + " " + a["structure"]) if a else "-", zc, v])
            counts[v] = counts.get(v, 0) + 1
    with open(os.path.join(here, "out/compare.tsv"), "w") as f:
        for r in compare:
            f.write("\t".join(r) + "\n")

    io = {}
    for p in json.load(open(pp_path))["parts"]:
        m = p.get("kind", {}).get("mcu")
        if m:
            io = m["io"]["pins"]
    text = {l.split("\t")[0]: l.split("\t")[3] for l in open(text_path)}
    vis = {l.split("\t")[1]: l.split("\t")[3] for l in open(visual_path)}
    t8 = [["pin", "power-parts", "table8-text", "table8-visual", "verdict"]]
    for pin in sorted(p for p in pins_seen if p not in ("-", "unsolved")):
        a = io.get(pin, {}).get("structure", "-" if pin in ("NRST", "BOOT0") else "unheld")
        b2, c = text.get(pin, "-"), vis.get(pin, "-")
        held = [x for x in (a, b2, c) if x != "-"]
        t8.append([pin, a, b2, c, "agree" if len(set(held)) == 1 else "DIFFER"])
    with open(os.path.join(here, "out/table8.tsv"), "w") as f:
        for r in t8:
            f.write("\t".join(r) + "\n")
    for k, v in sorted(counts.items()):
        print(f"{v}\t{k}")
    print(f"{sum(1 for r in t8[1:] if r[4] == 'agree')}/{len(t8) - 1}\ttable8 pins agree")


if __name__ == "__main__":
    main(*sys.argv[1:5])
