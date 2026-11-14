#!/usr/bin/env python3
"""S1: turn the emitted 2-layer reference board into 4- and 6-layer probe boards.

Usage: variants.py <reference.kicad_pcb> <out-dir>. Text edits only, so every variant differs
from the generator's bytes exactly where named here.
  b4        layers F, In1, In2, B; GND plane on In1.Cu, VDD_3V3 plane on In2.Cu (zones unfilled,
            outline = board); the B.Cu GND zone kept. Vias stay (layers "F.Cu" "B.Cu"): through.
  b6        layers F, In1..In4, B; GND on In1, VDD_3V3 on In4; In2/In3 signal, empty.
  b6_seed   b6 + two In2.Cu tracks of /I2C_SDA and /I2C_SCL 0.1 mm apart (clearance 0.2)
            + an In1.Cu track of /I2C_SDA ending on a /FLASH_CS via's centre (a short across a plane layer).
  b4_seedvia b4 + an In2.Cu track of /I2C_SCL ending on a /FLASH_CS via (In2 is the VDD plane's layer).
  b2_undecl 2-layer table unchanged + one In1.Cu track: copper on an undeclared layer.
"""
import re, sys, uuid, os

src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
text = open(src).read()

def u(tag):
    return str(uuid.uuid5(uuid.NAMESPACE_URL, "aet-4100-s1/" + tag))

def layers(inner):
    rows = "".join(f'\t\t({4 + 2 * i} "In{i + 1}.Cu" signal)\n' for i in range(inner))
    return text.replace('\t\t(2 "B.Cu" signal)\n', rows + '\t\t(2 "B.Cu" signal)\n', 1)

zone = re.search(r'\t\(zone\n.*?\n\t\)\n', text, re.S).group(0)
assert '(layer "B.Cu")' in zone and '(net "GND")' in zone

def plane(t, net, layer):
    z = zone.replace('(net "GND")', f'(net "{net}")').replace('(layer "B.Cu")', f'(layer "{layer}")')
    z = re.sub(r'\(uuid "[^"]+"\)', f'(uuid "{u(net + layer)}")', z)
    i = t.rindex("\t(embedded_fonts")
    return t[:i] + z + t[i:]

def seg(t, net, layer, a, b, tag):
    s = (f'\t(segment\n\t\t(start {a[0]} {a[1]})\n\t\t(end {b[0]} {b[1]})\n\t\t(width 0.2)\n'
         f'\t\t(layer "{layer}")\n\t\t(net "{net}")\n\t\t(uuid "{u(tag)}")\n\t)\n')
    i = t.rindex("\t(zone")
    return t[:i] + s + t[i:]

via = re.search(r'\t\(via\n\t\t\(at ([\d.]+) ([\d.]+)\)\n.*?\(net "/FLASH_CS"\)', text, re.S)
vx, vy = float(via.group(1)), float(via.group(2))

def write(name, t):
    open(f"{out}/{name}.kicad_pcb", "w").write(t)

b4 = plane(plane(layers(2), "GND", "In1.Cu"), "VDD_3V3", "In2.Cu")
write("b4", b4)
b6 = plane(plane(layers(4), "GND", "In1.Cu"), "VDD_3V3", "In4.Cu")
write("b6", b6)
# Seeds away from the parts: the board's outline is 72.2..117.4 x 76.15..116.55.
t = seg(b6, "/I2C_SDA", "In2.Cu", (75, 80), (80, 80), "sda-in2")
t = seg(t, "/I2C_SCL", "In2.Cu", (75, 80.3), (80, 80.3), "scl-in2")  # 0.3 c-c, 0.1 edge gap
t = seg(t, "/I2C_SDA", "In1.Cu", (vx - 2, vy), (vx, vy), "sda-in1-via")
write("b6_seed", t)
write("b4_seedvia", seg(b4, "/I2C_SCL", "In2.Cu", (vx, vy - 2), (vx, vy), "scl-in2-via"))
write("b2_undecl", seg(text, "/I2C_SDA", "In1.Cu", (75, 80), (80, 80), "undeclared"))
print(f"via /FLASH_CS at {vx} {vy}")

# Round 2. b4_short: an In1.Cu track of /I2C_SDA from an /I2C_SDA via to a /FLASH_CS via, across
# the GND plane's layer: copper joining two nets on an inner layer (shorting_items expected).
# b4_stackup: b4 with a (setup (stackup …)) whose dielectrics are absurd (0.01 mm, er 50), to see
# whether raw DRC or the Gerbers read it.
# Each via block on its own (a lazy match across blocks once paired the wrong via).
vias = re.findall(r'\t\(via\n\t\t\(at ([\d.]+) ([\d.]+)\)\n(?:\t\t[^\n]*\n)*?\t\t\(net "([^"]+)"\)', text)
sx, sy = next((float(x), float(y)) for x, y, n in vias if n == "/I2C_SDA")
assert (vx, vy) == next((float(x), float(y)) for x, y, n in vias if n == "/FLASH_CS")
write("b4_short", seg(b4, "/I2C_SDA", "In1.Cu", (sx, sy), (vx, vy), "sda-in1-short"))
stack = ('\t(setup\n\t\t(stackup\n'
         '\t\t\t(layer "F.Cu" (type "copper") (thickness 0.035))\n'
         '\t\t\t(layer "dielectric 1" (type "prepreg") (thickness 0.01) (material "FR4") (epsilon_r 50) (loss_tangent 0.02))\n'
         '\t\t\t(layer "In1.Cu" (type "copper") (thickness 0.035))\n'
         '\t\t\t(layer "dielectric 2" (type "core") (thickness 0.01) (material "FR4") (epsilon_r 50) (loss_tangent 0.02))\n'
         '\t\t\t(layer "In2.Cu" (type "copper") (thickness 0.035))\n'
         '\t\t\t(layer "dielectric 3" (type "prepreg") (thickness 0.01) (material "FR4") (epsilon_r 50) (loss_tangent 0.02))\n'
         '\t\t\t(layer "B.Cu" (type "copper") (thickness 0.035))\n'
         '\t\t)\n\t\t(pad_to_mask_clearance 0)\n\t)\n')
i = b4.index("\t(footprint")
write("b4_stackup", b4[:i] + stack + b4[i:])
print(f"via /I2C_SDA at {sx} {sy}")
