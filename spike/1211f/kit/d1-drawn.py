#!/usr/bin/env python3
"""AET-WO-1211f: D1 drawn, the gate specification. It is 1211a's committed D1
fixture with `board.drawn` removed, exactly as `pin_devices_drawn_tier1.rs`'s
`d1_drawn()` builds it. Usage: d1-drawn.py <repo> <out.json>"""
import json, sys
repo, out = sys.argv[1], sys.argv[2]
s = json.load(open(f"{repo}/crates/aethelgard-ir/tests/fixtures/d1-drv8876.json"))
del s["board"]["drawn"]
json.dump(s, open(out, "w"), indent=2)
open(out, "a").write("\n")
