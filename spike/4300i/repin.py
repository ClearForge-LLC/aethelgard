import collections
import glob
import json
import os
import subprocess
import sys

ae, root, W, out = sys.argv[1:5]
S3 = os.path.join(root, "crates/aethelgard-cli/tests/fixtures/sentinel-03.json")
orig = json.load(open(S3))
cp = json.loads(json.dumps(orig))
hit = [m for m in cp["motor_outputs"] if m["label"] == "MOTOR1"]
assert len(hit) == 1
hit[0]["pin"] = "PA8"
cpath = os.path.join(W, "repin", "sentinel-03-pa8.json")
json.dump(cp, open(cpath, "w"), indent=2)
# the copy differs from the original only in that value


def jdiff(a, b, p=""):
    if type(a) != type(b):
        return [(p, a, b)]
    if isinstance(a, dict):
        r = []
        for k in sorted(set(a) | set(b)):
            if k not in a:
                r.append((f"{p}/{k}", "<absent>", b[k]))
            elif k not in b:
                r.append((f"{p}/{k}", a[k], "<absent>"))
            else:
                r += jdiff(a[k], b[k], f"{p}/{k}")
        return r
    if isinstance(a, list):
        if len(a) != len(b):
            return [(p + "[len]", len(a), len(b))]
        r = []
        for i, (x, y) in enumerate(zip(a, b)):
            r += jdiff(x, y, f"{p}[{i}]")
        return r
    return [] if a == b else [(p, a, b)]


sd = jdiff(orig, json.load(open(cpath)))
assert sd == [("/motor_outputs[0]/pin", "PE9", "PA8")], sd
assert orig["motor_outputs"][0]["label"] == "MOTOR1"


def run(args):
    p = subprocess.run([ae] + args, capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def kind(o, e):
    for x in (o, e):
        try:
            return json.loads(x).get("error", {}).get("kind", "?")
        except Exception:
            pass
    return "?"


rv, vo, ve = run(["validate", "--file", cpath])
rs, so, se = run(["solve", "--file", cpath])
with open(os.path.join(out, "repin.tsv"), "w") as o:
    o.write("step\trc\tkind\n")
    o.write(f"spec_diff\t-\t{sd}\n")
    o.write(f"validate\t{rv}\t{'-' if rv == 0 else kind(vo, ve)}\n")
    o.write(f"solve\t{rs}\t{'-' if rs == 0 else kind(so, se)}\n")
    if rs != 0:
        o.write(f"solve_message\t-\t{(so + se).strip()[:500]}\n")
base = json.load(open(os.path.join(W, "solve", "sentinel-03.json")))
if rs != 0:
    open(os.path.join(out, "repin-diff.tsv"), "w").write("solve refused; no diff\n")
    sys.exit(0)
new = json.loads(so)
json.dump(new, open(os.path.join(W, "repin", "sentinel-03-pa8.solve.json"), "w"), indent=1, sort_keys=True)
with open(os.path.join(out, "repin-diff.tsv"), "w") as o:
    o.write("section\tnet\tbaseline\tafter\n")
    for sec in ("assignments", "gpio", "adc_inputs"):
        a, b = base.get(sec, {}), new.get(sec, {})
        for k in sorted(set(a) | set(b)):
            if a.get(k) != b.get(k):
                f = lambda v: "<absent>" if v is None else json.dumps(v, sort_keys=True)
                o.write(f"{sec}\t{k}\t{f(a.get(k))}\t{f(b.get(k))}\n")
    o.write("\n# full JSON-level diff: path\tbaseline\tafter\n")
    for p, x, y in jdiff(base, new):
        o.write(f"{p}\t{json.dumps(x, sort_keys=True)}\t{json.dumps(y, sort_keys=True)}\n")

# reserved pins
RES = {"PA11", "PA12", "PC9"}


def pins_in(o, path=""):
    r = []
    if isinstance(o, dict):
        for k, v in o.items():
            if isinstance(v, str) and v in RES:
                r.append((f"{path}/{k}", v))
            r += pins_in(v, f"{path}/{k}")
    elif isinstance(o, list):
        for i, v in enumerate(o):
            if isinstance(v, str) and v in RES:
                r.append((f"{path}[{i}]", v))
            r += pins_in(v, f"{path}[{i}]")
    return r


def hs(spec, net):
    # hand if a declaration of the net carries the pin
    for m in spec.get("motor_outputs", []):
        if m["label"] == net:
            return "hand" if m.get("pin") else "solved"
    return None


with open(os.path.join(out, "reserved.tsv"), "w") as o:
    o.write("run\tjson_path\tpin\tnet\tspec_pin_hand_or_solved\n")
    for tag, t in (("baseline", base), ("pa8_copy", new)):
        raw = json.dumps(t)
        o.write(f"# {tag}: {len(pins_in(t))} value hits; raw-text hits of PA11/PA12/PC9: {sum(raw.count(x) for x in RES)}\n")
    for tag, t, sp in (("baseline", base, orig), ("pa8_copy", new, cp)):
        for path, pin in pins_in(t):
            net = path.split("/")[2] if path.count("/") >= 2 and path.split("/")[1] in ("assignments", "gpio", "adc_inputs") else "-"
            # hand/solved: any occurrence of this pin string as a "pin" value in the spec
            hand = []

            def find(x, p=""):
                if isinstance(x, dict):
                    if x.get("pin") == pin:
                        hand.append(p)
                    for k, v in x.items():
                        find(v, f"{p}/{k}")
                elif isinstance(x, list):
                    for i, v in enumerate(x):
                        find(v, f"{p}[{i}]")
            find(sp)
            o.write(f"{tag}\t{path}\t{pin}\t{net}\t{'hand ' + ';'.join(hand) if hand else 'solved'}\n")
    o.write("\n# other boards whose solve output carries PA11/PA12/PC9 (informational)\n")
    for sj in sorted(glob.glob(os.path.join(W, "solve", "*.json"))):
        n = os.path.basename(sj)[:-5]
        if n == "sentinel-03":
            continue
        for path, pin in pins_in(json.load(open(sj))):
            o.write(f"other:{n}\t{path}\t{pin}\t-\t-\n")
