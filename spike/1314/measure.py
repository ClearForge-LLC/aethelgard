"""AET-WO-1314 measured stop: one specification through emit and extract.
Usage: measure.py <aethelgard binary> <spec.json> <out dir> <label>
Prints one JSON line: per bounded net its bound, terms and via count; via
counts by (net, size, drill); the supply gate's verdict when gated."""
import json, re, subprocess, sys, glob, os, collections
exe, spec, out, label = sys.argv[1:5]
os.makedirs(out, exist_ok=True)
r = subprocess.run([exe, "emit", "--file", spec, "--out", f"{out}/{label}"], capture_output=True, text=True)
if r.returncode != 0:
    print(json.dumps({"label": label, "emit": r.returncode, "error": r.stdout[:400]})); sys.exit()
board = glob.glob(f"{out}/{label}/*.kicad_pcb")[0]
text = open(board).read()
vias = collections.Counter()
for m in re.finditer(r'\(via\b(.*?)\n\t\)', text, re.S):
    body = m.group(1)
    size = re.search(r'\(size ([\d.]+)\)', body); drill = re.search(r'\(drill ([\d.]+)\)', body)
    net = re.search(r'\(net (?:\d+ )?"([^"]*)"\)', body)
    vias[(net.group(1) if net else "?", size.group(1) if size else "?", drill.group(1) if drill else "?")] += 1
x = subprocess.run([exe, "extract", "--board", board, "--spec", spec], capture_output=True, text=True)
v = json.loads(x.stdout) if x.stdout.strip() else {"error": x.stderr[-300:]}
lines = {l["net"]: {"c": l.get("c_bound_femtofarads"), "max": l.get("declared_femtofarads"), "terms": l.get("terms"), "verdict": l.get("verdict")} for l in v.get("lines", [])}
res = {"label": label, "extract_exit": x.returncode, "verdict": v.get("verdict"),
       "findings": sorted(set(f["kind"] for f in v.get("findings", []))),
       "lines": lines, "vias": {f"{k[0]}|{k[1]}/{k[2]}": n for k, n in sorted(vias.items())},
       "via_total": sum(vias.values())}
s = v.get("supplies")
if s:
    s = s[0]
    def side(k):
        b = [p["r_bound_microohms"] for p in s[k] if "r_bound_microohms" in p]
        return {"bounded": len(b), "of": len(s[k]), "max_uohm": max(b) if b else None,
                "refused": sorted(set(p["refused"]["kind"] for p in s[k] if "refused" in p))}
    res["supply"] = {"pins": side("pins"), "returns": side("returns"), "drop_uV": s.get("drop_bound_microvolts")}
print(json.dumps(res))
