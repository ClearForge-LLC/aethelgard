"""AET-WO-4300h walk 2: from every connector pad and test-point pad of an
emitted .kicad_pcb, inward to the MCU's pads.

Independent of walk 1 (facing.py): it knows nothing of the specification. It
reads the drawn netlist and DS12117 Table 8 as transcribed from the rendered
pages (`spike/1211e/out/table8-visual.tsv`, pad -> name, structure).

usage: python3 -I inward.py <board.kicad_pcb> <table8-visual.tsv> -> TSV on stdout

The pass-through rule (RULES.md, committed before the first run):
- an off-board end is any footprint whose reference is `J<n>` or `TP<n>`;
- a two-pad part whose reference is R, L, FB or D passes a DC path to its
  other pad's net (up to 3 such hops);
- every other part ends the walk and is listed by name: a capacitor (an AC
  path only), an IC (a translator, a driver, a regulator), a switch, a crystal;
  an IC that also has a pad on a net the MCU's I/O pads carry is flagged
  `ic_bridge`, so a translator or buffer path is visible, not dropped;
- a hop lands only on a signal net (a `/` name): a pull to a rail is not a
  series path, so the walk never continues into a rail from a signal net;
- an end on a rail (a global name) is walked one way, through two-pad parts
  into signal nets, and every MCU pad it reaches is `via_rail`: a domain's
  pull (A1 judges it), or, from `VBAT`, the pack through the sense divider
  (off-board, `1216b` judges it; compare.py reconciles it with walk 1's
  `power_input`); the MCU's supply pads are not
  I/O and are not listed, and an IC on a rail is its supply, not a bridge.
Every MCU I/O pad reached is one row; an end that reaches none is one row too.
"""
import re
import sys

PASS = ("R", "L", "FB", "D")
SUPPLY = re.compile(r'^(VDD|VDDA|VDD33USB|VDDLDO|VBAT|VSS|VSSA|VCAP|VREF\+|VREF-|VDDSMPS|VLXSMPS|VFBSMPS|VSSSMPS|VDD50USB|PDR_ON)')
HEADER = ["end", "end_net", "hops", "mcu_pad", "pin", "structure", "class", "detail"]


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
            if n:
                pads.setdefault(m.group(1), set()).add(n.group(1))
        out.append((ref, lib, val, {k: sorted(v) for k, v in pads.items()}))
    return out


def prefix(ref):
    return re.match(r"[A-Z]+", ref).group(0)


def main(pcb, visual):
    pos = {}
    for line in open(visual):
        f = line.rstrip("\n").split("\t")
        pos[f[0]] = (f[1], f[3])
    fps = footprints(open(pcb).read())
    mcu = [f for f in fps if "LQFP-144" in f[1]]
    assert len(mcu) == 1, "one LQFP-144 MCU"
    mref = mcu[0][0]
    parts = {f[0]: f for f in fps}
    bynet = {}
    for ref, _, _, pads in fps:
        for pad, nets in pads.items():
            for n in nets:
                bynet.setdefault(n, []).append((ref, pad))
    mcu_io_nets = {n for pad, nets in mcu[0][3].items() for n in nets
                   if not SUPPLY.match(pos.get(pad, ("?",))[0]) and n != "GND"}
    rows = []
    for ref, _, _, pads in sorted(fps, key=lambda f: f[0]):
        if prefix(ref) not in ("J", "TP"):
            continue
        for pad, nets in sorted(pads.items()):
            for net in nets:
                end = f"{ref}.{pad}"
                if net == "GND" or net.startswith("unconnected-"):
                    rows.append([end, net, "-", "-", "-", "-", "none", net])
                    continue
                found = 0
                seen_nets, seen_parts = {net}, {ref}
                frontier = [(net, [])]
                for _ in range(4):  # the end's own net, then up to 3 hops
                    nxt = []
                    for n, chain in frontier:
                        for r, p in bynet.get(n, []):
                            if r == mref:
                                name, struct = pos.get(p, ("?", "?"))
                                if SUPPLY.match(name):
                                    continue
                                found += 1
                                cls = ("direct" if not chain else "series") if net.startswith("/") else "via_rail"
                                rows.append([end, net, " ".join(chain) or "-", p, name, struct, cls, n])
                                continue
                            if r in seen_parts:
                                continue
                            seen_parts.add(r)
                            fp = parts[r]
                            k = prefix(r)
                            if len(fp[3]) == 2 and k in PASS:
                                for p2, ns in fp[3].items():
                                    if p2 == p:
                                        continue
                                    for m in ns:
                                        if m not in seen_nets and m.startswith("/"):
                                            seen_nets.add(m)
                                            nxt.append((m, chain + [f"{n}-{r}({fp[2]})-{m}"]))
                            else:
                                bridge = sorted({m for p2, ns in fp[3].items() if p2 != p for m in ns} & mcu_io_nets)
                                if bridge and n.startswith("/") and k not in ("J", "TP"):
                                    rows.append([end, net, " ".join(chain) or "-", "-", "-", "-", "ic_bridge",
                                                 f"{r} {fp[2]} on {n}; its other pads carry MCU nets {','.join(bridge)}"])
                    frontier = nxt
                if not found:
                    rows.append([end, net, "-", "-", "-", "-", "no_mcu_pin", net])
    print("\t".join(HEADER))
    for r in rows:
        print("\t".join(r))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
