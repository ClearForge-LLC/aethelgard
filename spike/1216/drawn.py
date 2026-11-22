"""AET-WO-1216 walk 2: the drawn netlist, read from an emitted .kicad_pcb.

Independent of walk 1 (spec.py): it knows nothing about the specification's
fields. For every MCU pad that is not a supply or ground pad, it lists every
other pad on that net and, through two-terminal parts, the net on their far
side, so a path the specification reader misses shows up here.

usage: python3 -I drawn.py <board.kicad_pcb> <table8-visual.tsv> <mcu.supply net>  -> TSV on stdout
columns: pad pin structure net peer class peer_pad value rails hop2 class off_domain
"""
import re
import sys

def footprints(text):
    out = []
    starts = [m.start() for m in re.finditer(r'\n\t\(footprint "', text)]
    starts.append(len(text))
    for a, b in zip(starts, starts[1:]):
        blk = text[a:b]
        lib = re.search(r'\(footprint "([^"]+)"', blk).group(1)
        ref = re.search(r'\(property "Reference" "([^"]+)"', blk).group(1)
        val = re.search(r'\(property "Value" "([^"]*)"', blk).group(1)
        pads = {}
        for m in re.finditer(r'\(pad "([^"]*)"(.*?)(?=\n\t\t\(pad "|\Z)', blk, re.S):
            n = re.search(r'\(net (?:\d+ )?"([^"]*)"\)', m.group(2))
            pads.setdefault(m.group(1), set()).add(n.group(1) if n else None)
        out.append((ref, lib, val, {k: sorted(x for x in v if x) for k, v in pads.items()}))
    return out

def main(pcb, visual, supply_net):
    pos = {}
    for line in open(visual):
        f = line.rstrip("\n").split("\t")
        pos[f[0]] = (f[1], f[3])  # position -> (name, structure)
    fps = footprints(open(pcb).read())
    mcu = [f for f in fps if "LQFP-144" in f[1]]
    assert len(mcu) == 1, "one LQFP-144 MCU"
    mref = mcu[0][0]
    bynet = {}
    for ref, lib, val, pads in fps:
        for pad, nets in pads.items():
            for n in nets:
                bynet.setdefault(n, []).append((ref, pad))
    parts = {f[0]: f for f in fps}
    supply_names = re.compile(r'^(VDD|VDDA|VDD33USB|VDDLDO|VBAT|VSS|VSSA|VCAP|VREF\+|VREF-|VDDSMPS|VLXSMPS|VFBSMPS|VSSSMPS|VDD50USB|PDR_ON)')
    def rail(n):
        return not n.startswith("/") and not n.startswith("unconnected-")
    def far(ref, pad):
        return sorted({n for p, ns in parts[ref][3].items() if p != pad for n in ns})
    supply = supply_net
    for pad, nets in sorted(mcu[0][3].items(), key=lambda kv: int(kv[0]) if kv[0].isdigit() else 999):
        name, struct = pos.get(pad, ("?", "?"))
        for n in nets:
            if supply_names.match(name) or n == "GND" or n.startswith("unconnected-"):
                continue
            peers = [(r, p) for r, p in bynet.get(n, []) if r != mref]
            for r, p in peers or [("-", "-")]:
                if r == "-":
                    print("\t".join([pad, name, struct, n, "-", "-", "-", "-", "-", "-", "unloaded", "no"]))
                    continue
                kind = re.match(r"[A-Z]+", r).group(0)
                fn = far(r, p)
                two = len(parts[r][3]) == 2
                rails = sorted(x for x in fn if rail(x))
                sigs = sorted(x for x in fn if not rail(x) and not x.startswith("unconnected-"))
                hop = []
                if two:
                    for m in sigs:
                        for r2, p2 in bynet.get(m, []):
                            if r2 not in (r, mref):
                                hop.append(f"{r2}.{p2}->{'|'.join(x for x in far(r2, p2) if rail(x)) or '|'.join(far(r2, p2))}")
                off = [x for x in rails if x not in ("GND", supply)]
                if kind == "J":
                    cls, flag = "connector", "off-board"
                elif kind == "TP":
                    cls, flag = "test_point", "probe"
                elif kind == "U" and not two:
                    cls, flag = "ic", ("yes:" + "|".join(off)) if off else "no"
                elif two:
                    cls = {"R": "resistor", "C": "capacitor", "SW": "switch", "L": "inductor", "D": "diode", "Y": "crystal"}.get(kind, kind)
                    flag = ("yes:" + "|".join(off)) if off else ("hop" if hop else "no")
                else:
                    cls, flag = kind, ("yes:" + "|".join(off)) if off else "no"
                print("\t".join([pad, name, struct, n, r, cls, p, parts[r][2], ",".join(rails) or "-", ";".join(hop) or "-", cls, flag]))

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3])
