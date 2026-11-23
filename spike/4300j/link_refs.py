"""AET-WO-4300j: pick each motor pull's link reference per board (D5): the next free R numbers above the highest R
used anywhere in the board's specification (every `reference`, the 4300i pulls included) and in its baseline emit
(BOM .csv and .kicad_sch under <emit>/<name>/, where the board emitted), in motor_outputs order. Keyed on the
universe name (`solve-stress`, not `stress`), as the emit directories are (4300i pull_refs.py keyed on the basename).

usage: python3 -I link_refs.py <repo root> <emit dir> <motors.tsv> > out/link-refs.tsv"""
import json, os, re, sys

root, emit, motors = sys.argv[1:4]
boards = {}
for line in open(motors).read().splitlines()[1:]:
    c = line.split("\t")
    boards.setdefault(c[0], c[1])


def refs_in(o, out):
    if isinstance(o, dict):
        for k, v in o.items():
            if k == "reference" and isinstance(v, str):
                out.add(v)
            refs_in(v, out)
    elif isinstance(o, list):
        for v in o:
            refs_in(v, out)


print("board\tfile\tsource\thighest_R\tresistors\tlinks")
for name, f in boards.items():
    s = json.load(open(os.path.join(root, f)))
    used = set()
    refs_in(s, used)
    src = "spec"
    d = os.path.join(emit, name)
    if os.path.isdir(d):
        src = "spec+emit"
        seen = 0
        for fn in sorted(os.listdir(d)):
            if fn.endswith((".csv", ".kicad_sch")):
                seen += 1
                t = open(os.path.join(d, fn), errors="replace").read()
                used |= set(re.findall(r"\bR\d+\b", t))
        assert seen == 2, (name, seen)
    nums = [int(r[1:]) for r in used if re.fullmatch(r"R\d+", r)]
    hi = max(nums, default=0)
    outs = s["motor_outputs"]
    picks = [f"R{hi + 1 + i}" for i in range(len(outs))]
    assert not {p.upper() for p in picks} & {u.upper() for u in used}, name
    resistors = [m["pull"]["resistor"]["reference"] for m in outs]
    print(f"{name}\t{f}\t{src}\t{hi}\t{','.join(resistors)}\t{','.join(picks)}")
