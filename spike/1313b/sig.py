#!/usr/bin/env python3
"""sig.py <dump.json>: a name-free signature of a placement (AET-WO-1313b): the outline and the
sorted (footprint, origin, rotation) of every part, hashed. Two placements with the same signature
put the same footprints at the same places, whatever the parts are called."""
import hashlib
import json
import sys

d = json.load(open(sys.argv[1]))
rows = sorted((p["fp"], p["at"][0], p["at"][1], p["rot"]) for p in d["parts"])
print(hashlib.sha256(json.dumps([d["outline"], rows]).encode()).hexdigest()[:16])
