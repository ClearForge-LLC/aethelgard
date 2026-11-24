"""AET-WO-2314: one line from a simulate verdict. python3 -I verdict.py NAME FILE EXIT"""
import json, sys
name, path, status = sys.argv[1:]
v = json.load(open(path))
if "error" in v and "verdict" not in v:
    print(f"ANSWER 2314: {name} exit={status} error={v['error']['kind']}: {v['error']['message'][:300]}")
    sys.exit()
b = v.get("boot") or v.get("foreign_boot") or {}
kinds = [f["kind"] for f in v.get("findings", []) if f["severity"] != "info"]
info = [f["kind"] for f in v.get("findings", []) if f["severity"] == "info"]
print(f"ANSWER 2314: {name} exit={status} verdict={v.get('verdict')} kinds={kinds} info={sorted(set(info))} "
      f"memory={v.get('memory')} pc={b.get('pc')} icsr={b.get('icsr')} booted={b.get('booted')} "
      f"kept={b.get('log_kept_bytes')} dropped={b.get('log_lines_dropped')} panic={(b.get('panic') or {}).get('entered_ns')}")
