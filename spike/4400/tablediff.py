"""Compare ArduPilot's AF table with the ST open pin data the solver uses (spike probe).

usage: python3 -I tablediff.py ARDUPILOT_STM32H753xx.py PACKAGE.xml GPIO_MODES.xml
ArduPilot's module is never imported: its AltFunction_map literal is read with ast.
"""
import ast, re, sys, xml.etree.ElementTree as ET
from collections import Counter

ap_src, pkg_xml, modes_xml = sys.argv[1:4]
tree = ast.parse(open(ap_src).read())
ap = None
for node in tree.body:
    if isinstance(node, ast.Assign) and any(getattr(t, "id", "") == "AltFunction_map" for t in node.targets):
        keys = [ast.literal_eval(k) for k in node.value.keys]
        ap = {}
        dup = [k for k, n in Counter(keys).items() if n > 1]
        for k, v in zip(keys, node.value.values):
            ap[tuple(k.split(":", 1))] = ast.literal_eval(v)
ns = {"m": "http://dummy.com"}
def strip(t):  # drop XML namespaces
    return t.split("}", 1)[-1]
pkg_pins = set()
for el in ET.parse(pkg_xml).getroot().iter():
    if strip(el.tag) == "Pin" and re.fullmatch(r"P[A-K]\d+", el.get("Name", "").split("-")[0].split(" ")[0]):
        pkg_pins.add(el.get("Name").split("-")[0].split(" ")[0])
st = {}
for gp in ET.parse(modes_xml).getroot().iter():
    if strip(gp.tag) != "GPIO_Pin":
        continue
    pin = gp.get("Name", "").split("-")[0].split(" ")[0].split("(")[0]
    for ps in gp:
        if strip(ps.tag) != "PinSignal":
            continue
        for pv in ps.iter():
            if strip(pv.tag) == "PossibleValue" and pv.text and pv.text.startswith("GPIO_AF"):
                m = re.match(r"GPIO_AF(\d+)_", pv.text)
                st[(pin, ps.get("Name"))] = int(m.group(1))
print(f"ArduPilot entries: {len(ap)} (duplicate keys: {len(dup)} {dup[:5]})")
print(f"ST modes entries with an AF: {len(st)}; LQFP144 GPIO pins: {len(pkg_pins)}")
ap_p = {k: v for k, v in ap.items() if k[0] in pkg_pins}
st_p = {k: v for k, v in st.items() if k[0] in pkg_pins}
both = set(ap_p) & set(st_p)
dis = sorted(k for k in both if ap_p[k] != st_p[k])
print(f"on LQFP144 pins: ArduPilot {len(ap_p)}, ST {len(st_p)}, same (pin, signal) name {len(both)}, AF disagree {len(dis)}")
for k in dis:
    print("  AF DISAGREE", f"{k[0]}:{k[1]}", "ArduPilot", ap_p[k], "ST", st_p[k])
only_ap = sorted(set(ap_p) - set(st_p)); only_st = sorted(set(st_p) - set(ap_p))
# a name-only difference: same pin and AF on the other side under another name
st_by = {}
for (p, s), af in st_p.items(): st_by.setdefault((p, af), []).append(s)
renamed = [(k, st_by.get((k[0], ap_p[k]))) for k in only_ap if st_by.get((k[0], ap_p[k]))]
unmatched = [k for k in only_ap if not st_by.get((k[0], ap_p[k]))]
print(f"ArduPilot-only names: {len(only_ap)}, of which {len(renamed)} have an ST signal at the same pin and AF (a naming difference) and {len(unmatched)} have none")
for k in unmatched:
    print("  NO ST SIGNAL at", f"{k[0]} AF{ap_p[k]}", "for ArduPilot", k[1])
for k, alts in renamed[:12]:
    print("  rename", f"{k[0]}:{k[1]} AF{ap_p[k]}", "~ ST", ",".join(alts))
print(f"ST-only (pin, signal) on LQFP144 pins: {len(only_st)}; families:",
      Counter(re.sub(r'\d.*', '', s) for _, s in only_st).most_common(12))
for fam in ("I2C", "SPI", "USART", "UART", "TIM", "OTG", "SDMMC"):
    miss = [f"{p}:{s}" for p, s in only_st if s.startswith(fam)]
    if miss: print(f"  ST-only {fam}: {len(miss)}", miss[:8])

# Strict pass over the families a flight controller uses: normalise ArduPilot's
# names to ST's, then every ArduPilot entry must equal an ST entry exactly.
def norm(s):
    s = re.sub(r"^CAN(\d)_", r"FDCAN\1_", s)
    s = re.sub(r"^OTG_(FS|HS)_", r"USB_OTG_\1_", s)
    return s
fams = ("I2C", "SPI", "USART", "UART", "TIM", "OTG", "SDMMC", "CAN", "QUADSPI", "LPUART")
ev = sum(1 for k in only_ap if k[1] == "EVENT-OUT")
print(f"of the {len(unmatched)} with no ST signal: {ev} EVENT-OUT at AF15 (the modes file lists no EVENTOUT)")
bad = []
for (p, s), af in sorted(ap_p.items()):
    if not s.startswith(fams):
        continue
    n = norm(s)
    if (p, n) in st_p:
        if st_p[(p, n)] != af:
            bad.append(f"{p}:{s} AF{af} vs ST {n} AF{st_p[(p, n)]}")
    else:
        alts = [x for x in [s2 for (p2, s2) in st_p if p2 == p] if x.startswith(n.split("_")[0])]
        bad.append(f"{p}:{s} AF{af}: ST has no {n} on {p} (same peripheral on {p}: {alts or 'none'})")
print(f"flight-controller families: {sum(1 for (p, s) in ap_p if s.startswith(fams))} ArduPilot entries; not equal to an ST entry after normalising: {len(bad)}")
for b in bad:
    print("  ", b)
