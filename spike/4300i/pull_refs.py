"""AET-WO-4300i: pick motor-pull references per board: the next free R numbers, one block above the highest R used
in the baseline emit (BOM + schematic) and the specification's declared references, in motor_outputs order."""
import json, os, re, sys
root, base = sys.argv[1], sys.argv[2]
files = [l.split()[0] for l in open(sys.argv[3]) if l.split()[3] not in ("0", "-")]
def refs_in(o, out):
    if isinstance(o, dict):
        for k, v in o.items():
            if k == "reference" and isinstance(v, str):
                out.add(v)
            refs_in(v, out)
    elif isinstance(o, list):
        for v in o:
            refs_in(v, out)
print("board\tfile\tsource\thighest_R\treferences")
for f in files:
    s = json.load(open(os.path.join(root, f)))
    name = os.path.basename(f)[:-5]
    used = set()
    refs_in(s, used)
    src = "spec"
    d = os.path.join(base, name)
    if os.path.isdir(d):
        src = "spec+emit"
        for fn in os.listdir(d):
            if fn.endswith((".csv", ".kicad_sch")):
                t = open(os.path.join(d, fn), errors="replace").read()
                used |= set(re.findall(r"\bR\d+\b", t))
    nums = [int(r[1:]) for r in used if re.fullmatch(r"R\d+", r)]
    hi = max(nums, default=0)
    n = len(s["motor_outputs"])
    picks = [f"R{hi + 1 + i}" for i in range(n)]
    assert not set(picks) & used
    print(f"{name}\t{f}\t{src}\t{hi}\t{','.join(picks)}")
