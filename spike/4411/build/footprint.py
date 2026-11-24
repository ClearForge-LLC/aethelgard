# AET-WO-4411 D4 (ruled): our own footprint for the ESP32-C3-WROOM-02, derived
# from KiCad 10.0.7's RF_Module:ESP32-C3-WROOM-02 (sha256
# 2eb3ceb2324d141108b2f5a46480e74117f3f2181b36f0a5dd9791efb847816b). Every
# change is listed in PROVENANCE below and written into the footprint's descr.
# Usage: python3 -I footprint.py <kicad ESP32-C3-WROOM-02.kicad_mod> <out.kicad_mod>
import re
import sys

SRC_SHA = "2eb3ceb2324d141108b2f5a46480e74117f3f2181b36f0a5dd9791efb847816b"
PROVENANCE = (
    "ESP32-C3-WROOM-02, derived for Aethelgard (AET-WO-4411 D4) from KiCad 10.0.7 "
    "RF_Module:ESP32-C3-WROOM-02 (kicad-footprints tag 10.0.7, sha256 " + SRC_SHA + "), "
    "CC-BY-SA-4.0 with KiCad's design exception. Changes, each with its reason: "
    "(1) the keepout zone: KiCad's one zone on all 32 copper layers, 5 mm around the antenna, is "
    "replaced by one zone per copper layer (F.Cu and B.Cu here; the emitter adds one per inner layer), "
    "each the antenna area only, 18 x 6 mm (datasheet v1.7 p. 37, Fig. 11-1): the gate refuses a "
    "zone naming a layer the board lacks, and the 15 mm clearance beyond the antenna "
    "(Hardware Design Guidelines Fig. 21) is judged by the gate from the part row, never from a zone; "
    "(2) the ground pad's 12 vias at 0.2 mm drill are removed: the engine emits the pad's thermal vias "
    "itself at its own 0.6 mm pad and 0.3 mm drill (ruled D4), 2 x 2 at the crossings of the 3 x 3 land "
    "pattern's gaps (p. 37, Fig. 11-1, 1.1 mm pitch), as the radio parts row declares; 12 at 0.3 mm on "
    "KiCad's 0.55 mm pitch would leave 0.25 mm between holes; "
    "(3) the courtyard: KiCad's outline around its 5 mm zone becomes the rectangle around the "
    "module's body, pads and antenna area, so the keepout lies inside it; "
    "(4) the silkscreen beside and above the antenna area, and the pin 1 triangle crossing it, are "
    "removed: the antenna area overhangs the board edge (D2), where silkscreen meets the edge; "
    "(5) KiCad's keep-out dimension drawings and texts on Cmts.User are removed with its zone."
)


def items(s):
    """Top-level items of the footprint, as (start, end) spans."""
    out, depth, start = [], 0, None
    for k, c in enumerate(s):
        if c == "(":
            depth += 1
            if depth == 2:
                start = k
        elif c == ")":
            if depth == 2:
                out.append((start, k + 1))
            depth -= 1
    return out


def keep(item):
    head = item[1:].split(None, 1)[0]
    layer = re.search(r'\(layer "([^"]+)"\)', item)
    layer = layer.group(1) if layer else ""
    if head == "zone":
        return False
    if head == "descr":
        return False
    if head == "pad" and "thru_hole" in item.split("\n")[0]:
        return False
    if layer == "Cmts.User" and not (head == "fp_text" and '"Antenna"' in item):
        return False
    if layer == "F.CrtYd":
        return False
    if layer == "F.SilkS" and head in ("fp_line", "fp_poly"):
        ys = [float(y) for y in re.findall(r"\((?:start|end|xy) [-\d.]+ ([-\d.]+)\)", item)]
        return min(ys) > -7.1
    return True


def main():
    src, out = sys.argv[1:3]
    s = open(src).read()
    spans = items(s)
    body = [s[a:b] for a, b in spans]
    kept = [i for i in body if keep(i)]
    crtyd = [
        ((-9.75, -13.35), (9.75, -13.35)),
        ((9.75, -13.35), (9.75, 7.15)),
        ((9.75, 7.15), (-9.75, 7.15)),
        ((-9.75, 7.15), (-9.75, -13.35)),
    ]
    added = []
    for n, ((x0, y0), (x1, y1)) in enumerate(crtyd):
        added.append(
            f'(fp_line\n\t\t(start {x0} {y0})\n\t\t(end {x1} {y1})\n\t\t(stroke\n\t\t\t(width 0.05)\n\t\t\t(type solid)\n\t\t)\n'
            f'\t\t(layer "F.CrtYd")\n\t\t(uuid "4411c0a7-0000-4000-8000-00000000000{n}")\n\t)'
        )
    for n, layer in enumerate(["F.Cu", "B.Cu"]):
        added.append(
            f'(zone\n\t\t(layer "{layer}")\n\t\t(uuid "4411c0a7-0000-4000-8000-00000000002{n}")\n'
            f'\t\t(hatch full 0.508)\n\t\t(connect_pads\n\t\t\t(clearance 0)\n\t\t)\n\t\t(min_thickness 0.254)\n'
            f'\t\t(keepout\n\t\t\t(tracks not_allowed)\n\t\t\t(vias not_allowed)\n\t\t\t(pads not_allowed)\n'
            f'\t\t\t(copperpour not_allowed)\n\t\t\t(footprints not_allowed)\n\t\t)\n'
            f'\t\t(placement\n\t\t\t(enabled no)\n\t\t\t(sheetname "")\n\t\t)\n'
            f'\t\t(fill\n\t\t\t(thermal_gap 0.508)\n\t\t\t(thermal_bridge_width 0.508)\n\t\t\t(island_removal_mode 0)\n\t\t)\n'
            f'\t\t(polygon\n\t\t\t(pts\n\t\t\t\t(xy -9 -13.1) (xy 9 -13.1) (xy 9 -7.1) (xy -9 -7.1)\n\t\t\t)\n\t\t)\n\t)'
        )
    descr = '(descr "' + PROVENANCE.replace('"', "'") + '")'
    # descr right after the layer, as KiCad writes it; the rest in KiCad's order,
    # the new items before the 3D model.
    head, rest = kept[:4], kept[4:]
    model = [i for i in rest if i.startswith("(model")]
    rest = [i for i in rest if not i.startswith("(model") and not i.startswith("(embedded_fonts")]
    fonts = [i for i in kept if i.startswith("(embedded_fonts")]
    order = head + [descr] + rest + added + fonts + model
    text = '(footprint "ESP32-C3-WROOM-02"\n' + "".join("\t" + i + "\n" for i in order) + ")\n"
    open(out, "w").write(text)


main()
