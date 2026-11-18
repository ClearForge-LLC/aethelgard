"""AET-WO-1303b measured stop: per pin, the certified bound at each pitch, the
uncertified two-pitch extrapolation (reference only), crop sensitivity, and
the VDD + VSS drop. Reads the *.out files beside it."""
import re, math, glob, os
D = os.path.dirname(os.path.abspath(__file__))
def read(name):
    p = f"{D}/{name}.out"
    if not os.path.exists(p): return None
    pins, vias, refused = {}, {}, {}
    for line in open(p):
        m = re.match(r"\s*U1\.\s*(\d+):\s+([\d.]+) mOhm.*vias carrying >= 1 %: (.*)", line)
        if m:
            pins[int(m.group(1))] = float(m.group(2))
            vias[int(m.group(1))] = len(re.findall(r"\(", m.group(3)))
        m = re.match(r"\s*U1\.\s*(\d+): REFUSED (.*)", line)
        if m: refused[int(m.group(1))] = m.group(2)
    return pins, vias, refused
LOAD_MA, LIMIT_MV = 658, 165
rows = {}
for b, label in [("s02", "sentinel-02-power (2 layers)"), ("s024l", "s024l (power fixture, 4 layers)"), ("s03", "sentinel-03")]:
    for net in ["vdd", "gnd"]:
        c = read(f"{b}-{net}-62500"); f = read(f"{b}-{net}-31250"); ff = read(f"{b}-{net}-15625")
        if not c: continue
        print(f"== {label} {net.upper()}")
        print("  pin   62.5um   31.25um" + ("  15.625um" if ff else "") + "  vias carrying >= 1 %")
        for pin in sorted(c[0]):
            line = f"  {pin:4d} {c[0][pin]:9.3f} {f[0][pin]:9.3f}"
            if ff: line += f" {ff[0][pin]:9.3f}"
            print(line + f"   {f[1][pin]}")
        fine = (ff or f)[0]
        print(f"  bounded {len(fine)} of {len(c[0]) + len(c[2])}; refused {c[2] or f[2] or 'none'}; finest pitch {min(fine.values()):.1f}..{max(fine.values()):.1f} mOhm")
        rows[(b, net)] = max(f[0].values())
        crop = read(f"{b}-{net}-62500-crop15")
        if crop:
            d = [crop[0][p] / c[0][p] - 1 for p in c[0]]
            print(f"  crop 15 mm against 5 mm at 62.5um: change {min(d)*100:+.2f}%..{max(d)*100:+.2f}%")
print()
print("== reference (UNCERTIFIED). 4 layers: the worst pin alone (one crop at both pitches) at 31.25 and")
print("   15.625 um, where a 0.2 mm track keeps 0.1875 mm of whole cells at both, extrapolated to first order.")
print("   It removes the plane's discretisation only: not the track-width loss, the junction or the ranges.")
refs = {}
for tag in ["s024l-vdd-17", "s024l-gnd-31", "s03-vdd-17", "s03-gnd-31"]:
    a = read(f"ref-{tag}-31250")[0]; bb = read(f"ref-{tag}-15625")[0]
    pin = next(iter(a)); r1, r2 = a[pin], bb[pin]
    refs[tag] = 2 * r2 - r1
    print(f"  {tag}: {r1:.3f} -> {r2:.3f} mOhm ({(r2/r1-1)*100:+.2f}%); reference {2*r2-r1:.3f}")
print("   2 layers: every VDD_3V3 segment is axis-aligned on the 125 um grid, so a 0.2 mm track keeps")
print("   0.1875 mm at both fine pitches; reference = bound x 0.9375 [inferred: track runs only].")
print()
print("== drop at the load: (worst VDD pin + worst GND pin) x load; load the MCU's 658 mA alone (the net's total is the fixtures' work)")
for b in ["s024l", "s03"]:
    v, g = rows[(b, "vdd")], rows[(b, "gnd")]
    r = v + g
    print(f"  {b} bound at 31.25 um (all pins, one crop): {v:.1f} + {g:.1f} = {r:.1f} mOhm: {r*LOAD_MA/1000:.1f} mV at {LOAD_MA} mA; {LIMIT_MV} mV holds to {LIMIT_MV/r*1000:.0f} mA")
    rr = refs[f"{b}-vdd-17"] + refs[f"{b}-gnd-31"]
    one = sum(next(iter(read(f"ref-{b}-{t}-31250")[0].values())) for t in ("vdd-17", "gnd-31"))
    print(f"  {b} the same pins each with its own crop (less plane): bound {one:.1f} mOhm, reference {rr:.1f} mOhm ({(rr/one-1)*100:+.1f}%): {rr*LOAD_MA/1000:.1f} mV at {LOAD_MA} mA")
v = read("s02-vdd-15625")[0]
lo, hi = min(v.values()), max(v.values())
print(f"  s02 VDD alone (GND refused by name, GroundReturnUnread): bound {lo:.1f}..{hi:.1f} mOhm, {lo*LOAD_MA/1000:.0f}..{hi*LOAD_MA/1000:.0f} mV at {LOAD_MA} mA; reference x0.9375: {lo*0.9375:.1f}..{hi*0.9375:.1f} mOhm, {lo*0.9375*LOAD_MA/1000:.0f}..{hi*0.9375*LOAD_MA/1000:.0f} mV")
