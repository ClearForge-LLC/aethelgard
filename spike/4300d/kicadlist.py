"""Lists each candidate symbol's pins and default footprint, and each footprint's pad kinds and sha256,
from the libraries kicadlib.sh copied out of the pinned image. Usage: python3 -I kicadlist.py > kicadlib.out"""
import hashlib, os, re
K = os.path.expanduser("~/.cache/aet-4300d/kicad")
SYMS = [("Regulator_Switching", ["LMR33630ADDA", "TPS62933", "LMR51430", "AP63205WU", "AP63203WU"]),
        ("Regulator_Linear", ["TLV75518PDBV", "TLV75533PDBV", "AP2112K-1.8"]),
        ("Logic_LevelTranslator", ["TXU0304PW", "TXB0104PW"])]
FPS = ["Package_SO.pretty/Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm_ThermalVias", "Package_SO.pretty/Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm",
       "Package_TO_SOT_SMD.pretty/SOT-583-8", "Package_TO_SOT_SMD.pretty/SOT-23-6", "Package_TO_SOT_SMD.pretty/TSOT-23-6",
       "Package_TO_SOT_SMD.pretty/SOT-23-5", "Package_SO.pretty/TSSOP-14_4.4x5mm_P0.65mm",
       "Connector_AMASS.pretty/AMASS_XT30PW-M_1x02_P2.50mm_Horizontal", "Connector_AMASS.pretty/AMASS_XT60PW-M_1x02_P7.20mm_Horizontal"]
for lib, names in SYMS:
    path = f"{K}/symbols/{lib}.kicad_sym"
    t = open(path).read()
    print(f"== {lib}.kicad_sym sha256 {hashlib.sha256(t.encode()).hexdigest()}")
    for n in names:
        i = t.find(f'(symbol "{n}"')
        if i < 0:
            print(f"  {n}: ABSENT"); continue
        blk = t[i:i + 3000]
        ext = re.search(r'\(extends "([^"]+)"\)', blk)
        fp = re.search(r'\(property "Footprint" "([^"]*)"', blk)
        base = ext.group(1) if ext else n
        j = t.find(f'(symbol "{base}"'); b = t[j:]; k = b.find('\n\t(symbol "', 10); b = b[:k if k > 0 else len(b)]
        pins = sorted(set((num, nm, ty) for ty, nm, num in re.findall(r'\(pin (\w+) \w+.*?\(name "([^"]*)".*?\(number "([^"]*)"', b, re.S)),
                      key=lambda x: (len(x[0]), x[0]))
        print(f"  {n}: footprint {fp.group(1) if fp else None}; pins " + ", ".join(f"{a} {b_}:{c}" for a, b_, c in pins))
print("== footprints (pad name/kind/shape counts)")
for f in FPS:
    p = f"{K}/footprints/{f}.kicad_mod"
    t = open(p).read()
    pads = re.findall(r'\(pad "([^"]*)" ([a-z_]+) ([a-z]+)', t)
    kinds = {}
    for name, kind, shape in pads:
        kinds[f"{kind}/{shape}"] = kinds.get(f"{kind}/{shape}", 0) + 1
    print(f"  {f}: sha256 {hashlib.sha256(t.encode()).hexdigest()[:16]} pads {len(pads)} {kinds}")
