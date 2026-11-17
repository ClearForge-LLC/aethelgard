#!/usr/bin/env python3
"""AET-WO-3400 §5 (adversarial, targeted): does any variant pass because a constraint was dropped?

For each variant, compare against V0 and print every V0 item that is removed or loosened:
net constraints (capacitance, rise time), board (layers, stackup, rules), bus speeds and pull-ups,
MCU fields, acknowledgements and lifecycle acceptances, classes and pairs. Additions are listed
separately. Run from the repository root: python3 -I spike/3400/relaxations.py
"""
import json
import pathlib

V = pathlib.Path("spike/3400/variants")
base = json.loads((V / "v0.json").read_text())


def keyed(items, key):
    return {i[key]: i for i in items}


def compare(name, v):
    relaxed, added, other = [], [], []
    b, n = keyed(base["net_constraints"], "net_name"), keyed(v["net_constraints"], "net_name")
    for k, c in b.items():
        if k not in n:
            relaxed.append(f"net_constraint {k} removed")
            continue
        for f in ("max_capacitance_femtofarads", "max_rise_time_nanoseconds"):
            if n[k][f] > c[f]:
                relaxed.append(f"net_constraint {k}.{f} {c[f]} -> {n[k][f]}")
        if n[k]["impedance_ohms"] != c["impedance_ohms"]:
            other.append(f"net_constraint {k}.impedance_ohms changed")
    added += [f"net_constraint {k}" for k in n if k not in b]
    if v["board"] != base["board"]:
        relaxed.append("board section differs from V0 (layers, stackup or rules)")
    for kind in ("i2c_buses", "spi_buses"):
        bb, nb = keyed(base[kind], "id"), keyed(v[kind], "id")
        for k, x in bb.items():
            if k not in nb:
                relaxed.append(f"{kind} {k} removed")
            elif nb[k] != x:
                relaxed.append(f"{kind} {k} changed: {x} -> {nb[k]}")
        added += [f"{kind} {k}" for k in nb if k not in bb]
    for kind, key in (("i2c_devices", "reference"), ("spi_devices", "reference"), ("serial_ports", "label"),
                      ("connectors", "reference")):
        bb, nb = keyed(base[kind], key), keyed(v[kind], key)
        for k, x in bb.items():
            if k not in nb:
                relaxed.append(f"{kind} {k} removed")
            elif nb[k] != x:
                d = [f for f in x if x[f] != nb[k][f]]
                other.append(f"{kind} {k} changed in {d}")
        added += [f"{kind} {k}" for k in nb if k not in bb]
    for f in base["mcu"]:
        if base["mcu"][f] != v["mcu"][f]:
            other.append(f"mcu.{f}: {base['mcu'][f]} -> {v['mcu'][f]}")
    ba = {(a["register"], a["class"]): set(a["fields"]) for a in base["acknowledge_unwitnessed"]}
    na = {(a["register"], a["class"]): set(a["fields"]) for a in v["acknowledge_unwitnessed"]}
    for k, f in ba.items():
        if k not in na:
            other.append(f"acknowledgement {k} removed")
        elif na[k] != f:
            other.append(f"acknowledgement {k} fields removed: {sorted(f - na[k])}, added: {sorted(na[k] - f)}")
    for k in na:
        if k not in ba or not na[k] <= ba[k]:
            relaxed.append(f"acknowledgement {k} acknowledges a field V0 did not")
    if v["accept_lifecycle"] != base["accept_lifecycle"]:
        relaxed.append("accept_lifecycle differs")
    if base["net_classes"] or base["diff_pairs"]:
        relaxed.append("V0 declares classes or pairs: compare them by hand")
    added += [f"net_class {c['name']} ({c['track_width_um']} um; nets {c['nets']})" for c in v["net_classes"]]
    added += [f"motor_output {m['label']} pin {m['pin']}" for m in v["motor_outputs"]]
    if v["power"]:
        added.append(f"power: regulators {[r['reference'] + ' ' + r['part'] + ' -> ' + r['output'] for r in v['power']['regulators']]}")
    print(f"== {name}")
    print("  RELAXED:", relaxed or "none")
    print("  changed, not relaxed:", other or "none")
    print("  added:", added)


for p in sorted(V.glob("v*.json")):
    if p.name != "v0.json":
        compare(p.stem, json.loads(p.read_text()))
