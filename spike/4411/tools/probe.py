# AET-WO-4411 §1.1 probe boards: S03's 4-layer header, one KiCad RF_Module footprint
# (as KiCad 10.0.7 ships it) placed with its module top edge on the board's top edge,
# GND/VDD planes on In1/In2, and one planted item per variant.
# Usage: probe.py <s03.kicad_pcb> <footprint.kicad_mod> <outdir>
import re, sys, uuid

s03, mod, out = sys.argv[1:4]
s = open(s03).read()
header = s[: s.find("\n\t(footprint")]
f = open(mod).read()
name = re.search(r'\(footprint "([^"]+)"', f).group(1)
c3 = "C3" in name
W, H = 60.0, 50.0
X0 = W / 2
TOP = 13.1 if c3 else 12.75  # F.Fab top edge of the module body, KiCad's footprint
AREA = 6.0 if c3 else 6.19   # antenna area depth, datasheet land pattern (C3 p.37, 32E p.43)
inner = f[f.find("\n"):].rstrip().rstrip(")")
inner = re.sub(r'\n\t\((version|generator|generator_version) [^\n]*\)', "", inner)


def footprint(y0):
    # zone points stand in board coordinates inside a board file (AET-WO-1310 D4)
    def zone(z):
        return re.sub(r'\(xy ([-\d.]+) ([-\d.]+)\)',
                      lambda m: f"(xy {float(m.group(1)) + X0:.4f} {float(m.group(2)) + y0:.4f})", z.group(0))
    body = re.sub(r'\n\t\(zone\b.*?\n\t\)', zone, inner, flags=re.S)
    # the footprint keeps its own (layer "F.Cu"); add only uuid and at
    body = body.replace('(layer "F.Cu")', f'(layer "F.Cu")\n\t\t(uuid "{uuid.uuid4()}")\n\t\t(at {X0} {y0})', 1)
    return f'\t(footprint "RF_Module:{name}"' + body + "\n\t)\n"


def plane(net, layer):
    return (f'\t(zone (net "{net}") (layer "{layer}") (uuid "{uuid.uuid4()}") (hatch edge 0.5) (min_thickness 0.25) '
            f'(filled_areas_thickness no) (fill (thermal_gap 0.5) (thermal_bridge_width 0.5) (island_removal_mode 0)) '
            f'(polygon (pts (xy 0 0) (xy {W} 0) (xy {W} {H}) (xy 0 {H}))))\n')


edge = f'\t(gr_rect (start 0 0) (end {W} {H}) (stroke (width 0.05) (type default)) (fill no) (layer "Edge.Cuts") (uuid "{uuid.uuid4()}"))\n'
planes = plane("GND", "In1.Cu") + plane("VDD_3V3", "In2.Cu")
base = footprint(TOP) + edge + planes
yk = 3.0  # inside the antenna area and the keep-out, on the board
seg = lambda layer: f'\t(segment (start 5 {yk}) (end 55 {yk}) (width 0.25) (layer "{layer}") (net "SIG") (uuid "{uuid.uuid4()}"))\n'
variants = {
    "base": base,
    "track-F": base + seg("F.Cu"),
    "track-In1": base + seg("In1.Cu"),
    "track-B": base + seg("B.Cu"),
    "via": base + f'\t(via (at 30 {yk}) (size 0.6) (drill 0.3) (layers "F.Cu" "B.Cu") (net "SIG") (uuid "{uuid.uuid4()}"))\n',
    "pour-F": base + plane("GND", "F.Cu"),
    # Espressif's recommended position: the antenna area wholly off the board
    "overhang": footprint(TOP - AREA) + edge + planes,
}
tag = "c3" if c3 else "w32e"
for k, v in variants.items():
    open(f"{out}/{tag}-{k}.kicad_pcb", "w").write(header + "\n" + v + ")\n")
    print(f"{out}/{tag}-{k}.kicad_pcb")
