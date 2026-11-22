"""AET-WO-1216 §1.4–1.5: the three schema options, as executable rules over
walk 1's rows (out/spec/<board>.tsv) and the specification itself.

Written and committed before it ran (RULES.md says the same in prose).

The limit at VDD = 0 (DS12117 Rev 10 Table 10, p. 103): BOOT0 9.0 V; every
other pin 4.0 V (FT_xxx: Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V) at 0;
TT_xx 4.0 V; any other pin 4.0 V). Table 11 note 3 (p. 104): positive
injection "is not possible on these I/Os and does not occur for input voltages
lower than the specified maximum value", so the voltage is the whole test.

A path is a walk-1 row whose `can_lead` starts with "yes": its source can be up
while mcu.supply is below 1 V on what the specification declares.

Options, each with its variants:
- (c)  VDD = 0 for every off-domain path.
       c1: refuse a stated worst above the limit; a path with no stated
           bound is left unjudged.
       c2: as c1, and refuse every path with no stated bound
           (OffDomainUnbounded), INV-06's reading.
- (b)  the order from the tree's EN/PG wiring alone. A non-supply domain is
       ordered after mcu.supply only if it is downstream of it (its input chain
       passes through mcu.supply) or its regulator's EN is wired to a PG of a
       regulator in mcu.supply's chain. Otherwise it is undeclared (no tree:
       SourceUndeclared) or racing (a sibling: RailsRace). The battery leads by
       definition, and its paths are judged as c1.
       b1: refuse every undeclared or racing domain.
       b2: refuse one only when it reaches an MCU pin (a row whose rail it is,
           or a connector it powers).
- (a)  a declared `power.sequence`. No committed specification has one.
       a1: required of every board with a domain other than mcu.supply or a tree.
       a2: required only of a board with a path from a non-supply rail (a
           domain, the battery, or a connector a domain powers).
       a-sidecar: sequences/<board>.json, declared by hand for the four boards
       checked by hand; each step's paths, when the step is not after
       mcu.supply's, judged at VDD = 0 (c1's test), and a step with an empty
       source refused (SequenceStepUnsourced).

- lean: FEEDBACK §4's rule (added after the first run, at the adversarial
       pass's close, so the lean's count is run and not inferred from c1's).

usage: python3 -I judge.py <out/spec dir> <universe.sh output> <sequences dir> -> TSV on stdout
"""
import csv
import json
import os
import sys

LIMIT = {"B": 9000}


def limit(structure):
    return LIMIT.get(structure, 4000)


def stated(r):
    return r["worst_mv"].isdigit()


def paths(rows):
    return [r for r in rows if r["can_lead"].startswith("yes")]


def c1(rows):
    return [("OffDomainAboveLimit", f"{r['net']}@{r['pin']} {r['worst_mv']} mV > {limit(r['structure'])} mV")
            for r in paths(rows) if stated(r) and int(r["worst_mv"]) > limit(r["structure"])]


def c2(rows):
    return c1(rows) + [("OffDomainUnbounded", f"{r['net']}@{r['pin']} {r['class']}")
                       for r in paths(rows) if not stated(r)]


def domains_status(spec):
    supply = spec["mcu"]["supply"]
    power = spec.get("power")
    src = {r["output"]: r for r in (power or {}).get("regulators", [])}

    def chain(d):
        out = [d]
        while d in src:
            d = src[d]["input"]
            out.append(d)
        return out
    pg_nets_on_supply_chain = set()
    for d in chain(supply):
        r = src.get(d)
        if r:
            for k in r.get("controls", []):
                if k["pin"] == "PG" and k.get("wire"):
                    pg_nets_on_supply_chain.add(k["wire"]["net"])
    out = {}
    for d in spec.get("voltage_domains", []):
        dom = d["id"]
        if dom == supply:
            continue
        if not power or dom not in src:
            out[dom] = "SourceUndeclared"
        elif supply in chain(dom)[1:]:
            out[dom] = "after"
        elif any(k["pin"] == "EN" and k.get("wire") and k["wire"]["net"] in pg_nets_on_supply_chain
                 for k in src[dom].get("controls", [])):
            out[dom] = "after"
        else:
            out[dom] = "RailsRace"
    return out


