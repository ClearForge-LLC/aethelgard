import json, os, sys
root = sys.argv[1]
def walk(o, path=""):
    if isinstance(o, dict):
        for k, v in o.items():
            yield from walk(v, f"{path}.{k}")
    elif isinstance(o, list):
        for i, v in enumerate(o):
            yield from walk(v, f"{path}[{i}]")
    else:
        yield path, o
for d in sorted(os.listdir(root)):
    p = os.path.join(root, d, "solve.out")
    if not os.path.isfile(p):
        continue
    try:
        t = json.load(open(p))
    except ValueError:
        print(d, "solve.out not JSON"); continue
    if not t.get("ok", True) and "topology" not in t:
        print(f"{d}\tsolve refused\t{t.get('error',{}).get('kind')}")
        continue
    hits = [(k, v) for k, v in walk(t) if isinstance(v, str) and v in ("PC2", "PC3", "PC2_C", "PC3_C")]
    keys = [k for k, _ in walk(t) if any(x in k for x in (".PC2.", ".PC3.", ".PC2_C", ".PC3_C"))]
    print(f"{d}\t{len(hits)} value hits\t{hits}\t{keys[:4]}")
