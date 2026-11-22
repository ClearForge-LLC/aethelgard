#!/usr/bin/env python3
"""AET-WO-1218: each emitted board's references, from its .kicad_pcb footprints (the schematic also holds library placeholders), one line per board.
Usage: python3 -I refs.py <emit dir of boards>"""
import os, re, sys
root = sys.argv[1]
for n in sorted(os.listdir(root)):
    refs = []
    for dp, _, fs in os.walk(f"{root}/{n}"):
        for f in fs:
            if f.endswith(".kicad_pcb"):
                refs += re.findall(r'\(property "Reference" "([^"#][^"]*)"', open(f"{dp}/{f}").read())
    print(f"{n}\t{' '.join(sorted(set(refs), key=lambda r: (re.sub(r'\d+$', '', r), int(re.search(r'(\d*)$', r).group(1) or 0), r)))}")
