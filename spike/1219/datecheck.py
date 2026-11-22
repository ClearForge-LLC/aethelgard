"""AET-WO-1219 adversarial check of the per-version count: a key dated
version N (from versions::CHANGES) must not appear in a committed
specification of a version below N, and every key dated 1 must appear in a
committed version-1 file. Version 0 is excluded: it never existed
(spike/1215/in/v0.json is a current-shape file stamped 0, on purpose)."""
import json, pathlib, subprocess, sys
repo = pathlib.Path(sys.argv[1])
inv = json.loads((repo / "spike/1219/out/inventory.json").read_text())
def paths(v, p=""):
    out = set()
    if isinstance(v, dict):
        for k, x in v.items():
            q = (p + "." if p else "") + k; out.add(q); out |= paths(x, q)
    elif isinstance(v, list):
        for x in v: out |= paths(x, p + "[]")
    return out
files = []
for f in subprocess.run(["git", "-C", str(repo), "ls-files", "*.json"], capture_output=True, text=True).stdout.split():
    try: v = json.loads((repo / f).read_text())
    except Exception: continue
    if isinstance(v, dict) and isinstance(v.get("mcu"), dict) and isinstance(v.get("schema_version"), int) and v["schema_version"] >= 1:
        files.append((v["schema_version"], f, paths(v)))
bad = [(e["id"], e["version"], f, ver) for e in inv["K"] for ver, f, ps in files if ver < e["version"] and any(p in ps for p in e["paths"])]
v1 = set().union(*[ps for ver, _, ps in files if ver == 1])
undated = [e["id"] for e in inv["K"] if e["version"] == 1 and not any(p in v1 for p in e["paths"])]
for b in bad: print("EARLIER", *b)
for u in undated: print("V1 KEY IN NO V1 FILE", u)
print("%d keys, %d versioned files (versions %s); earlier than dated: %d; v1 keys in no v1 file: %d" % (len(inv["K"]), len(files), sorted({v for v, _, _ in files}), len(bad), len(undated)))
sys.exit(1 if bad or undated else 0)
