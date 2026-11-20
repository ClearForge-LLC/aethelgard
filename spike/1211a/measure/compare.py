#!/usr/bin/env python3
"""AET-WO-1211a §1.1: judge run.sh's two outputs, `main`'s on the committed files (BASE) and the
branch's on the migrated ones (AFTER), against PREDICTIONS.md M1-M4. A whitelist: exactly the
differences M2 predicts are taken out of `solve`'s output (the version, the hash, `gpio`'s shape,
`adc_inputs`, each PWM output's new `pwm` keys), and everything else must be equal.
- M1: every command's exit code is the same; a refusal keeps its kind (and its message, but for a
  version number or a hash).
- M2: `solve`'s output equal under the whitelist.
- M3: every emitted file byte-identical.
- M4: every generated file identical but for lines naming the specification's sha256 (old or new).
Usage: python3 -I compare.py BASE AFTER > result.json"""
import collections, json, os, re, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from topology_v14 import migrate  # noqa: E402  (the rule, written once)

base, after = sys.argv[1], sys.argv[2]
O = collections.OrderedDict
HEX = re.compile(r"[0-9a-f]{64}")
out = O(files=0, moved=[], hash_lines=O(), notes=[])


def load(p):
    return json.loads(open(p).read(), object_pairs_hook=O)


def text(p):
    return open(p, "rb").read()


def refusal(t):
    try:
        e = json.loads(t)["error"]
    except (ValueError, KeyError, TypeError):
        return None
    # A version number or a hash in a message is the bump's, never a word.
    return e.get("kind"), re.sub(r"\b(1[0-9]|2[0-9])\b", "N", HEX.sub("H", e.get("message", "")))


for spec in open(f"{base}/specs.txt").read().split():
    d = spec.replace("/", "__")
    b, a = f"{base}/{d}", f"{after}/{d}"
    out["files"] += 1
    for cmd in ("validate", "solve", "emit", "generate"):
        eb, ea = text(f"{b}/{cmd}.exit").strip(), text(f"{a}/{cmd}.exit").strip()
        if eb != ea:
            out["moved"].append(O(file=spec, cmd=cmd, what="exit", base=eb.decode(), after=ea.decode()))
            continue
        ob, oa = text(f"{b}/{cmd}.out").decode(), text(f"{a}/{cmd}.out").decode()
        if eb != b"0":
            if refusal(ob) != refusal(oa):
                out["moved"].append(O(file=spec, cmd=cmd, what="refusal", base=refusal(ob), after=refusal(oa)))
            continue
        if cmd == "solve":
            tb, ta = migrate(load(f"{b}/solve.out")), load(f"{a}/solve.out")
            tb["schema_version"] = ta["schema_version"]
            tb["specification_sha256"] = ta["specification_sha256"]
            if tb != ta:
                keys = [k for k in set(tb) | set(ta) if tb.get(k) != ta.get(k)]
                out["moved"].append(O(file=spec, cmd=cmd, what="words", keys=sorted(keys)))
        elif cmd in ("emit", "generate"):
            sha_b, sha_a = json.loads(ob).get("specification_sha256"), json.loads(oa).get("specification_sha256")
            root_b, root_a = f"{b}/{cmd}", f"{a}/{cmd}"
            names = set()
            for root in (root_b, root_a):
                for dp, _, fs in os.walk(root):
                    names |= {os.path.relpath(os.path.join(dp, f), root) for f in fs}
            for n in sorted(names):
                pb, pa = os.path.join(root_b, n), os.path.join(root_a, n)
                if not (os.path.exists(pb) and os.path.exists(pa)):
                    out["moved"].append(O(file=spec, cmd=cmd, what="file set", name=n))
                    continue
                xb, xa = text(pb), text(pa)
                if xb == xa:
                    continue
                lb, la = xb.split(b"\n"), xa.split(b"\n")
                diff = [i for i in range(max(len(lb), len(la)))
                        if (lb[i] if i < len(lb) else None) != (la[i] if i < len(la) else None)]
                by_hash = len(lb) == len(la) and all(
                    lb[i].replace(sha_b.encode(), sha_a.encode()) == la[i] for i in diff)
                if cmd == "generate" and by_hash:
                    out["hash_lines"].setdefault(spec, O())[n] = len(diff)
                else:
                    out["moved"].append(O(file=spec, cmd=cmd, what="bytes", name=n, lines=len(diff)))
json.dump(out, sys.stdout, indent=1)
print()
