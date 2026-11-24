"""AET-WO-2314: one line from bmp.sh's simulate verdict. python3 -I bmpline.py FILE I EXIT WALL"""
import json, sys
path, i, status, w = sys.argv[1:]
v = json.load(open(path))
b = v.get("boot") or {}
print(f"ANSWER 2314: bmp581 undisturbed {i} exit={status} wall_s={w} verdict={v.get('verdict')} "
      f"boot_word_ns={b.get('boot_word_ns')} findings={[(f['kind'], f['severity']) for f in v.get('findings', [])]} "
      f"memory={v.get('memory')}")