def reaches(rows, dom):
    return [r for r in paths(rows) if r["rail"] == dom or dom in r["powered_by"].split(",")]


def b(spec, rows, variant):
    out = []
    for dom, st in domains_status(spec).items():
        if st == "after":
            continue
        hit = reaches(rows, dom)
        if variant == 1 or hit:
            out.append((st, f"{dom}" + (f" reaches {','.join(sorted({r['net'] for r in hit}))}" if hit else "")))
    out += [k for k in c1([r for r in rows if r["rail"] == "battery"])]
    return out


def lean(spec, rows):
    """FEEDBACK §4: c1, with (b)'s derivation as the only exemption: a path whose
    rail domains_status places after mcu.supply is not judged at VDD = 0."""
    after = {d for d, st in domains_status(spec).items() if st == "after"}
    return c1([r for r in rows if r["rail"] not in after])


def a(spec, rows, variant, seq):
    if seq is not None:
        return a_sidecar(spec, rows, seq)
    if spec.get("power", {}) and spec["power"].get("sequence"):
        raise SystemExit("a committed specification declares a sequence: the rule needs reading")
    supply = spec["mcu"]["supply"]
    others = [d["id"] for d in spec.get("voltage_domains", []) if d["id"] != supply]
    if variant == 1:
        need = bool(others) or bool(spec.get("power"))
    else:
        need = any(r["rail"] not in ("off-board",) or r["powered_by"] != "-" for r in paths(rows))
    return [("SequenceUndeclared", ",".join(others + (["battery"] if spec.get("power") else [])))] if need else []


def a_sidecar(spec, rows, seq):
    supply = spec["mcu"]["supply"]
    out = []
    up = []
    out += [("SequenceStepUnsourced", f"steps[{i}]") for i, step in enumerate(seq["steps"])
            if not step.get("source", "").strip()]
    for step in seq["steps"]:
        if supply in step["rails"]:
            # rails in the MCU's own step may lead it within the step: judge them too
            led = [x for x in step["rails"] if x != supply]
            up += led
            break
        up += step["rails"]
    led = set(up)
    for r in paths(rows):
        rails = {r["rail"]} | set(x for x in r["powered_by"].split(",") if x != "-")
        if rails & led and stated(r) and int(r["worst_mv"]) > limit(r["structure"]):
            out.append(("OffDomainAboveLimit", f"{r['net']}@{r['pin']} {r['worst_mv']} mV > {limit(r['structure'])} mV"
                        f" (led by {','.join(sorted(rails & led))})"))
    return out


def main(spec_dir, boards, seq_dir):
    root = os.popen("git rev-parse --show-toplevel").read().strip()
    print("\t".join(["board", "option", "refused", "refusals"]))
    for n, f in (l.rstrip("\n").split("\t") for l in open(boards) if l.strip()):
        spec = json.load(open(os.path.join(root, f)))
        rows = list(csv.DictReader(open(os.path.join(spec_dir, n + ".tsv")), delimiter="\t"))
        seqf = os.path.join(seq_dir, n + ".json")
        res = {"c1": c1(rows), "c2": c2(rows), "b1": b(spec, rows, 1), "b2": b(spec, rows, 2),
               "a1": a(spec, rows, 1, None), "a2": a(spec, rows, 2, None), "lean": lean(spec, rows)}
        if os.path.exists(seqf):
            for name, seq in json.load(open(seqf)).items():
                res["a-sidecar:" + name] = a(spec, rows, 0, seq)
        for k, v in res.items():
            print("\t".join([n, k, "yes" if v else "no", "; ".join(f"{x}: {y}" for x, y in v) or "-"]))


if __name__ == "__main__":
    main(*sys.argv[1:4])
